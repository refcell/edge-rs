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

use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

use crate::schema::{
    EvmBaseType, EvmConstant, EvmContext, EvmExpr, EvmProgram, EvmType, RcExpr,
};

/// Scope tree node for memory region allocation.
///
/// Models control-flow mutual exclusivity so that regions in different
/// branches of an `If` can share the same memory offsets.
enum RegionScope {
    /// Children execute sequentially — all must be non-overlapping.
    Sequential(Vec<Self>),
    /// Children are mutually exclusive (If branches) — can share base offset.
    Exclusive(Vec<Self>),
    /// A single memory region allocation.
    Leaf { region_id: i64, size_bytes: usize },
}

/// Assign concrete memory offsets to all `MemRegion` nodes in an expression.
///
/// Returns `(rewritten_expr, memory_high_water)` where `memory_high_water` is
/// the first free byte offset after all allocated regions.
pub fn assign_memory_offsets(
    expr: &RcExpr,
    region_var_map: &indexmap::IndexMap<i64, String>,
) -> (RcExpr, usize) {
    let scope = collect_region_scopes(expr);
    let scope = simplify_scope(scope);

    let mut assignments = BTreeMap::new();
    let hw = assign_scoped_offsets(&scope, 0, &mut assignments);

    if assignments.is_empty() && region_var_map.is_empty() {
        return (Rc::clone(expr), 0);
    }

    tracing::debug!("  mem_region hw={hw} ({} regions)", assignments.len());

    let ctx = RegionResolveCtx {
        assignments,
        region_var_map: region_var_map.clone(),
    };
    let rewritten = replace_regions(expr, &ctx);
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

/// Check whether a subtree contains any MemRegion nodes (memoized by Rc pointer).
fn has_mem_region(expr: &RcExpr, cache: &mut HashMap<usize, bool>) -> bool {
    let ptr = Rc::as_ptr(expr) as usize;
    if let Some(&result) = cache.get(&ptr) {
        return result;
    }
    let result = match expr.as_ref() {
        EvmExpr::MemRegion(..) => true,
        EvmExpr::Concat(a, b)
        | EvmExpr::Bop(_, a, b)
        | EvmExpr::DoWhile(a, b)
        | EvmExpr::EnvRead1(_, a, b) => {
            has_mem_region(a, cache) || has_mem_region(b, cache)
        }
        EvmExpr::If(a, b, c, d) => {
            has_mem_region(a, cache)
                || has_mem_region(b, cache)
                || has_mem_region(c, cache)
                || has_mem_region(d, cache)
        }
        EvmExpr::LetBind(_, init, body) => {
            has_mem_region(init, cache) || has_mem_region(body, cache)
        }
        EvmExpr::Top(_, a, b, c)
        | EvmExpr::Revert(a, b, c)
        | EvmExpr::ReturnOp(a, b, c) => {
            has_mem_region(a, cache)
                || has_mem_region(b, cache)
                || has_mem_region(c, cache)
        }
        EvmExpr::Function(_, _, _, body) => has_mem_region(body, cache),
        EvmExpr::Uop(_, a)
        | EvmExpr::Get(a, _)
        | EvmExpr::VarStore(_, a)
        | EvmExpr::DynAlloc(a)
        | EvmExpr::AllocRegion(_, a, _)
        | EvmExpr::EnvRead(_, a) => has_mem_region(a, cache),
        EvmExpr::RegionStore(_, _, val, state) => {
            has_mem_region(val, cache) || has_mem_region(state, cache)
        }
        EvmExpr::RegionLoad(_, _, state) => has_mem_region(state, cache),
        EvmExpr::Log(_, topics, d, s, st) => {
            topics.iter().any(|t| has_mem_region(t, cache))
                || has_mem_region(d, cache)
                || has_mem_region(s, cache)
                || has_mem_region(st, cache)
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            [a, b, c, d, e, f, g]
                .iter()
                .any(|x| has_mem_region(x, cache))
        }
        EvmExpr::Call(_, args) => args.iter().any(|a| has_mem_region(a, cache)),
        EvmExpr::InlineAsm(inputs, ..) => inputs.iter().any(|a| has_mem_region(a, cache)),
        _ => false,
    };
    cache.insert(ptr, result);
    result
}

/// Build a scope tree from an IR expression.
fn collect_region_scopes(expr: &RcExpr) -> RegionScope {
    let mut hmr_cache = HashMap::new();
    collect_region_scopes_inner(expr, &mut hmr_cache)
}

fn collect_region_scopes_inner(
    expr: &RcExpr,
    hmr_cache: &mut HashMap<usize, bool>,
) -> RegionScope {
    // Fast path: if this subtree contains no MemRegion nodes, skip traversal.
    if !has_mem_region(expr, hmr_cache) {
        return RegionScope::Sequential(vec![]);
    }

    match expr.as_ref() {
        EvmExpr::MemRegion(id, size_words) => RegionScope::Leaf {
            region_id: *id,
            size_bytes: (*size_words as usize) * 32,
        },

        // If: condition+inputs sequential, then/else exclusive
        EvmExpr::If(cond, inputs, then_br, else_br) => RegionScope::Sequential(vec![
            collect_region_scopes_inner(cond, hmr_cache),
            collect_region_scopes_inner(inputs, hmr_cache),
            RegionScope::Exclusive(vec![
                collect_region_scopes_inner(then_br, hmr_cache),
                collect_region_scopes_inner(else_br, hmr_cache),
            ]),
        ]),

        // Sequential composition
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) => RegionScope::Sequential(vec![
            collect_region_scopes_inner(a, hmr_cache),
            collect_region_scopes_inner(b, hmr_cache),
        ]),
        EvmExpr::LetBind(_, init, body) => RegionScope::Sequential(vec![
            collect_region_scopes_inner(init, hmr_cache),
            collect_region_scopes_inner(body, hmr_cache),
        ]),
        EvmExpr::DoWhile(inputs, body) => RegionScope::Sequential(vec![
            collect_region_scopes_inner(inputs, hmr_cache),
            collect_region_scopes_inner(body, hmr_cache),
        ]),
        EvmExpr::Function(_, _, _, body) => collect_region_scopes_inner(body, hmr_cache),

        // Ternary children — sequential
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            RegionScope::Sequential(vec![
                collect_region_scopes_inner(a, hmr_cache),
                collect_region_scopes_inner(b, hmr_cache),
                collect_region_scopes_inner(c, hmr_cache),
            ])
        }

        // Unary children
        EvmExpr::Uop(_, a)
        | EvmExpr::Get(a, _)
        | EvmExpr::VarStore(_, a)
        | EvmExpr::DynAlloc(a)
        | EvmExpr::AllocRegion(_, a, _) => collect_region_scopes_inner(a, hmr_cache),

        // Multi-child nodes
        EvmExpr::Log(_, topics, d, s, st) => {
            let mut children: Vec<_> = topics
                .iter()
                .map(|t| collect_region_scopes_inner(t, hmr_cache))
                .collect();
            children.push(collect_region_scopes_inner(d, hmr_cache));
            children.push(collect_region_scopes_inner(s, hmr_cache));
            children.push(collect_region_scopes_inner(st, hmr_cache));
            RegionScope::Sequential(children)
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => RegionScope::Sequential(
            [a, b, c, d, e, f, g]
                .into_iter()
                .map(|x| collect_region_scopes_inner(x, hmr_cache))
                .collect(),
        ),
        EvmExpr::Call(_, args) => RegionScope::Sequential(
            args.iter()
                .map(|a| collect_region_scopes_inner(a, hmr_cache))
                .collect(),
        ),
        EvmExpr::RegionStore(_, _, val, state) => RegionScope::Sequential(vec![
            collect_region_scopes_inner(val, hmr_cache),
            collect_region_scopes_inner(state, hmr_cache),
        ]),
        EvmExpr::RegionLoad(_, _, state) => collect_region_scopes_inner(state, hmr_cache),
        EvmExpr::EnvRead(_, s) => collect_region_scopes_inner(s, hmr_cache),
        EvmExpr::EnvRead1(_, a, s) => RegionScope::Sequential(vec![
            collect_region_scopes_inner(a, hmr_cache),
            collect_region_scopes_inner(s, hmr_cache),
        ]),
        EvmExpr::InlineAsm(inputs, ..) => RegionScope::Sequential(
            inputs
                .iter()
                .map(|a| collect_region_scopes_inner(a, hmr_cache))
                .collect(),
        ),

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
pub fn assign_program_offsets(
    program: &mut crate::schema::EvmProgram,
    region_var_map: &indexmap::IndexMap<i64, String>,
) {
    for contract in &mut program.contracts {
        let (new_runtime, hw) = assign_memory_offsets(&contract.runtime, region_var_map);
        contract.runtime = new_runtime;

        // Also process internal functions
        let mut max_hw = hw;
        for func in &mut contract.internal_functions {
            let (new_func, func_hw) = assign_memory_offsets(func, region_var_map);
            *func = new_func;
            max_hw = max_hw.max(func_hw);
        }

        // Also process constructor
        let (new_ctor, ctor_hw) = assign_memory_offsets(&contract.constructor, region_var_map);
        contract.constructor = new_ctor;
        max_hw = max_hw.max(ctor_hw);

        contract.memory_high_water = max_hw;
    }
}

/// Context for region resolution: both MemRegion offset assignments and
/// RegionStore/RegionLoad → MStore/MLoad variable mappings.
#[derive(Debug)]
pub struct RegionResolveCtx {
    /// MemRegion id → concrete byte offset
    pub assignments: BTreeMap<i64, usize>,
    /// Region id → LetBind variable name (for &dm struct field access)
    pub region_var_map: indexmap::IndexMap<i64, String>,
}

/// Replace all `MemRegion(id, _)` with `Const(SmallInt(offset))` and
/// all `RegionStore`/`RegionLoad` with `MStore`/`MLoad` using the variable base pointer.
fn replace_regions(expr: &RcExpr, ctx: &RegionResolveCtx) -> RcExpr {
    let mut cache = std::collections::HashMap::new();
    replace_regions_memo(expr, ctx, &mut cache)
}

fn replace_regions_memo(
    expr: &RcExpr,
    ctx: &RegionResolveCtx,
    cache: &mut std::collections::HashMap<usize, RcExpr>,
) -> RcExpr {
    let id = Rc::as_ptr(expr) as usize;
    if let Some(cached) = cache.get(&id) {
        return Rc::clone(cached);
    }
    let result = replace_regions_inner(expr, ctx, cache);
    cache.insert(id, Rc::clone(&result));
    result
}

fn replace_regions_inner(
    expr: &RcExpr,
    ctx: &RegionResolveCtx,
    cache: &mut std::collections::HashMap<usize, RcExpr>,
) -> RcExpr {
    // Shorthand for recursive calls
    macro_rules! rec {
        ($e:expr) => {
            replace_regions_memo($e, ctx, cache)
        };
    }

    match expr.as_ref() {
        EvmExpr::MemRegion(id, _size) => {
            let offset = ctx.assignments[id];
            Rc::new(EvmExpr::Const(
                EvmConstant::SmallInt(offset as i64),
                EvmType::Base(EvmBaseType::UIntT(256)),
                EvmContext::InFunction("__mem__".to_owned()),
            ))
        }
        EvmExpr::Bop(op, a, b) => {
            let na = rec!(a);
            let nb = rec!(b);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, na, nb))
        }
        EvmExpr::Uop(op, a) => {
            let na = rec!(a);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, na))
        }
        EvmExpr::Top(op, a, b, c) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Top(*op, na, nb, nc))
        }
        EvmExpr::Concat(a, b) => {
            let na = rec!(a);
            let nb = rec!(b);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Concat(na, nb))
        }
        EvmExpr::Get(a, idx) => {
            let na = rec!(a);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(na, *idx))
        }
        EvmExpr::If(c, i, t, e) => {
            let nc = rec!(c);
            let ni = rec!(i);
            let nt = rec!(t);
            let ne = rec!(e);
            if Rc::ptr_eq(&nc, c) && Rc::ptr_eq(&ni, i) && Rc::ptr_eq(&nt, t) && Rc::ptr_eq(&ne, e) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::If(nc, ni, nt, ne))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let ni = rec!(inputs);
            let nb = rec!(body);
            if Rc::ptr_eq(&ni, inputs) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DoWhile(ni, nb))
        }
        EvmExpr::LetBind(name, init, body) => {
            let ni = rec!(init);
            let nb = rec!(body);
            if Rc::ptr_eq(&ni, init) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::LetBind(name.clone(), ni, nb))
        }
        EvmExpr::VarStore(name, val) => {
            let nv = rec!(val);
            if Rc::ptr_eq(&nv, val) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::VarStore(name.clone(), nv))
        }
        EvmExpr::Revert(a, b, c) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Revert(na, nb, nc))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ReturnOp(na, nb, nc))
        }
        EvmExpr::Log(count, topics, d, s, st) => {
            let nt: Vec<_> = topics
                .iter()
                .map(|t| rec!(t))
                .collect();
            let nd = rec!(d);
            let ns = rec!(s);
            let nst = rec!(st);
            if nt.iter().zip(topics.iter()).all(|(n, o)| Rc::ptr_eq(n, o))
                && Rc::ptr_eq(&nd, d) && Rc::ptr_eq(&ns, s) && Rc::ptr_eq(&nst, st)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Log(*count, nt, nd, ns, nst))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            let nd = rec!(d);
            let ne = rec!(e);
            let nf = rec!(f);
            let ng = rec!(g);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c)
                && Rc::ptr_eq(&nd, d) && Rc::ptr_eq(&ne, e) && Rc::ptr_eq(&nf, f)
                && Rc::ptr_eq(&ng, g)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ExtCall(na, nb, nc, nd, ne, nf, ng))
        }
        EvmExpr::Call(name, args) => {
            let new_args: Vec<_> = args
                .iter()
                .map(|a| rec!(a))
                .collect();
            if new_args.iter().zip(args.iter()).all(|(n, o)| Rc::ptr_eq(n, o)) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let nb = rec!(body);
            if Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Function(
                name.clone(),
                in_ty.clone(),
                out_ty.clone(),
                nb,
            ))
        }
        EvmExpr::EnvRead(op, s) => {
            let ns = rec!(s);
            if Rc::ptr_eq(&ns, s) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::EnvRead(*op, ns))
        }
        EvmExpr::EnvRead1(op, a, s) => {
            let na = rec!(a);
            let ns = rec!(s);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&ns, s) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::EnvRead1(*op, na, ns))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let ni: Vec<_> = inputs
                .iter()
                .map(|i| rec!(i))
                .collect();
            if ni.iter().zip(inputs.iter()).all(|(n, o)| Rc::ptr_eq(n, o)) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::InlineAsm(ni, hex.clone(), *num_outputs))
        }
        EvmExpr::DynAlloc(size) => {
            let ns = rec!(size);
            if Rc::ptr_eq(&ns, size) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DynAlloc(ns))
        }
        EvmExpr::AllocRegion(id, num_fields, is_dynamic) => {
            let nf = rec!(num_fields);
            if Rc::ptr_eq(&nf, num_fields) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::AllocRegion(*id, nf, *is_dynamic))
        }
        // RegionStore/RegionLoad: just recurse, don't resolve here.
        // These survive into egglog for symbolic forwarding and get resolved
        // post-egglog by `resolve_regions_post_egglog`.
        EvmExpr::RegionStore(id, field_idx, val, state) => {
            let nv = rec!(val);
            let ns = rec!(state);
            if Rc::ptr_eq(&nv, val) && Rc::ptr_eq(&ns, state) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::RegionStore(*id, *field_idx, nv, ns))
        }
        EvmExpr::RegionLoad(id, field_idx, state) => {
            let ns = rec!(state);
            if Rc::ptr_eq(&ns, state) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::RegionLoad(*id, *field_idx, ns))
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

/// Resolve RegionStore/RegionLoad → MStore/MLoad after egglog optimization.
/// This runs post-egglog so that egglog forwarding rules can fire first.
pub fn resolve_regions_post_egglog(
    program: &mut EvmProgram,
    region_var_map: &indexmap::IndexMap<i64, String>,
) {
    if region_var_map.is_empty() {
        return;
    }
    for contract in &mut program.contracts {
        contract.runtime = resolve_region_expr(&contract.runtime, region_var_map);
        for func in &mut contract.internal_functions {
            *func = resolve_region_expr(func, region_var_map);
        }
        contract.constructor = resolve_region_expr(&contract.constructor, region_var_map);
    }
}

fn resolve_region_expr(
    expr: &RcExpr,
    region_var_map: &indexmap::IndexMap<i64, String>,
) -> RcExpr {
    let mut cache = std::collections::HashMap::new();
    resolve_region_memo(expr, region_var_map, &mut cache)
}

fn resolve_region_memo(
    expr: &RcExpr,
    rvm: &indexmap::IndexMap<i64, String>,
    cache: &mut std::collections::HashMap<usize, RcExpr>,
) -> RcExpr {
    let id = Rc::as_ptr(expr) as usize;
    if let Some(cached) = cache.get(&id) {
        return Rc::clone(cached);
    }
    let result = resolve_region_inner(expr, rvm, cache);
    cache.insert(id, Rc::clone(&result));
    result
}

fn resolve_region_inner(
    expr: &RcExpr,
    rvm: &indexmap::IndexMap<i64, String>,
    cache: &mut std::collections::HashMap<usize, RcExpr>,
) -> RcExpr {
    macro_rules! rec {
        ($e:expr) => {
            resolve_region_memo($e, rvm, cache)
        };
    }

    fn region_offset(var_name: &str, field_idx: i64) -> RcExpr {
        let base = Rc::new(EvmExpr::Var(var_name.to_string()));
        if field_idx == 0 {
            base
        } else {
            Rc::new(EvmExpr::Bop(
                crate::schema::EvmBinaryOp::Add,
                base,
                Rc::new(EvmExpr::Const(
                    EvmConstant::SmallInt(field_idx * 32),
                    EvmType::Base(EvmBaseType::UIntT(256)),
                    EvmContext::InFunction("__mem__".to_owned()),
                )),
            ))
        }
    }

    match expr.as_ref() {
        EvmExpr::RegionStore(id, field_idx, val, state) => {
            let nv = rec!(val);
            let ns = rec!(state);
            if let Some(var_name) = rvm.get(id) {
                let offset = region_offset(var_name, *field_idx);
                Rc::new(EvmExpr::Top(
                    crate::schema::EvmTernaryOp::MStore,
                    offset,
                    nv,
                    ns,
                ))
            } else {
                // Unknown region — shouldn't happen, but pass through
                Rc::new(EvmExpr::RegionStore(*id, *field_idx, nv, ns))
            }
        }
        EvmExpr::RegionLoad(id, field_idx, state) => {
            let ns = rec!(state);
            if let Some(var_name) = rvm.get(id) {
                let offset = region_offset(var_name, *field_idx);
                Rc::new(EvmExpr::Bop(
                    crate::schema::EvmBinaryOp::MLoad,
                    offset,
                    ns,
                ))
            } else {
                Rc::new(EvmExpr::RegionLoad(*id, *field_idx, ns))
            }
        }
        // For all other nodes, just recurse
        EvmExpr::Bop(op, a, b) => {
            let na = rec!(a);
            let nb = rec!(b);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, na, nb))
        }
        EvmExpr::Uop(op, a) => {
            let na = rec!(a);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, na))
        }
        EvmExpr::Top(op, a, b, c) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Top(*op, na, nb, nc))
        }
        EvmExpr::Concat(a, b) => {
            let na = rec!(a);
            let nb = rec!(b);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Concat(na, nb))
        }
        EvmExpr::Get(a, idx) => {
            let na = rec!(a);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(na, *idx))
        }
        EvmExpr::If(c, i, t, e) => {
            let nc = rec!(c);
            let ni = rec!(i);
            let nt = rec!(t);
            let ne = rec!(e);
            if Rc::ptr_eq(&nc, c)
                && Rc::ptr_eq(&ni, i)
                && Rc::ptr_eq(&nt, t)
                && Rc::ptr_eq(&ne, e)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::If(nc, ni, nt, ne))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let ni = rec!(inputs);
            let nb = rec!(body);
            if Rc::ptr_eq(&ni, inputs) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DoWhile(ni, nb))
        }
        EvmExpr::LetBind(name, init, body) => {
            let ni = rec!(init);
            let nb = rec!(body);
            if Rc::ptr_eq(&ni, init) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::LetBind(name.clone(), ni, nb))
        }
        EvmExpr::VarStore(name, val) => {
            let nv = rec!(val);
            if Rc::ptr_eq(&nv, val) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::VarStore(name.clone(), nv))
        }
        EvmExpr::Revert(a, b, c) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Revert(na, nb, nc))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ReturnOp(na, nb, nc))
        }
        EvmExpr::Log(count, topics, d, s, st) => {
            let nt: Vec<_> = topics.iter().map(|t| rec!(t)).collect();
            let nd = rec!(d);
            let ns = rec!(s);
            let nst = rec!(st);
            if nt.iter().zip(topics.iter()).all(|(n, o)| Rc::ptr_eq(n, o))
                && Rc::ptr_eq(&nd, d)
                && Rc::ptr_eq(&ns, s)
                && Rc::ptr_eq(&nst, st)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Log(*count, nt, nd, ns, nst))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let na = rec!(a);
            let nb = rec!(b);
            let nc = rec!(c);
            let nd = rec!(d);
            let ne = rec!(e);
            let nf = rec!(f);
            let ng = rec!(g);
            if Rc::ptr_eq(&na, a)
                && Rc::ptr_eq(&nb, b)
                && Rc::ptr_eq(&nc, c)
                && Rc::ptr_eq(&nd, d)
                && Rc::ptr_eq(&ne, e)
                && Rc::ptr_eq(&nf, f)
                && Rc::ptr_eq(&ng, g)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ExtCall(na, nb, nc, nd, ne, nf, ng))
        }
        EvmExpr::Call(name, args) => {
            let new_args: Vec<_> = args.iter().map(|a| rec!(a)).collect();
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
            let nb = rec!(body);
            if Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Function(name.clone(), in_ty.clone(), out_ty.clone(), nb))
        }
        EvmExpr::EnvRead(op, s) => {
            let ns = rec!(s);
            if Rc::ptr_eq(&ns, s) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::EnvRead(*op, ns))
        }
        EvmExpr::EnvRead1(op, a, s) => {
            let na = rec!(a);
            let ns = rec!(s);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&ns, s) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::EnvRead1(*op, na, ns))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let ni: Vec<_> = inputs.iter().map(|i| rec!(i)).collect();
            if ni.iter().zip(inputs.iter()).all(|(n, o)| Rc::ptr_eq(n, o)) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::InlineAsm(ni, hex.clone(), *num_outputs))
        }
        EvmExpr::DynAlloc(size) => {
            let ns = rec!(size);
            if Rc::ptr_eq(&ns, size) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DynAlloc(ns))
        }
        // Leaves — no children to recurse
        EvmExpr::AllocRegion(..)
        | EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::MemRegion(..)
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

        let (result, hw) = assign_memory_offsets(&mstore, &indexmap::IndexMap::new());
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

        let (result, hw) = assign_memory_offsets(&expr, &indexmap::IndexMap::new());
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
        let (result, hw) = assign_memory_offsets(&expr, &indexmap::IndexMap::new());
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

        let (result, hw) = assign_memory_offsets(&if_expr, &indexmap::IndexMap::new());
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

        let (_result, hw) = assign_memory_offsets(&expr, &indexmap::IndexMap::new());
        // r_shared=32 bytes, then branches max(64,96)=96 → total 32+96=128
        assert_eq!(hw, 128);
    }
}
