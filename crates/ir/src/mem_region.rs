//! Post-egglog pass: assign concrete memory offsets to symbolic `MemRegion` nodes.
//!
//! After egglog extraction, the IR contains `MemRegion(id, size_words)` nodes
//! representing symbolic memory allocations. This pass:
//!
//! 1. Collects all `MemRegion` nodes from the IR tree
//! 2. Assigns concrete byte offsets via a bump allocator (starting at 0)
//! 3. Replaces each `MemRegion(id, size)` with `Const(SmallInt(offset))`
//! 4. Returns the total memory used (memory high water mark)
//!
//! The assigned offsets become the `memory_high_water` value that codegen uses
//! to start `LetBind` variable slots above all IR-allocated regions.

use std::{collections::BTreeMap, rc::Rc};

use crate::schema::{EvmBaseType, EvmConstant, EvmContext, EvmExpr, EvmType, RcExpr};

/// Assign concrete memory offsets to all `MemRegion` nodes in an expression.
///
/// Returns `(rewritten_expr, memory_high_water)` where `memory_high_water` is
/// the first free byte offset after all allocated regions.
pub fn assign_memory_offsets(expr: &RcExpr) -> (RcExpr, usize) {
    let mut regions = BTreeMap::new();
    collect_regions(expr, &mut regions);

    if regions.is_empty() {
        return (Rc::clone(expr), 0);
    }

    // Assign concrete offsets via bump allocator
    let mut offset = 0usize;
    let mut assignments = BTreeMap::new();
    for (id, size_words) in &regions {
        assignments.insert(*id, offset);
        offset += (*size_words as usize) * 32;
    }

    let rewritten = replace_regions(expr, &assignments);
    (rewritten, offset)
}

/// Assign memory offsets for an entire program.
///
/// Updates `memory_high_water` on each contract.
pub fn assign_program_offsets(program: &mut crate::schema::EvmProgram) {
    for contract in &mut program.contracts {
        let (new_runtime, hw) = assign_memory_offsets(&contract.runtime);
        contract.runtime = new_runtime;

        // Also process internal functions
        let mut max_hw = hw;
        for func in &mut contract.internal_functions {
            let (new_func, func_hw) = assign_memory_offsets(func);
            *func = new_func;
            max_hw = max_hw.max(func_hw);
        }

        // Also process constructor
        let (new_ctor, ctor_hw) = assign_memory_offsets(&contract.constructor);
        contract.constructor = new_ctor;
        max_hw = max_hw.max(ctor_hw);

        contract.memory_high_water = max_hw;
    }
}

/// Collect all `MemRegion(id, size_words)` from an expression tree.
fn collect_regions(expr: &RcExpr, regions: &mut BTreeMap<i64, i64>) {
    match expr.as_ref() {
        EvmExpr::MemRegion(id, size) => {
            // If the same region ID appears multiple times (shared via Rc),
            // verify the size is consistent.
            regions
                .entry(*id)
                .and_modify(|existing_size| {
                    debug_assert_eq!(
                        *existing_size, *size,
                        "MemRegion {id} has inconsistent sizes: {existing_size} vs {size}"
                    );
                })
                .or_insert(*size);
        }
        // Recurse into all children
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) | EvmExpr::DoWhile(a, b) => {
            collect_regions(a, regions);
            collect_regions(b, regions);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) | EvmExpr::VarStore(_, a) => {
            collect_regions(a, regions);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_regions(a, regions);
            collect_regions(b, regions);
            collect_regions(c, regions);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_regions(c, regions);
            collect_regions(i, regions);
            collect_regions(t, regions);
            collect_regions(e, regions);
        }
        EvmExpr::LetBind(_, init, body) => {
            collect_regions(init, regions);
            collect_regions(body, regions);
        }
        EvmExpr::Log(_, topics, d, s, st) => {
            for t in topics {
                collect_regions(t, regions);
            }
            collect_regions(d, regions);
            collect_regions(s, regions);
            collect_regions(st, regions);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            for x in [a, b, c, d, e, f, g] {
                collect_regions(x, regions);
            }
        }
        EvmExpr::Call(_, args) => {
            for a in args {
                collect_regions(a, regions);
            }
        }
        EvmExpr::Function(_, _, _, body) => collect_regions(body, regions),
        EvmExpr::EnvRead(_, s) => collect_regions(s, regions),
        EvmExpr::EnvRead1(_, a, s) => {
            collect_regions(a, regions);
            collect_regions(s, regions);
        }
        EvmExpr::InlineAsm(inputs, ..) => {
            for inp in inputs {
                collect_regions(inp, regions);
            }
        }
        // Leaf nodes — no children
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => {}
    }
}

/// Replace all `MemRegion(id, _)` with `Const(SmallInt(offset))`.
fn replace_regions(expr: &RcExpr, assignments: &BTreeMap<i64, usize>) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::MemRegion(id, _size) => {
            let offset = assignments[id];
            Rc::new(EvmExpr::Const(
                EvmConstant::SmallInt(offset as i64),
                EvmType::Base(EvmBaseType::UIntT(256)),
                EvmContext::InFunction("__mem__".to_owned()),
            ))
        }
        // Recurse into all children (same structure as collect_regions)
        EvmExpr::Bop(op, a, b) => {
            let na = replace_regions(a, assignments);
            let nb = replace_regions(b, assignments);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, na, nb))
        }
        EvmExpr::Uop(op, a) => {
            let na = replace_regions(a, assignments);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, na))
        }
        EvmExpr::Top(op, a, b, c) => {
            let na = replace_regions(a, assignments);
            let nb = replace_regions(b, assignments);
            let nc = replace_regions(c, assignments);
            Rc::new(EvmExpr::Top(*op, na, nb, nc))
        }
        EvmExpr::Concat(a, b) => {
            let na = replace_regions(a, assignments);
            let nb = replace_regions(b, assignments);
            Rc::new(EvmExpr::Concat(na, nb))
        }
        EvmExpr::Get(a, idx) => {
            let na = replace_regions(a, assignments);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(na, *idx))
        }
        EvmExpr::If(c, i, t, e) => {
            let nc = replace_regions(c, assignments);
            let ni = replace_regions(i, assignments);
            let nt = replace_regions(t, assignments);
            let ne = replace_regions(e, assignments);
            Rc::new(EvmExpr::If(nc, ni, nt, ne))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let ni = replace_regions(inputs, assignments);
            let nb = replace_regions(body, assignments);
            Rc::new(EvmExpr::DoWhile(ni, nb))
        }
        EvmExpr::LetBind(name, init, body) => {
            let ni = replace_regions(init, assignments);
            let nb = replace_regions(body, assignments);
            Rc::new(EvmExpr::LetBind(name.clone(), ni, nb))
        }
        EvmExpr::VarStore(name, val) => {
            let nv = replace_regions(val, assignments);
            Rc::new(EvmExpr::VarStore(name.clone(), nv))
        }
        EvmExpr::Revert(a, b, c) => {
            let na = replace_regions(a, assignments);
            let nb = replace_regions(b, assignments);
            let nc = replace_regions(c, assignments);
            Rc::new(EvmExpr::Revert(na, nb, nc))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let na = replace_regions(a, assignments);
            let nb = replace_regions(b, assignments);
            let nc = replace_regions(c, assignments);
            Rc::new(EvmExpr::ReturnOp(na, nb, nc))
        }
        EvmExpr::Log(count, topics, d, s, st) => {
            let nt: Vec<_> = topics
                .iter()
                .map(|t| replace_regions(t, assignments))
                .collect();
            let nd = replace_regions(d, assignments);
            let ns = replace_regions(s, assignments);
            let nst = replace_regions(st, assignments);
            Rc::new(EvmExpr::Log(*count, nt, nd, ns, nst))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => Rc::new(EvmExpr::ExtCall(
            replace_regions(a, assignments),
            replace_regions(b, assignments),
            replace_regions(c, assignments),
            replace_regions(d, assignments),
            replace_regions(e, assignments),
            replace_regions(f, assignments),
            replace_regions(g, assignments),
        )),
        EvmExpr::Call(name, args) => {
            let new_args: Vec<_> = args
                .iter()
                .map(|a| replace_regions(a, assignments))
                .collect();
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let nb = replace_regions(body, assignments);
            Rc::new(EvmExpr::Function(
                name.clone(),
                in_ty.clone(),
                out_ty.clone(),
                nb,
            ))
        }
        EvmExpr::EnvRead(op, s) => {
            let ns = replace_regions(s, assignments);
            Rc::new(EvmExpr::EnvRead(*op, ns))
        }
        EvmExpr::EnvRead1(op, a, s) => {
            let na = replace_regions(a, assignments);
            let ns = replace_regions(s, assignments);
            Rc::new(EvmExpr::EnvRead1(*op, na, ns))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let ni: Vec<_> = inputs
                .iter()
                .map(|i| replace_regions(i, assignments))
                .collect();
            Rc::new(EvmExpr::InlineAsm(ni, hex.clone(), *num_outputs))
        }
        // Leaf nodes — no MemRegion possible
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => Rc::clone(expr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_helpers;

    #[test]
    fn test_single_region_assignment() {
        let ctx = EvmContext::InFunction("test".to_owned());
        // MStore(MemRegion(0, 3), val, state)
        let region = ast_helpers::mem_region(0, 3);
        let val = ast_helpers::const_int(42, ctx.clone());
        let state = Rc::new(EvmExpr::Arg(EvmType::Base(EvmBaseType::StateT), ctx));
        let mstore = ast_helpers::mstore(Rc::clone(&region), val, state);

        let (result, hw) = assign_memory_offsets(&mstore);
        assert_eq!(hw, 96); // 3 words * 32 bytes

        // The MemRegion should be replaced with Const(0)
        if let EvmExpr::Top(_, offset, _, _) = result.as_ref() {
            if let EvmExpr::Const(EvmConstant::SmallInt(0), _, _) = offset.as_ref() {
                // Good — region 0 assigned offset 0
            } else {
                panic!("expected Const(0), got: {offset:?}");
            }
        } else {
            panic!("expected Top, got: {result:?}");
        }
    }

    #[test]
    fn test_multiple_regions_non_overlapping() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let r0 = ast_helpers::mem_region(0, 2); // 64 bytes
        let r1 = ast_helpers::mem_region(1, 3); // 96 bytes
        let val = ast_helpers::const_int(1, ctx.clone());
        let state = Rc::new(EvmExpr::Arg(EvmType::Base(EvmBaseType::StateT), ctx));

        // Concat(MStore(r0, val, state), MStore(r1, val, state))
        let ms0 = ast_helpers::mstore(r0, Rc::clone(&val), Rc::clone(&state));
        let ms1 = ast_helpers::mstore(r1, val, state);
        let expr = ast_helpers::concat(ms0, ms1);

        let (result, hw) = assign_memory_offsets(&expr);
        assert_eq!(hw, 160); // 2*32 + 3*32 = 160

        // Verify the offsets are 0 and 64
        if let EvmExpr::Concat(left, right) = result.as_ref() {
            if let EvmExpr::Top(_, off0, _, _) = left.as_ref() {
                assert!(matches!(
                    off0.as_ref(),
                    EvmExpr::Const(EvmConstant::SmallInt(0), _, _)
                ));
            }
            if let EvmExpr::Top(_, off1, _, _) = right.as_ref() {
                assert!(matches!(
                    off1.as_ref(),
                    EvmExpr::Const(EvmConstant::SmallInt(64), _, _)
                ));
            }
        }
    }

    #[test]
    fn test_no_regions_passthrough() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let expr = ast_helpers::const_int(42, ctx);
        let (result, hw) = assign_memory_offsets(&expr);
        assert_eq!(hw, 0);
        assert_eq!(*result, *expr);
    }
}
