//! Post-egglog IR cleanup pass.
//!
//! After egglog extraction, state parameters often contain massive nested
//! chains of SStore/SLoad operations. Since codegen ignores state parameters
//! entirely (side-effect ordering is implicit in Concat sequencing), these
//! chains are pure bloat. This pass:
//!
//! 1. Replaces all state parameters with a simple `Arg(StateT)` sentinel.
//! 2. Removes dead code after halting operations (ReturnOp/Revert) in Concat chains.

use std::rc::Rc;

use crate::schema::{EvmBaseType, EvmContext, EvmExpr, EvmProgram, EvmTernaryOp, EvmType, RcExpr};

/// Sentinel state token used to replace bloated state chains.
fn state_sentinel() -> RcExpr {
    Rc::new(EvmExpr::Arg(
        EvmType::Base(EvmBaseType::StateT),
        EvmContext::InFunction("__state__".to_owned()),
    ))
}

/// Clean up a program after egglog extraction.
pub fn cleanup_program(program: &mut EvmProgram) {
    for contract in &mut program.contracts {
        contract.runtime = cleanup_expr(&contract.runtime);
        contract.constructor = cleanup_expr(&contract.constructor);
    }
}

/// Public entry point for cleaning a single expression.
pub fn cleanup_expr_pub(expr: &RcExpr) -> RcExpr {
    cleanup_expr(expr)
}

/// Recursively clean an expression tree.
fn cleanup_expr(expr: &RcExpr) -> RcExpr {
    match expr.as_ref() {
        // --- State parameter simplification ---

        // Top(op, a, b, state) — SStore, TStore, MStore, MStore8, Keccak256
        EvmExpr::Top(op, a, b, _state) => {
            let a2 = cleanup_expr(a);
            let b2 = cleanup_expr(b);
            let state2 = if op.has_state() {
                state_sentinel()
            } else {
                // Select doesn't have state
                cleanup_expr(_state)
            };
            Rc::new(EvmExpr::Top(*op, a2, b2, state2))
        }

        // Bop(op, a, state) — SLoad, TLoad, MLoad, CalldataLoad
        EvmExpr::Bop(op, a, b) => {
            let a2 = cleanup_expr(a);
            let b2 = if op.has_state() {
                state_sentinel()
            } else {
                cleanup_expr(b)
            };
            Rc::new(EvmExpr::Bop(*op, a2, b2))
        }

        // ReturnOp(offset, size, state)
        EvmExpr::ReturnOp(off, sz, _state) => {
            let off2 = cleanup_expr(off);
            let sz2 = cleanup_expr(sz);
            Rc::new(EvmExpr::ReturnOp(off2, sz2, state_sentinel()))
        }

        // Revert(offset, size, state)
        EvmExpr::Revert(off, sz, _state) => {
            let off2 = cleanup_expr(off);
            let sz2 = cleanup_expr(sz);
            Rc::new(EvmExpr::Revert(off2, sz2, state_sentinel()))
        }

        // Log(count, topics, data_offset, data_size, state)
        EvmExpr::Log(count, topics, data_offset, data_size, _state) => {
            let topics2: Vec<_> = topics.iter().map(cleanup_expr).collect();
            let off2 = cleanup_expr(data_offset);
            let sz2 = cleanup_expr(data_size);
            Rc::new(EvmExpr::Log(*count, topics2, off2, sz2, state_sentinel()))
        }

        // ExtCall(target, value, args_off, args_len, ret_off, ret_len, state)
        EvmExpr::ExtCall(a, b, c, d, e, f, _state) => Rc::new(EvmExpr::ExtCall(
            cleanup_expr(a),
            cleanup_expr(b),
            cleanup_expr(c),
            cleanup_expr(d),
            cleanup_expr(e),
            cleanup_expr(f),
            state_sentinel(),
        )),

        // EnvRead(op, state)
        EvmExpr::EnvRead(op, _state) => Rc::new(EvmExpr::EnvRead(*op, state_sentinel())),

        // EnvRead1(op, arg, state)
        EvmExpr::EnvRead1(op, arg, _state) => {
            Rc::new(EvmExpr::EnvRead1(*op, cleanup_expr(arg), state_sentinel()))
        }

        // --- Dead code elimination after halting ops in Concat chains ---
        EvmExpr::Concat(left, right) => {
            let left2 = cleanup_expr(left);
            if is_halting(&left2) {
                // Right side is dead code — drop it
                left2
            } else {
                let right2 = cleanup_expr(right);
                Rc::new(EvmExpr::Concat(left2, right2))
            }
        }

        // --- Recurse through everything else ---
        EvmExpr::Uop(op, a) => Rc::new(EvmExpr::Uop(*op, cleanup_expr(a))),

        EvmExpr::If(cond, inputs, then_b, else_b) => Rc::new(EvmExpr::If(
            cleanup_expr(cond),
            cleanup_expr(inputs),
            cleanup_expr(then_b),
            cleanup_expr(else_b),
        )),

        EvmExpr::DoWhile(inputs, body) => {
            Rc::new(EvmExpr::DoWhile(cleanup_expr(inputs), cleanup_expr(body)))
        }

        EvmExpr::Get(inner, idx) => Rc::new(EvmExpr::Get(cleanup_expr(inner), *idx)),

        EvmExpr::LetBind(name, init, body) => Rc::new(EvmExpr::LetBind(
            name.clone(),
            cleanup_expr(init),
            cleanup_expr(body),
        )),

        EvmExpr::VarStore(name, val) => Rc::new(EvmExpr::VarStore(name.clone(), cleanup_expr(val))),

        EvmExpr::Call(name, args) => Rc::new(EvmExpr::Call(
            name.clone(),
            args.iter().map(cleanup_expr).collect(),
        )),

        EvmExpr::Function(name, in_ty, out_ty, body) => Rc::new(EvmExpr::Function(
            name.clone(),
            in_ty.clone(),
            out_ty.clone(),
            cleanup_expr(body),
        )),

        EvmExpr::InlineAsm(inputs, hex, num_outputs) => Rc::new(EvmExpr::InlineAsm(
            inputs.iter().map(cleanup_expr).collect(),
            hex.clone(),
            *num_outputs,
        )),

        // Leaf nodes — no children to clean
        EvmExpr::Arg(..)
        | EvmExpr::Const(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(..)
        | EvmExpr::Drop(..)
        | EvmExpr::Selector(..)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..) => Rc::clone(expr),
    }
}

/// Returns true if an expression always halts execution (RETURN, REVERT, STOP).
fn is_halting(expr: &RcExpr) -> bool {
    match expr.as_ref() {
        EvmExpr::ReturnOp(..) | EvmExpr::Revert(..) => true,
        // A Concat where the right side halts means the whole thing halts
        EvmExpr::Concat(_, right) => is_halting(right),
        // If both branches halt, the If halts
        EvmExpr::If(_, _, then_b, else_b) => is_halting(then_b) && is_halting(else_b),
        // LetBind halts if its body halts
        EvmExpr::LetBind(_, _, body) => is_halting(body),
        _ => false,
    }
}

impl EvmTernaryOp {
    /// Returns true if the third operand is a state token (ignored by codegen).
    pub const fn has_state(&self) -> bool {
        matches!(
            self,
            Self::SStore | Self::TStore | Self::MStore | Self::MStore8 | Self::Keccak256
        )
    }
}
