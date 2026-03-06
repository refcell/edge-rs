//! Function selector dispatch table generation.
//!
//! Generates the EVM bytecode that reads the first 4 bytes of calldata
//! (the function selector) and jumps to the matching function body.

use edge_ir::{schema::EvmContract, var_opt};

use crate::{assembler::Assembler, expr_compiler::ExprCompiler};

/// Generate the function dispatcher for a contract.
///
/// The dispatcher compiles the runtime IR which contains the full
/// selector-checking if-else chain with inlined function bodies.
/// Each branch loads the selector from calldata, compares it, and
/// executes the matching function body (which terminates with RETURN/STOP).
pub fn generate_dispatcher(asm: &mut Assembler, contract: &EvmContract) {
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
    // Start LetBind slots after IR-allocated memory regions (arrays, structs)
    let mut compiler =
        ExprCompiler::with_allocations_and_base(asm, allocations, contract.memory_high_water);
    // Collect fn_info from both runtime and internal functions
    compiler.collect_fn_info(&contract.runtime);
    for func in &contract.internal_functions {
        compiler.collect_fn_info(func);
    }
    compiler.compile_expr(&contract.runtime);
    // Compile internal function subroutines
    for func in &contract.internal_functions {
        compiler.compile_expr(func);
    }
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
