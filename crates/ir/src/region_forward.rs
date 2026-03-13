//! Region store forwarding pass.
//!
//! Walks the IR in program order (Concat chains) and forwards known
//! RegionStore values to subsequent RegionLoad reads. This enables
//! compile-time resolution of struct field access patterns like Vec's
//! len/capacity fields.
//!
//! Runs after lowering, before egglog. Forwarded constants enable
//! egglog's constant folding and dead-branch elimination.

use std::collections::HashMap;
use std::rc::Rc;

use crate::schema::{EvmBinaryOp, EvmConstant, EvmExpr, EvmProgram, EvmUnaryOp, RcExpr};

/// Forward region stores across the entire program.
pub fn forward_region_stores_program(
    program: &mut EvmProgram,
    region_var_map: &indexmap::IndexMap<i64, String>,
) {
    if region_var_map.is_empty() {
        return;
    }

    tracing::debug!("region_forward: region_var_map = {:?}", region_var_map);

    // Reverse map: variable name → set of region_ids.
    // Multiple region_ids can map to the same variable name (e.g., different
    // test functions each declare `let v = Vec::new()`).
    let mut reverse_map: HashMap<String, Vec<i64>> = HashMap::new();
    for (&rid, name) in region_var_map {
        reverse_map.entry(name.clone()).or_default().push(rid);
    }
    tracing::debug!("region_forward: reverse_map = {:?}", reverse_map);

    for contract in &mut program.contracts {
        let mut state = ForwardState {
            known: HashMap::new(),
            reverse_map: &reverse_map,
        };
        contract.runtime = forward_expr(&contract.runtime, &mut state);

        for func in &mut contract.internal_functions {
            let mut state = ForwardState {
                known: HashMap::new(),
                reverse_map: &reverse_map,
            };
            *func = forward_expr(func, &mut state);
        }
    }
}

struct ForwardState<'a> {
    /// Known values for (region_id, field_idx).
    known: HashMap<(i64, i64), RcExpr>,
    /// Variable name → all region_ids for that variable name.
    reverse_map: &'a HashMap<String, Vec<i64>>,
}

impl ForwardState<'_> {
    /// Clear all known values for a specific region.
    fn clear_region(&mut self, rid: i64) {
        self.known.retain(|&(r, _), _| r != rid);
    }

    /// Clear all known values.
    fn clear_all(&mut self) {
        self.known.clear();
    }
}

fn forward_expr(expr: &RcExpr, state: &mut ForwardState<'_>) -> RcExpr {
    match expr.as_ref() {
        // Concat: process left (side effects) then right (more effects or result).
        // This is the key ordering construct in the IR.
        EvmExpr::Concat(a, b) => {
            let na = forward_expr(a, state);
            let nb = forward_expr(b, state);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Concat(na, nb))
        }

        // LetBind: process init first (establishes values), then body.
        // If this LetBind is for a region variable, extract initial field values.
        EvmExpr::LetBind(name, init, body) => {
            let ni = forward_expr(init, state);

            // Check if this is a region variable by looking at all candidate region_ids
            // for this variable name and finding the one used in the body.
            if let Some(rids) = state.reverse_map.get(name.as_str()) {
                // Find which region_id is actually used in this LetBind's body
                let rid = if rids.len() == 1 {
                    Some(rids[0])
                } else {
                    // Multiple region_ids map to this name — find which one
                    // appears in the body of this LetBind
                    find_region_id_in_expr(body, rids)
                };

                if let Some(rid) = rid {
                    // Try to extract initial field values from the init expression.
                    if let Some(inner_var) = find_return_var(&ni) {
                        let field_values = extract_init_field_values(&ni, &inner_var);
                        for (field_idx, val) in field_values {
                            tracing::trace!(
                                "region_forward: init field ({}, {}) = {:?}",
                                rid,
                                field_idx,
                                val
                            );
                            state.known.insert((rid, field_idx), val);
                        }
                    }
                }
            }

            let nb = forward_expr(body, state);
            if Rc::ptr_eq(&ni, init) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::LetBind(name.clone(), ni, nb))
        }

        // RegionLoad: forward if known value exists.
        // NOTE: Do NOT recurse into state parameter. Codegen ignores state params,
        // and recursing would traverse the entire state chain (all prior operations),
        // causing stores/loads to be re-processed with wrong forwarding context.
        EvmExpr::RegionLoad(rid, fid, _st) => {
            if let Some(val) = state.known.get(&(*rid, *fid)) {
                tracing::trace!("region_forward: load ({}, {}) → forwarded", rid, fid);
                return Rc::clone(val);
            }
            // Not forwarded — keep as-is (don't recurse into state)
            Rc::clone(expr)
        }

        // RegionStore: process value (forwarding inner loads), record, keep the store.
        // NOTE: Do NOT recurse into state parameter — same reason as RegionLoad.
        EvmExpr::RegionStore(rid, fid, val, st) => {
            let nv = forward_expr(val, state);
            // Record the (already-forwarded) value
            state.known.insert((*rid, *fid), Rc::clone(&nv));
            tracing::trace!("region_forward: store ({}, {}) = {:?}", rid, fid, nv);
            if Rc::ptr_eq(&nv, val) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::RegionStore(*rid, *fid, nv, Rc::clone(st)))
        }

        // VarStore: if storing to a region variable (pointer reassignment, e.g. growth),
        // clear all known values for that region.
        EvmExpr::VarStore(name, val) => {
            let nv = forward_expr(val, state);
            if let Some(rids) = state.reverse_map.get(name.as_str()) {
                for &rid in rids {
                    if state.known.keys().any(|&(r, _)| r == rid) {
                        tracing::trace!("region_forward: VarStore to region var '{}' → clear region {}", name, rid);
                        state.clear_region(rid);
                    }
                }
            }
            if Rc::ptr_eq(&nv, val) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::VarStore(name.clone(), nv))
        }

        // If: process cond, then try compile-time branch evaluation.
        // If branch is known dead, skip it and preserve known values.
        EvmExpr::If(cond, inputs, then_br, else_br) => {
            let nc = forward_expr(cond, state);
            let ni = forward_expr(inputs, state);

            // Try compile-time evaluation of the condition
            if let Some(cond_val) = try_eval_const(&nc) {
                tracing::trace!("region_forward: If condition evaluated to {}", cond_val);
                if cond_val {
                    // Condition is true → only then-branch executes
                    let nt = forward_expr(then_br, state);
                    // Emit the If with the original condition (egglog will fold it)
                    return Rc::new(EvmExpr::If(nc, ni, nt, Rc::clone(else_br)));
                } else {
                    // Condition is false → only else-branch executes
                    let ne = forward_expr(else_br, state);
                    return Rc::new(EvmExpr::If(nc, ni, Rc::clone(then_br), ne));
                }
            }

            // Can't evaluate → conservative: process both, clear modified regions
            tracing::trace!("region_forward: If condition could NOT be evaluated: {:?}", nc);
            let saved = state.known.clone();
            let nt = forward_expr(then_br, state);
            let then_known = state.known.clone();
            state.known = saved;
            let ne = forward_expr(else_br, state);
            // After if: only keep values that are identical in both branches
            let mut merged = HashMap::new();
            for (key, then_val) in &then_known {
                if let Some(else_val) = state.known.get(key) {
                    if Rc::ptr_eq(then_val, else_val) {
                        merged.insert(*key, Rc::clone(then_val));
                    }
                }
            }
            state.known = merged;

            if Rc::ptr_eq(&nc, cond)
                && Rc::ptr_eq(&ni, inputs)
                && Rc::ptr_eq(&nt, then_br)
                && Rc::ptr_eq(&ne, else_br)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::If(nc, ni, nt, ne))
        }

        // DoWhile: clear all known values (can't reason about loops).
        EvmExpr::DoWhile(inputs, body) => {
            state.clear_all();
            let ni = forward_expr(inputs, state);
            let nb = forward_expr(body, state);
            state.clear_all();
            if Rc::ptr_eq(&ni, inputs) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DoWhile(ni, nb))
        }

        // --- Recurse into all other node types ---
        // For state-carrying Bop ops (MLoad, CalldataLoad, SLoad, TLoad),
        // second arg is state — skip it. For others, recurse into both.
        EvmExpr::Bop(op, a, b) => {
            let is_state_carrying = matches!(
                op,
                EvmBinaryOp::MLoad
                    | EvmBinaryOp::CalldataLoad
                    | EvmBinaryOp::SLoad
                    | EvmBinaryOp::TLoad
            );
            let na = forward_expr(a, state);
            if is_state_carrying {
                if Rc::ptr_eq(&na, a) {
                    return Rc::clone(expr);
                }
                return Rc::new(EvmExpr::Bop(*op, na, Rc::clone(b)));
            }
            let nb = forward_expr(b, state);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, na, nb))
        }
        EvmExpr::Uop(op, a) => {
            let na = forward_expr(a, state);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, na))
        }
        // Top: third arg is state parameter — skip recursion into it.
        EvmExpr::Top(op, a, b, _c) => {
            let na = forward_expr(a, state);
            let nb = forward_expr(b, state);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Top(*op, na, nb, Rc::clone(_c)))
        }
        EvmExpr::Get(a, idx) => {
            let na = forward_expr(a, state);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(na, *idx))
        }
        // Revert/ReturnOp: third arg is state — skip it.
        EvmExpr::Revert(a, b, _c) => {
            let na = forward_expr(a, state);
            let nb = forward_expr(b, state);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Revert(na, nb, Rc::clone(_c)))
        }
        EvmExpr::ReturnOp(a, b, _c) => {
            let na = forward_expr(a, state);
            let nb = forward_expr(b, state);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ReturnOp(na, nb, Rc::clone(_c)))
        }
        // Log: last arg is state — skip it.
        EvmExpr::Log(count, topics, d, s, st) => {
            let nt: Vec<_> = topics.iter().map(|t| forward_expr(t, state)).collect();
            let nd = forward_expr(d, state);
            let ns = forward_expr(s, state);
            if nt.iter().zip(topics.iter()).all(|(n, o)| Rc::ptr_eq(n, o))
                && Rc::ptr_eq(&nd, d)
                && Rc::ptr_eq(&ns, s)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Log(*count, nt, nd, ns, Rc::clone(st)))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            // ExtCall: last arg is state — skip it. Clear all known values.
            let na = forward_expr(a, state);
            let nb = forward_expr(b, state);
            let nc = forward_expr(c, state);
            let nd = forward_expr(d, state);
            let ne = forward_expr(e, state);
            let nf = forward_expr(f, state);
            state.clear_all();
            if Rc::ptr_eq(&na, a)
                && Rc::ptr_eq(&nb, b)
                && Rc::ptr_eq(&nc, c)
                && Rc::ptr_eq(&nd, d)
                && Rc::ptr_eq(&ne, e)
                && Rc::ptr_eq(&nf, f)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ExtCall(na, nb, nc, nd, ne, nf, Rc::clone(g)))
        }
        EvmExpr::Call(name, args) => {
            let new_args: Vec<_> = args.iter().map(|a| forward_expr(a, state)).collect();
            // Call could modify memory — clear known values
            state.clear_all();
            if new_args
                .iter()
                .zip(args.iter())
                .all(|(n, o)| Rc::ptr_eq(n, o))
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let nb = forward_expr(body, state);
            if Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Function(name.clone(), in_ty.clone(), out_ty.clone(), nb))
        }
        // EnvRead: state parameter — skip it.
        EvmExpr::EnvRead(_op, _s) => Rc::clone(expr),
        EvmExpr::EnvRead1(op, a, _s) => {
            let na = forward_expr(a, state);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::EnvRead1(*op, na, Rc::clone(_s)))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let ni: Vec<_> = inputs.iter().map(|i| forward_expr(i, state)).collect();
            state.clear_all(); // inline asm could do anything
            if ni.iter().zip(inputs.iter()).all(|(n, o)| Rc::ptr_eq(n, o)) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::InlineAsm(ni, hex.clone(), *num_outputs))
        }
        EvmExpr::DynAlloc(size) => {
            let ns = forward_expr(size, state);
            if Rc::ptr_eq(&ns, size) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DynAlloc(ns))
        }
        EvmExpr::AllocRegion(id, num_fields, is_dynamic) => {
            let nf = forward_expr(num_fields, state);
            if Rc::ptr_eq(&nf, num_fields) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::AllocRegion(*id, nf, *is_dynamic))
        }

        // Leaves — no children
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::MemRegion(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => Rc::clone(expr),
    }
}

/// Find which region_id from the candidate list appears in the expression.
/// Used to disambiguate when multiple region_ids map to the same variable name.
fn find_region_id_in_expr(expr: &RcExpr, candidates: &[i64]) -> Option<i64> {
    match expr.as_ref() {
        EvmExpr::RegionLoad(rid, _, _) | EvmExpr::RegionStore(rid, _, _, _) => {
            if candidates.contains(rid) {
                return Some(*rid);
            }
            // Recurse into children
            match expr.as_ref() {
                EvmExpr::RegionLoad(_, _, st) => find_region_id_in_expr(st, candidates),
                EvmExpr::RegionStore(_, _, val, st) => find_region_id_in_expr(val, candidates)
                    .or_else(|| find_region_id_in_expr(st, candidates)),
                _ => None,
            }
        }
        EvmExpr::Concat(a, b) => find_region_id_in_expr(a, candidates)
            .or_else(|| find_region_id_in_expr(b, candidates)),
        EvmExpr::LetBind(_, init, body) => find_region_id_in_expr(init, candidates)
            .or_else(|| find_region_id_in_expr(body, candidates)),
        EvmExpr::If(cond, _, then_br, else_br) => find_region_id_in_expr(cond, candidates)
            .or_else(|| find_region_id_in_expr(then_br, candidates))
            .or_else(|| find_region_id_in_expr(else_br, candidates)),
        EvmExpr::Bop(_, a, b) => find_region_id_in_expr(a, candidates)
            .or_else(|| find_region_id_in_expr(b, candidates)),
        EvmExpr::Uop(_, a) => find_region_id_in_expr(a, candidates),
        EvmExpr::Top(_, a, b, c) => find_region_id_in_expr(a, candidates)
            .or_else(|| find_region_id_in_expr(b, candidates))
            .or_else(|| find_region_id_in_expr(c, candidates)),
        _ => None,
    }
}

/// Find the "return variable" of an expression — the variable whose value
/// is the result of evaluating the expression. Traces through Concat chains
/// (which return b) and LetBind (which returns body).
fn find_return_var(expr: &RcExpr) -> Option<String> {
    match expr.as_ref() {
        EvmExpr::Var(name) => Some(name.clone()),
        EvmExpr::Concat(_, b) => find_return_var(b),
        EvmExpr::LetBind(_, _, body) => find_return_var(body),
        _ => None,
    }
}

/// Extract initial field values from an init expression.
///
/// Scans for MStore patterns that write to fields of the given inner variable:
/// - `MStore(Var(inner), val, _)` → field 0
/// - `MStore(Add(Var(inner), Const(32)), val, _)` → field 1
/// - `MStore(Add(Var(inner), Const(64)), val, _)` → field 2
/// - etc.
fn extract_init_field_values(expr: &RcExpr, inner_var: &str) -> HashMap<i64, RcExpr> {
    let mut result = HashMap::new();
    collect_mstore_fields(expr, inner_var, &mut result);
    result
}

fn collect_mstore_fields(expr: &RcExpr, inner_var: &str, out: &mut HashMap<i64, RcExpr>) {
    match expr.as_ref() {
        EvmExpr::Top(crate::schema::EvmTernaryOp::MStore, offset, val, _state) => {
            // Check: MStore(Var(inner), val, _) → field 0
            if let EvmExpr::Var(name) = offset.as_ref() {
                if name == inner_var {
                    out.insert(0, Rc::clone(val));
                    return;
                }
            }
            // Check: MStore(Add(Var(inner), Const(N)), val, _) → field N/32
            if let EvmExpr::Bop(EvmBinaryOp::Add, a, b) = offset.as_ref() {
                let (base, offset_val) = if matches!(a.as_ref(), EvmExpr::Var(_)) {
                    (a, b)
                } else if matches!(b.as_ref(), EvmExpr::Var(_)) {
                    (b, a)
                } else {
                    return;
                };
                if let EvmExpr::Var(name) = base.as_ref() {
                    if name == inner_var {
                        if let Some(off) = const_value(offset_val) {
                            if off >= 0 && off % 32 == 0 {
                                let field_idx = off / 32;
                                out.insert(field_idx, Rc::clone(val));
                            }
                        }
                    }
                }
            }
            // Check: MStore(CheckedAdd(Var(inner), Const(N)), val, _) → field N/32
            if let EvmExpr::Bop(EvmBinaryOp::CheckedAdd, a, b) = offset.as_ref() {
                let (base, offset_val) = if matches!(a.as_ref(), EvmExpr::Var(_)) {
                    (a, b)
                } else if matches!(b.as_ref(), EvmExpr::Var(_)) {
                    (b, a)
                } else {
                    return;
                };
                if let EvmExpr::Var(name) = base.as_ref() {
                    if name == inner_var {
                        if let Some(off) = const_value(offset_val) {
                            if off >= 0 && off % 32 == 0 {
                                let field_idx = off / 32;
                                out.insert(field_idx, Rc::clone(val));
                            }
                        }
                    }
                }
            }
        }
        // Recurse into Concat, LetBind to find nested MStores
        EvmExpr::Concat(a, b) => {
            collect_mstore_fields(a, inner_var, out);
            collect_mstore_fields(b, inner_var, out);
        }
        EvmExpr::LetBind(_, init, body) => {
            collect_mstore_fields(init, inner_var, out);
            collect_mstore_fields(body, inner_var, out);
        }
        _ => {}
    }
}

/// Extract a constant integer value from an expression.
fn const_value(expr: &RcExpr) -> Option<i64> {
    match expr.as_ref() {
        EvmExpr::Const(EvmConstant::SmallInt(n), _, _) => Some(*n),
        _ => None,
    }
}

/// Try to evaluate an expression as a boolean constant (true/false).
/// Returns `Some(true)` if the expression is known to be non-zero,
/// `Some(false)` if known to be zero, `None` if unknown.
fn try_eval_const(expr: &RcExpr) -> Option<bool> {
    match expr.as_ref() {
        EvmExpr::Const(EvmConstant::SmallInt(n), _, _) => Some(*n != 0),
        EvmExpr::Const(EvmConstant::LargeInt(b), _, _) => {
            // LargeInt is stored as a hex string
            Some(b != "0" && b != "0x0" && !b.chars().all(|c| c == '0' || c == 'x'))
        }
        EvmExpr::Uop(EvmUnaryOp::IsZero, a) => {
            try_eval_const(a).map(|v| !v)
        }
        EvmExpr::Bop(EvmBinaryOp::Lt, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            Some(av < bv)
        }
        EvmExpr::Bop(EvmBinaryOp::Gt, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            Some(av > bv)
        }
        EvmExpr::Bop(EvmBinaryOp::Eq, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            Some(av == bv)
        }
        _ => None,
    }
}

/// Try to evaluate an expression as a U256 value.
fn try_eval_u256(expr: &RcExpr) -> Option<u64> {
    match expr.as_ref() {
        EvmExpr::Const(EvmConstant::SmallInt(n), _, _) => {
            if *n >= 0 {
                Some(*n as u64)
            } else {
                None
            }
        }
        EvmExpr::Bop(EvmBinaryOp::Add | EvmBinaryOp::CheckedAdd, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            av.checked_add(bv)
        }
        EvmExpr::Bop(EvmBinaryOp::Sub | EvmBinaryOp::CheckedSub, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            av.checked_sub(bv)
        }
        EvmExpr::Bop(EvmBinaryOp::Mul | EvmBinaryOp::CheckedMul, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            av.checked_mul(bv)
        }
        EvmExpr::Uop(EvmUnaryOp::IsZero, a) => {
            let av = try_eval_u256(a)?;
            Some(if av == 0 { 1 } else { 0 })
        }
        EvmExpr::Bop(EvmBinaryOp::Lt, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            Some(if av < bv { 1 } else { 0 })
        }
        EvmExpr::Bop(EvmBinaryOp::Gt, a, b) => {
            let av = try_eval_u256(a)?;
            let bv = try_eval_u256(b)?;
            Some(if av > bv { 1 } else { 0 })
        }
        _ => None,
    }
}
