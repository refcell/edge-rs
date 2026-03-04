//! Variable optimization pass.
//!
//! Runs on the `RcExpr` IR tree BEFORE egglog equality saturation.
//! Performs analyses that egglog can't do (counting variable references)
//! and applies deterministic transforms:
//!
//! 1. **Store-forwarding**: `LetBind(x, 0, Concat(VarStore(x, real), rest))` → `LetBind(x, real, rest)`
//! 2. **Dead variable elimination**: Remove LetBinds whose variable is never read
//! 3. **Single-use inlining**: Inline LetBind init directly at sole Var reference
//! 4. **Multi-use constant propagation**: Replace Var refs with the constant value

use std::rc::Rc;

use crate::schema::{EvmConstant, EvmExpr, EvmTernaryOp, RcExpr};

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

/// Optimize an entire program's contract runtimes.
pub fn optimize_program(program: &mut crate::schema::EvmProgram) {
    for contract in &mut program.contracts {
        contract.runtime = optimize_expr(&contract.runtime);
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
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..) => expr.clone(),
    }
}

/// Apply optimization transforms at a single node (children already optimized).
fn apply_transforms(expr: &RcExpr) -> RcExpr {
    if let EvmExpr::LetBind(name, init, body) = expr.as_ref() {
        // Analyze variable usage in the body
        let info = analyze_var(name, body);

        // 1. Store-forwarding: LetBind(x, 0, Concat(VarStore(x, real), rest)) → LetBind(x, real, rest)
        if is_zero_const(init) {
            if let Some((real_init, rest)) = peel_leading_var_store(name, body) {
                // Re-analyze with the VarStore removed
                let new_info = analyze_var(name, &rest);
                // Apply further optimizations on the forwarded result
                let forwarded = Rc::new(EvmExpr::LetBind(name.clone(), real_init, rest));
                return apply_letbind_opts(name, &forwarded, &new_info);
            }
        }

        return apply_letbind_opts(name, expr, &info);
    }

    expr.clone()
}

/// Apply LetBind-specific optimizations given usage info.
fn apply_letbind_opts(name: &str, expr: &RcExpr, info: &VarInfo) -> RcExpr {
    let (init, body) = match expr.as_ref() {
        EvmExpr::LetBind(_, init, body) => (init, body),
        _ => return expr.clone(),
    };

    // 2. Dead variable elimination: never read → remove LetBind
    if info.read_count == 0 && info.write_count == 0 {
        if is_pure(init) {
            return body.clone();
        } else {
            // Keep side effects
            return Rc::new(EvmExpr::Concat(init.clone(), body.clone()));
        }
    }

    // 3. Single-use inlining: read once, never written, not in loop, pure init
    if info.read_count == 1 && info.write_count == 0 && !info.in_loop && is_pure(init) {
        return substitute_var(name, init, body);
    }

    // 4. Multi-use constant propagation: constant init, never written
    if info.write_count == 0 && !info.in_loop && is_const(init) {
        return substitute_var_all(name, init, body);
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
        EvmExpr::Var(_) => {}
        EvmExpr::Const(..) | EvmExpr::Arg(..) | EvmExpr::Empty(..) | EvmExpr::Selector(_) | EvmExpr::StorageField(..) => {}
        EvmExpr::Bop(_, a, b) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, b, in_loop, info);
        }
        EvmExpr::Uop(_, a) => {
            analyze_var_inner(name, a, in_loop, info);
        }
        EvmExpr::Top(_, a, b, c) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, b, in_loop, info);
            analyze_var_inner(name, c, in_loop, info);
        }
        EvmExpr::Get(a, _) => {
            analyze_var_inner(name, a, in_loop, info);
        }
        EvmExpr::Concat(a, b) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, b, in_loop, info);
        }
        EvmExpr::If(c, i, t, e) => {
            analyze_var_inner(name, c, in_loop, info);
            analyze_var_inner(name, i, in_loop, info);
            analyze_var_inner(name, t, in_loop, info);
            analyze_var_inner(name, e, in_loop, info);
        }
        EvmExpr::DoWhile(inputs, body) => {
            analyze_var_inner(name, inputs, in_loop, info);
            // Everything inside a loop body is "in_loop"
            analyze_var_inner(name, body, true, info);
        }
        EvmExpr::EnvRead(_, s) => {
            analyze_var_inner(name, s, in_loop, info);
        }
        EvmExpr::EnvRead1(_, a, s) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, s, in_loop, info);
        }
        EvmExpr::Log(_, topics, data, state) => {
            for t in topics {
                analyze_var_inner(name, t, in_loop, info);
            }
            analyze_var_inner(name, data, in_loop, info);
            analyze_var_inner(name, state, in_loop, info);
        }
        EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, b, in_loop, info);
            analyze_var_inner(name, c, in_loop, info);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            analyze_var_inner(name, a, in_loop, info);
            analyze_var_inner(name, b, in_loop, info);
            analyze_var_inner(name, c, in_loop, info);
            analyze_var_inner(name, d, in_loop, info);
            analyze_var_inner(name, e, in_loop, info);
            analyze_var_inner(name, f, in_loop, info);
            analyze_var_inner(name, g, in_loop, info);
        }
        EvmExpr::Call(_, args) => {
            analyze_var_inner(name, args, in_loop, info);
        }
        EvmExpr::Function(_, _, _, body) => {
            analyze_var_inner(name, body, in_loop, info);
        }
    }
}

/// Check if an expression is a zero constant.
fn is_zero_const(expr: &RcExpr) -> bool {
    matches!(
        expr.as_ref(),
        EvmExpr::Const(EvmConstant::SmallInt(0), _, _)
            | EvmExpr::Const(EvmConstant::Bool(false), _, _)
    )
}

/// Check if an expression is any constant.
fn is_const(expr: &RcExpr) -> bool {
    matches!(expr.as_ref(), EvmExpr::Const(..))
}

/// Check if an expression is pure (no side effects).
/// Conservative: only things we're sure are pure.
fn is_pure(expr: &RcExpr) -> bool {
    match expr.as_ref() {
        EvmExpr::Const(..) | EvmExpr::Arg(..) | EvmExpr::Empty(..) | EvmExpr::Var(_) | EvmExpr::Selector(_) => true,
        EvmExpr::Bop(op, a, b) => {
            // Storage/memory reads have state dependencies but no visible side effects
            // for our purposes (we're checking if we can DROP the expression, not reorder it).
            // However, to be safe, only mark pure arithmetic/comparison ops.
            use crate::schema::EvmBinaryOp::*;
            match op {
                Add | Sub | Mul | Div | SDiv | Mod | SMod | Exp
                | Lt | Gt | SLt | SGt | Eq
                | And | Or | Xor | Shl | Shr | Sar | Byte
                | LogAnd | LogOr => is_pure(a) && is_pure(b),
                // SLoad, TLoad, MLoad, CalldataLoad — reads are pure for dead code elimination
                SLoad | TLoad | MLoad | CalldataLoad => is_pure(a) && is_pure(b),
            }
        }
        EvmExpr::Uop(_, a) => is_pure(a),
        EvmExpr::Top(op, a, b, c) => {
            match op {
                EvmTernaryOp::Select | EvmTernaryOp::Keccak256 => is_pure(a) && is_pure(b) && is_pure(c),
                // SStore, TStore, MStore, MStore8 are NOT pure
                _ => false,
            }
        }
        EvmExpr::Get(a, _) => is_pure(a),
        EvmExpr::Concat(a, b) => is_pure(a) && is_pure(b),
        EvmExpr::EnvRead(..) | EvmExpr::EnvRead1(..) => true,
        EvmExpr::LetBind(_, init, body) => is_pure(init) && is_pure(body),
        // Side-effecting
        EvmExpr::VarStore(..) | EvmExpr::Log(..) | EvmExpr::Revert(..)
        | EvmExpr::ReturnOp(..) | EvmExpr::ExtCall(..) => false,
        EvmExpr::If(c, i, t, e) => is_pure(c) && is_pure(i) && is_pure(t) && is_pure(e),
        EvmExpr::DoWhile(..) => false, // loops might diverge
        EvmExpr::Call(..) => false,
        EvmExpr::Function(..) | EvmExpr::StorageField(..) => true,
    }
}

/// Flatten a left-nested Concat chain into an ordered list of statements.
///
/// `Concat(Concat(Concat(s0, s1), s2), s3)` → `[s0, s1, s2, s3]`
fn flatten_left_concat(expr: &RcExpr) -> Vec<RcExpr> {
    match expr.as_ref() {
        EvmExpr::Concat(left, right) => {
            let mut stmts = flatten_left_concat(left);
            stmts.push(right.clone());
            stmts
        }
        _ => vec![expr.clone()],
    }
}

/// Rebuild a left-nested Concat chain from a list of statements.
fn rebuild_left_concat(stmts: &[RcExpr]) -> RcExpr {
    assert!(!stmts.is_empty());
    let mut result = stmts[0].clone();
    for stmt in &stmts[1..] {
        result = Rc::new(EvmExpr::Concat(result, stmt.clone()));
    }
    result
}

/// Find and remove the first VarStore for `name` from a Concat chain.
///
/// Flattens the left-nested chain, finds the first `VarStore(name, val)`,
/// removes it, and returns `(val, rebuilt_chain_without_varstore)`.
fn peel_leading_var_store(name: &str, body: &RcExpr) -> Option<(RcExpr, RcExpr)> {
    // Handle bare VarStore (no Concat wrapper)
    if let EvmExpr::VarStore(n, val) = body.as_ref() {
        if n == name {
            return Some((val.clone(), Rc::new(EvmExpr::Const(
                EvmConstant::SmallInt(0),
                crate::schema::EvmType::Base(crate::schema::EvmBaseType::UIntT(256)),
                crate::schema::EvmContext::InFunction("__init__".to_owned()),
            ))));
        }
    }

    // Only flatten Concat chains
    if !matches!(body.as_ref(), EvmExpr::Concat(..)) {
        return None;
    }

    let stmts = flatten_left_concat(body);

    // Find the first VarStore(name, _) in statement order
    let idx = stmts.iter().position(|s| {
        matches!(s.as_ref(), EvmExpr::VarStore(n, _) if n == name)
    })?;

    // Safety check: all statements BEFORE the VarStore must be pure.
    // Moving the VarStore's value to the LetBind init means it executes
    // before these statements. If any are side-effectful (e.g., other
    // VarStores that set variables used in this VarStore's value),
    // reordering would change semantics.
    for stmt in &stmts[..idx] {
        if !is_pure(stmt) {
            return None;
        }
    }

    let real_init = match stmts[idx].as_ref() {
        EvmExpr::VarStore(_, val) => val.clone(),
        _ => unreachable!(),
    };

    // Rebuild without the VarStore
    let remaining: Vec<RcExpr> = stmts
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != idx)
        .map(|(_, s)| s)
        .collect();

    if remaining.is_empty() {
        return Some((real_init, Rc::new(EvmExpr::Const(
            EvmConstant::SmallInt(0),
            crate::schema::EvmType::Base(crate::schema::EvmBaseType::UIntT(256)),
            crate::schema::EvmContext::InFunction("__init__".to_owned()),
        ))));
    }

    let rest = rebuild_left_concat(&remaining);
    Some((real_init, rest))
}

/// Substitute all occurrences of `Var(name)` with `replacement` in `expr`.
/// Returns the new expression.
fn substitute_var(name: &str, replacement: &RcExpr, expr: &RcExpr) -> RcExpr {
    substitute_var_inner(name, replacement, expr)
}

/// Substitute all occurrences (for multi-use constant propagation).
fn substitute_var_all(name: &str, replacement: &RcExpr, expr: &RcExpr) -> RcExpr {
    substitute_var_inner(name, replacement, expr)
}

fn substitute_var_inner(name: &str, replacement: &RcExpr, expr: &RcExpr) -> RcExpr {
    match expr.as_ref() {
        EvmExpr::Var(n) if n == name => replacement.clone(),
        EvmExpr::Var(_) => expr.clone(),

        // Stop at shadowing LetBind
        EvmExpr::LetBind(n, init, body) => {
            let new_init = substitute_var_inner(name, replacement, init);
            if n == name {
                // Shadowed — don't substitute in body
                Rc::new(EvmExpr::LetBind(n.clone(), new_init, body.clone()))
            } else {
                let new_body = substitute_var_inner(name, replacement, body);
                Rc::new(EvmExpr::LetBind(n.clone(), new_init, new_body))
            }
        }

        EvmExpr::VarStore(n, val) => {
            let new_val = substitute_var_inner(name, replacement, val);
            Rc::new(EvmExpr::VarStore(n.clone(), new_val))
        }

        // Leaf nodes
        EvmExpr::Const(..) | EvmExpr::Arg(..) | EvmExpr::Empty(..) | EvmExpr::Selector(_) | EvmExpr::StorageField(..) => expr.clone(),

        EvmExpr::Bop(op, a, b) => {
            let a2 = substitute_var_inner(name, replacement, a);
            let b2 = substitute_var_inner(name, replacement, b);
            Rc::new(EvmExpr::Bop(*op, a2, b2))
        }
        EvmExpr::Uop(op, a) => {
            let a2 = substitute_var_inner(name, replacement, a);
            Rc::new(EvmExpr::Uop(*op, a2))
        }
        EvmExpr::Top(op, a, b, c) => {
            let a2 = substitute_var_inner(name, replacement, a);
            let b2 = substitute_var_inner(name, replacement, b);
            let c2 = substitute_var_inner(name, replacement, c);
            Rc::new(EvmExpr::Top(*op, a2, b2, c2))
        }
        EvmExpr::Get(a, idx) => {
            let a2 = substitute_var_inner(name, replacement, a);
            Rc::new(EvmExpr::Get(a2, *idx))
        }
        EvmExpr::Concat(a, b) => {
            let a2 = substitute_var_inner(name, replacement, a);
            let b2 = substitute_var_inner(name, replacement, b);
            Rc::new(EvmExpr::Concat(a2, b2))
        }
        EvmExpr::If(c, i, t, e) => {
            let c2 = substitute_var_inner(name, replacement, c);
            let i2 = substitute_var_inner(name, replacement, i);
            let t2 = substitute_var_inner(name, replacement, t);
            let e2 = substitute_var_inner(name, replacement, e);
            Rc::new(EvmExpr::If(c2, i2, t2, e2))
        }
        EvmExpr::DoWhile(inputs, body) => {
            let i2 = substitute_var_inner(name, replacement, inputs);
            let b2 = substitute_var_inner(name, replacement, body);
            Rc::new(EvmExpr::DoWhile(i2, b2))
        }
        EvmExpr::EnvRead(op, state) => {
            let s2 = substitute_var_inner(name, replacement, state);
            Rc::new(EvmExpr::EnvRead(*op, s2))
        }
        EvmExpr::EnvRead1(op, arg, state) => {
            let a2 = substitute_var_inner(name, replacement, arg);
            let s2 = substitute_var_inner(name, replacement, state);
            Rc::new(EvmExpr::EnvRead1(*op, a2, s2))
        }
        EvmExpr::Log(count, topics, data, state) => {
            let ts: Vec<_> = topics.iter().map(|t| substitute_var_inner(name, replacement, t)).collect();
            let d2 = substitute_var_inner(name, replacement, data);
            let s2 = substitute_var_inner(name, replacement, state);
            Rc::new(EvmExpr::Log(*count, ts, d2, s2))
        }
        EvmExpr::Revert(a, b, c) => {
            let a2 = substitute_var_inner(name, replacement, a);
            let b2 = substitute_var_inner(name, replacement, b);
            let c2 = substitute_var_inner(name, replacement, c);
            Rc::new(EvmExpr::Revert(a2, b2, c2))
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let a2 = substitute_var_inner(name, replacement, a);
            let b2 = substitute_var_inner(name, replacement, b);
            let c2 = substitute_var_inner(name, replacement, c);
            Rc::new(EvmExpr::ReturnOp(a2, b2, c2))
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let a2 = substitute_var_inner(name, replacement, a);
            let b2 = substitute_var_inner(name, replacement, b);
            let c2 = substitute_var_inner(name, replacement, c);
            let d2 = substitute_var_inner(name, replacement, d);
            let e2 = substitute_var_inner(name, replacement, e);
            let f2 = substitute_var_inner(name, replacement, f);
            let g2 = substitute_var_inner(name, replacement, g);
            Rc::new(EvmExpr::ExtCall(a2, b2, c2, d2, e2, f2, g2))
        }
        EvmExpr::Call(n, args) => {
            let a2 = substitute_var_inner(name, replacement, args);
            Rc::new(EvmExpr::Call(n.clone(), a2))
        }
        EvmExpr::Function(n, in_ty, out_ty, body) => {
            let b2 = substitute_var_inner(name, replacement, body);
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
    fn test_store_forwarding() {
        // LetBind(x, 0, Concat(VarStore(x, 42), Var(x)))
        // → LetBind(x, 42, Var(x))
        let name = "__local_x".to_owned();
        let zero = ast_helpers::const_int(0, ctx());
        let forty_two = ast_helpers::const_int(42, ctx());
        let body = Rc::new(EvmExpr::Concat(
            Rc::new(EvmExpr::VarStore(name.clone(), forty_two.clone())),
            Rc::new(EvmExpr::Var(name.clone())),
        ));
        let expr = Rc::new(EvmExpr::LetBind(name.clone(), zero, body));

        let optimized = optimize_expr(&expr);
        // Should be LetBind(x, 42, Var(x))
        match optimized.as_ref() {
            EvmExpr::LetBind(n, init, body) => {
                assert_eq!(n, &name);
                // init should be 42 (single-use inlining may also fire)
                // Actually, after store-forwarding we get LetBind(x, 42, Var(x))
                // Then single-use inlining fires: read_count=1, write_count=0 → inline
                // Result should be just 42
                match optimized.as_ref() {
                    EvmExpr::Const(EvmConstant::SmallInt(42), _, _) => {}
                    EvmExpr::LetBind(_, init, _) => {
                        // Store-forwarding happened, single-use may or may not fire
                        assert!(matches!(init.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(42), _, _)));
                    }
                    other => panic!("unexpected: {other:?}"),
                }
            }
            EvmExpr::Const(EvmConstant::SmallInt(42), _, _) => {
                // Perfect: store-forwarding + single-use inlining
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn test_dead_variable_elimination() {
        // LetBind(x, 0, Const(99)) — x is never read
        let name = "__local_x".to_owned();
        let zero = ast_helpers::const_int(0, ctx());
        let ninety_nine = ast_helpers::const_int(99, ctx());
        let expr = Rc::new(EvmExpr::LetBind(name, zero, ninety_nine.clone()));

        let optimized = optimize_expr(&expr);
        // Should be just 99
        assert!(matches!(optimized.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(99), _, _)));
    }

    #[test]
    fn test_single_use_inline() {
        // LetBind(x, 42, Var(x)) → 42
        let name = "__local_x".to_owned();
        let val = ast_helpers::const_int(42, ctx());
        let expr = Rc::new(EvmExpr::LetBind(
            name.clone(),
            val,
            Rc::new(EvmExpr::Var(name)),
        ));

        let optimized = optimize_expr(&expr);
        assert!(matches!(optimized.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(42), _, _)));
    }

    #[test]
    fn test_multi_use_const_prop() {
        // LetBind(x, 42, Add(Var(x), Var(x))) → Add(42, 42)
        let name = "__local_x".to_owned();
        let val = ast_helpers::const_int(42, ctx());
        let body = ast_helpers::add(
            Rc::new(EvmExpr::Var(name.clone())),
            Rc::new(EvmExpr::Var(name.clone())),
        );
        let expr = Rc::new(EvmExpr::LetBind(name, val.clone(), body));

        let optimized = optimize_expr(&expr);
        // Should be Add(42, 42)
        match optimized.as_ref() {
            EvmExpr::Bop(EvmBinaryOp::Add, a, b) => {
                assert!(matches!(a.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(42), _, _)));
                assert!(matches!(b.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(42), _, _)));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn test_no_inline_in_loop() {
        // LetBind(x, 42, DoWhile(Empty, Var(x))) — x is in a loop, don't inline
        let name = "__local_x".to_owned();
        let val = ast_helpers::const_int(42, ctx());
        let loop_body = Rc::new(EvmExpr::DoWhile(
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx()),
            Rc::new(EvmExpr::Var(name.clone())),
        ));
        let expr = Rc::new(EvmExpr::LetBind(name.clone(), val, loop_body));

        let optimized = optimize_expr(&expr);
        // Should still be a LetBind (single-use inline blocked by in_loop)
        // BUT: const prop should fire (write_count=0, const init) regardless of loop
        // Actually, our rule checks !in_loop for const prop too... let's verify
        // The rule is: write_count == 0 && !in_loop && is_const → propagate
        // So with in_loop=true, it should stay as LetBind
        assert!(matches!(optimized.as_ref(), EvmExpr::LetBind(..)));
    }

    #[test]
    fn test_no_inline_with_writes() {
        // LetBind(x, 0, Concat(VarStore(x, 42), Var(x))) — has writes
        // After store-forwarding: LetBind(x, 42, Var(x)) — now write_count=0, read_count=1
        // Should inline to 42
        let name = "__local_x".to_owned();
        let zero = ast_helpers::const_int(0, ctx());
        let forty_two = ast_helpers::const_int(42, ctx());
        let body = Rc::new(EvmExpr::Concat(
            Rc::new(EvmExpr::VarStore(name.clone(), forty_two.clone())),
            Rc::new(EvmExpr::Var(name.clone())),
        ));
        let expr = Rc::new(EvmExpr::LetBind(name, zero, body));

        let optimized = optimize_expr(&expr);
        // After store-forwarding + single-use inline: should be 42
        assert!(matches!(optimized.as_ref(), EvmExpr::Const(EvmConstant::SmallInt(42), _, _)));
    }
}
