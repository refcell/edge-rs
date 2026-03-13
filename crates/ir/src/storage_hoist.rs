//! Storage-to-local hoisting (LICM for SLoad/SStore in loops).
//!
//! When a loop body repeatedly reads/writes storage slots with constant
//! indices, this pass hoists the values into local variables, replacing
//! SLoad/SStore with Var/VarStore inside the loop. Write-backs are
//! emitted after the loop exits.

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::schema::{
    EvmBaseType, EvmBinaryOp, EvmConstant, EvmContext, EvmExpr, EvmTernaryOp, EvmType, RcExpr,
};

/// Which kind of storage operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum StorageKind {
    Persistent,
    Transient,
}

/// Key identifying a storage slot to hoist.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SlotKey {
    kind: StorageKind,
    slot_value: i64,
}

/// Usage information for a storage slot within a loop body.
#[derive(Debug, Clone, Default)]
struct SlotUsage {
    has_load: bool,
    has_store: bool,
}

/// Entry point: hoist storage operations out of loops in all contracts.
pub fn hoist_program(program: &mut crate::schema::EvmProgram) {
    let mut counter = 0usize;
    for contract in &mut program.contracts {
        contract.runtime = hoist_expr(&contract.runtime, &mut counter);
        for func in &mut contract.internal_functions {
            *func = hoist_expr(func, &mut counter);
        }
    }
}

// ============================================================
// General SStore→SLoad forwarding + dead store elimination
// ============================================================

/// Forward `SStore` values through subsequent `SLoads` in straight-line code
/// and eliminate dead intermediate `SStores`.
///
/// For a sequence like:
///   `SStore(0, 0); SStore(0, SLoad(0) + 1); return SLoad(0)`
/// This becomes:
///   `SStore(0, 0+1); return (0+1)`
pub fn forward_stores_program(program: &mut crate::schema::EvmProgram) {
    for contract in &mut program.contracts {
        contract.runtime = forward_stores_expr(&contract.runtime);
        for func in &mut contract.internal_functions {
            *func = forward_stores_expr(func);
        }
    }
}

/// Top-level entry: find Concat chains and apply forwarding.
fn forward_stores_expr(expr: &RcExpr) -> RcExpr {
    let mut cache = HashMap::new();
    forward_stores_expr_inner(expr, &mut cache)
}

fn forward_stores_expr_inner(
    expr: &RcExpr,
    cache: &mut HashMap<usize, RcExpr>,
) -> RcExpr {
    let ptr = Rc::as_ptr(expr) as usize;
    if let Some(cached) = cache.get(&ptr) {
        return Rc::clone(cached);
    }

    let result = match expr.as_ref() {
        EvmExpr::Concat(..) => {
            // Flatten this Concat chain
            let mut stmts = Vec::new();
            flatten_concat(expr, &mut stmts);

            // Phase 1+2: Forward SLoads and eliminate dead SStores
            let processed = forward_chain(stmts);

            // Phase 3: Recurse into structural sub-bodies of each statement
            let recursed: Vec<RcExpr> = processed
                .into_iter()
                .map(|s| recurse_substructures_inner(&s, cache))
                .collect();

            rebuild_concat(&recursed)
        }
        // For non-Concat nodes, just recurse into sub-structures
        _ => recurse_substructures_inner(expr, cache),
    };

    cache.insert(ptr, Rc::clone(&result));
    result
}

/// Recurse into structural sub-bodies (If branches, `DoWhile` body, `LetBind` body).
/// These get their own independent forwarding context.
fn recurse_substructures_inner(
    expr: &RcExpr,
    cache: &mut HashMap<usize, RcExpr>,
) -> RcExpr {
    let ptr = Rc::as_ptr(expr) as usize;
    if let Some(cached) = cache.get(&ptr) {
        return Rc::clone(cached);
    }

    let result = match expr.as_ref() {
        EvmExpr::If(c, i, t, e) => Rc::new(EvmExpr::If(
            Rc::clone(c),
            Rc::clone(i),
            forward_stores_expr_inner(t, cache),
            forward_stores_expr_inner(e, cache),
        )),
        EvmExpr::LetBind(name, init, body) => Rc::new(EvmExpr::LetBind(
            name.clone(),
            forward_stores_expr_inner(init, cache),
            forward_stores_expr_inner(body, cache),
        )),
        EvmExpr::Function(name, in_ty, out_ty, body) => Rc::new(EvmExpr::Function(
            name.clone(),
            in_ty.clone(),
            out_ty.clone(),
            forward_stores_expr_inner(body, cache),
        )),
        // Don't forward inside DoWhile bodies — they're cyclic (SStores from
        // iteration N affect SLoads at iteration N+1). Loop hoisting handles these.
        _ => Rc::clone(expr),
    };

    cache.insert(ptr, Rc::clone(&result));
    result
}

/// Forward `SStore` values and eliminate dead stores in a flat statement list.
fn forward_chain(stmts: Vec<&RcExpr>) -> Vec<RcExpr> {
    if stmts.len() <= 1 {
        return stmts.into_iter().cloned().collect();
    }

    // Phase 1: Forward SLoad → known value from preceding SStore
    let mut known: HashMap<SlotKey, RcExpr> = HashMap::new();
    let mut forwarded: Vec<RcExpr> = Vec::new();

    for stmt in &stmts {
        // Replace SLoads with known values (inline parts only, not structural sub-bodies)
        let fwd = replace_sloads_inline(stmt, &known);

        // Update known values if this is a top-level SStore
        if let Some((key, val)) = match_sstore_const_slot(&fwd) {
            known.insert(key, val);
        } else if might_modify_storage(&fwd) {
            // If/DoWhile/ExtCall might change storage — invalidate all knowledge
            known.clear();
        }

        forwarded.push(fwd);
    }

    // Phase 2: Eliminate dead intermediate SStores (backward walk)
    let mut later_stored: HashMap<SlotKey, ()> = HashMap::new();
    let mut keep = vec![true; forwarded.len()];

    for i in (0..forwarded.len()).rev() {
        let stmt = &forwarded[i];

        // If this statement reads any storage slots, we can't eliminate earlier
        // stores to those slots (the reads depend on them).
        let read_slots = collect_sload_slots_deep(stmt);
        for key in &read_slots {
            later_stored.remove(key);
        }

        // ExtCall/If/DoWhile might observe storage — flush
        if might_observe_storage(stmt) {
            later_stored.clear();
        }

        // Check if this is a top-level SStore
        if let Some((key, _)) = match_sstore_const_slot(stmt) {
            if let std::collections::hash_map::Entry::Vacant(e) = later_stored.entry(key) {
                e.insert(());
            } else {
                keep[i] = false;
            }
        }
    }

    forwarded
        .into_iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, s)| s)
        .collect()
}

/// Replace SLoad/TLoad with known values, recursing into inline parts only.
///
/// Does NOT recurse into structural sub-bodies (If branches, `DoWhile` body,
/// `LetBind` body) since those have independent execution contexts.
fn replace_sloads_inline(expr: &RcExpr, known: &HashMap<SlotKey, RcExpr>) -> RcExpr {
    if known.is_empty() {
        return Rc::clone(expr);
    }
    let mut cache = HashMap::new();
    replace_sloads_inline_inner(expr, known, &mut cache)
}

fn replace_sloads_inline_inner(
    expr: &RcExpr,
    known: &HashMap<SlotKey, RcExpr>,
    cache: &mut HashMap<usize, RcExpr>,
) -> RcExpr {
    let ptr = Rc::as_ptr(expr) as usize;
    if let Some(cached) = cache.get(&ptr) {
        return Rc::clone(cached);
    }

    let result = replace_sloads_inline_match(expr, known, cache);
    cache.insert(ptr, Rc::clone(&result));
    result
}

fn replace_sloads_inline_match(
    expr: &RcExpr,
    known: &HashMap<SlotKey, RcExpr>,
    cache: &mut HashMap<usize, RcExpr>,
) -> RcExpr {
    match expr.as_ref() {
        // SLoad/TLoad → known value
        EvmExpr::Bop(op @ (EvmBinaryOp::SLoad | EvmBinaryOp::TLoad), slot, _state) => {
            let kind = if *op == EvmBinaryOp::SLoad {
                StorageKind::Persistent
            } else {
                StorageKind::Transient
            };
            if let Some(sv) = const_slot_value(slot) {
                let key = SlotKey {
                    kind,
                    slot_value: sv,
                };
                if let Some(val) = known.get(&key) {
                    return Rc::clone(val);
                }
            }
            let ns = replace_sloads_inline_inner(slot, known, cache);
            Rc::new(EvmExpr::Bop(*op, ns, Rc::clone(_state)))
        }

        // Structural nodes: forward into inline parts ONLY
        EvmExpr::If(cond, inputs, then_b, else_b) => Rc::new(EvmExpr::If(
            replace_sloads_inline_inner(cond, known, cache),
            replace_sloads_inline_inner(inputs, known, cache),
            Rc::clone(then_b), // don't forward into branches
            Rc::clone(else_b),
        )),
        EvmExpr::DoWhile(inputs, body) => Rc::new(EvmExpr::DoWhile(
            replace_sloads_inline_inner(inputs, known, cache),
            Rc::clone(body), // don't forward into loop body
        )),
        EvmExpr::LetBind(name, init, body) => Rc::new(EvmExpr::LetBind(
            name.clone(),
            replace_sloads_inline_inner(init, known, cache),
            Rc::clone(body), // don't forward into body
        )),

        // All other nodes: recurse normally
        EvmExpr::Bop(op, a, b) => Rc::new(EvmExpr::Bop(
            *op,
            replace_sloads_inline_inner(a, known, cache),
            replace_sloads_inline_inner(b, known, cache),
        )),
        EvmExpr::Uop(op, a) => Rc::new(EvmExpr::Uop(*op, replace_sloads_inline_inner(a, known, cache))),
        EvmExpr::Top(op, a, b, c) => Rc::new(EvmExpr::Top(
            *op,
            replace_sloads_inline_inner(a, known, cache),
            replace_sloads_inline_inner(b, known, cache),
            replace_sloads_inline_inner(c, known, cache),
        )),
        EvmExpr::Concat(a, b) => Rc::new(EvmExpr::Concat(
            replace_sloads_inline_inner(a, known, cache),
            replace_sloads_inline_inner(b, known, cache),
        )),
        EvmExpr::Get(a, idx) => Rc::new(EvmExpr::Get(replace_sloads_inline_inner(a, known, cache), *idx)),
        EvmExpr::VarStore(name, val) => Rc::new(EvmExpr::VarStore(
            name.clone(),
            replace_sloads_inline_inner(val, known, cache),
        )),
        EvmExpr::Revert(a, b, c) => Rc::new(EvmExpr::Revert(
            replace_sloads_inline_inner(a, known, cache),
            replace_sloads_inline_inner(b, known, cache),
            replace_sloads_inline_inner(c, known, cache),
        )),
        EvmExpr::ReturnOp(a, b, c) => Rc::new(EvmExpr::ReturnOp(
            replace_sloads_inline_inner(a, known, cache),
            replace_sloads_inline_inner(b, known, cache),
            replace_sloads_inline_inner(c, known, cache),
        )),
        EvmExpr::Log(count, topics, data_offset, data_size, state) => {
            let ts: Vec<_> = topics
                .iter()
                .map(|t| replace_sloads_inline_inner(t, known, cache))
                .collect();
            Rc::new(EvmExpr::Log(
                *count,
                ts,
                replace_sloads_inline_inner(data_offset, known, cache),
                replace_sloads_inline_inner(data_size, known, cache),
                replace_sloads_inline_inner(state, known, cache),
            ))
        }
        EvmExpr::EnvRead(op, s) => Rc::new(EvmExpr::EnvRead(*op, replace_sloads_inline_inner(s, known, cache))),
        EvmExpr::EnvRead1(op, a, s) => Rc::new(EvmExpr::EnvRead1(
            *op,
            replace_sloads_inline_inner(a, known, cache),
            replace_sloads_inline_inner(s, known, cache),
        )),
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => Rc::new(EvmExpr::ExtCall(
            replace_sloads_inline_inner(a, known, cache),
            replace_sloads_inline_inner(b, known, cache),
            replace_sloads_inline_inner(c, known, cache),
            replace_sloads_inline_inner(d, known, cache),
            replace_sloads_inline_inner(e, known, cache),
            replace_sloads_inline_inner(f, known, cache),
            replace_sloads_inline_inner(g, known, cache),
        )),
        EvmExpr::Call(name, args) => Rc::new(EvmExpr::Call(
            name.clone(),
            args.iter()
                .map(|a| replace_sloads_inline_inner(a, known, cache))
                .collect(),
        )),
        EvmExpr::Function(name, in_ty, out_ty, body) => Rc::new(EvmExpr::Function(
            name.clone(),
            in_ty.clone(),
            out_ty.clone(),
            replace_sloads_inline_inner(body, known, cache),
        )),

        // Leaf nodes
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(..)
        | EvmExpr::Drop(..)
        | EvmExpr::Selector(..)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => Rc::clone(expr),
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs
                .iter()
                .map(|i| replace_sloads_inline_inner(i, known, cache))
                .collect();
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }
        EvmExpr::DynAlloc(size) => {
            let ns = replace_sloads_inline_inner(size, known, cache);
            Rc::new(EvmExpr::DynAlloc(ns))
        }
        EvmExpr::AllocRegion(id, num_fields, is_dynamic) => {
            let nf = replace_sloads_inline_inner(num_fields, known, cache);
            Rc::new(EvmExpr::AllocRegion(*id, nf, *is_dynamic))
        }
        EvmExpr::RegionStore(id, field_idx, val, state) => {
            let nv = replace_sloads_inline_inner(val, known, cache);
            let ns = replace_sloads_inline_inner(state, known, cache);
            Rc::new(EvmExpr::RegionStore(*id, *field_idx, nv, ns))
        }
        EvmExpr::RegionLoad(id, field_idx, state) => {
            let ns = replace_sloads_inline_inner(state, known, cache);
            Rc::new(EvmExpr::RegionLoad(*id, *field_idx, ns))
        }
    }
}

/// Check if an expression (non-top-level `SStore`) might modify storage.
fn might_modify_storage(expr: &RcExpr) -> bool {
    let mut visited = HashSet::new();
    might_modify_storage_inner(expr, &mut visited)
}

fn might_modify_storage_inner(expr: &RcExpr, visited: &mut HashSet<usize>) -> bool {
    let ptr = Rc::as_ptr(expr) as usize;
    if !visited.insert(ptr) {
        return false;
    }

    match expr.as_ref() {
        EvmExpr::ExtCall(..)
        | EvmExpr::InlineAsm(..)
        | EvmExpr::Top(EvmTernaryOp::SStore | EvmTernaryOp::TStore, ..) => true,
        EvmExpr::If(c, i, t, e) => {
            might_modify_storage_inner(c, visited)
                || might_modify_storage_inner(i, visited)
                || might_modify_storage_inner(t, visited)
                || might_modify_storage_inner(e, visited)
        }
        EvmExpr::DoWhile(i, b) => {
            might_modify_storage_inner(i, visited) || might_modify_storage_inner(b, visited)
        }
        EvmExpr::LetBind(_, init, body) => {
            might_modify_storage_inner(init, visited) || might_modify_storage_inner(body, visited)
        }
        EvmExpr::Concat(a, b) => {
            might_modify_storage_inner(a, visited) || might_modify_storage_inner(b, visited)
        }
        _ => false,
    }
}

/// Check if an expression might observe storage state (for dead store elimination).
fn might_observe_storage(expr: &RcExpr) -> bool {
    match expr.as_ref() {
        // External calls can read any storage; branches/loops might contain storage reads
        EvmExpr::ExtCall(..) | EvmExpr::If(..) | EvmExpr::DoWhile(..) => true,
        // Top-level SStore doesn't "observe" — it writes; everything else doesn't observe
        _ => false,
    }
}

/// Collect all SLoad/TLoad slot keys anywhere in an expression tree (deep recursive).
fn collect_sload_slots_deep(expr: &RcExpr) -> Vec<SlotKey> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    collect_sload_slots_inner(expr, &mut result, &mut visited);
    result
}

fn collect_sload_slots_inner(expr: &RcExpr, out: &mut Vec<SlotKey>, visited: &mut HashSet<usize>) {
    let ptr = Rc::as_ptr(expr) as usize;
    if !visited.insert(ptr) {
        return;
    }

    match expr.as_ref() {
        EvmExpr::Bop(op @ (EvmBinaryOp::SLoad | EvmBinaryOp::TLoad), slot, _) => {
            let kind = if *op == EvmBinaryOp::SLoad {
                StorageKind::Persistent
            } else {
                StorageKind::Transient
            };
            if let Some(sv) = const_slot_value(slot) {
                out.push(SlotKey {
                    kind,
                    slot_value: sv,
                });
            }
            collect_sload_slots_inner(slot, out, visited);
        }
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) => {
            collect_sload_slots_inner(a, out, visited);
            collect_sload_slots_inner(b, out, visited);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => collect_sload_slots_inner(a, out, visited),
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_sload_slots_inner(a, out, visited);
            collect_sload_slots_inner(b, out, visited);
            collect_sload_slots_inner(c, out, visited);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_sload_slots_inner(c, out, visited);
            collect_sload_slots_inner(i, out, visited);
            collect_sload_slots_inner(t, out, visited);
            collect_sload_slots_inner(e, out, visited);
        }
        EvmExpr::DoWhile(i, b) => {
            collect_sload_slots_inner(i, out, visited);
            collect_sload_slots_inner(b, out, visited);
        }
        EvmExpr::LetBind(_, init, body) => {
            collect_sload_slots_inner(init, out, visited);
            collect_sload_slots_inner(body, out, visited);
        }
        EvmExpr::VarStore(_, val) => collect_sload_slots_inner(val, out, visited),
        EvmExpr::Log(_, topics, data_offset, data_size, state) => {
            for t in topics {
                collect_sload_slots_inner(t, out, visited);
            }
            collect_sload_slots_inner(data_offset, out, visited);
            collect_sload_slots_inner(data_size, out, visited);
            collect_sload_slots_inner(state, out, visited);
        }
        EvmExpr::EnvRead(_, s) => collect_sload_slots_inner(s, out, visited),
        EvmExpr::EnvRead1(_, a, s) => {
            collect_sload_slots_inner(a, out, visited);
            collect_sload_slots_inner(s, out, visited);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            for x in [a, b, c, d, e, f, g] {
                collect_sload_slots_inner(x, out, visited);
            }
        }
        EvmExpr::Call(_, args) => {
            for arg in args {
                collect_sload_slots_inner(arg, out, visited);
            }
        }
        EvmExpr::Function(_, _, _, body) => collect_sload_slots_inner(body, out, visited),
        _ => {}
    }
}

/// Rebuild a Concat chain from a list of statements.
fn rebuild_concat(stmts: &[RcExpr]) -> RcExpr {
    assert!(!stmts.is_empty());
    let mut it = stmts.iter();
    let mut acc = Rc::clone(it.next().unwrap());
    for next in it {
        acc = Rc::new(EvmExpr::Concat(acc, Rc::clone(next)));
    }
    acc
}

fn opt_ctx() -> EvmContext {
    EvmContext::InFunction("__opt__".to_owned())
}

fn state_placeholder() -> RcExpr {
    Rc::new(EvmExpr::Arg(EvmType::Base(EvmBaseType::StateT), opt_ctx()))
}

fn unit_empty() -> RcExpr {
    Rc::new(EvmExpr::Empty(EvmType::Base(EvmBaseType::UnitT), opt_ctx()))
}

fn const_slot(val: i64) -> RcExpr {
    Rc::new(EvmExpr::Const(
        EvmConstant::SmallInt(val),
        EvmType::Base(EvmBaseType::UIntT(256)),
        opt_ctx(),
    ))
}

/// Top-down walk looking for while-loop patterns to optimize.
fn hoist_expr(expr: &RcExpr, counter: &mut usize) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::Concat(left, rest) => {
            let empty_pre = HashMap::new();

            // Pattern 1: Concat(If(cond, inputs, DoWhile(...), Empty), rest)
            if let Some((cond, inputs, di, db)) = match_while_if(left) {
                if let Some(result) =
                    try_hoist_while(cond, inputs, di, db, rest, counter, &empty_pre)
                {
                    return result;
                }
            }

            // Pattern 2: Concat(Concat(pre, If(cond, inputs, DoWhile(...), Empty)), rest)
            // Common case: pre-loop stores precede the while loop in a Concat chain.
            // We scan `pre` for SStores to slots that the loop uses, forwarding
            // the stored value into the LetBind init and removing the redundant SStore.
            if let EvmExpr::Concat(pre, while_if) = left.as_ref() {
                if let Some((cond, inputs, di, db)) = match_while_if(while_if) {
                    // First, collect which slots the loop uses so we know what to look for
                    let mut loop_slots = HashMap::new();
                    collect_storage_slots(db, &mut loop_slots);
                    collect_storage_slots(cond, &mut loop_slots);
                    loop_slots.retain(|_, usage| usage.has_load);

                    // Extract pre-loop SStore values for those slots
                    let (stripped_pre, pre_store_vals) = extract_pre_stores(pre, &loop_slots);

                    if let Some(hoisted) =
                        try_hoist_while(cond, inputs, di, db, rest, counter, &pre_store_vals)
                    {
                        if let Some(stripped) = stripped_pre {
                            return Rc::new(EvmExpr::Concat(
                                hoist_expr(&stripped, counter),
                                hoisted,
                            ));
                        }
                        // All pre-stores were consumed
                        return hoisted;
                    }
                }
            }

            let new_left = hoist_expr(left, counter);
            let new_rest = hoist_expr(rest, counter);
            if Rc::ptr_eq(&new_left, left) && Rc::ptr_eq(&new_rest, rest) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Concat(new_left, new_rest))
            }
        }
        EvmExpr::LetBind(name, init, body) => {
            let new_init = hoist_expr(init, counter);
            let new_body = hoist_expr(body, counter);
            if Rc::ptr_eq(&new_init, init) && Rc::ptr_eq(&new_body, body) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::LetBind(name.clone(), new_init, new_body))
            }
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let new_body = hoist_expr(body, counter);
            if Rc::ptr_eq(&new_body, body) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Function(
                    name.clone(),
                    in_ty.clone(),
                    out_ty.clone(),
                    new_body,
                ))
            }
        }
        EvmExpr::If(c, i, t, e) => {
            let nc = hoist_expr(c, counter);
            let ni = hoist_expr(i, counter);
            let nt = hoist_expr(t, counter);
            let ne = hoist_expr(e, counter);
            if Rc::ptr_eq(&nc, c) && Rc::ptr_eq(&ni, i) && Rc::ptr_eq(&nt, t) && Rc::ptr_eq(&ne, e)
            {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::If(nc, ni, nt, ne))
            }
        }
        EvmExpr::DoWhile(inputs, body) => {
            let ni = hoist_expr(inputs, counter);
            let nb = hoist_expr(body, counter);
            if Rc::ptr_eq(&ni, inputs) && Rc::ptr_eq(&nb, body) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::DoWhile(ni, nb))
            }
        }
        _ => Rc::clone(expr),
    }
}

/// Attempt to hoist storage ops from a while-loop pattern.
///
/// Pattern: `Concat(If(cond, inputs, DoWhile(di, db), Empty), rest)`
///
/// `pre_stores` maps slot keys to their pre-loop initialization values,
/// extracted from `SStores` that precede the loop.
fn try_hoist_while(
    cond: &RcExpr,
    inputs: &RcExpr,
    di: &RcExpr,
    db: &RcExpr,
    rest: &RcExpr,
    counter: &mut usize,
    pre_stores: &HashMap<SlotKey, RcExpr>,
) -> Option<RcExpr> {
    // Reject loops with external calls or nested loops
    if has_disqualifying_ops(db) {
        return None;
    }

    // Collect storage slots used in the DoWhile body (includes re-evaluated condition)
    let mut slots = HashMap::new();
    collect_storage_slots(db, &mut slots);
    // Also scan the initial condition
    collect_storage_slots(cond, &mut slots);

    // Only hoist slots that have at least one load
    slots.retain(|_, usage| usage.has_load);
    if slots.is_empty() {
        return None;
    }

    // Sort for deterministic output
    let mut slot_keys: Vec<SlotKey> = slots.keys().cloned().collect();
    slot_keys.sort_by_key(|k| (k.kind as u8, k.slot_value));

    // Generate variable names
    let slot_vars: Vec<(SlotKey, String)> = slot_keys
        .iter()
        .map(|key| {
            let prefix = if key.kind == StorageKind::Transient {
                "t"
            } else {
                "s"
            };
            let name = format!("__hoist_{}{}_{}", prefix, key.slot_value, *counter);
            *counter += 1;
            (key.clone(), name)
        })
        .collect();

    // Rewrite the DoWhile body: SLoad→Var, SStore→VarStore
    let mut new_db = Rc::clone(db);
    for (key, name) in &slot_vars {
        new_db = replace_storage(&new_db, key, name, true);
    }

    // Rewrite the initial condition: SLoad→Var
    let mut new_cond = Rc::clone(cond);
    for (key, name) in &slot_vars {
        new_cond = replace_storage(&new_cond, key, name, false);
    }

    // Rewrite the post-loop continuation: SLoad→Var (no SStore replacement)
    let mut new_rest = Rc::clone(rest);
    for (key, name) in &slot_vars {
        new_rest = replace_storage(&new_rest, key, name, false);
    }

    let state = state_placeholder();

    // Build write-backs + rest chain (write-backs go before post-loop code)
    let mut after_loop = new_rest;
    for (key, name) in slot_vars.iter().rev() {
        if slots[key].has_store {
            let op = match key.kind {
                StorageKind::Persistent => EvmTernaryOp::SStore,
                StorageKind::Transient => EvmTernaryOp::TStore,
            };
            let wb = Rc::new(EvmExpr::Top(
                op,
                const_slot(key.slot_value),
                Rc::new(EvmExpr::Var(name.clone())),
                Rc::clone(&state),
            ));
            after_loop = Rc::new(EvmExpr::Concat(wb, after_loop));
        }
    }

    // Build the new If/DoWhile
    let new_dowhile = Rc::new(EvmExpr::DoWhile(Rc::clone(di), new_db));
    let new_if = Rc::new(EvmExpr::If(
        new_cond,
        Rc::clone(inputs),
        new_dowhile,
        unit_empty(),
    ));
    let mut result = Rc::new(EvmExpr::Concat(new_if, after_loop));

    // Wrap in LetBinds (outermost = first slot)
    // If a pre-loop SStore wrote a known value to this slot, use that
    // value directly instead of emitting an SLoad.
    for (key, name) in slot_vars.iter().rev() {
        let init = pre_stores.get(key).map_or_else(
            || {
                let load_op = match key.kind {
                    StorageKind::Persistent => EvmBinaryOp::SLoad,
                    StorageKind::Transient => EvmBinaryOp::TLoad,
                };
                Rc::new(EvmExpr::Bop(
                    load_op,
                    const_slot(key.slot_value),
                    Rc::clone(&state),
                ))
            },
            Rc::clone,
        );
        result = Rc::new(EvmExpr::LetBind(name.clone(), init, result));
    }

    Some(result)
}

/// Extract pre-loop `SStore` values from a Concat chain.
///
/// Walks a right-leaning Concat chain of `SStore` nodes, collecting values
/// for slots that appear in `loop_slots`. Returns the remaining (non-consumed)
/// pre-code and the map of forwarded values.
///
/// Only removes a pre-store when the loop also writes to that slot
/// (i.e., `has_store=true`), since the write-back will cover it.
fn extract_pre_stores(
    pre: &RcExpr,
    loop_slots: &HashMap<SlotKey, SlotUsage>,
) -> (Option<RcExpr>, HashMap<SlotKey, RcExpr>) {
    let mut forwarded = HashMap::new();

    // Flatten the Concat chain into a list of statements
    let mut stmts = Vec::new();
    flatten_concat(pre, &mut stmts);

    // Walk statements, extracting matching SStores
    let mut remaining = Vec::new();
    for stmt in &stmts {
        if let Some((key, val)) = match_sstore_const_slot(stmt) {
            if let Some(usage) = loop_slots.get(&key) {
                // Forward the value
                forwarded.insert(key.clone(), Rc::clone(&val));

                // Only remove the pre-store if the loop writes back
                if usage.has_store {
                    continue; // consume this SStore
                }
            }
        }
        remaining.push(Rc::clone(*stmt));
    }

    let pre_expr = if remaining.is_empty() {
        None
    } else {
        // Rebuild Concat chain from remaining
        let mut it = remaining.into_iter();
        let mut acc = it.next().unwrap();
        for next in it {
            acc = Rc::new(EvmExpr::Concat(acc, next));
        }
        Some(acc)
    };

    (pre_expr, forwarded)
}

/// Flatten a Concat chain into a list of leaf expressions.
fn flatten_concat<'a>(expr: &'a RcExpr, out: &mut Vec<&'a RcExpr>) {
    if let EvmExpr::Concat(left, right) = expr.as_ref() {
        flatten_concat(left, out);
        flatten_concat(right, out);
    } else {
        out.push(expr);
    }
}

/// Match `Top(SStore, Const(slot), value, state)` and return (`SlotKey`, value).
fn match_sstore_const_slot(expr: &RcExpr) -> Option<(SlotKey, RcExpr)> {
    match expr.as_ref() {
        EvmExpr::Top(op @ (EvmTernaryOp::SStore | EvmTernaryOp::TStore), slot, val, _state) => {
            let kind = if *op == EvmTernaryOp::SStore {
                StorageKind::Persistent
            } else {
                StorageKind::Transient
            };
            let sv = const_slot_value(slot)?;
            Some((
                SlotKey {
                    kind,
                    slot_value: sv,
                },
                Rc::clone(val),
            ))
        }
        _ => None,
    }
}

fn is_empty(expr: &RcExpr) -> bool {
    matches!(expr.as_ref(), EvmExpr::Empty(..))
}

/// Check if an expression is `If(cond, inputs, DoWhile(di, db), Empty)` — a while-loop pattern.
fn match_while_if(expr: &RcExpr) -> Option<(&RcExpr, &RcExpr, &RcExpr, &RcExpr)> {
    if let EvmExpr::If(cond, inputs, then_body, else_body) = expr.as_ref() {
        if let EvmExpr::DoWhile(di, db) = then_body.as_ref() {
            if is_empty(else_body) {
                return Some((cond, inputs, di, db));
            }
        }
    }
    None
}

fn const_slot_value(expr: &RcExpr) -> Option<i64> {
    match expr.as_ref() {
        EvmExpr::Const(EvmConstant::SmallInt(v), _, _) => Some(*v),
        _ => None,
    }
}

/// Check for operations that disqualify a loop from hoisting.
fn has_disqualifying_ops(expr: &RcExpr) -> bool {
    match expr.as_ref() {
        EvmExpr::ExtCall(..) | EvmExpr::DoWhile(..) => true, // ExtCall or nested loops — bail
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) => {
            has_disqualifying_ops(a) || has_disqualifying_ops(b)
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => has_disqualifying_ops(a),
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            has_disqualifying_ops(a) || has_disqualifying_ops(b) || has_disqualifying_ops(c)
        }
        EvmExpr::If(c, i, t, e) => {
            has_disqualifying_ops(c)
                || has_disqualifying_ops(i)
                || has_disqualifying_ops(t)
                || has_disqualifying_ops(e)
        }
        EvmExpr::LetBind(_, init, body) => {
            has_disqualifying_ops(init) || has_disqualifying_ops(body)
        }
        EvmExpr::VarStore(_, val) => has_disqualifying_ops(val),
        EvmExpr::Log(_, topics, data_offset, data_size, state) => {
            topics.iter().any(has_disqualifying_ops)
                || has_disqualifying_ops(data_offset)
                || has_disqualifying_ops(data_size)
                || has_disqualifying_ops(state)
        }
        EvmExpr::EnvRead(_, s) => has_disqualifying_ops(s),
        EvmExpr::EnvRead1(_, a, s) => has_disqualifying_ops(a) || has_disqualifying_ops(s),
        EvmExpr::Call(_, args) => args.iter().any(has_disqualifying_ops),
        EvmExpr::Function(_, _, _, body) => has_disqualifying_ops(body),
        _ => false,
    }
}

/// Scan an expression for SLoad/SStore/TLoad/TStore with constant slot numbers.
fn collect_storage_slots(expr: &RcExpr, result: &mut HashMap<SlotKey, SlotUsage>) {
    match expr.as_ref() {
        EvmExpr::Bop(op @ (EvmBinaryOp::SLoad | EvmBinaryOp::TLoad), slot, _state) => {
            if let Some(sv) = const_slot_value(slot) {
                let kind = if *op == EvmBinaryOp::SLoad {
                    StorageKind::Persistent
                } else {
                    StorageKind::Transient
                };
                result
                    .entry(SlotKey {
                        kind,
                        slot_value: sv,
                    })
                    .or_default()
                    .has_load = true;
            }
            collect_storage_slots(slot, result);
        }
        EvmExpr::Top(op @ (EvmTernaryOp::SStore | EvmTernaryOp::TStore), slot, val, _state) => {
            if let Some(sv) = const_slot_value(slot) {
                let kind = if *op == EvmTernaryOp::SStore {
                    StorageKind::Persistent
                } else {
                    StorageKind::Transient
                };
                result
                    .entry(SlotKey {
                        kind,
                        slot_value: sv,
                    })
                    .or_default()
                    .has_store = true;
            }
            collect_storage_slots(slot, result);
            collect_storage_slots(val, result);
        }
        // Generic recursion
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) => {
            collect_storage_slots(a, result);
            collect_storage_slots(b, result);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => {
            collect_storage_slots(a, result);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_storage_slots(a, result);
            collect_storage_slots(b, result);
            collect_storage_slots(c, result);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_storage_slots(c, result);
            collect_storage_slots(i, result);
            collect_storage_slots(t, result);
            collect_storage_slots(e, result);
        }
        EvmExpr::DoWhile(inputs, body) => {
            collect_storage_slots(inputs, result);
            collect_storage_slots(body, result);
        }
        EvmExpr::LetBind(_, init, body) => {
            collect_storage_slots(init, result);
            collect_storage_slots(body, result);
        }
        EvmExpr::VarStore(_, val) => {
            collect_storage_slots(val, result);
        }
        EvmExpr::Log(_, topics, data_offset, data_size, state) => {
            for t in topics {
                collect_storage_slots(t, result);
            }
            collect_storage_slots(data_offset, result);
            collect_storage_slots(data_size, result);
            collect_storage_slots(state, result);
        }
        EvmExpr::EnvRead(_, s) => collect_storage_slots(s, result),
        EvmExpr::EnvRead1(_, a, s) => {
            collect_storage_slots(a, result);
            collect_storage_slots(s, result);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            for x in [a, b, c, d, e, f, g] {
                collect_storage_slots(x, result);
            }
        }
        EvmExpr::Call(_, args) => {
            for arg in args {
                collect_storage_slots(arg, result);
            }
        }
        EvmExpr::Function(_, _, _, body) => collect_storage_slots(body, result),
        _ => {}
    }
}

/// Replace storage operations with local variable operations.
///
/// - SLoad/TLoad with matching slot → `Var(var_name)`
/// - If `replace_stores`: SStore/TStore with matching slot → `VarStore(var_name`, val)
fn replace_storage(expr: &RcExpr, key: &SlotKey, var_name: &str, replace_stores: bool) -> RcExpr {
    match expr.as_ref() {
        // SLoad/TLoad → Var
        EvmExpr::Bop(op @ (EvmBinaryOp::SLoad | EvmBinaryOp::TLoad), slot, _state) => {
            let kind = if *op == EvmBinaryOp::SLoad {
                StorageKind::Persistent
            } else {
                StorageKind::Transient
            };
            if kind == key.kind {
                if let Some(sv) = const_slot_value(slot) {
                    if sv == key.slot_value {
                        return Rc::new(EvmExpr::Var(var_name.to_owned()));
                    }
                }
            }
            let a = replace_storage(slot, key, var_name, replace_stores);
            Rc::new(EvmExpr::Bop(*op, a, Rc::clone(_state)))
        }
        // SStore/TStore → VarStore (only when replace_stores is true)
        EvmExpr::Top(op @ (EvmTernaryOp::SStore | EvmTernaryOp::TStore), slot, val, state)
            if replace_stores =>
        {
            let kind = if *op == EvmTernaryOp::SStore {
                StorageKind::Persistent
            } else {
                StorageKind::Transient
            };
            if kind == key.kind {
                if let Some(sv) = const_slot_value(slot) {
                    if sv == key.slot_value {
                        let new_val = replace_storage(val, key, var_name, replace_stores);
                        return Rc::new(EvmExpr::VarStore(var_name.to_owned(), new_val));
                    }
                }
            }
            let a = replace_storage(slot, key, var_name, replace_stores);
            let b = replace_storage(val, key, var_name, replace_stores);
            Rc::new(EvmExpr::Top(*op, a, b, Rc::clone(state)))
        }
        // Generic recursion for all other nodes
        EvmExpr::Bop(op, a, b) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            let nb = replace_storage(b, key, var_name, replace_stores);
            Rc::new(EvmExpr::Bop(*op, na, nb))
        }
        EvmExpr::Uop(op, a) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            Rc::new(EvmExpr::Uop(*op, na))
        }
        EvmExpr::Top(op, a, b, c) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            let nb = replace_storage(b, key, var_name, replace_stores);
            let nc = replace_storage(c, key, var_name, replace_stores);
            Rc::new(EvmExpr::Top(*op, na, nb, nc))
        }
        EvmExpr::Get(a, idx) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            Rc::new(EvmExpr::Get(na, *idx))
        }
        EvmExpr::Concat(a, b) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            let nb = replace_storage(b, key, var_name, replace_stores);
            Rc::new(EvmExpr::Concat(na, nb))
        }
        EvmExpr::If(c, i, t, e) => {
            let nc = replace_storage(c, key, var_name, replace_stores);
            let ni = replace_storage(i, key, var_name, replace_stores);
            let nt = replace_storage(t, key, var_name, replace_stores);
            let ne = replace_storage(e, key, var_name, replace_stores);
            Rc::new(EvmExpr::If(nc, ni, nt, ne))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let ni = replace_storage(inputs, key, var_name, replace_stores);
            let nb = replace_storage(body, key, var_name, replace_stores);
            Rc::new(EvmExpr::DoWhile(ni, nb))
        }
        EvmExpr::LetBind(name, init, body) => {
            let ni = replace_storage(init, key, var_name, replace_stores);
            let nb = replace_storage(body, key, var_name, replace_stores);
            Rc::new(EvmExpr::LetBind(name.clone(), ni, nb))
        }
        EvmExpr::VarStore(name, val) => {
            let nv = replace_storage(val, key, var_name, replace_stores);
            Rc::new(EvmExpr::VarStore(name.clone(), nv))
        }
        EvmExpr::Revert(a, b, c) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            let nb = replace_storage(b, key, var_name, replace_stores);
            let nc = replace_storage(c, key, var_name, replace_stores);
            Rc::new(EvmExpr::Revert(na, nb, nc))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            let nb = replace_storage(b, key, var_name, replace_stores);
            let nc = replace_storage(c, key, var_name, replace_stores);
            Rc::new(EvmExpr::ReturnOp(na, nb, nc))
        }
        EvmExpr::EnvRead(op, s) => {
            let ns = replace_storage(s, key, var_name, replace_stores);
            Rc::new(EvmExpr::EnvRead(*op, ns))
        }
        EvmExpr::EnvRead1(op, a, s) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            let ns = replace_storage(s, key, var_name, replace_stores);
            Rc::new(EvmExpr::EnvRead1(*op, na, ns))
        }
        EvmExpr::Log(count, topics, data_offset, data_size, state) => {
            let ts: Vec<_> = topics
                .iter()
                .map(|t| replace_storage(t, key, var_name, replace_stores))
                .collect();
            let noff = replace_storage(data_offset, key, var_name, replace_stores);
            let nsz = replace_storage(data_size, key, var_name, replace_stores);
            let ns = replace_storage(state, key, var_name, replace_stores);
            Rc::new(EvmExpr::Log(*count, ts, noff, nsz, ns))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let na = replace_storage(a, key, var_name, replace_stores);
            let nb = replace_storage(b, key, var_name, replace_stores);
            let nc = replace_storage(c, key, var_name, replace_stores);
            let nd = replace_storage(d, key, var_name, replace_stores);
            let ne = replace_storage(e, key, var_name, replace_stores);
            let nf = replace_storage(f, key, var_name, replace_stores);
            let ng = replace_storage(g, key, var_name, replace_stores);
            Rc::new(EvmExpr::ExtCall(na, nb, nc, nd, ne, nf, ng))
        }
        EvmExpr::Call(name, args) => {
            let new_args: Vec<_> = args
                .iter()
                .map(|a| replace_storage(a, key, var_name, replace_stores))
                .collect();
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let nb = replace_storage(body, key, var_name, replace_stores);
            Rc::new(EvmExpr::Function(
                name.clone(),
                in_ty.clone(),
                out_ty.clone(),
                nb,
            ))
        }
        EvmExpr::DynAlloc(size) => {
            let ns = replace_storage(size, key, var_name, replace_stores);
            Rc::new(EvmExpr::DynAlloc(ns))
        }
        EvmExpr::AllocRegion(id, num_fields, is_dynamic) => {
            let nf = replace_storage(num_fields, key, var_name, replace_stores);
            Rc::new(EvmExpr::AllocRegion(*id, nf, *is_dynamic))
        }
        EvmExpr::RegionStore(id, field_idx, val, state) => {
            let nv = replace_storage(val, key, var_name, replace_stores);
            let ns = replace_storage(state, key, var_name, replace_stores);
            Rc::new(EvmExpr::RegionStore(*id, *field_idx, nv, ns))
        }
        EvmExpr::RegionLoad(id, field_idx, state) => {
            let ns = replace_storage(state, key, var_name, replace_stores);
            Rc::new(EvmExpr::RegionLoad(*id, *field_idx, ns))
        }
        // Leaf nodes — no children
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => Rc::clone(expr),
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs.iter().map(forward_stores_expr).collect();
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_helpers;

    fn ctx() -> EvmContext {
        EvmContext::InFunction("test".to_owned())
    }

    #[test]
    fn test_no_hoist_without_loop() {
        // A simple SLoad outside a loop should not be touched
        let expr = ast_helpers::sload(ast_helpers::const_int(0, ctx()), state_placeholder());
        let mut counter = 0;
        let result = hoist_expr(&expr, &mut counter);
        assert_eq!(counter, 0, "no hoisting should occur");
        assert!(matches!(
            result.as_ref(),
            EvmExpr::Bop(EvmBinaryOp::SLoad, _, _)
        ));
    }

    #[test]
    fn test_hoist_sload_in_while() {
        // Build: Concat(If(SLoad(0) != 0, Empty, DoWhile(Empty, Concat(SStore(0, SLoad(0)+1), SLoad(0)!=0)), Empty), rest)
        let slot = ast_helpers::const_int(0, ctx());
        let state = state_placeholder();
        let sload_0 = ast_helpers::sload(Rc::clone(&slot), Rc::clone(&state));

        // Condition: SLoad(0) != 0
        let cond = Rc::new(EvmExpr::Uop(
            crate::schema::EvmUnaryOp::IsZero,
            Rc::new(EvmExpr::Uop(
                crate::schema::EvmUnaryOp::IsZero,
                Rc::clone(&sload_0),
            )),
        ));

        // Loop body: SStore(0, SLoad(0) + 1, state)
        let body = Rc::new(EvmExpr::Top(
            EvmTernaryOp::SStore,
            slot,
            ast_helpers::add(Rc::clone(&sload_0), ast_helpers::const_int(1, ctx())),
            Rc::clone(&state),
        ));

        let dowhile = Rc::new(EvmExpr::DoWhile(
            unit_empty(),
            Rc::new(EvmExpr::Concat(body, Rc::clone(&cond))),
        ));

        let if_node = Rc::new(EvmExpr::If(cond, unit_empty(), dowhile, unit_empty()));

        // Post-loop: return SLoad(0)
        let rest = Rc::new(EvmExpr::ReturnOp(
            sload_0,
            ast_helpers::const_int(32, ctx()),
            state,
        ));

        let expr = Rc::new(EvmExpr::Concat(if_node, rest));

        let mut counter = 0;
        let result = hoist_expr(&expr, &mut counter);

        // Should be wrapped in a LetBind
        assert!(
            matches!(result.as_ref(), EvmExpr::LetBind(..)),
            "expected LetBind, got: {result:?}"
        );

        // The DoWhile body should contain VarStore, not SStore
        fn contains_var_store(e: &EvmExpr) -> bool {
            match e {
                EvmExpr::VarStore(..) => true,
                EvmExpr::Concat(a, b) => contains_var_store(a) || contains_var_store(b),
                EvmExpr::DoWhile(_, b) | EvmExpr::If(_, _, b, _) | EvmExpr::LetBind(_, _, b) => {
                    contains_var_store(b)
                }
                _ => false,
            }
        }
        assert!(
            contains_var_store(&result),
            "expected VarStore in hoisted loop"
        );
    }
}
