//! Variable optimization pass (counting-based transforms).
//!
//! Runs on the `RcExpr` IR tree BEFORE egglog equality saturation.
//! Only performs transforms that require occurrence counting, which
//! egglog's pattern matching cannot express:
//!
//! 1. **Dead variable elimination**: Remove LetBinds whose variable is never read
//! 2. **Single-use inlining**: Inline LetBind init directly at sole Var reference
//! 3. **Multi-use constant propagation**: Replace Var refs with the constant value
//!
//! Store-forwarding is handled at the lowering level (to_egglog.rs), not here.

use std::collections::HashMap;
use std::rc::Rc;

use crate::schema::{EvmExpr, EvmTernaryOp, RcExpr};

/// How a variable should be allocated at codegen time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationMode {
    /// Keep on the EVM stack; use DUP to read.
    Stack,
    /// Spill to memory (MSTORE/MLOAD) — the default.
    Memory,
}

/// Per-variable info passed to codegen: allocation mode + total read count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarAllocation {
    /// Stack vs memory allocation decision.
    pub mode: AllocationMode,
    /// Total number of Var(name) reads in the LetBind body (for last-use DUP elision).
    pub read_count: usize,
}

/// Per-variable usage statistics.
#[derive(Debug, Clone, Default)]
struct VarInfo {
    /// Number of times `Var(name)` appears in the LetBind body
    read_count: usize,
    /// Number of times `VarStore(name, _)` appears in the LetBind body
    write_count: usize,
    /// Whether any reference is inside a DoWhile loop
    in_loop: bool,
}

/// Analyze all LetBind variables in an expression and decide allocation mode.
///
/// Returns a map from variable name to `VarAllocation`. Variables not in the
/// map default to `Memory` with `read_count` 0.
///
/// A variable is eligible for stack allocation if:
/// - It is never reassigned (`write_count == 0`)
/// - It is not referenced inside a loop
/// - It has a bounded number of reads (`read_count <= 8`)
pub fn analyze_allocations(expr: &RcExpr) -> HashMap<String, VarAllocation> {
    let mut result = HashMap::new();
    collect_allocations(expr, &mut result);
    result
}

fn collect_allocations(expr: &RcExpr, result: &mut HashMap<String, VarAllocation>) {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            collect_allocations(init, result);
            let info = analyze_var(name, body);
            let mode = if info.write_count == 0 && !info.in_loop && info.read_count <= 8 {
                AllocationMode::Stack
            } else {
                AllocationMode::Memory
            };
            let alloc = VarAllocation { mode, read_count: info.read_count };
            // If the name already exists (from another function's same-named local),
            // use the more conservative allocation: Memory beats Stack.
            result.entry(name.clone())
                .and_modify(|existing| {
                    if alloc.mode == AllocationMode::Memory {
                        existing.mode = AllocationMode::Memory;
                    }
                    existing.read_count = existing.read_count.max(alloc.read_count);
                })
                .or_insert(alloc);
            collect_allocations(body, result);
        }
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) => {
            collect_allocations(a, result);
            collect_allocations(b, result);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => {
            collect_allocations(a, result);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_allocations(a, result);
            collect_allocations(b, result);
            collect_allocations(c, result);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_allocations(c, result);
            collect_allocations(i, result);
            collect_allocations(t, result);
            collect_allocations(e, result);
        }
        EvmExpr::DoWhile(inputs, body) => {
            collect_allocations(inputs, result);
            collect_allocations(body, result);
        }
        EvmExpr::EnvRead(_, s) => collect_allocations(s, result),
        EvmExpr::EnvRead1(_, a, s) => {
            collect_allocations(a, result);
            collect_allocations(s, result);
        }
        EvmExpr::Log(_, topics, data, state) => {
            for t in topics {
                collect_allocations(t, result);
            }
            collect_allocations(data, result);
            collect_allocations(state, result);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            for x in [a, b, c, d, e, f, g] {
                collect_allocations(x, result);
            }
        }
        EvmExpr::Call(_, args) => collect_allocations(args, result),
        EvmExpr::VarStore(_, val) => collect_allocations(val, result),
        EvmExpr::Function(_, _, _, body) => collect_allocations(body, result),
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => {}
    }
}

/// Optimize an entire program's contract runtimes.
pub fn optimize_program(program: &mut crate::schema::EvmProgram) {
    for contract in &mut program.contracts {
        contract.runtime = optimize_expr(&contract.runtime);
        // Insert early Drops in halting branches for better dead-var-elim
        contract.runtime = insert_early_drops(&contract.runtime);
        contract.constructor = insert_early_drops(&contract.constructor);
    }
}

/// Optimize an expression tree, applying all variable optimizations bottom-up.
fn optimize_expr(expr: &RcExpr) -> RcExpr {
    // Bottom-up: optimize children first, then apply transforms at this node.
    let rebuilt = rebuild_children(expr);
    apply_transforms(&rebuilt)
}

/// Recursively rebuild an expression with optimized children.
fn rebuild_children(expr: &RcExpr) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::Bop(op, lhs, rhs) => {
            let l = optimize_expr(lhs);
            let r = optimize_expr(rhs);
            if Rc::ptr_eq(&l, lhs) && Rc::ptr_eq(&r, rhs) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Bop(*op, l, r))
        }
        EvmExpr::Uop(op, inner) => {
            let i = optimize_expr(inner);
            if Rc::ptr_eq(&i, inner) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Uop(*op, i))
        }
        EvmExpr::Top(op, a, b, c) => {
            let a2 = optimize_expr(a);
            let b2 = optimize_expr(b);
            let c2 = optimize_expr(c);
            if Rc::ptr_eq(&a2, a) && Rc::ptr_eq(&b2, b) && Rc::ptr_eq(&c2, c) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Top(*op, a2, b2, c2))
        }
        EvmExpr::Get(inner, idx) => {
            let i = optimize_expr(inner);
            if Rc::ptr_eq(&i, inner) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Get(i, *idx))
        }
        EvmExpr::Concat(a, b) => {
            let a2 = optimize_expr(a);
            let b2 = optimize_expr(b);
            if Rc::ptr_eq(&a2, a) && Rc::ptr_eq(&b2, b) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Concat(a2, b2))
        }
        EvmExpr::If(cond, inputs, then_body, else_body) => {
            let c = optimize_expr(cond);
            let i = optimize_expr(inputs);
            let t = optimize_expr(then_body);
            let e = optimize_expr(else_body);
            if Rc::ptr_eq(&c, cond)
                && Rc::ptr_eq(&i, inputs)
                && Rc::ptr_eq(&t, then_body)
                && Rc::ptr_eq(&e, else_body)
            {
                return expr.clone();
            }
            Rc::new(EvmExpr::If(c, i, t, e))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let i = optimize_expr(inputs);
            let b = optimize_expr(body);
            if Rc::ptr_eq(&i, inputs) && Rc::ptr_eq(&b, body) {
                return expr.clone();
            }
            Rc::new(EvmExpr::DoWhile(i, b))
        }
        EvmExpr::EnvRead(op, state) => {
            let s = optimize_expr(state);
            if Rc::ptr_eq(&s, state) {
                return expr.clone();
            }
            Rc::new(EvmExpr::EnvRead(*op, s))
        }
        EvmExpr::EnvRead1(op, arg, state) => {
            let a = optimize_expr(arg);
            let s = optimize_expr(state);
            if Rc::ptr_eq(&a, arg) && Rc::ptr_eq(&s, state) {
                return expr.clone();
            }
            Rc::new(EvmExpr::EnvRead1(*op, a, s))
        }
        EvmExpr::Log(count, topics, data, state) => {
            let ts: Vec<_> = topics.iter().map(optimize_expr).collect();
            let d = optimize_expr(data);
            let s = optimize_expr(state);
            Rc::new(EvmExpr::Log(*count, ts, d, s))
        }
        EvmExpr::Revert(off, sz, state) => {
            let o = optimize_expr(off);
            let s = optimize_expr(sz);
            let st = optimize_expr(state);
            if Rc::ptr_eq(&o, off) && Rc::ptr_eq(&s, sz) && Rc::ptr_eq(&st, state) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Revert(o, s, st))
        }
        EvmExpr::ReturnOp(off, sz, state) => {
            let o = optimize_expr(off);
            let s = optimize_expr(sz);
            let st = optimize_expr(state);
            if Rc::ptr_eq(&o, off) && Rc::ptr_eq(&s, sz) && Rc::ptr_eq(&st, state) {
                return expr.clone();
            }
            Rc::new(EvmExpr::ReturnOp(o, s, st))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let a2 = optimize_expr(a);
            let b2 = optimize_expr(b);
            let c2 = optimize_expr(c);
            let d2 = optimize_expr(d);
            let e2 = optimize_expr(e);
            let f2 = optimize_expr(f);
            let g2 = optimize_expr(g);
            Rc::new(EvmExpr::ExtCall(a2, b2, c2, d2, e2, f2, g2))
        }
        EvmExpr::Call(name, args) => {
            let a = optimize_expr(args);
            if Rc::ptr_eq(&a, args) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Call(name.clone(), a))
        }
        EvmExpr::LetBind(name, value, body) => {
            let v = optimize_expr(value);
            let b = optimize_expr(body);
            // Don't short-circuit here — apply_transforms handles LetBind optimizations
            Rc::new(EvmExpr::LetBind(name.clone(), v, b))
        }
        EvmExpr::VarStore(name, value) => {
            let v = optimize_expr(value);
            if Rc::ptr_eq(&v, value) {
                return expr.clone();
            }
            Rc::new(EvmExpr::VarStore(name.clone(), v))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let b = optimize_expr(body);
            if Rc::ptr_eq(&b, body) {
                return expr.clone();
            }
            Rc::new(EvmExpr::Function(name.clone(), in_ty.clone(), out_ty.clone(), b))
        }
        // Leaf nodes — no children to optimize
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => expr.clone(),
    }
}

/// Apply optimization transforms at a single node (children already optimized).
fn apply_transforms(expr: &RcExpr) -> RcExpr {
    if let EvmExpr::LetBind(name, init, body) = expr.as_ref() {
        let info = analyze_var(name, body);
        return apply_letbind_opts(name, init, body, expr, &info);
    }

    expr.clone()
}

/// Apply LetBind-specific optimizations given usage info.
fn apply_letbind_opts(
    name: &str,
    init: &RcExpr,
    body: &RcExpr,
    expr: &RcExpr,
    info: &VarInfo,
) -> RcExpr {
    // 1. Dead variable elimination: never read → remove LetBind
    if info.read_count == 0 && info.write_count == 0 {
        if is_pure(init) {
            return body.clone();
        } else {
            // Keep side effects
            return Rc::new(EvmExpr::Concat(init.clone(), body.clone()));
        }
    }

    // 2. Single-use inlining: read once, never written, not in loop, pure init
    if info.read_count == 1 && info.write_count == 0 && !info.in_loop && is_pure(init) {
        return substitute_var(name, init, body);
    }

    // 2b. Last-store forwarding: exactly one VarStore, one Var read, not in loop.
    // Pattern: Concat(VarStore(x, val), ...Var(x)...) → substitute val for Var(x)
    // and remove the VarStore. The LetBind becomes dead (no reads or writes).
    // This handles "c = expr; return c;" → "return expr;".
    if info.write_count == 1 && info.read_count == 1 && !info.in_loop {
        if let Some(new_body) = forward_last_store(name, body) {
            // LetBind is now dead — eliminate it
            if is_pure(init) {
                return new_body;
            } else {
                return Rc::new(EvmExpr::Concat(init.clone(), new_body));
            }
        }
    }

    // 3. Multi-use constant propagation: constant init, never written
    if info.write_count == 0 && !info.in_loop && is_const(init) {
        return substitute_var(name, init, body);
    }

    expr.clone()
}

/// Analyze how a variable is used within an expression.
fn analyze_var(name: &str, expr: &RcExpr) -> VarInfo {
    let mut info = VarInfo::default();
    analyze_var_inner(name, expr, false, &mut info);
    info
}

fn analyze_var_inner(name: &str, expr: &RcExpr, in_loop: bool, info: &mut VarInfo) {
    match expr.as_ref() {
        EvmExpr::Var(n) if n == name => {
            info.read_count += 1;
            if in_loop {
                info.in_loop = true;
            }
        }
        EvmExpr::VarStore(n, val) if n == name => {
            info.write_count += 1;
            if in_loop {
                info.in_loop = true;
            }
            analyze_var_inner(name, val, in_loop, info);
        }
        EvmExpr::VarStore(_, val) => {
            analyze_var_inner(name, val, in_loop, info);
        }
        EvmExpr::LetBind(n, init, body) => {
            analyze_var_inner(name, init, in_loop, info);
            // If this LetBind shadows our variable, don't count refs in its body
            if n != name {
                analyze_var_inner(name, body, in_loop, info);
            }
        }
        EvmExpr::Var(_)
        | EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Selector(_)
        | EvmExpr::Drop(_)
        | EvmExpr::StorageField(..) => {}
        // For Bop: skip the state parameter (2nd arg) of stateful ops.
        // Codegen ignores state parameters, so Var refs there are phantom.
        EvmExpr::Bop(op, a, b) => {
            analyze_var_inner(name, a, in_loop, info);
            if !op.has_state() {
                analyze_var_inner(name, b, in_loop, info);
            }
        }
        EvmExpr::Concat(a, b) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, b, in_loop, info);
        }
        EvmExpr::DoWhile(inputs, body) => {
            analyze_var_inner(name, inputs, in_loop, info);
            analyze_var_inner(name, body, true, info);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => {
            analyze_var_inner(name, a, in_loop, info);
        }
        // Top/ReturnOp/Revert: last arg (c) is the state parameter — skip it.
        EvmExpr::Top(_, a, b, _c) | EvmExpr::Revert(a, b, _c) | EvmExpr::ReturnOp(a, b, _c) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, b, in_loop, info);
        }
        EvmExpr::If(c, i, t, e) => {
            analyze_var_inner(name, c, in_loop, info);
            analyze_var_inner(name, i, in_loop, info);
            analyze_var_inner(name, t, in_loop, info);
            analyze_var_inner(name, e, in_loop, info);
        }
        // EnvRead/EnvRead1: last arg is state — skip it.
        EvmExpr::EnvRead(_, _s) => {}
        EvmExpr::EnvRead1(_, a, _s) => {
            analyze_var_inner(name, a, in_loop, info);
        }
        // Log: last arg is state — skip it.
        EvmExpr::Log(_, topics, data, _state) => {
            for t in topics {
                analyze_var_inner(name, t, in_loop, info);
            }
            analyze_var_inner(name, data, in_loop, info);
        }
        // ExtCall: last arg (g) is state — skip it.
        EvmExpr::ExtCall(a, b, c, d, e, f, _g) => {
            for x in [a, b, c, d, e, f] {
                analyze_var_inner(name, x, in_loop, info);
            }
        }
        EvmExpr::Call(_, args) => {
            analyze_var_inner(name, args, in_loop, info);
        }
        EvmExpr::Function(_, _, _, body) => {
            analyze_var_inner(name, body, in_loop, info);
        }
    }
}

/// Forward the value from a VarStore to the subsequent Var read.
///
/// Finds the VarStore(name, val) in the body's Concat chain, removes it,
/// and substitutes val for the single Var(name) read.
///
/// Returns `Some(new_body)` if the forwarding succeeded, `None` otherwise.
fn forward_last_store(name: &str, body: &RcExpr) -> Option<RcExpr> {
    // Extract the VarStore value and remove it from the body
    let (val, cleaned) = extract_store_value(name, body)?;
    // Substitute the stored value for the Var read
    Some(substitute_var(name, &val, &cleaned))
}

/// Extract the value from VarStore(name, val) in a Concat chain,
/// returning (val, body_without_VarStore).
fn extract_store_value(name: &str, expr: &RcExpr) -> Option<(RcExpr, RcExpr)> {
    match expr.as_ref() {
        EvmExpr::VarStore(n, val) if n == name => {
            // Replace VarStore with Empty (side-effect-free placeholder)
            Some((
                val.clone(),
                Rc::new(EvmExpr::Empty(
                    crate::schema::EvmType::Base(crate::schema::EvmBaseType::UnitT),
                    crate::schema::EvmContext::InFunction("__opt__".to_owned()),
                )),
            ))
        }
        EvmExpr::Concat(a, b) => {
            // Try left side first
            if let Some((val, new_a)) = extract_store_value(name, a) {
                return Some((val, Rc::new(EvmExpr::Concat(new_a, b.clone()))));
            }
            // Try right side
            if let Some((val, new_b)) = extract_store_value(name, b) {
                return Some((val, Rc::new(EvmExpr::Concat(a.clone(), new_b))));
            }
            None
        }
        _ => None,
    }
}

/// Check if an expression is any constant.
fn is_const(expr: &RcExpr) -> bool {
    matches!(expr.as_ref(), EvmExpr::Const(..))
}

/// Check if an expression is pure (no side effects).
/// Conservative: only things we're sure are pure.
fn is_pure(expr: &RcExpr) -> bool {
    match expr.as_ref() {
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_) => true,
        EvmExpr::Bop(op, a, b) => {
            use crate::schema::EvmBinaryOp::*;
            match op {
                Add | Sub | Mul | Div | SDiv | Mod | SMod | Exp | Lt | Gt | SLt | SGt | Eq
                | And | Or | Xor | Shl | Shr | Sar | Byte | LogAnd | LogOr | SLoad | TLoad
                | MLoad | CalldataLoad => is_pure(a) && is_pure(b),
                // Checked arithmetic can revert — not pure
                CheckedAdd | CheckedSub | CheckedMul => false,
            }
        }
        EvmExpr::Uop(_, a) => is_pure(a),
        EvmExpr::Top(op, a, b, c) => match op {
            EvmTernaryOp::Select | EvmTernaryOp::Keccak256 => {
                is_pure(a) && is_pure(b) && is_pure(c)
            }
            _ => false,
        },
        EvmExpr::Get(a, _) => is_pure(a),
        EvmExpr::Concat(a, b) => is_pure(a) && is_pure(b),
        EvmExpr::EnvRead(..) | EvmExpr::EnvRead1(..) => true,
        EvmExpr::LetBind(_, init, body) => is_pure(init) && is_pure(body),
        EvmExpr::If(c, i, t, e) => is_pure(c) && is_pure(i) && is_pure(t) && is_pure(e),
        EvmExpr::VarStore(..) | EvmExpr::Log(..) | EvmExpr::Revert(..) | EvmExpr::ReturnOp(..)
        | EvmExpr::ExtCall(..) => false,
        EvmExpr::DoWhile(..) => false,
        EvmExpr::Call(..) => false,
        EvmExpr::Function(..) | EvmExpr::StorageField(..) => true,
    }
}

/// Collect names of immutable variables (LetBinds with no VarStore in body).
///
/// These variables always have the same value as their init expression,
/// so egglog can propagate bounds from the init to Var references.
pub fn collect_immutable_vars(expr: &RcExpr) -> Vec<String> {
    let mut result = Vec::new();
    collect_immutable_vars_rec(expr, &mut result);
    result
}

fn collect_immutable_vars_rec(expr: &RcExpr, out: &mut Vec<String>) {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            let info = analyze_var(name, body);
            if info.write_count == 0 {
                out.push(name.clone());
            }
            collect_immutable_vars_rec(init, out);
            collect_immutable_vars_rec(body, out);
        }
        EvmExpr::Concat(a, b) => {
            collect_immutable_vars_rec(a, out);
            collect_immutable_vars_rec(b, out);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_immutable_vars_rec(c, out);
            collect_immutable_vars_rec(i, out);
            collect_immutable_vars_rec(t, out);
            collect_immutable_vars_rec(e, out);
        }
        EvmExpr::DoWhile(inputs, body) => {
            collect_immutable_vars_rec(inputs, out);
            collect_immutable_vars_rec(body, out);
        }
        EvmExpr::Bop(_, a, b) => {
            collect_immutable_vars_rec(a, out);
            collect_immutable_vars_rec(b, out);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => {
            collect_immutable_vars_rec(a, out);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_immutable_vars_rec(a, out);
            collect_immutable_vars_rec(b, out);
            collect_immutable_vars_rec(c, out);
        }
        EvmExpr::Log(_, topics, data, state) => {
            for t in topics {
                collect_immutable_vars_rec(t, out);
            }
            collect_immutable_vars_rec(data, out);
            collect_immutable_vars_rec(state, out);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            for x in [a, b, c, d, e, f, g] {
                collect_immutable_vars_rec(x, out);
            }
        }
        EvmExpr::VarStore(_, val) => {
            collect_immutable_vars_rec(val, out);
        }
        EvmExpr::Call(_, args) => {
            collect_immutable_vars_rec(args, out);
        }
        EvmExpr::Function(_, _, _, body) => {
            collect_immutable_vars_rec(body, out);
        }
        EvmExpr::EnvRead(_, s) => {
            collect_immutable_vars_rec(s, out);
        }
        EvmExpr::EnvRead1(_, a, s) => {
            collect_immutable_vars_rec(a, out);
            collect_immutable_vars_rec(s, out);
        }
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => {}
    }
}

// ============================================================
// Early Drop Insertion for Halting Branches
// ============================================================
//
// When a LetBind's body contains an If with a halting branch (RETURN/REVERT)
// that doesn't reference the variable, insert Drop(var) before the terminal.
// This enables egglog dead-var-elim and proper stack cleanup at codegen.

/// Insert early Drop nodes for variables in halting branches that don't use them.
///
/// Call this on the full expression tree after other var_opt passes.
pub fn insert_early_drops(expr: &RcExpr) -> RcExpr {
    insert_drops_rec(expr, &[])
}

fn insert_drops_rec(expr: &RcExpr, vars_in_scope: &[String]) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            let new_init = insert_drops_rec(init, vars_in_scope);
            let mut new_scope = vars_in_scope.to_vec();
            new_scope.push(name.clone());
            let new_body = insert_drops_rec(body, &new_scope);
            Rc::new(EvmExpr::LetBind(name.clone(), new_init, new_body))
        }
        EvmExpr::If(cond, inputs, then_body, else_body) => {
            // Recurse first into sub-expressions
            let new_cond = insert_drops_rec(cond, vars_in_scope);
            let new_inputs = insert_drops_rec(inputs, vars_in_scope);
            let mut new_then = insert_drops_rec(then_body, vars_in_scope);
            let mut new_else = insert_drops_rec(else_body, vars_in_scope);

            // For each halting branch, add Drops for unreferenced in-scope vars
            if expr_definitely_halts(&new_then) {
                for var in vars_in_scope {
                    if !references_var(&new_then, var) {
                        new_then = prepend_drop(&new_then, var);
                    }
                }
            }
            if expr_definitely_halts(&new_else) {
                for var in vars_in_scope {
                    if !references_var(&new_else, var) {
                        new_else = prepend_drop(&new_else, var);
                    }
                }
            }

            Rc::new(EvmExpr::If(new_cond, new_inputs, new_then, new_else))
        }
        EvmExpr::Concat(a, b) => {
            let new_a = insert_drops_rec(a, vars_in_scope);
            let new_b = insert_drops_rec(b, vars_in_scope);
            Rc::new(EvmExpr::Concat(new_a, new_b))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let new_inputs = insert_drops_rec(inputs, vars_in_scope);
            let new_body = insert_drops_rec(body, vars_in_scope);
            Rc::new(EvmExpr::DoWhile(new_inputs, new_body))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let new_body = insert_drops_rec(body, vars_in_scope);
            Rc::new(EvmExpr::Function(name.clone(), in_ty.clone(), out_ty.clone(), new_body))
        }
        // Leaf and other nodes: no structural changes needed
        _ => expr.clone(),
    }
}

/// Check if an expression is guaranteed to halt (ends with RETURN or REVERT).
fn expr_definitely_halts(expr: &RcExpr) -> bool {
    match expr.as_ref() {
        EvmExpr::ReturnOp(_, _, _) | EvmExpr::Revert(_, _, _) => true,
        // Concat: the expression halts if its tail halts
        EvmExpr::Concat(_, b) => expr_definitely_halts(b),
        // If: halts if BOTH branches halt
        EvmExpr::If(_, _, then_body, else_body) => {
            expr_definitely_halts(then_body) && expr_definitely_halts(else_body)
        }
        // LetBind: halts if body halts
        EvmExpr::LetBind(_, _, body) => expr_definitely_halts(body),
        _ => false,
    }
}

/// Check if an expression references a variable by name (Var or VarStore).
fn references_var(expr: &RcExpr, name: &str) -> bool {
    match expr.as_ref() {
        EvmExpr::Var(n) => n == name,
        EvmExpr::VarStore(n, val) => n == name || references_var(val, name),
        EvmExpr::Drop(n) => n == name,
        EvmExpr::LetBind(n, init, body) => {
            references_var(init, name) || (n != name && references_var(body, name))
        }
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) => {
            references_var(a, name) || references_var(b, name)
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => references_var(a, name),
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            references_var(a, name) || references_var(b, name) || references_var(c, name)
        }
        EvmExpr::If(c, i, t, e) => {
            references_var(c, name) || references_var(i, name)
                || references_var(t, name) || references_var(e, name)
        }
        EvmExpr::DoWhile(inputs, body) => {
            references_var(inputs, name) || references_var(body, name)
        }
        EvmExpr::EnvRead(_, s) => references_var(s, name),
        EvmExpr::EnvRead1(_, a, s) => references_var(a, name) || references_var(s, name),
        EvmExpr::Log(_, topics, data, state) => {
            topics.iter().any(|t| references_var(t, name))
                || references_var(data, name)
                || references_var(state, name)
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            [a, b, c, d, e, f, g].iter().any(|x| references_var(x, name))
        }
        EvmExpr::Call(_, args) => references_var(args, name),
        EvmExpr::Function(_, _, _, body) => references_var(body, name),
        EvmExpr::Const(..) | EvmExpr::Arg(..) | EvmExpr::Empty(..)
        | EvmExpr::Selector(_) | EvmExpr::StorageField(..) => false,
    }
}

/// Prepend a Drop(var) before a halting expression.
///
/// If the expression is `Concat(a, halt)`, inserts before the halt:
/// `Concat(a, Concat(Drop(var), halt))`.
/// Otherwise wraps: `Concat(Drop(var), expr)`.
fn prepend_drop(expr: &RcExpr, var: &str) -> RcExpr {
    match expr.as_ref() {
        // For Concat chains, insert the Drop just before the tail
        EvmExpr::Concat(head, tail) if expr_definitely_halts(tail) => {
            let new_tail = prepend_drop(tail, var);
            Rc::new(EvmExpr::Concat(head.clone(), new_tail))
        }
        // Base case: wrap with Drop
        _ => Rc::new(EvmExpr::Concat(
            Rc::new(EvmExpr::Drop(var.to_owned())),
            expr.clone(),
        )),
    }
}

/// Substitute all occurrences of `Var(name)` with `replacement` in `expr`.
fn substitute_var(name: &str, replacement: &RcExpr, expr: &RcExpr) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::Var(n) if n == name => replacement.clone(),
        EvmExpr::Var(_) => expr.clone(),

        // Stop at shadowing LetBind
        EvmExpr::LetBind(n, init, body) => {
            let new_init = substitute_var(name, replacement, init);
            if n == name {
                Rc::new(EvmExpr::LetBind(n.clone(), new_init, body.clone()))
            } else {
                let new_body = substitute_var(name, replacement, body);
                Rc::new(EvmExpr::LetBind(n.clone(), new_init, new_body))
            }
        }

        EvmExpr::VarStore(n, val) => {
            let new_val = substitute_var(name, replacement, val);
            Rc::new(EvmExpr::VarStore(n.clone(), new_val))
        }

        // Leaf nodes
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Selector(_)
        | EvmExpr::Drop(_)
        | EvmExpr::StorageField(..) => expr.clone(),

        EvmExpr::Bop(op, a, b) => {
            let a2 = substitute_var(name, replacement, a);
            let b2 = substitute_var(name, replacement, b);
            Rc::new(EvmExpr::Bop(*op, a2, b2))
        }
        EvmExpr::Uop(op, a) => {
            let a2 = substitute_var(name, replacement, a);
            Rc::new(EvmExpr::Uop(*op, a2))
        }
        EvmExpr::Top(op, a, b, c) => {
            let a2 = substitute_var(name, replacement, a);
            let b2 = substitute_var(name, replacement, b);
            let c2 = substitute_var(name, replacement, c);
            Rc::new(EvmExpr::Top(*op, a2, b2, c2))
        }
        EvmExpr::Get(a, idx) => {
            let a2 = substitute_var(name, replacement, a);
            Rc::new(EvmExpr::Get(a2, *idx))
        }
        EvmExpr::Concat(a, b) => {
            let a2 = substitute_var(name, replacement, a);
            let b2 = substitute_var(name, replacement, b);
            Rc::new(EvmExpr::Concat(a2, b2))
        }
        EvmExpr::If(c, i, t, e) => {
            let c2 = substitute_var(name, replacement, c);
            let i2 = substitute_var(name, replacement, i);
            let t2 = substitute_var(name, replacement, t);
            let e2 = substitute_var(name, replacement, e);
            Rc::new(EvmExpr::If(c2, i2, t2, e2))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let i2 = substitute_var(name, replacement, inputs);
            let b2 = substitute_var(name, replacement, body);
            Rc::new(EvmExpr::DoWhile(i2, b2))
        }
        EvmExpr::EnvRead(op, state) => {
            let s2 = substitute_var(name, replacement, state);
            Rc::new(EvmExpr::EnvRead(*op, s2))
        }
        EvmExpr::EnvRead1(op, arg, state) => {
            let a2 = substitute_var(name, replacement, arg);
            let s2 = substitute_var(name, replacement, state);
            Rc::new(EvmExpr::EnvRead1(*op, a2, s2))
        }
        EvmExpr::Log(count, topics, data, state) => {
            let ts: Vec<_> = topics
                .iter()
                .map(|t| substitute_var(name, replacement, t))
                .collect();
            let d2 = substitute_var(name, replacement, data);
            let s2 = substitute_var(name, replacement, state);
            Rc::new(EvmExpr::Log(*count, ts, d2, s2))
        }
        EvmExpr::Revert(a, b, c) => {
            let a2 = substitute_var(name, replacement, a);
            let b2 = substitute_var(name, replacement, b);
            let c2 = substitute_var(name, replacement, c);
            Rc::new(EvmExpr::Revert(a2, b2, c2))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let a2 = substitute_var(name, replacement, a);
            let b2 = substitute_var(name, replacement, b);
            let c2 = substitute_var(name, replacement, c);
            Rc::new(EvmExpr::ReturnOp(a2, b2, c2))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let a2 = substitute_var(name, replacement, a);
            let b2 = substitute_var(name, replacement, b);
            let c2 = substitute_var(name, replacement, c);
            let d2 = substitute_var(name, replacement, d);
            let e2 = substitute_var(name, replacement, e);
            let f2 = substitute_var(name, replacement, f);
            let g2 = substitute_var(name, replacement, g);
            Rc::new(EvmExpr::ExtCall(a2, b2, c2, d2, e2, f2, g2))
        }
        EvmExpr::Call(n, args) => {
            let a2 = substitute_var(name, replacement, args);
            Rc::new(EvmExpr::Call(n.clone(), a2))
        }
        EvmExpr::Function(n, in_ty, out_ty, body) => {
            let b2 = substitute_var(name, replacement, body);
            Rc::new(EvmExpr::Function(n.clone(), in_ty.clone(), out_ty.clone(), b2))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_helpers;
    use crate::schema::*;

    fn ctx() -> EvmContext {
        EvmContext::InFunction("test".to_owned())
    }

    #[test]
    fn test_dead_variable_elimination() {
        // LetBind(x, 0, Drop(x)) — x is never read
        let name = "__local_x".to_owned();
        let zero = ast_helpers::const_int(0, ctx());
        let body = ast_helpers::drop_var(name.clone());
        let expr = Rc::new(EvmExpr::LetBind(name, zero, body));

        let optimized = optimize_expr(&expr);
        // Should eliminate the LetBind since init is pure and no reads
        assert!(matches!(optimized.as_ref(), EvmExpr::Drop(_)));
    }

    #[test]
    fn test_single_use_inline() {
        // LetBind(x, 42, Concat(Var(x), Drop(x))) → Concat(42, Drop(x))
        let name = "__local_x".to_owned();
        let val = ast_helpers::const_int(42, ctx());
        let body = Rc::new(EvmExpr::Concat(
            Rc::new(EvmExpr::Var(name.clone())),
            ast_helpers::drop_var(name.clone()),
        ));
        let expr = Rc::new(EvmExpr::LetBind(name, val, body));

        let optimized = optimize_expr(&expr);
        // Single-use inline should substitute 42 for Var(x)
        match optimized.as_ref() {
            EvmExpr::Concat(a, _drop) => {
                assert!(matches!(
                    a.as_ref(),
                    EvmExpr::Const(EvmConstant::SmallInt(42), _, _)
                ));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn test_multi_use_const_prop() {
        // LetBind(x, 42, Concat(Add(Var(x), Var(x)), Drop(x))) → Concat(Add(42, 42), Drop(x))
        let name = "__local_x".to_owned();
        let val = ast_helpers::const_int(42, ctx());
        let add_expr = ast_helpers::add(
            Rc::new(EvmExpr::Var(name.clone())),
            Rc::new(EvmExpr::Var(name.clone())),
        );
        let body = Rc::new(EvmExpr::Concat(add_expr, ast_helpers::drop_var(name.clone())));
        let expr = Rc::new(EvmExpr::LetBind(name, val.clone(), body));

        let optimized = optimize_expr(&expr);
        // Should be Concat(Add(42, 42), Drop(x))
        match optimized.as_ref() {
            EvmExpr::Concat(add, _drop) => match add.as_ref() {
                EvmExpr::Bop(EvmBinaryOp::Add, a, b) => {
                    assert!(matches!(
                        a.as_ref(),
                        EvmExpr::Const(EvmConstant::SmallInt(42), _, _)
                    ));
                    assert!(matches!(
                        b.as_ref(),
                        EvmExpr::Const(EvmConstant::SmallInt(42), _, _)
                    ));
                }
                other => panic!("unexpected add: {other:?}"),
            },
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn test_no_inline_in_loop() {
        // LetBind(x, 42, DoWhile(Empty, Var(x))) — x in loop, don't inline
        let name = "__local_x".to_owned();
        let val = ast_helpers::const_int(42, ctx());
        let loop_body = Rc::new(EvmExpr::DoWhile(
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx()),
            Rc::new(EvmExpr::Var(name.clone())),
        ));
        let expr = Rc::new(EvmExpr::LetBind(name.clone(), val, loop_body));

        let optimized = optimize_expr(&expr);
        // Should still be a LetBind (in_loop blocks inline and const prop)
        assert!(matches!(optimized.as_ref(), EvmExpr::LetBind(..)));
    }

    #[test]
    fn test_store_then_return_forwarding() {
        // LetBind(c, 0,
        //   Concat(VarStore(c, 42),
        //     Concat(Var(c), Drop(c))))
        // → Concat(42, Drop(c))
        // (VarStore value forwarded to Var read, LetBind eliminated)
        let name = "__local_c".to_owned();
        let zero = ast_helpers::const_int(0, ctx());
        let store_val = ast_helpers::const_int(42, ctx());
        let body = Rc::new(EvmExpr::Concat(
            Rc::new(EvmExpr::VarStore(name.clone(), store_val)),
            Rc::new(EvmExpr::Concat(
                Rc::new(EvmExpr::Var(name.clone())),
                ast_helpers::drop_var(name.clone()),
            )),
        ));
        let expr = Rc::new(EvmExpr::LetBind(name, zero, body));

        let optimized = optimize_expr(&expr);
        // Should forward 42 from VarStore to Var, eliminate LetBind.
        // Result: Concat(Empty, Concat(42, Drop(c))) — Empty from removed VarStore.
        // Verify no LetBind and no VarStore remain, and 42 is present.
        assert!(!matches!(optimized.as_ref(), EvmExpr::LetBind(..)), "LetBind should be eliminated");
        // Check that the value 42 appears somewhere in the result
        fn contains_42(e: &EvmExpr) -> bool {
            match e {
                EvmExpr::Const(EvmConstant::SmallInt(42), _, _) => true,
                EvmExpr::Concat(a, b) => contains_42(a) || contains_42(b),
                _ => false,
            }
        }
        assert!(contains_42(&optimized), "expected 42 in result, got: {optimized:?}");
    }
}
