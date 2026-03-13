//! Function selector dispatch table generation.
//!
//! Generates the EVM bytecode that reads the first 4 bytes of calldata
//! (the function selector) and jumps to the matching function body.

use edge_ir::{schema::EvmContract, var_opt};

use crate::{assembler::Assembler, expr_compiler::ExprCompiler};

/// Recursively check if an IR tree contains any `DynAlloc` nodes.
fn contains_dyn_alloc(expr: &edge_ir::schema::RcExpr) -> bool {
    let mut visited = std::collections::HashSet::new();
    contains_dyn_alloc_inner(expr, &mut visited)
}

fn contains_dyn_alloc_inner(
    expr: &edge_ir::schema::RcExpr,
    visited: &mut std::collections::HashSet<usize>,
) -> bool {
    if !visited.insert(std::rc::Rc::as_ptr(expr) as usize) {
        return false;
    }
    use edge_ir::schema::EvmExpr;
    match expr.as_ref() {
        EvmExpr::DynAlloc(_) | EvmExpr::AllocRegion(_, _, true) => true,
        EvmExpr::Bop(_, a, b)
        | EvmExpr::Concat(a, b)
        | EvmExpr::DoWhile(a, b)
        | EvmExpr::LetBind(_, a, b)
        | EvmExpr::EnvRead1(_, a, b)
        | EvmExpr::RegionStore(_, _, a, b) => {
            contains_dyn_alloc_inner(a, visited) || contains_dyn_alloc_inner(b, visited)
        }
        EvmExpr::Uop(_, a)
        | EvmExpr::VarStore(_, a)
        | EvmExpr::EnvRead(_, a)
        | EvmExpr::Function(_, _, _, a)
        | EvmExpr::Get(a, _)
        | EvmExpr::RegionLoad(_, _, a) => contains_dyn_alloc_inner(a, visited),
        EvmExpr::Top(_, a, b, c)
        | EvmExpr::If(_, a, b, c)
        | EvmExpr::Revert(a, b, c)
        | EvmExpr::ReturnOp(a, b, c) => {
            contains_dyn_alloc_inner(a, visited)
                || contains_dyn_alloc_inner(b, visited)
                || contains_dyn_alloc_inner(c, visited)
        }
        EvmExpr::Log(_, topics, offset, size, state) => {
            topics.iter().any(|t| contains_dyn_alloc_inner(t, visited))
                || contains_dyn_alloc_inner(offset, visited)
                || contains_dyn_alloc_inner(size, visited)
                || contains_dyn_alloc_inner(state, visited)
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            contains_dyn_alloc_inner(a, visited)
                || contains_dyn_alloc_inner(b, visited)
                || contains_dyn_alloc_inner(c, visited)
                || contains_dyn_alloc_inner(d, visited)
                || contains_dyn_alloc_inner(e, visited)
                || contains_dyn_alloc_inner(f, visited)
                || contains_dyn_alloc_inner(g, visited)
        }
        EvmExpr::Call(_, args) => args.iter().any(|a| contains_dyn_alloc_inner(a, visited)),
        EvmExpr::InlineAsm(inputs, _, _) => {
            inputs.iter().any(|i| contains_dyn_alloc_inner(i, visited))
        }
        EvmExpr::AllocRegion(_, nf, false) => contains_dyn_alloc_inner(nf, visited),
        EvmExpr::Const(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Arg(_, _)
        | EvmExpr::Empty(_, _)
        | EvmExpr::StorageField(_, _, _)
        | EvmExpr::MemRegion(_, _)
        | EvmExpr::Selector(_) => false,
    }
}

/// Generate the function dispatcher for a contract.
///
/// The dispatcher compiles the runtime IR which contains the full
/// selector-checking if-else chain with inlined function bodies.
/// Each branch loads the selector from calldata, compares it, and
/// executes the matching function body (which terminates with RETURN/STOP).
pub fn generate_dispatcher(asm: &mut Assembler, contract: &EvmContract) {
    let t = std::time::Instant::now();
    // Analyze variable allocations to decide stack vs memory
    let mut allocations = var_opt::analyze_allocations(&contract.runtime);
    // Also analyze internal function bodies
    for func in &contract.internal_functions {
        let func_allocs = var_opt::analyze_allocations(func);
        // Merge conservatively: Memory beats Stack for same-named vars
        for (name, alloc) in func_allocs {
            allocations
                .entry(name)
                .and_modify(|existing| {
                    if alloc.mode == var_opt::AllocationMode::Memory {
                        existing.mode = var_opt::AllocationMode::Memory;
                    }
                    existing.read_count = existing.read_count.max(alloc.read_count);
                })
                .or_insert(alloc);
        }
    }
    tracing::debug!("      analyze_allocations: {:?}", t.elapsed());

    let t = std::time::Instant::now();
    // Compute the DynAlloc floor: the minimum address DynAlloc may return.
    // Without this, DynAlloc (which uses MSIZE) could return pointers that
    // overlap with LetBind slots whose MSTORE hasn't happened yet.
    //
    // We simulate the codegen's LetBind allocation to find the peak offset.
    // This mirrors compile_if (doesn't restore next_let_offset across branches)
    // and Function (does restore), giving the exact peak rather than a loose bound.
    let has_dyn_alloc = contains_dyn_alloc(&contract.runtime)
        || contract.internal_functions.iter().any(contains_dyn_alloc);
    let dyn_alloc_floor = if has_dyn_alloc {
        let mut all_exprs: Vec<&edge_ir::schema::RcExpr> = vec![&contract.runtime];
        for func in &contract.internal_functions {
            all_exprs.push(func);
        }
        ExprCompiler::compute_peak_let_offset(&allocations, contract.memory_high_water, &all_exprs)
    } else {
        0
    };
    tracing::debug!("      dyn_alloc_floor: {:?}", t.elapsed());

    // Start LetBind slots after IR-allocated memory regions (arrays, structs)
    let mut compiler = ExprCompiler::with_allocations_base_and_floor(
        asm,
        allocations,
        contract.memory_high_water,
        dyn_alloc_floor,
    );
    let t = std::time::Instant::now();
    // Collect fn_info from both runtime and internal functions
    compiler.collect_fn_info(&contract.runtime);
    for func in &contract.internal_functions {
        compiler.collect_fn_info(func);
    }
    tracing::debug!("      collect_fn_info: {:?}", t.elapsed());

    let t = std::time::Instant::now();
    compiler.compile_expr(&contract.runtime);
    tracing::debug!("      compile_expr(runtime): {:?}", t.elapsed());
    // Compile internal function subroutines
    let t = std::time::Instant::now();
    for func in &contract.internal_functions {
        compiler.compile_expr(func);
    }
    tracing::debug!("      compile_expr(fns): {:?}", t.elapsed());
    compiler.emit_overflow_revert_trampoline();
}

/// Compute the 4-byte function selector from a function signature.
///
/// `sig` should be in the form "functionName(type1,type2,...)"
pub fn compute_selector(sig: &str) -> [u8; 4] {
    let mut hash = [0u8; 32];
    edge_types::bytes::hash_bytes(&mut hash, sig);
    [hash[0], hash[1], hash[2], hash[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_selectors() {
        // Well-known ERC-20 selectors
        assert_eq!(
            compute_selector("transfer(address,uint256)"),
            [0xa9, 0x05, 0x9c, 0xbb]
        );
        assert_eq!(
            compute_selector("balanceOf(address)"),
            [0x70, 0xa0, 0x82, 0x31]
        );
        assert_eq!(
            compute_selector("approve(address,uint256)"),
            [0x09, 0x5e, 0xa7, 0xb3]
        );
        assert_eq!(compute_selector("totalSupply()"), [0x18, 0x16, 0x0d, 0xdd]);
    }

    #[test]
    fn test_simple_selectors() {
        // Counter contract selectors
        let inc = compute_selector("increment()");
        let dec = compute_selector("decrement()");
        let get = compute_selector("get()");
        let reset = compute_selector("reset()");

        // Just verify they're all different
        assert_ne!(inc, dec);
        assert_ne!(inc, get);
        assert_ne!(inc, reset);
        assert_ne!(dec, get);
        assert_ne!(dec, reset);
        assert_ne!(get, reset);
    }
}
