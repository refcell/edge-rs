//! Variable optimization pass (counting-based transforms).
//!
//! Runs on the `RcExpr` IR tree BEFORE egglog equality saturation.
//! Only performs transforms that require occurrence counting, which
//! egglog's pattern matching cannot express:
//!
//! 1. **Dead variable elimination**: Remove `LetBinds` whose variable is never read
//! 2. **Single-use inlining**: Inline `LetBind` init directly at sole Var reference
//! 3. **Multi-use constant propagation**: Replace Var refs with the constant value
//!
//! Store-forwarding is handled at the lowering level (`to_egglog.rs`), not here.

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::schema::{EvmBinaryOp, EvmConstant, EvmContext, EvmExpr, EvmTernaryOp, EvmType, RcExpr};

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
    /// Total number of Var(name) reads in the `LetBind` body (for last-use DUP elision).
    pub read_count: usize,
}

/// Per-variable usage statistics.
#[derive(Debug, Clone, Default)]
struct VarInfo {
    /// Number of times `Var(name)` appears in the `LetBind` body
    read_count: usize,
    /// Number of times `VarStore(name, _)` appears in the `LetBind` body
    write_count: usize,
    /// Whether any reference is inside a `DoWhile` loop
    in_loop: bool,
}

/// Analyze all `LetBind` variables in an expression and decide allocation mode.
///
/// Returns a map from variable name to `VarAllocation`. Variables not in the
/// map default to `Memory` with `read_count` 0.
///
/// A variable is eligible for stack allocation if:
/// - It is not referenced inside a loop
/// - It has a bounded number of reads (`read_count <= 16`)
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
            let mode = if !info.in_loop && info.read_count <= 16 {
                AllocationMode::Stack
            } else {
                AllocationMode::Memory
            };
            let alloc = VarAllocation {
                mode,
                read_count: info.read_count,
            };
            // If the name already exists (from another function's same-named local),
            // use the more conservative allocation: Memory beats Stack.
            result
                .entry(name.clone())
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
        EvmExpr::Log(_, topics, data_offset, data_size, state) => {
            for t in topics {
                collect_allocations(t, result);
            }
            collect_allocations(data_offset, result);
            collect_allocations(data_size, result);
            collect_allocations(state, result);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            for x in [a, b, c, d, e, f, g] {
                collect_allocations(x, result);
            }
        }
        EvmExpr::Call(_, args) => {
            for arg in args {
                collect_allocations(arg, result);
            }
        }
        EvmExpr::VarStore(_, val) => collect_allocations(val, result),
        EvmExpr::Function(_, _, _, body) => collect_allocations(body, result),
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => {}
        EvmExpr::InlineAsm(inputs, ..) => {
            for input in inputs {
                collect_allocations(input, result);
            }
        }
    }
}

/// Optimize an entire program's contract runtimes.
///
/// At O1+, inline all Call nodes by substituting arguments and renaming
/// locals for uniqueness. At O0, keep original calls (codegen handles
/// arg passing via the stack as JUMP subroutines).
pub fn optimize_program(program: &mut crate::schema::EvmProgram, optimization_level: u8) {
    for contract in &mut program.contracts {
        contract.runtime = optimize_expr(&contract.runtime);
        if optimization_level >= 1 {
            // Inline: substitute args, rename locals, splice body at call site.
            // Include both internal and free functions.
            // Free functions first so internal functions (value-only, no
            // MSTORE/RETURN epilogue) overwrite in the HashMap when names match.
            let all_functions: Vec<_> = program
                .free_functions
                .iter()
                .chain(contract.internal_functions.iter())
                .cloned()
                .collect();
            inline_calls(&mut contract.runtime, &all_functions);
        }
        // Insert early Drops in halting branches for better dead-var-elim
        contract.runtime = insert_early_drops(&contract.runtime);
        contract.constructor = insert_early_drops(&contract.constructor);
        // NOTE: tighten_drops runs LATER in the pipeline (after store forwarding
        // at O0, or after egglog at O1+) because store forwarding can expose new
        // Var references that were previously hidden in state chains.
        // Optimize internal function bodies
        for func in &mut contract.internal_functions {
            *func = optimize_expr(func);
        }
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
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, l, r))
        }
        EvmExpr::Uop(op, inner) => {
            let i = optimize_expr(inner);
            if Rc::ptr_eq(&i, inner) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, i))
        }
        EvmExpr::Top(op, a, b, c) => {
            let a2 = optimize_expr(a);
            let b2 = optimize_expr(b);
            let c2 = optimize_expr(c);
            if Rc::ptr_eq(&a2, a) && Rc::ptr_eq(&b2, b) && Rc::ptr_eq(&c2, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Top(*op, a2, b2, c2))
        }
        EvmExpr::Get(inner, idx) => {
            let i = optimize_expr(inner);
            if Rc::ptr_eq(&i, inner) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(i, *idx))
        }
        EvmExpr::Concat(a, b) => {
            let a2 = optimize_expr(a);
            let b2 = optimize_expr(b);
            if Rc::ptr_eq(&a2, a) && Rc::ptr_eq(&b2, b) {
                return Rc::clone(expr);
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
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::If(c, i, t, e))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let i = optimize_expr(inputs);
            let b = optimize_expr(body);
            if Rc::ptr_eq(&i, inputs) && Rc::ptr_eq(&b, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DoWhile(i, b))
        }
        EvmExpr::EnvRead(op, state) => {
            let s = optimize_expr(state);
            if Rc::ptr_eq(&s, state) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::EnvRead(*op, s))
        }
        EvmExpr::EnvRead1(op, arg, state) => {
            let a = optimize_expr(arg);
            let s = optimize_expr(state);
            if Rc::ptr_eq(&a, arg) && Rc::ptr_eq(&s, state) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::EnvRead1(*op, a, s))
        }
        EvmExpr::Log(count, topics, data_offset, data_size, state) => {
            let ts: Vec<_> = topics.iter().map(optimize_expr).collect();
            let doff = optimize_expr(data_offset);
            let dsz = optimize_expr(data_size);
            let s = optimize_expr(state);
            Rc::new(EvmExpr::Log(*count, ts, doff, dsz, s))
        }
        EvmExpr::Revert(off, sz, state) => {
            let o = optimize_expr(off);
            let s = optimize_expr(sz);
            let st = optimize_expr(state);
            if Rc::ptr_eq(&o, off) && Rc::ptr_eq(&s, sz) && Rc::ptr_eq(&st, state) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Revert(o, s, st))
        }
        EvmExpr::ReturnOp(off, sz, state) => {
            let o = optimize_expr(off);
            let s = optimize_expr(sz);
            let st = optimize_expr(state);
            if Rc::ptr_eq(&o, off) && Rc::ptr_eq(&s, sz) && Rc::ptr_eq(&st, state) {
                return Rc::clone(expr);
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
            let new_args: Vec<_> = args.iter().map(optimize_expr).collect();
            if new_args
                .iter()
                .zip(args.iter())
                .all(|(n, o)| Rc::ptr_eq(n, o))
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Call(name.clone(), new_args))
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
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::VarStore(name.clone(), v))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let b = optimize_expr(body);
            if Rc::ptr_eq(&b, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Function(
                name.clone(),
                in_ty.clone(),
                out_ty.clone(),
                b,
            ))
        }
        // Leaf nodes — no children to optimize
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => Rc::clone(expr),
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs.iter().map(optimize_expr).collect();
            if new_inputs
                .iter()
                .zip(inputs.iter())
                .all(|(n, o)| Rc::ptr_eq(n, o))
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }
    }
}

/// Apply optimization transforms at a single node (children already optimized).
fn apply_transforms(expr: &RcExpr) -> RcExpr {
    if let EvmExpr::LetBind(name, init, body) = expr.as_ref() {
        let info = analyze_var(name, body);
        return apply_letbind_opts(name, init, body, expr, &info);
    }

    Rc::clone(expr)
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
            return Rc::clone(body);
        }
        // Keep side effects
        return Rc::new(EvmExpr::Concat(Rc::clone(init), Rc::clone(body)));
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
            }
            return Rc::new(EvmExpr::Concat(Rc::clone(init), new_body));
        }
    }

    // 3. Multi-use constant propagation: constant init, never written
    if info.write_count == 0 && !info.in_loop && is_const(init) {
        return substitute_var(name, init, body);
    }

    Rc::clone(expr)
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
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => {}
        EvmExpr::InlineAsm(inputs, ..) => {
            for input in inputs {
                analyze_var_inner(name, input, in_loop, info);
            }
        }
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
        EvmExpr::Log(_, topics, data_offset, data_size, _state) => {
            for t in topics {
                analyze_var_inner(name, t, in_loop, info);
            }
            analyze_var_inner(name, data_offset, in_loop, info);
            analyze_var_inner(name, data_size, in_loop, info);
        }
        // ExtCall: last arg (g) is state — skip it.
        EvmExpr::ExtCall(a, b, c, d, e, f, _g) => {
            for x in [a, b, c, d, e, f] {
                analyze_var_inner(name, x, in_loop, info);
            }
        }
        EvmExpr::Call(_, args) => {
            for arg in args {
                analyze_var_inner(name, arg, in_loop, info);
            }
        }
        EvmExpr::Function(_, _, _, body) => {
            analyze_var_inner(name, body, in_loop, info);
        }
    }
}

/// Forward the value from a `VarStore` to the subsequent Var read.
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
/// returning (val, `body_without_VarStore`).
fn extract_store_value(name: &str, expr: &RcExpr) -> Option<(RcExpr, RcExpr)> {
    match expr.as_ref() {
        EvmExpr::VarStore(n, val) if n == name => {
            // Replace VarStore with Empty (side-effect-free placeholder)
            Some((
                Rc::clone(val),
                Rc::new(EvmExpr::Empty(
                    crate::schema::EvmType::Base(crate::schema::EvmBaseType::UnitT),
                    crate::schema::EvmContext::InFunction("__opt__".to_owned()),
                )),
            ))
        }
        EvmExpr::Concat(a, b) => {
            // Try left side first
            if let Some((val, new_a)) = extract_store_value(name, a) {
                return Some((val, Rc::new(EvmExpr::Concat(new_a, Rc::clone(b)))));
            }
            // Try right side
            if let Some((val, new_b)) = extract_store_value(name, b) {
                return Some((val, Rc::new(EvmExpr::Concat(Rc::clone(a), new_b))));
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
        | EvmExpr::Selector(_)
        | EvmExpr::EnvRead(..)
        | EvmExpr::EnvRead1(..)
        | EvmExpr::Function(..)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => true,
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
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => is_pure(a),
        EvmExpr::Top(op, a, b, c) => match op {
            EvmTernaryOp::Select | EvmTernaryOp::Keccak256 => {
                is_pure(a) && is_pure(b) && is_pure(c)
            }
            _ => false,
        },
        EvmExpr::Concat(a, b) => is_pure(a) && is_pure(b),
        EvmExpr::LetBind(_, init, body) => is_pure(init) && is_pure(body),
        EvmExpr::If(c, i, t, e) => is_pure(c) && is_pure(i) && is_pure(t) && is_pure(e),
        // InlineAsm is opaque — may have side effects
        EvmExpr::InlineAsm(..)
        | EvmExpr::VarStore(..)
        | EvmExpr::Log(..)
        | EvmExpr::Revert(..)
        | EvmExpr::ReturnOp(..)
        | EvmExpr::ExtCall(..)
        | EvmExpr::DoWhile(..)
        | EvmExpr::Call(..) => false,
    }
}

/// Collect names of immutable variables (`LetBinds` with no `VarStore` in body).
///
/// These variables always have the same value as their init expression,
/// so egglog can propagate bounds from the init to Var references.
pub fn collect_immutable_vars(expr: &RcExpr) -> Vec<String> {
    let mut immutable = HashSet::new();
    let mut mutable = HashSet::new();
    collect_immutable_vars_rec(expr, &mut immutable, &mut mutable);
    // A name is only truly immutable if ALL LetBinds with that name have
    // write_count == 0. Different functions can reuse the same local name
    // (e.g., two functions both having a variable `r` → `$__local_r`),
    // and egglog merges identical Var(name) nodes across the e-graph.
    // If one LetBind is mutable (has VarStore) and another is immutable,
    // const_prop on the immutable one would corrupt the mutable one.
    immutable.difference(&mutable).cloned().collect()
}

fn collect_immutable_vars_rec(
    expr: &RcExpr,
    immutable: &mut HashSet<String>,
    mutable: &mut HashSet<String>,
) {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            let info = analyze_var(name, body);
            if info.write_count == 0 {
                immutable.insert(name.clone());
            } else {
                mutable.insert(name.clone());
            }
            collect_immutable_vars_rec(init, immutable, mutable);
            collect_immutable_vars_rec(body, immutable, mutable);
        }
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) | EvmExpr::DoWhile(a, b) => {
            collect_immutable_vars_rec(a, immutable, mutable);
            collect_immutable_vars_rec(b, immutable, mutable);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_immutable_vars_rec(c, immutable, mutable);
            collect_immutable_vars_rec(i, immutable, mutable);
            collect_immutable_vars_rec(t, immutable, mutable);
            collect_immutable_vars_rec(e, immutable, mutable);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => {
            collect_immutable_vars_rec(a, immutable, mutable);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_immutable_vars_rec(a, immutable, mutable);
            collect_immutable_vars_rec(b, immutable, mutable);
            collect_immutable_vars_rec(c, immutable, mutable);
        }
        EvmExpr::Log(_, topics, data_offset, data_size, state) => {
            for t in topics {
                collect_immutable_vars_rec(t, immutable, mutable);
            }
            collect_immutable_vars_rec(data_offset, immutable, mutable);
            collect_immutable_vars_rec(data_size, immutable, mutable);
            collect_immutable_vars_rec(state, immutable, mutable);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            for x in [a, b, c, d, e, f, g] {
                collect_immutable_vars_rec(x, immutable, mutable);
            }
        }
        EvmExpr::VarStore(_, val) => {
            collect_immutable_vars_rec(val, immutable, mutable);
        }
        EvmExpr::Call(_, args) => {
            for arg in args {
                collect_immutable_vars_rec(arg, immutable, mutable);
            }
        }
        EvmExpr::Function(_, _, _, body) => {
            collect_immutable_vars_rec(body, immutable, mutable);
        }
        EvmExpr::EnvRead(_, s) => {
            collect_immutable_vars_rec(s, immutable, mutable);
        }
        EvmExpr::EnvRead1(_, a, s) => {
            collect_immutable_vars_rec(a, immutable, mutable);
            collect_immutable_vars_rec(s, immutable, mutable);
        }
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(_)
        | EvmExpr::Drop(_)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => {}
        EvmExpr::InlineAsm(inputs, ..) => {
            for input in inputs {
                collect_immutable_vars_rec(input, immutable, mutable);
            }
        }
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
/// Call this on the full expression tree after other `var_opt` passes.
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
            Rc::new(EvmExpr::Function(
                name.clone(),
                in_ty.clone(),
                out_ty.clone(),
                new_body,
            ))
        }
        // Leaf and other nodes: no structural changes needed
        _ => Rc::clone(expr),
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

/// Check if an expression references a variable by name (Var, `VarStore`, or Drop).
///
/// This follows ALL sub-expressions including state parameters.
/// Used by `insert_early_drops` which needs full reachability.
fn references_var(expr: &RcExpr, name: &str) -> bool {
    references_var_inner(expr, name, true)
}

/// Check if an expression references a variable in a data-flow sense.
///
/// Ignores state parameters (which chain through all prior side-effecting ops
/// and would make every expression appear to reference every prior variable).
/// Also ignores Drop nodes, which are lifetime markers, not data uses.
/// Used by `tighten_drops` to find the last actual use of a variable.
fn references_var_dataflow(expr: &RcExpr, name: &str) -> bool {
    references_var_inner(expr, name, false)
}

fn references_var_inner(expr: &RcExpr, name: &str, follow_state: bool) -> bool {
    match expr.as_ref() {
        EvmExpr::Var(n) => n == name,
        EvmExpr::Drop(n) => follow_state && n == name,
        EvmExpr::VarStore(n, val) => n == name || references_var_inner(val, name, follow_state),
        EvmExpr::LetBind(n, init, body) => {
            references_var_inner(init, name, follow_state)
                || (n != name && references_var_inner(body, name, follow_state))
        }
        EvmExpr::Concat(a, b) => {
            references_var_inner(a, name, follow_state)
                || references_var_inner(b, name, follow_state)
        }
        EvmExpr::Bop(op, a, b) => {
            use crate::schema::EvmBinaryOp::*;
            let a_ref = references_var_inner(a, name, follow_state);
            // For state-consuming binary ops, b is the state parameter
            let b_is_state = matches!(op, SLoad | TLoad | MLoad | CalldataLoad);
            let b_ref = if b_is_state && !follow_state {
                false
            } else {
                references_var_inner(b, name, follow_state)
            };
            a_ref || b_ref
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => references_var_inner(a, name, follow_state),
        EvmExpr::Top(op, a, b, c) => {
            use crate::schema::EvmTernaryOp::*;
            let c_is_state = matches!(
                op,
                SStore | TStore | MStore | MStore8 | Keccak256 | CalldataCopy | Mcopy
            );
            references_var_inner(a, name, follow_state)
                || references_var_inner(b, name, follow_state)
                || if c_is_state && !follow_state {
                    false
                } else {
                    references_var_inner(c, name, follow_state)
                }
        }
        EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            // c is always state for Revert/ReturnOp
            references_var_inner(a, name, follow_state)
                || references_var_inner(b, name, follow_state)
                || if follow_state {
                    references_var_inner(c, name, follow_state)
                } else {
                    false
                }
        }
        EvmExpr::If(c, i, t, e) => {
            references_var_inner(c, name, follow_state)
                || references_var_inner(i, name, follow_state)
                || references_var_inner(t, name, follow_state)
                || references_var_inner(e, name, follow_state)
        }
        EvmExpr::DoWhile(inputs, body) => {
            references_var_inner(inputs, name, follow_state)
                || references_var_inner(body, name, follow_state)
        }
        EvmExpr::EnvRead(_, s) => {
            if follow_state {
                references_var_inner(s, name, follow_state)
            } else {
                false
            }
        }
        EvmExpr::EnvRead1(_, a, s) => {
            references_var_inner(a, name, follow_state)
                || if follow_state {
                    references_var_inner(s, name, follow_state)
                } else {
                    false
                }
        }
        EvmExpr::Log(_, topics, data_offset, data_size, state) => {
            topics
                .iter()
                .any(|t| references_var_inner(t, name, follow_state))
                || references_var_inner(data_offset, name, follow_state)
                || references_var_inner(data_size, name, follow_state)
                || if follow_state {
                    references_var_inner(state, name, follow_state)
                } else {
                    false
                }
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            // Last arg (g) is state
            let args: &[&RcExpr] = if follow_state {
                &[a, b, c, d, e, f, g]
            } else {
                &[a, b, c, d, e, f]
            };
            args.iter()
                .any(|x| references_var_inner(x, name, follow_state))
        }
        EvmExpr::Call(_, args) => args
            .iter()
            .any(|a| references_var_inner(a, name, follow_state)),
        EvmExpr::Function(_, _, _, body) => references_var_inner(body, name, follow_state),
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => false,
        EvmExpr::InlineAsm(inputs, ..) => inputs
            .iter()
            .any(|i| references_var_inner(i, name, follow_state)),
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
            Rc::new(EvmExpr::Concat(Rc::clone(head), new_tail))
        }
        // Base case: wrap with Drop
        _ => Rc::new(EvmExpr::Concat(
            Rc::new(EvmExpr::Drop(var.to_owned())),
            Rc::clone(expr),
        )),
    }
}

// ============================================================
// Linear Last-Use Drop Tightening
// ============================================================
//
// Moves Drop(var) from the end of a LetBind body to immediately after
// the last top-level statement that references the variable. Only
// operates on the top-level Concat chain (linear segments) — does not
// move Drops into or across If/DoWhile boundaries.
//
// This reduces variable lifetimes, lowering peak live variables and
// enabling more stack-eligible allocations (3 gas DUP vs 6 gas MLOAD).

/// Tighten Drop placement for all contracts in a program.
/// Must be called AFTER store forwarding (which can expose new Var references).
pub fn tighten_drops_program(program: &mut crate::schema::EvmProgram) {
    for contract in &mut program.contracts {
        contract.runtime = tighten_drops_rec(&contract.runtime);
        contract.constructor = tighten_drops_rec(&contract.constructor);
        for func in &mut contract.internal_functions {
            *func = tighten_drops_rec(func);
        }
    }
}

fn tighten_drops_rec(expr: &RcExpr) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            // First recurse into init and body
            let new_init = tighten_drops_rec(init);
            let new_body = tighten_drops_rec(body);
            // Then try to tighten this LetBind's Drop
            tighten_letbind_drop(name, &new_init, &new_body)
        }
        EvmExpr::Concat(a, b) => {
            let new_a = tighten_drops_rec(a);
            let new_b = tighten_drops_rec(b);
            Rc::new(EvmExpr::Concat(new_a, new_b))
        }
        EvmExpr::If(cond, inputs, then_body, else_body) => {
            let new_cond = tighten_drops_rec(cond);
            let new_inputs = tighten_drops_rec(inputs);
            let new_then = tighten_drops_rec(then_body);
            let new_else = tighten_drops_rec(else_body);
            Rc::new(EvmExpr::If(new_cond, new_inputs, new_then, new_else))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let new_inputs = tighten_drops_rec(inputs);
            let new_body = tighten_drops_rec(body);
            Rc::new(EvmExpr::DoWhile(new_inputs, new_body))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let new_body = tighten_drops_rec(body);
            Rc::new(EvmExpr::Function(
                name.clone(),
                in_ty.clone(),
                out_ty.clone(),
                new_body,
            ))
        }
        // Leaf and other nodes: no structural changes needed
        _ => Rc::clone(expr),
    }
}

/// Flatten a Concat chain into a Vec of statements (left-to-right execution order).
/// Non-Concat nodes become single elements. Nested `LetBind`s are kept as opaque elements.
fn flatten_concat(expr: &RcExpr, out: &mut Vec<RcExpr>) {
    match expr.as_ref() {
        EvmExpr::Concat(a, b) => {
            flatten_concat(a, out);
            flatten_concat(b, out);
        }
        _ => out.push(Rc::clone(expr)),
    }
}

/// Rebuild a left-leaning Concat chain from a Vec of statements.
fn rebuild_concat(stmts: &[RcExpr]) -> RcExpr {
    assert!(!stmts.is_empty());
    let mut result = Rc::clone(&stmts[0]);
    for stmt in &stmts[1..] {
        result = Rc::new(EvmExpr::Concat(result, Rc::clone(stmt)));
    }
    result
}

/// Try to move Drop(name) closer to the last use in a `LetBind` body.
///
/// Algorithm:
/// 1. Flatten the body into a linear statement list
/// 2. Find and remove Drop(name) from the list
/// 3. Scan the entire list to find the last dataflow use
/// 4. Insert Drop(name) right after that use
/// 5. Rebuild the Concat chain
fn tighten_letbind_drop(name: &str, init: &RcExpr, body: &RcExpr) -> RcExpr {
    // Flatten the body into a linear list of statements.
    let mut stmts = Vec::new();
    flatten_concat(body, &mut stmts);

    // Find and remove Drop(name). It might be a bare Drop or inside a LetBind tail.
    let _drop_idx = find_and_remove_drop(name, &mut stmts);
    let Some(_drop_idx) = _drop_idx else {
        // No Drop found — nothing to tighten.
        return Rc::new(EvmExpr::LetBind(
            name.to_owned(),
            Rc::clone(init),
            Rc::clone(body),
        ));
    };

    // Find the last statement that references the variable (dataflow only).
    let mut last_use_idx = None;
    for (i, stmt) in stmts.iter().enumerate() {
        if stmt_references_var_deep(stmt, name) {
            last_use_idx = Some(i);
        }
    }

    let drop_node = Rc::new(EvmExpr::Drop(name.to_owned()));

    match last_use_idx {
        Some(idx) => {
            // Try to push Drop deeper into the last-use statement if it's a LetBind or If.
            // This handles the common case where LetBind(y, init_y, body_y) is
            // the last top-level statement referencing x — we want Drop(x) inside
            // body_y right after the last use of x there.
            //
            // Always insert at optimal position (right after last use). The Drop
            // may have been found inside a nested LetBind (via try_remove_drop_from_letbind),
            // so drop_idx is not comparable with insert_pos — we must not use it as a guard.
            if let Some(modified) = try_insert_drop_into_stmt(name, &stmts[idx], &drop_node) {
                stmts[idx] = modified;
            } else {
                stmts.insert(idx + 1, drop_node);
            }
        }
        None => {
            // No use found — variable is dead. Put Drop at start.
            stmts.insert(0, drop_node);
        }
    }

    let new_body = rebuild_concat(&stmts);
    Rc::new(EvmExpr::LetBind(name.to_owned(), Rc::clone(init), new_body))
}

/// Find Drop(name) in the flattened statement list and remove it.
/// Returns the index where it was found, or None.
/// Also searches inside `LetBind` bodies (recursively) since inner tightenings
/// may have moved the Drop into a nested `LetBind`.
fn find_and_remove_drop(name: &str, stmts: &mut Vec<RcExpr>) -> Option<usize> {
    for i in (0..stmts.len()).rev() {
        if let EvmExpr::Drop(n) = stmts[i].as_ref() {
            if n == name {
                stmts.remove(i);
                return Some(i);
            }
        }
        // Check inside LetBind bodies
        if let Some(new_stmt) = try_remove_drop_from_letbind(name, &stmts[i]) {
            stmts[i] = new_stmt;
            return Some(i);
        }
    }
    None
}

/// Try to remove Drop(name) from inside a `LetBind`'s body.
/// Returns `Some(modified_letbind)` if found, None otherwise.
fn try_remove_drop_from_letbind(name: &str, expr: &RcExpr) -> Option<RcExpr> {
    match expr.as_ref() {
        EvmExpr::LetBind(n, init, body) => {
            // Flatten body, look for Drop(name)
            let mut inner_stmts = Vec::new();
            flatten_concat(body, &mut inner_stmts);
            for i in (0..inner_stmts.len()).rev() {
                if let EvmExpr::Drop(dn) = inner_stmts[i].as_ref() {
                    if dn == name {
                        inner_stmts.remove(i);
                        if inner_stmts.is_empty() {
                            let empty = Rc::new(EvmExpr::Empty(
                                EvmType::Base(crate::schema::EvmBaseType::UnitT),
                                crate::schema::EvmContext::InFunction("__drop_removed__".into()),
                            ));
                            return Some(Rc::new(EvmExpr::LetBind(
                                n.clone(),
                                Rc::clone(init),
                                empty,
                            )));
                        }
                        let new_body = rebuild_concat(&inner_stmts);
                        return Some(Rc::new(EvmExpr::LetBind(
                            n.clone(),
                            Rc::clone(init),
                            new_body,
                        )));
                    }
                }
                // Recurse into nested LetBinds
                if let Some(new_inner) = try_remove_drop_from_letbind(name, &inner_stmts[i]) {
                    inner_stmts[i] = new_inner;
                    let new_body = rebuild_concat(&inner_stmts);
                    return Some(Rc::new(EvmExpr::LetBind(
                        n.clone(),
                        Rc::clone(init),
                        new_body,
                    )));
                }
            }
            None
        }
        _ => None,
    }
}

/// Try to insert Drop(name) inside a statement (`LetBind` or If with halting branch).
/// Returns `Some(modified_stmt)` if successful, None if we can't go deeper.
fn try_insert_drop_into_stmt(name: &str, stmt: &RcExpr, drop_node: &RcExpr) -> Option<RcExpr> {
    match stmt.as_ref() {
        EvmExpr::LetBind(n, init, body) => {
            // Push Drop inside this LetBind's body.
            let mut inner_stmts = Vec::new();
            flatten_concat(body, &mut inner_stmts);

            // Find last use of `name` inside the body
            let mut last_use_idx = None;
            for (i, s) in inner_stmts.iter().enumerate() {
                if stmt_references_var_deep(s, name) {
                    last_use_idx = Some(i);
                }
            }

            match last_use_idx {
                Some(idx) => {
                    // Try to go deeper if last use is also a LetBind
                    if let Some(modified) =
                        try_insert_drop_into_stmt(name, &inner_stmts[idx], drop_node)
                    {
                        inner_stmts[idx] = modified;
                    } else {
                        inner_stmts.insert(idx + 1, Rc::clone(drop_node));
                    }
                    let new_body = rebuild_concat(&inner_stmts);
                    Some(Rc::new(EvmExpr::LetBind(
                        n.clone(),
                        Rc::clone(init),
                        new_body,
                    )))
                }
                None => {
                    // Variable only used in init, not body — insert at start of body
                    inner_stmts.insert(0, Rc::clone(drop_node));
                    let new_body = rebuild_concat(&inner_stmts);
                    Some(Rc::new(EvmExpr::LetBind(
                        n.clone(),
                        Rc::clone(init),
                        new_body,
                    )))
                }
            }
        }
        EvmExpr::If(cond, inputs, then_body, else_body) => {
            let then_halts = expr_definitely_halts(then_body);
            let else_halts = expr_definitely_halts(else_body);
            if then_halts && !else_halts {
                // Push Drop into else (non-halting) branch
                let mut else_stmts = Vec::new();
                flatten_concat(else_body, &mut else_stmts);
                else_stmts.push(Rc::clone(drop_node));
                let new_else = rebuild_concat(&else_stmts);
                Some(Rc::new(EvmExpr::If(
                    Rc::clone(cond),
                    Rc::clone(inputs),
                    Rc::clone(then_body),
                    new_else,
                )))
            } else if else_halts && !then_halts {
                let mut then_stmts = Vec::new();
                flatten_concat(then_body, &mut then_stmts);
                then_stmts.push(Rc::clone(drop_node));
                let new_then = rebuild_concat(&then_stmts);
                Some(Rc::new(EvmExpr::If(
                    Rc::clone(cond),
                    Rc::clone(inputs),
                    new_then,
                    Rc::clone(else_body),
                )))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if a statement references a variable in a dataflow sense.
/// For top-level statements, this checks the statement itself.
/// For `LetBind`s, this recursively checks both init and body (since
/// variables from outer scopes can be used in inner `LetBind` inits).
fn stmt_references_var_deep(stmt: &RcExpr, name: &str) -> bool {
    match stmt.as_ref() {
        EvmExpr::LetBind(_, init, body) => {
            // Check init — outer variables can be used here
            references_var_dataflow(init, name) || stmt_references_var_deep_in_body(body, name)
        }
        EvmExpr::Drop(_) => false, // Drops aren't data-flow uses
        _ => references_var_dataflow(stmt, name),
    }
}

/// Check if a `LetBind` body (which may contain nested `LetBind`s) references
/// a variable from an outer scope.
fn stmt_references_var_deep_in_body(body: &RcExpr, name: &str) -> bool {
    let mut stmts = Vec::new();
    flatten_concat(body, &mut stmts);
    stmts.iter().any(|s| stmt_references_var_deep(s, name))
}

/// Substitute all occurrences of `Var(name)` with `replacement` in `expr`.
fn substitute_var(name: &str, replacement: &RcExpr, expr: &RcExpr) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::Var(n) if n == name => Rc::clone(replacement),
        // Leaf nodes
        EvmExpr::Var(_)
        | EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Selector(_)
        | EvmExpr::Drop(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => Rc::clone(expr),
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs
                .iter()
                .map(|i| substitute_var(name, replacement, i))
                .collect();
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }

        // Stop at shadowing LetBind
        EvmExpr::LetBind(n, init, body) => {
            let new_init = substitute_var(name, replacement, init);
            if n == name {
                Rc::new(EvmExpr::LetBind(n.clone(), new_init, Rc::clone(body)))
            } else {
                let new_body = substitute_var(name, replacement, body);
                Rc::new(EvmExpr::LetBind(n.clone(), new_init, new_body))
            }
        }

        EvmExpr::VarStore(n, val) => {
            let new_val = substitute_var(name, replacement, val);
            Rc::new(EvmExpr::VarStore(n.clone(), new_val))
        }

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
        EvmExpr::Log(count, topics, data_offset, data_size, state) => {
            let ts: Vec<_> = topics
                .iter()
                .map(|t| substitute_var(name, replacement, t))
                .collect();
            let doff = substitute_var(name, replacement, data_offset);
            let dsz = substitute_var(name, replacement, data_size);
            let s2 = substitute_var(name, replacement, state);
            Rc::new(EvmExpr::Log(*count, ts, doff, dsz, s2))
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
            let new_args: Vec<_> = args
                .iter()
                .map(|a| substitute_var(name, replacement, a))
                .collect();
            Rc::new(EvmExpr::Call(n.clone(), new_args))
        }
        EvmExpr::Function(n, in_ty, out_ty, body) => {
            let b2 = substitute_var(name, replacement, body);
            Rc::new(EvmExpr::Function(
                n.clone(),
                in_ty.clone(),
                out_ty.clone(),
                b2,
            ))
        }
    }
}

// ============================================================
// Call-site monomorphization
// ============================================================
//
// For each `Call("f", [a, b, c])`, create a specialized nullary function
// `Function("f__site_N", UnitT, out_ty, body_with_args_substituted)`
// and replace the call with `Call("f__site_N", [])`.
//
// This lets egglog control the inlining decision via the existing
// nullary inline rule: `Call(name, Nil) + Function(name, ..., body) → body`.
// At O0 (no egglog), the specialized functions remain as JUMP subroutines.

/// Monomorphize call sites: specialize each Call into a nullary Call + specialized Function.
/// Returns the new specialized functions to add to `internal_functions`.
///
/// Recursively monomorphizes calls within specialized function bodies too
/// (e.g. if `_triple` calls `_double`, the specialized `_triple__site_N` body
/// will also have its `_double(...)` call monomorphized).
/// Inline all Call nodes in `runtime` by substituting function arguments
/// and renaming local variables for uniqueness. Recursive calls within
/// inlined bodies are also resolved.
pub fn inline_calls(runtime: &mut RcExpr, functions: &[RcExpr]) {
    // Build map: name → (in_ty, out_ty, body)
    let mut func_map: HashMap<String, (EvmType, EvmType, RcExpr)> = HashMap::new();
    for func in functions {
        if let EvmExpr::Function(name, in_ty, out_ty, body) = func.as_ref() {
            func_map.insert(
                name.clone(),
                (in_ty.clone(), out_ty.clone(), Rc::clone(body)),
            );
        }
    }
    if func_map.is_empty() {
        return;
    }
    let mut site_counter: usize = 0;
    let mut new_functions: Vec<RcExpr> = Vec::new();
    *runtime = monomorphize_rec(runtime, &func_map, &mut site_counter, &mut new_functions);
}

#[allow(clippy::only_used_in_recursion)]
fn monomorphize_rec(
    expr: &RcExpr,
    funcs: &HashMap<String, (EvmType, EvmType, RcExpr)>,
    site_counter: &mut usize,
    new_functions: &mut Vec<RcExpr>,
) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::Call(name, args) => {
            // Recursively monomorphize within the args first
            let new_args: Vec<RcExpr> = args
                .iter()
                .map(|a| monomorphize_rec(a, funcs, site_counter, new_functions))
                .collect();
            if let Some((in_ty, _out_ty, body)) = funcs.get(name) {
                // Inline the function body directly at the call site,
                // substituting arguments and renaming locals for uniqueness.
                let site_id = *site_counter;
                *site_counter += 1;
                let substituted = substitute_args(body, in_ty, &new_args);
                let inlined = rename_locals(&substituted, &format!("_s{site_id}"));
                // Recursively process the inlined body (it may contain more calls)
                monomorphize_rec(&inlined, funcs, site_counter, new_functions)
            } else {
                Rc::new(EvmExpr::Call(name.clone(), new_args))
            }
        }
        // Recurse into all children
        EvmExpr::Concat(a, b) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(b, funcs, site_counter, new_functions);
            if Rc::ptr_eq(&a2, a) && Rc::ptr_eq(&b2, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Concat(a2, b2))
        }
        EvmExpr::Bop(op, a, b) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(b, funcs, site_counter, new_functions);
            if Rc::ptr_eq(&a2, a) && Rc::ptr_eq(&b2, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, a2, b2))
        }
        EvmExpr::Uop(op, a) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            if Rc::ptr_eq(&a2, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, a2))
        }
        EvmExpr::Get(a, idx) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            if Rc::ptr_eq(&a2, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(a2, *idx))
        }
        EvmExpr::If(c, i, t, e) => {
            let c2 = monomorphize_rec(c, funcs, site_counter, new_functions);
            let i2 = monomorphize_rec(i, funcs, site_counter, new_functions);
            let t2 = monomorphize_rec(t, funcs, site_counter, new_functions);
            let e2 = monomorphize_rec(e, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::If(c2, i2, t2, e2))
        }
        EvmExpr::LetBind(name, init, body) => {
            let i2 = monomorphize_rec(init, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(body, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::LetBind(name.clone(), i2, b2))
        }
        EvmExpr::VarStore(name, val) => {
            let v2 = monomorphize_rec(val, funcs, site_counter, new_functions);
            if Rc::ptr_eq(&v2, val) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::VarStore(name.clone(), v2))
        }
        EvmExpr::Top(op, a, b, c) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(b, funcs, site_counter, new_functions);
            let c2 = monomorphize_rec(c, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::Top(*op, a2, b2, c2))
        }
        EvmExpr::Revert(a, b, c) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(b, funcs, site_counter, new_functions);
            let c2 = monomorphize_rec(c, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::Revert(a2, b2, c2))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(b, funcs, site_counter, new_functions);
            let c2 = monomorphize_rec(c, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::ReturnOp(a2, b2, c2))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let i2 = monomorphize_rec(inputs, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(body, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::DoWhile(i2, b2))
        }
        EvmExpr::Log(count, topics, data_off, data_sz, state) => {
            let topics2: Vec<_> = topics
                .iter()
                .map(|t| monomorphize_rec(t, funcs, site_counter, new_functions))
                .collect();
            let d2 = monomorphize_rec(data_off, funcs, site_counter, new_functions);
            let s2 = monomorphize_rec(data_sz, funcs, site_counter, new_functions);
            let st2 = monomorphize_rec(state, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::Log(*count, topics2, d2, s2, st2))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let a2 = monomorphize_rec(a, funcs, site_counter, new_functions);
            let b2 = monomorphize_rec(b, funcs, site_counter, new_functions);
            let c2 = monomorphize_rec(c, funcs, site_counter, new_functions);
            let d2 = monomorphize_rec(d, funcs, site_counter, new_functions);
            let e2 = monomorphize_rec(e, funcs, site_counter, new_functions);
            let f2 = monomorphize_rec(f, funcs, site_counter, new_functions);
            let g2 = monomorphize_rec(g, funcs, site_counter, new_functions);
            Rc::new(EvmExpr::ExtCall(a2, b2, c2, d2, e2, f2, g2))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs
                .iter()
                .map(|i| monomorphize_rec(i, funcs, site_counter, new_functions))
                .collect();
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }
        // Leaves (EnvRead, EnvRead1, Function, Const, Var, Arg, etc.)
        _ => Rc::clone(expr),
    }
}

/// Substitute `Arg(in_ty, _)` and `Get(Arg(in_ty, _), i)` in `body`
/// with the corresponding actual arguments.
fn substitute_args(body: &RcExpr, in_ty: &EvmType, args: &[RcExpr]) -> RcExpr {
    match body.as_ref() {
        // Single-arg function: Arg(ty, ctx) → args[0]
        EvmExpr::Arg(ty, _ctx) if ty == in_ty && args.len() == 1 => Rc::clone(&args[0]),
        // Multi-arg function: Get(Arg(ty, ctx), i) → args[i]
        EvmExpr::Get(inner, idx) => {
            if let EvmExpr::Arg(ty, _ctx) = inner.as_ref() {
                if ty == in_ty {
                    if let Some(arg) = args.get(*idx) {
                        return Rc::clone(arg);
                    }
                }
            }
            // Not an arg get — recurse
            let i2 = substitute_args(inner, in_ty, args);
            if Rc::ptr_eq(&i2, inner) {
                return Rc::clone(body);
            }
            Rc::new(EvmExpr::Get(i2, *idx))
        }
        // Recurse into children
        EvmExpr::Concat(a, b) => {
            let a2 = substitute_args(a, in_ty, args);
            let b2 = substitute_args(b, in_ty, args);
            Rc::new(EvmExpr::Concat(a2, b2))
        }
        EvmExpr::Bop(op, a, b) => {
            let a2 = substitute_args(a, in_ty, args);
            let b2 = substitute_args(b, in_ty, args);
            Rc::new(EvmExpr::Bop(*op, a2, b2))
        }
        EvmExpr::Uop(op, a) => {
            let a2 = substitute_args(a, in_ty, args);
            Rc::new(EvmExpr::Uop(*op, a2))
        }
        EvmExpr::If(c, i, t, e) => {
            let c2 = substitute_args(c, in_ty, args);
            let i2 = substitute_args(i, in_ty, args);
            let t2 = substitute_args(t, in_ty, args);
            let e2 = substitute_args(e, in_ty, args);
            Rc::new(EvmExpr::If(c2, i2, t2, e2))
        }
        EvmExpr::LetBind(name, init, body_inner) => {
            let i2 = substitute_args(init, in_ty, args);
            let b2 = substitute_args(body_inner, in_ty, args);
            Rc::new(EvmExpr::LetBind(name.clone(), i2, b2))
        }
        EvmExpr::VarStore(name, val) => {
            let v2 = substitute_args(val, in_ty, args);
            Rc::new(EvmExpr::VarStore(name.clone(), v2))
        }
        EvmExpr::Top(op, a, b, c) => {
            let a2 = substitute_args(a, in_ty, args);
            let b2 = substitute_args(b, in_ty, args);
            let c2 = substitute_args(c, in_ty, args);
            Rc::new(EvmExpr::Top(*op, a2, b2, c2))
        }
        EvmExpr::DoWhile(inputs, body_inner) => {
            let i2 = substitute_args(inputs, in_ty, args);
            let b2 = substitute_args(body_inner, in_ty, args);
            Rc::new(EvmExpr::DoWhile(i2, b2))
        }
        EvmExpr::Revert(a, b, c) => {
            let a2 = substitute_args(a, in_ty, args);
            let b2 = substitute_args(b, in_ty, args);
            let c2 = substitute_args(c, in_ty, args);
            Rc::new(EvmExpr::Revert(a2, b2, c2))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let a2 = substitute_args(a, in_ty, args);
            let b2 = substitute_args(b, in_ty, args);
            let c2 = substitute_args(c, in_ty, args);
            Rc::new(EvmExpr::ReturnOp(a2, b2, c2))
        }
        EvmExpr::Call(name, call_args) => {
            let new_args: Vec<_> = call_args
                .iter()
                .map(|a| substitute_args(a, in_ty, args))
                .collect();
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::Log(count, topics, data_off, data_sz, state) => {
            let topics2: Vec<_> = topics
                .iter()
                .map(|t| substitute_args(t, in_ty, args))
                .collect();
            let d2 = substitute_args(data_off, in_ty, args);
            let s2 = substitute_args(data_sz, in_ty, args);
            let st2 = substitute_args(state, in_ty, args);
            Rc::new(EvmExpr::Log(*count, topics2, d2, s2, st2))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let a2 = substitute_args(a, in_ty, args);
            let b2 = substitute_args(b, in_ty, args);
            let c2 = substitute_args(c, in_ty, args);
            let d2 = substitute_args(d, in_ty, args);
            let e2 = substitute_args(e, in_ty, args);
            let f2 = substitute_args(f, in_ty, args);
            let g2 = substitute_args(g, in_ty, args);
            Rc::new(EvmExpr::ExtCall(a2, b2, c2, d2, e2, f2, g2))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs
                .iter()
                .map(|i| substitute_args(i, in_ty, args))
                .collect();
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }
        // Leaves
        _ => Rc::clone(body),
    }
}

/// Rename local variables defined by `LetBind` in an expression.
/// Only renames variables that are defined within this expression tree
/// (have a `LetBind`), not variables from outer scopes.
fn rename_locals(expr: &RcExpr, suffix: &str) -> RcExpr {
    // First, collect all variable names defined by LetBind in this tree.
    let mut defined = std::collections::HashSet::new();
    collect_letbind_names(expr, &mut defined);
    if defined.is_empty() {
        return Rc::clone(expr);
    }
    rename_locals_rec(expr, suffix, &defined)
}

/// Collect all variable names defined by `LetBind` nodes in the tree.
fn collect_letbind_names(expr: &RcExpr, names: &mut std::collections::HashSet<String>) {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            names.insert(name.clone());
            collect_letbind_names(init, names);
            collect_letbind_names(body, names);
        }
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) | EvmExpr::DoWhile(a, b) => {
            collect_letbind_names(a, names);
            collect_letbind_names(b, names);
        }
        EvmExpr::Uop(_, a) | EvmExpr::VarStore(_, a) | EvmExpr::Get(a, _) => {
            collect_letbind_names(a, names);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_letbind_names(c, names);
            collect_letbind_names(i, names);
            collect_letbind_names(t, names);
            collect_letbind_names(e, names);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_letbind_names(a, names);
            collect_letbind_names(b, names);
            collect_letbind_names(c, names);
        }
        EvmExpr::Log(_, topics, d, s, st) => {
            for t in topics {
                collect_letbind_names(t, names);
            }
            collect_letbind_names(d, names);
            collect_letbind_names(s, names);
            collect_letbind_names(st, names);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            collect_letbind_names(a, names);
            collect_letbind_names(b, names);
            collect_letbind_names(c, names);
            collect_letbind_names(d, names);
            collect_letbind_names(e, names);
            collect_letbind_names(f, names);
            collect_letbind_names(g, names);
        }
        EvmExpr::Call(_, args) => {
            for a in args {
                collect_letbind_names(a, names);
            }
        }
        EvmExpr::InlineAsm(inputs, ..) => {
            for input in inputs {
                collect_letbind_names(input, names);
            }
        }
        _ => {}
    }
}

fn rename_locals_rec(
    expr: &RcExpr,
    suffix: &str,
    defined: &std::collections::HashSet<String>,
) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            let new_name = if defined.contains(name) {
                format!("{name}{suffix}")
            } else {
                name.clone()
            };
            let i2 = rename_locals_rec(init, suffix, defined);
            let b2 = rename_locals_rec(body, suffix, defined);
            Rc::new(EvmExpr::LetBind(new_name, i2, b2))
        }
        EvmExpr::Var(name) => {
            if defined.contains(name) {
                Rc::new(EvmExpr::Var(format!("{name}{suffix}")))
            } else {
                Rc::clone(expr)
            }
        }
        EvmExpr::VarStore(name, val) => {
            let v2 = rename_locals_rec(val, suffix, defined);
            if defined.contains(name) {
                Rc::new(EvmExpr::VarStore(format!("{name}{suffix}"), v2))
            } else {
                Rc::new(EvmExpr::VarStore(name.clone(), v2))
            }
        }
        EvmExpr::Drop(name) => {
            if defined.contains(name) {
                Rc::new(EvmExpr::Drop(format!("{name}{suffix}")))
            } else {
                Rc::clone(expr)
            }
        }
        EvmExpr::Concat(a, b) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            let b2 = rename_locals_rec(b, suffix, defined);
            Rc::new(EvmExpr::Concat(a2, b2))
        }
        EvmExpr::Bop(op, a, b) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            let b2 = rename_locals_rec(b, suffix, defined);
            Rc::new(EvmExpr::Bop(*op, a2, b2))
        }
        EvmExpr::Uop(op, a) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            Rc::new(EvmExpr::Uop(*op, a2))
        }
        EvmExpr::If(c, i, t, e) => {
            let c2 = rename_locals_rec(c, suffix, defined);
            let i2 = rename_locals_rec(i, suffix, defined);
            let t2 = rename_locals_rec(t, suffix, defined);
            let e2 = rename_locals_rec(e, suffix, defined);
            Rc::new(EvmExpr::If(c2, i2, t2, e2))
        }
        EvmExpr::Top(op, a, b, c) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            let b2 = rename_locals_rec(b, suffix, defined);
            let c2 = rename_locals_rec(c, suffix, defined);
            Rc::new(EvmExpr::Top(*op, a2, b2, c2))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let i2 = rename_locals_rec(inputs, suffix, defined);
            let b2 = rename_locals_rec(body, suffix, defined);
            Rc::new(EvmExpr::DoWhile(i2, b2))
        }
        EvmExpr::Revert(a, b, c) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            let b2 = rename_locals_rec(b, suffix, defined);
            let c2 = rename_locals_rec(c, suffix, defined);
            Rc::new(EvmExpr::Revert(a2, b2, c2))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            let b2 = rename_locals_rec(b, suffix, defined);
            let c2 = rename_locals_rec(c, suffix, defined);
            Rc::new(EvmExpr::ReturnOp(a2, b2, c2))
        }
        EvmExpr::Get(a, idx) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            Rc::new(EvmExpr::Get(a2, *idx))
        }
        EvmExpr::Call(name, call_args) => {
            let new_args: Vec<_> = call_args
                .iter()
                .map(|a| rename_locals_rec(a, suffix, defined))
                .collect();
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::Log(count, topics, data_off, data_sz, state) => {
            let topics2: Vec<_> = topics
                .iter()
                .map(|t| rename_locals_rec(t, suffix, defined))
                .collect();
            let d2 = rename_locals_rec(data_off, suffix, defined);
            let s2 = rename_locals_rec(data_sz, suffix, defined);
            let st2 = rename_locals_rec(state, suffix, defined);
            Rc::new(EvmExpr::Log(*count, topics2, d2, s2, st2))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let a2 = rename_locals_rec(a, suffix, defined);
            let b2 = rename_locals_rec(b, suffix, defined);
            let c2 = rename_locals_rec(c, suffix, defined);
            let d2 = rename_locals_rec(d, suffix, defined);
            let e2 = rename_locals_rec(e, suffix, defined);
            let f2 = rename_locals_rec(f, suffix, defined);
            let g2 = rename_locals_rec(g, suffix, defined);
            Rc::new(EvmExpr::ExtCall(a2, b2, c2, d2, e2, f2, g2))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs
                .iter()
                .map(|i| rename_locals_rec(i, suffix, defined))
                .collect();
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }
        _ => Rc::clone(expr),
    }
}

// ============================================================
// Dead Store Elimination
// ============================================================
// Removes VarStore(x, val) when x hasn't been read since its last write
// (LetBind init or previous VarStore). This handles patterns like:
//   let x = 0; x = 0; ... → remove redundant VarStore
//   let x = 0; x = real_val; ... → forward real_val into LetBind init

/// Eliminate dead stores across the whole program.
pub fn dead_store_elim_program(program: &mut crate::schema::EvmProgram) {
    for contract in &mut program.contracts {
        // Run DSE in a fixed-point loop: each iteration can forward one "layer" of
        // variables through nested LetBinds. Once variable `a` is forwarded (its
        // VarStore removed), the next iteration sees `a` as non-mutable, allowing
        // variables that depend on `a` to be forwarded too.
        loop {
            let new = dead_store_elim_rec(&contract.runtime);
            if Rc::ptr_eq(&new, &contract.runtime) {
                break;
            }
            contract.runtime = new;
        }
        for func in &mut contract.internal_functions {
            loop {
                let new = dead_store_elim_rec(func);
                if Rc::ptr_eq(&new, func) {
                    break;
                }
                *func = new;
            }
        }
    }
}

/// Check if an expression reads a variable (contains Var(name)).
/// Unlike `references_var_dataflow`, this does NOT count `VarStore(name, _)` as a read —
/// only the value inside `VarStore` or standalone Var nodes count.
fn reads_var(expr: &RcExpr, name: &str) -> bool {
    match expr.as_ref() {
        EvmExpr::Var(n) => n == name,
        EvmExpr::VarStore(_, val) => {
            // Writing to name is not a read, but reading name inside the value IS.
            reads_var(val, name)
        }
        EvmExpr::LetBind(n, init, body) => {
            reads_var(init, name) || (n != name && reads_var(body, name))
        }
        EvmExpr::Concat(a, b) => reads_var(a, name) || reads_var(b, name),
        EvmExpr::Bop(op, a, b) => {
            use crate::schema::EvmBinaryOp::*;
            let b_is_state = matches!(op, SLoad | TLoad | MLoad | CalldataLoad);
            reads_var(a, name) || (!b_is_state && reads_var(b, name))
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => reads_var(a, name),
        EvmExpr::Top(op, a, b, c) => {
            use crate::schema::EvmTernaryOp::*;
            let c_is_state = matches!(
                op,
                SStore | TStore | MStore | MStore8 | Keccak256 | CalldataCopy | Mcopy
            );
            reads_var(a, name) || reads_var(b, name) || (!c_is_state && reads_var(c, name))
        }
        EvmExpr::If(c, i, t, e) => {
            reads_var(c, name) || reads_var(i, name) || reads_var(t, name) || reads_var(e, name)
        }
        EvmExpr::DoWhile(inputs, body) => reads_var(inputs, name) || reads_var(body, name),
        EvmExpr::InlineAsm(inputs, _, _) => inputs.iter().any(|inp| reads_var(inp, name)),
        EvmExpr::Call(_, args) => args.iter().any(|a| reads_var(a, name)),
        EvmExpr::ReturnOp(_, size, offset) | EvmExpr::Revert(_, size, offset) => {
            reads_var(size, name) || reads_var(offset, name)
        }
        EvmExpr::Log(_, topics, doff, dsz, _) => {
            topics.iter().any(|t| reads_var(t, name))
                || reads_var(doff, name)
                || reads_var(dsz, name)
        }
        _ => false,
    }
}

/// Walk through a nested `LetBind` chain to find and forward a `VarStore` for `name`.
///
/// When lowering creates `LetBind("a", 0, LetBind("b", 0, ... Concat(VarStore("a", val), ...)))`,
/// the normal flat DSE can't see the `VarStore` for "a" because it's buried in nested bodies.
/// This function walks through the chain, checking that:
/// 1. No nested `LetBind` init reads `name`
/// 2. No statement before the first `VarStore(name)` reads `name`
/// 3. The forwarded value doesn't reference any sibling `LetBind` variable names
///    (those variables have init=0 and get their real values from `VarStore`s in the
///    Concat chain, which haven't executed when the outer `LetBind` init is evaluated)
fn forward_through_nested_letbinds(
    name: &str,
    current_init: &RcExpr,
    body: &RcExpr,
) -> Option<RcExpr> {
    // Walk through nested LetBinds, collecting sibling variable names
    let mut sibling_names: HashSet<&str> = HashSet::new();
    let mut innermost = body;
    // Walk through nested LetBinds, collecting sibling variable names
    loop {
        match innermost.as_ref() {
            EvmExpr::LetBind(inner_name, inner_init, inner_body) => {
                if reads_var(inner_init, name) {
                    return None;
                }
                if inner_name == name {
                    return None;
                }
                sibling_names.insert(inner_name.as_str());
                innermost = inner_body;
            }
            _ => break,
        }
    }
    let depth = sibling_names.len();

    if depth == 0 {
        return None;
    }

    // Flatten the innermost body and scan for VarStore(name)
    let mut stmts = Vec::new();
    flatten_concat(innermost, &mut stmts);

    // Collect ALL variable names that are modified by VarStores in the Concat chain
    // (both at top level and inside If branches). Any variable with a VarStore has
    // a mutable value — reading it in a LetBind init (which runs before the chain)
    // would get the stale init value, not the updated one.
    let mut mutable_vars: HashSet<&str> = HashSet::new();
    for stmt in &stmts {
        collect_varstore_names(stmt, &mut mutable_vars);
    }
    // Also include sibling LetBind names — their inits are typically 0 placeholders
    mutable_vars.extend(sibling_names.iter());

    let mut read_before = false;
    for (idx, stmt) in stmts.iter().enumerate() {
        if let EvmExpr::VarStore(n, val) = stmt.as_ref() {
            if n == name {
                if read_before {
                    return None;
                }
                if !is_pure(current_init) {
                    return None;
                }
                // The forwarded value must not reference any mutable variable,
                // because those vars may have different values at LetBind init time
                // vs VarStore execution time.
                if references_any_in_set(val, &mutable_vars) {
                    return None;
                }
                let forwarded = Rc::clone(val);
                let ctx = EvmContext::InFunction("__opt__".to_string());
                let mut new_stmts = stmts;
                new_stmts[idx] = Rc::new(EvmExpr::Empty(
                    EvmType::Base(crate::schema::EvmBaseType::UnitT),
                    ctx,
                ));
                let new_innermost = rebuild_concat(&new_stmts);
                let rebuilt = rebuild_nested_letbinds(body, depth, &new_innermost);
                return Some(Rc::new(EvmExpr::LetBind(
                    name.to_owned(),
                    forwarded,
                    rebuilt,
                )));
            }
        }
        if reads_var(stmt, name) {
            read_before = true;
        }
    }
    None
}

/// Collect all variable names that are targets of `VarStore` nodes in an expression tree.
fn collect_varstore_names<'a>(expr: &'a RcExpr, names: &mut HashSet<&'a str>) {
    match expr.as_ref() {
        EvmExpr::VarStore(n, val) => {
            names.insert(n.as_str());
            collect_varstore_names(val, names);
        }
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) => {
            collect_varstore_names(a, names);
            collect_varstore_names(b, names);
        }
        EvmExpr::If(c, i, t, e) => {
            collect_varstore_names(c, names);
            collect_varstore_names(i, names);
            collect_varstore_names(t, names);
            collect_varstore_names(e, names);
        }
        EvmExpr::LetBind(_, init, body) => {
            collect_varstore_names(init, names);
            collect_varstore_names(body, names);
        }
        EvmExpr::DoWhile(a, b) => {
            collect_varstore_names(a, names);
            collect_varstore_names(b, names);
        }
        _ => {}
    }
}

/// Check if an expression references any variable name in the given set.
fn references_any_in_set(expr: &RcExpr, names: &HashSet<&str>) -> bool {
    match expr.as_ref() {
        EvmExpr::Var(n) => names.contains(n.as_str()),
        EvmExpr::VarStore(_, val) => references_any_in_set(val, names),
        EvmExpr::Drop(_) => false,
        EvmExpr::Const(..) | EvmExpr::Arg(..) | EvmExpr::Selector(..) | EvmExpr::Empty(..) => false,
        EvmExpr::LetBind(_, init, body) => {
            references_any_in_set(init, names) || references_any_in_set(body, names)
        }
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) => {
            references_any_in_set(a, names) || references_any_in_set(b, names)
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => references_any_in_set(a, names),
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            references_any_in_set(a, names)
                || references_any_in_set(b, names)
                || references_any_in_set(c, names)
        }
        EvmExpr::If(c, i, t, e) => {
            references_any_in_set(c, names)
                || references_any_in_set(i, names)
                || references_any_in_set(t, names)
                || references_any_in_set(e, names)
        }
        EvmExpr::DoWhile(a, b) => {
            references_any_in_set(a, names) || references_any_in_set(b, names)
        }
        EvmExpr::InlineAsm(inputs, _, _) => {
            inputs.iter().any(|inp| references_any_in_set(inp, names))
        }
        EvmExpr::Call(_, args) => args.iter().any(|a| references_any_in_set(a, names)),
        EvmExpr::Log(_, topics, doff, dsz, _) => {
            topics.iter().any(|t| references_any_in_set(t, names))
                || references_any_in_set(doff, names)
                || references_any_in_set(dsz, names)
        }
        _ => false,
    }
}

/// Rebuild a nested `LetBind` chain, replacing the innermost body.
fn rebuild_nested_letbinds(body: &RcExpr, depth: usize, new_innermost: &RcExpr) -> RcExpr {
    if depth == 0 {
        return Rc::clone(new_innermost);
    }
    match body.as_ref() {
        EvmExpr::LetBind(n, init, inner) => {
            let rebuilt_inner = rebuild_nested_letbinds(inner, depth - 1, new_innermost);
            Rc::new(EvmExpr::LetBind(n.clone(), Rc::clone(init), rebuilt_inner))
        }
        _ => Rc::clone(new_innermost), // shouldn't happen
    }
}

fn dead_store_elim_rec(expr: &RcExpr) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::LetBind(name, init, body) => {
            let new_init = dead_store_elim_rec(init);
            let new_body = dead_store_elim_rec(body);

            // Flatten body into statement list
            let mut stmts = Vec::new();
            flatten_concat(&new_body, &mut stmts);

            // Collect all variable names modified by VarStores in the body (excluding
            // the current variable). If the forwarded value references any of these,
            // it would read stale init values when moved to the LetBind init position.
            let body_mutable_owned: HashSet<String> = {
                let mut vars: HashSet<&str> = HashSet::new();
                for stmt in &stmts {
                    collect_varstore_names(stmt, &mut vars);
                }
                vars.remove(name.as_str());
                vars.into_iter().map(String::from).collect()
            };
            let body_mutable_vars: HashSet<&str> =
                body_mutable_owned.iter().map(|s| s.as_str()).collect();

            // Scan for dead stores: VarStore(name, val) where name hasn't been read
            // since the LetBind init (or previous VarStore to name).
            let mut read_since_write = false;
            let mut changed = false;
            let mut forwarded_init = Rc::clone(&new_init);

            for stmt in &mut stmts {
                // Check if this statement reads name
                if reads_var(stmt, name) {
                    read_since_write = true;
                }

                // Extract VarStore info before mutating stmt
                let store_info = if let EvmExpr::VarStore(n, val) = stmt.as_ref() {
                    if n == name {
                        Some((Rc::clone(val), is_pure(val), reads_var(val, name)))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((val, val_is_pure, val_reads_name)) = store_info {
                    if !read_since_write
                        && is_pure(&forwarded_init)
                        && (val_is_pure || is_pure(&forwarded_init))
                        && !references_any_in_set(&val, &body_mutable_vars)
                    {
                        // Dead store: forward new value into LetBind init,
                        // replace VarStore with Empty.
                        forwarded_init = Rc::clone(&val);
                        let ctx = EvmContext::InFunction("__opt__".to_string());
                        *stmt = Rc::new(EvmExpr::Empty(
                            EvmType::Base(crate::schema::EvmBaseType::UnitT),
                            ctx,
                        ));
                        changed = true;
                    }
                    // This VarStore writes name — reset read tracking.
                    read_since_write = val_reads_name;
                }
            }

            if changed {
                let new_body = rebuild_concat(&stmts);
                Rc::new(EvmExpr::LetBind(name.clone(), forwarded_init, new_body))
            } else if is_pure(&new_init) {
                // Try to look through nested LetBind chains for VarStores of `name`.
                // Pattern: LetBind("a", 0, LetBind("b", 0, LetBind("c", 0,
                //   Concat(VarStore("a", val), ...))))
                // The VarStore for "a" is in the innermost body, invisible to flat scan.
                if let Some(result) = forward_through_nested_letbinds(name, &new_init, &new_body) {
                    result
                } else if Rc::ptr_eq(&new_init, init) && Rc::ptr_eq(&new_body, body) {
                    Rc::clone(expr)
                } else {
                    Rc::new(EvmExpr::LetBind(name.clone(), new_init, new_body))
                }
            } else if Rc::ptr_eq(&new_init, init) && Rc::ptr_eq(&new_body, body) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::LetBind(name.clone(), new_init, new_body))
            }
        }
        EvmExpr::Concat(a, b) => {
            let na = dead_store_elim_rec(a);
            let nb = dead_store_elim_rec(b);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Concat(na, nb))
        }
        EvmExpr::If(cond, inputs, then_b, else_b) => {
            let nc = dead_store_elim_rec(cond);
            let ni = dead_store_elim_rec(inputs);
            let nt = dead_store_elim_rec(then_b);
            let ne = dead_store_elim_rec(else_b);
            if Rc::ptr_eq(&nc, cond)
                && Rc::ptr_eq(&ni, inputs)
                && Rc::ptr_eq(&nt, then_b)
                && Rc::ptr_eq(&ne, else_b)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::If(nc, ni, nt, ne))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let ni = dead_store_elim_rec(inputs);
            let nb = dead_store_elim_rec(body);
            if Rc::ptr_eq(&ni, inputs) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DoWhile(ni, nb))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let nb = dead_store_elim_rec(body);
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
        EvmExpr::VarStore(name, val) => {
            let nv = dead_store_elim_rec(val);
            if Rc::ptr_eq(&nv, val) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::VarStore(name.clone(), nv))
        }
        EvmExpr::Bop(op, a, b) => {
            let na = dead_store_elim_rec(a);
            let nb = dead_store_elim_rec(b);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, na, nb))
        }
        EvmExpr::Uop(op, a) => {
            let na = dead_store_elim_rec(a);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, na))
        }
        EvmExpr::Top(op, a, b, c) => {
            let na = dead_store_elim_rec(a);
            let nb = dead_store_elim_rec(b);
            let nc = dead_store_elim_rec(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Top(*op, na, nb, nc))
        }
        EvmExpr::Get(a, idx) => {
            let na = dead_store_elim_rec(a);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(na, *idx))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let na = dead_store_elim_rec(a);
            let nb = dead_store_elim_rec(b);
            let nc = dead_store_elim_rec(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ReturnOp(na, nb, nc))
        }
        EvmExpr::Revert(a, b, c) => {
            let na = dead_store_elim_rec(a);
            let nb = dead_store_elim_rec(b);
            let nc = dead_store_elim_rec(c);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Revert(na, nb, nc))
        }
        _ => Rc::clone(expr),
    }
}

// ============================================================
// CalldataLoad CSE
// ============================================================
// After egglog extraction, identical CalldataLoad(offset, state) nodes may
// appear multiple times in the IR tree. This pass hoists repeated calldata
// loads into LetBind variables at the top of the expression scope, replacing
// duplicates with Var references. Since calldata is immutable, this is always safe.

/// Deduplicate `CalldataLoad` nodes across the whole program.
pub fn calldataload_cse_program(program: &mut crate::schema::EvmProgram) {
    for contract in &mut program.contracts {
        contract.runtime = calldataload_cse(&contract.runtime);
        for func in &mut contract.internal_functions {
            *func = calldataload_cse(func);
        }
    }
}

/// Count `CalldataLoad(Const(offset), _)` occurrences by offset value.
fn count_calldataloads(expr: &RcExpr, counts: &mut HashMap<i64, usize>) {
    match expr.as_ref() {
        EvmExpr::Bop(EvmBinaryOp::CalldataLoad, offset, _state) => {
            if let EvmExpr::Const(EvmConstant::SmallInt(n), _, _) = offset.as_ref() {
                *counts.entry(*n).or_insert(0) += 1;
            }
        }
        // Recurse into all sub-expressions
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) | EvmExpr::DoWhile(a, b) => {
            count_calldataloads(a, counts);
            count_calldataloads(b, counts);
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) | EvmExpr::Function(_, _, _, a) => {
            count_calldataloads(a, counts);
        }
        EvmExpr::Top(_, a, b, c) => {
            count_calldataloads(a, counts);
            count_calldataloads(b, counts);
            count_calldataloads(c, counts);
        }
        EvmExpr::If(cond, inputs, then_b, else_b) => {
            count_calldataloads(cond, counts);
            count_calldataloads(inputs, counts);
            count_calldataloads(then_b, counts);
            count_calldataloads(else_b, counts);
        }
        EvmExpr::LetBind(_, init, body) => {
            count_calldataloads(init, counts);
            count_calldataloads(body, counts);
        }
        EvmExpr::VarStore(_, val) => {
            count_calldataloads(val, counts);
        }
        EvmExpr::InlineAsm(inputs, _, _) => {
            for inp in inputs {
                count_calldataloads(inp, counts);
            }
        }
        EvmExpr::Call(_, args) => {
            for a in args {
                count_calldataloads(a, counts);
            }
        }
        EvmExpr::ReturnOp(_, size, offset) | EvmExpr::Revert(_, size, offset) => {
            count_calldataloads(size, counts);
            count_calldataloads(offset, counts);
        }
        EvmExpr::Log(_, topics, doff, dsz, _) => {
            for t in topics {
                count_calldataloads(t, counts);
            }
            count_calldataloads(doff, counts);
            count_calldataloads(dsz, counts);
        }
        _ => {}
    }
}

/// Replace `CalldataLoad(Const(offset), _)` with Var references for hoisted offsets.
fn replace_calldataloads(expr: &RcExpr, hoisted: &HashMap<i64, String>) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::Bop(EvmBinaryOp::CalldataLoad, offset, _state) => {
            if let EvmExpr::Const(EvmConstant::SmallInt(n), _, _) = offset.as_ref() {
                if let Some(var_name) = hoisted.get(n) {
                    return Rc::new(EvmExpr::Var(var_name.clone()));
                }
            }
            Rc::clone(expr)
        }
        EvmExpr::Bop(op, a, b) => {
            let na = replace_calldataloads(a, hoisted);
            let nb = replace_calldataloads(b, hoisted);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Bop(*op, na, nb))
        }
        EvmExpr::Uop(op, a) => {
            let na = replace_calldataloads(a, hoisted);
            if Rc::ptr_eq(&na, a) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Uop(*op, na))
        }
        EvmExpr::Top(op, a, b, c) => {
            let na = replace_calldataloads(a, hoisted);
            let nb = replace_calldataloads(b, hoisted);
            let nc = replace_calldataloads(c, hoisted);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Top(*op, na, nb, nc))
        }
        EvmExpr::Concat(a, b) => {
            let na = replace_calldataloads(a, hoisted);
            let nb = replace_calldataloads(b, hoisted);
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Concat(na, nb))
        }
        EvmExpr::If(cond, inputs, then_b, else_b) => {
            let nc = replace_calldataloads(cond, hoisted);
            let ni = replace_calldataloads(inputs, hoisted);
            let nt = replace_calldataloads(then_b, hoisted);
            let ne = replace_calldataloads(else_b, hoisted);
            if Rc::ptr_eq(&nc, cond)
                && Rc::ptr_eq(&ni, inputs)
                && Rc::ptr_eq(&nt, then_b)
                && Rc::ptr_eq(&ne, else_b)
            {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::If(nc, ni, nt, ne))
        }
        EvmExpr::LetBind(name, init, body) => {
            let ni = replace_calldataloads(init, hoisted);
            let nb = replace_calldataloads(body, hoisted);
            if Rc::ptr_eq(&ni, init) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::LetBind(name.clone(), ni, nb))
        }
        EvmExpr::VarStore(name, val) => {
            let nv = replace_calldataloads(val, hoisted);
            if Rc::ptr_eq(&nv, val) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::VarStore(name.clone(), nv))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let ni = replace_calldataloads(inputs, hoisted);
            let nb = replace_calldataloads(body, hoisted);
            if Rc::ptr_eq(&ni, inputs) && Rc::ptr_eq(&nb, body) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::DoWhile(ni, nb))
        }
        EvmExpr::Get(inner, idx) => {
            let ni = replace_calldataloads(inner, hoisted);
            if Rc::ptr_eq(&ni, inner) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Get(ni, *idx))
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let nb = replace_calldataloads(body, hoisted);
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
        EvmExpr::InlineAsm(inputs, hex, num_out) => {
            let new_inputs: Vec<_> = inputs
                .iter()
                .map(|i| replace_calldataloads(i, hoisted))
                .collect();
            let changed = new_inputs
                .iter()
                .zip(inputs.iter())
                .any(|(n, o)| !Rc::ptr_eq(n, o));
            if !changed {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_out))
        }
        EvmExpr::Call(name, args) => {
            let new_args: Vec<_> = args
                .iter()
                .map(|a| replace_calldataloads(a, hoisted))
                .collect();
            let changed = new_args
                .iter()
                .zip(args.iter())
                .any(|(n, o)| !Rc::ptr_eq(n, o));
            if !changed {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::ReturnOp(state, size, offset) => {
            let ns = replace_calldataloads(size, hoisted);
            let no = replace_calldataloads(offset, hoisted);
            if Rc::ptr_eq(&ns, size) && Rc::ptr_eq(&no, offset) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::ReturnOp(Rc::clone(state), ns, no))
        }
        EvmExpr::Revert(state, size, offset) => {
            let ns = replace_calldataloads(size, hoisted);
            let no = replace_calldataloads(offset, hoisted);
            if Rc::ptr_eq(&ns, size) && Rc::ptr_eq(&no, offset) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Revert(Rc::clone(state), ns, no))
        }
        EvmExpr::Log(count, topics, doff, dsz, state) => {
            let new_topics: Vec<_> = topics
                .iter()
                .map(|t| replace_calldataloads(t, hoisted))
                .collect();
            let nd = replace_calldataloads(doff, hoisted);
            let nz = replace_calldataloads(dsz, hoisted);
            let topics_changed = new_topics
                .iter()
                .zip(topics.iter())
                .any(|(n, o)| !Rc::ptr_eq(n, o));
            if !topics_changed && Rc::ptr_eq(&nd, doff) && Rc::ptr_eq(&nz, dsz) {
                return Rc::clone(expr);
            }
            Rc::new(EvmExpr::Log(*count, new_topics, nd, nz, Rc::clone(state)))
        }
        _ => Rc::clone(expr),
    }
}

/// Apply `CalldataLoad` CSE to an expression tree.
/// Hoists repeated `CalldataLoad(const_offset)` into `LetBind` variables.
fn calldataload_cse(expr: &RcExpr) -> RcExpr {
    let mut counts = HashMap::new();
    count_calldataloads(expr, &mut counts);

    // Only hoist offsets that appear more than once
    let hoisted: HashMap<i64, String> = counts
        .iter()
        .filter(|(_, &count)| count > 1)
        .map(|(&offset, _)| (offset, format!("__cd_{offset}")))
        .collect();

    if hoisted.is_empty() {
        return Rc::clone(expr);
    }

    // Replace all CalldataLoad references with Var references
    let replaced = replace_calldataloads(expr, &hoisted);

    // Wrap in LetBind chain: LetBind(__cd_4, CalldataLoad(4, state), LetBind(__cd_36, ..., body))
    let state = Rc::new(EvmExpr::Arg(
        EvmType::Base(crate::schema::EvmBaseType::StateT),
        EvmContext::InFunction("__opt__".to_string()),
    ));
    let uint_ty = EvmType::Base(crate::schema::EvmBaseType::UIntT(256));
    let ctx = EvmContext::InFunction("__opt__".to_string());

    // Sort offsets for deterministic output
    let mut sorted_offsets: Vec<_> = hoisted.iter().collect();
    sorted_offsets.sort_by_key(|(offset, _)| *offset);

    let mut result = replaced;
    // Wrap innermost first (reverse order so outermost LetBind has smallest offset)
    for (&offset, var_name) in sorted_offsets.iter().rev() {
        let cd_load = Rc::new(EvmExpr::Bop(
            EvmBinaryOp::CalldataLoad,
            Rc::new(EvmExpr::Const(
                EvmConstant::SmallInt(offset),
                uint_ty.clone(),
                ctx.clone(),
            )),
            Rc::clone(&state),
        ));
        // Wrap body in Concat(body, Drop(var)) to mark lifetime end
        let body_with_drop = Rc::new(EvmExpr::Concat(
            result,
            Rc::new(EvmExpr::Drop((*var_name).clone())),
        ));
        result = Rc::new(EvmExpr::LetBind(
            (*var_name).clone(),
            cd_load,
            body_with_drop,
        ));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast_helpers, schema::*};

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
        let body = Rc::new(EvmExpr::Concat(
            add_expr,
            ast_helpers::drop_var(name.clone()),
        ));
        let expr = Rc::new(EvmExpr::LetBind(name, val, body));

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
        let expr = Rc::new(EvmExpr::LetBind(name, val, loop_body));

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
        assert!(
            !matches!(optimized.as_ref(), EvmExpr::LetBind(..)),
            "LetBind should be eliminated"
        );
        // Check that the value 42 appears somewhere in the result
        fn contains_42(e: &EvmExpr) -> bool {
            match e {
                EvmExpr::Const(EvmConstant::SmallInt(42), _, _) => true,
                EvmExpr::Concat(a, b) => contains_42(a) || contains_42(b),
                _ => false,
            }
        }
        assert!(
            contains_42(&optimized),
            "expected 42 in result, got: {optimized:?}"
        );
    }
}
