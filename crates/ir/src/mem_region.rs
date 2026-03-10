//! Post-egglog pass: assign concrete memory offsets to symbolic `MemRegion` nodes.
//!
//! After egglog extraction, the IR contains `MemRegion(id, size_words)` nodes
//! representing symbolic memory allocations. This pass:
//!
//! 1. Builds a scope tree reflecting control-flow mutual exclusivity
//! 2. Assigns concrete byte offsets — regions in mutually exclusive branches
//!    (If then/else) share the same base offset
//! 3. Replaces each `MemRegion(id, size)` with `Const(SmallInt(offset))`
//! 4. Returns the total memory used (memory high water mark)
//!
//! The assigned offsets become the `memory_high_water` value that codegen uses
//! to start `LetBind` variable slots above all IR-allocated regions.

use std::{collections::BTreeMap, rc::Rc};

use crate::schema::{EvmBaseType, EvmConstant, EvmContext, EvmExpr, EvmType, RcExpr};

/// Scope tree node for memory region allocation.
///
/// Models control-flow mutual exclusivity so that regions in different
/// branches of an `If` can share the same memory offsets.
enum RegionScope {
    /// Children execute sequentially — all must be non-overlapping.
    Sequential(Vec<RegionScope>),
    /// Children are mutually exclusive (If branches) — can share base offset.
    Exclusive(Vec<RegionScope>),
    /// A single memory region allocation.
    Leaf { region_id: i64, size_bytes: usize },
}

/// Assign concrete memory offsets to all `MemRegion` nodes in an expression.
///
/// Returns `(rewritten_expr, memory_high_water)` where `memory_high_water` is
/// the first free byte offset after all allocated regions.
pub fn assign_memory_offsets(expr: &RcExpr) -> (RcExpr, usize) {
    let scope = collect_region_scopes(expr);
    let scope = simplify_scope(scope);

    let mut assignments = BTreeMap::new();
    let hw = assign_scoped_offsets(&scope, 0, &mut assignments);

    if assignments.is_empty() {
        return (Rc::clone(expr), 0);
    }

    tracing::debug!("  mem_region hw={hw} ({} regions)", assignments.len());

    let rewritten = replace_regions(expr, &assignments);
    (rewritten, hw)
}

/// Recursively assign offsets respecting mutual exclusivity.
///
/// Returns the total bytes consumed from `base_offset`.
fn assign_scoped_offsets(
    scope: &RegionScope,
    base_offset: usize,
    assignments: &mut BTreeMap<i64, usize>,
) -> usize {
    match scope {
        RegionScope::Leaf {
            region_id,
            size_bytes,
        } => {
            // Only count size for the first occurrence of a region.
            // Shared Rc nodes cause the same region to appear multiple times.
            if assignments.contains_key(region_id) {
                0
            } else {
                assignments.insert(*region_id, base_offset);
                *size_bytes
            }
        }
        RegionScope::Sequential(children) => {
            let mut cursor = base_offset;
            for child in children {
                let used = assign_scoped_offsets(child, cursor, assignments);
                cursor += used;
            }
            cursor - base_offset
        }
        RegionScope::Exclusive(branches) => {
            let mut max_used = 0;
            for branch in branches {
                let used = assign_scoped_offsets(branch, base_offset, assignments);
                max_used = max_used.max(used);
            }
            max_used
        }
    }
}

/// Build a scope tree from an IR expression.
fn collect_region_scopes(expr: &RcExpr) -> RegionScope {
    match expr.as_ref() {
        EvmExpr::MemRegion(id, size_words) => RegionScope::Leaf {
            region_id: *id,
            size_bytes: (*size_words as usize) * 32,
        },

        // If: condition+inputs sequential, then/else exclusive
        EvmExpr::If(cond, inputs, then_br, else_br) => RegionScope::Sequential(vec![
            collect_region_scopes(cond),
            collect_region_scopes(inputs),
            RegionScope::Exclusive(vec![
                collect_region_scopes(then_br),
                collect_region_scopes(else_br),
            ]),
        ]),

        // Sequential composition
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) => {
            RegionScope::Sequential(vec![collect_region_scopes(a), collect_region_scopes(b)])
        }
        EvmExpr::LetBind(_, init, body) => RegionScope::Sequential(vec![
            collect_region_scopes(init),
            collect_region_scopes(body),
        ]),
        EvmExpr::DoWhile(inputs, body) => RegionScope::Sequential(vec![
            collect_region_scopes(inputs),
            collect_region_scopes(body),
        ]),
        EvmExpr::Function(_, _, _, body) => collect_region_scopes(body),

        // Ternary children — sequential
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            RegionScope::Sequential(vec![
                collect_region_scopes(a),
                collect_region_scopes(b),
                collect_region_scopes(c),
            ])
        }

        // Unary children
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) | EvmExpr::VarStore(_, a) => {
            collect_region_scopes(a)
        }

        // Multi-child nodes
        EvmExpr::Log(_, topics, d, s, st) => {
            let mut children: Vec<_> = topics.iter().map(collect_region_scopes).collect();
            children.push(collect_region_scopes(d));
            children.push(collect_region_scopes(s));
            children.push(collect_region_scopes(st));
            RegionScope::Sequential(children)
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => RegionScope::Sequential(
            [a, b, c, d, e, f, g]
                .into_iter()
                .map(collect_region_scopes)
                .collect(),
        ),
        EvmExpr::Call(_, args) => {
            RegionScope::Sequential(args.iter().map(collect_region_scopes).collect())
        }
        EvmExpr::EnvRead(_, s) => collect_region_scopes(s),
        EvmExpr::EnvRead1(_, a, s) => {
            RegionScope::Sequential(vec![collect_region_scopes(a), collect_region_scopes(s)])
        }
        EvmExpr::InlineAsm(inputs, ..) => {
            RegionScope::Sequential(inputs.iter().map(collect_region_scopes).collect())
        }

        // Leaf nodes — no regions
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => RegionScope::Sequential(vec![]),
    }
}

/// Returns true if a scope tree contains any Leaf nodes.
fn has_regions(scope: &RegionScope) -> bool {
    match scope {
        RegionScope::Leaf { .. } => true,
        RegionScope::Sequential(children) | RegionScope::Exclusive(children) => {
            children.iter().any(has_regions)
        }
    }
}

/// Simplify a scope tree: remove empty nodes, flatten nested Sequential.
fn simplify_scope(scope: RegionScope) -> RegionScope {
    match scope {
        RegionScope::Leaf { .. } => scope,
        RegionScope::Sequential(children) => {
            // Recursively simplify, drop empty, flatten nested Sequential
            let mut flat = Vec::new();
            for child in children {
                let simplified = simplify_scope(child);
                if !has_regions(&simplified) {
                    continue;
                }
                match simplified {
                    RegionScope::Sequential(inner) => flat.extend(inner),
                    other => flat.push(other),
                }
            }
            match flat.len() {
                0 => RegionScope::Sequential(vec![]),
                1 => flat.into_iter().next().unwrap(),
                _ => RegionScope::Sequential(flat),
            }
        }
        RegionScope::Exclusive(children) => {
            let simplified: Vec<_> = children
                .into_iter()
                .map(simplify_scope)
                .filter(has_regions)
                .collect();
            match simplified.len() {
                0 => RegionScope::Sequential(vec![]),
                1 => simplified.into_iter().next().unwrap(),
                _ => RegionScope::Exclusive(simplified),
            }
        }
    }
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

    #[test]
    fn test_exclusive_branches_share_offsets() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let r0 = ast_helpers::mem_region(0, 2); // 64 bytes — then branch
        let r1 = ast_helpers::mem_region(1, 3); // 96 bytes — else branch
        let val = ast_helpers::const_int(1, ctx.clone());
        let state = Rc::new(EvmExpr::Arg(
            EvmType::Base(EvmBaseType::StateT),
            ctx.clone(),
        ));
        let cond = ast_helpers::const_int(1, ctx.clone());
        let inputs = Rc::new(EvmExpr::Empty(EvmType::Base(EvmBaseType::UnitT), ctx));

        let then_br = ast_helpers::mstore(r0, Rc::clone(&val), Rc::clone(&state));
        let else_br = ast_helpers::mstore(r1, val, state);
        let if_expr = Rc::new(EvmExpr::If(cond, inputs, then_br, else_br));

        let (result, hw) = assign_memory_offsets(&if_expr);
        // Branches are exclusive: hw = max(64, 96) = 96, NOT 64+96=160
        assert_eq!(hw, 96);

        // Both regions should start at offset 0
        if let EvmExpr::If(_, _, then_b, else_b) = result.as_ref() {
            if let EvmExpr::Top(_, off, _, _) = then_b.as_ref() {
                assert!(
                    matches!(off.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(0), _, _)),
                    "then branch region should be at offset 0"
                );
            }
            if let EvmExpr::Top(_, off, _, _) = else_b.as_ref() {
                assert!(
                    matches!(off.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(0), _, _)),
                    "else branch region should be at offset 0"
                );
            }
        }
    }

    #[test]
    fn test_sequential_before_exclusive() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let r_shared = ast_helpers::mem_region(0, 1); // 32 bytes — before if
        let r_then = ast_helpers::mem_region(1, 2); // 64 bytes — then branch
        let r_else = ast_helpers::mem_region(2, 3); // 96 bytes — else branch
        let val = ast_helpers::const_int(1, ctx.clone());
        let state = Rc::new(EvmExpr::Arg(
            EvmType::Base(EvmBaseType::StateT),
            ctx.clone(),
        ));
        let cond = ast_helpers::const_int(1, ctx.clone());
        let inputs = Rc::new(EvmExpr::Empty(EvmType::Base(EvmBaseType::UnitT), ctx));

        let pre = ast_helpers::mstore(r_shared, Rc::clone(&val), Rc::clone(&state));
        let then_br = ast_helpers::mstore(r_then, Rc::clone(&val), Rc::clone(&state));
        let else_br = ast_helpers::mstore(r_else, val, state);
        let if_expr = Rc::new(EvmExpr::If(cond, inputs, then_br, else_br));
        let expr = ast_helpers::concat(pre, if_expr);

        let (_result, hw) = assign_memory_offsets(&expr);
        // r_shared=32 bytes, then branches max(64,96)=96 → total 32+96=128
        assert_eq!(hw, 128);
    }
}
