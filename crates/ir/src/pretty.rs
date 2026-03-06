//! Pretty-printer for `EvmExpr` IR trees.
//!
//! Produces a human-readable, indented representation of the post-egglog
//! optimized IR.

use crate::schema::{EvmConstant, EvmContract, EvmExpr, EvmTernaryOp, EvmType, RcExpr};

/// Max inline width before we break to multi-line.
const MAX_INLINE: usize = 80;

/// Pretty-print an `EvmExpr` tree.
pub fn pretty_print(expr: &RcExpr) -> String {
    let mut buf = String::new();
    pp(expr, 0, &mut buf);
    buf
}

/// Pretty-print an `EvmContract`.
pub fn pretty_print_contract(contract: &EvmContract) -> String {
    let mut buf = String::new();
    buf.push_str(&format!("contract {} {{\n", contract.name));

    if !contract.storage_fields.is_empty() {
        for field in &contract.storage_fields {
            pp(field, 1, &mut buf);
            buf.push('\n');
        }
        buf.push('\n');
    }

    buf.push_str("  constructor:\n");
    pp(&contract.constructor, 2, &mut buf);
    buf.push_str("\n\n");

    buf.push_str("  runtime:\n");
    pp(&contract.runtime, 2, &mut buf);
    buf.push('\n');

    for func in &contract.internal_functions {
        pp(func, 2, &mut buf);
        buf.push('\n');
    }

    buf.push_str("}\n");
    buf
}

fn fmt_type(ty: &EvmType) -> String {
    match ty {
        EvmType::Base(b) => format!("{b}"),
        EvmType::TupleT(ts) => {
            let inner: Vec<_> = ts.iter().map(|t| format!("{t}")).collect();
            format!("({})", inner.join(", "))
        }
    }
}

fn indent(depth: usize, buf: &mut String) {
    for _ in 0..depth {
        buf.push_str("  ");
    }
}

/// Estimate the width of an expression if printed inline.
/// Returns `None` if it shouldn't be inlined (contains control flow, concat, etc).
fn inline_width(expr: &RcExpr) -> Option<usize> {
    match expr.as_ref() {
        EvmExpr::Arg(ty, _) => Some(4 + fmt_type(ty).len()),
        EvmExpr::Const(c, ty, _) => {
            let val_len = match c {
                EvmConstant::SmallInt(n) => format!("{n}").len(),
                EvmConstant::LargeInt(s) => 2 + s.len(),
                EvmConstant::Bool(b) => format!("{b}").len(),
                EvmConstant::Addr(a) => 1 + a.len(),
            };
            Some(val_len + 1 + fmt_type(ty).len())
        }
        EvmExpr::Empty(_, _) => Some(5),
        EvmExpr::Var(name) => Some(1 + name.len()),
        EvmExpr::Drop(name) => Some(6 + name.len()),
        EvmExpr::Selector(sig) => Some(11 + sig.len()),
        EvmExpr::EnvRead(op, _) => Some(format!("{op}()").len()),
        EvmExpr::EnvRead1(op, arg, _) => {
            let inner = inline_width(arg)?;
            Some(format!("{op}").len() + 1 + inner + 1)
        }
        EvmExpr::Bop(op, lhs, rhs) => {
            let l = inline_width(lhs)?;
            if op.has_state() {
                Some(format!("{op}").len() + 1 + l + 8) // ", state)"
            } else {
                let r = inline_width(rhs)?;
                Some(format!("{op}").len() + 1 + l + 2 + r + 1)
            }
        }
        EvmExpr::Uop(op, inner) => {
            let w = inline_width(inner)?;
            Some(format!("{op}").len() + 1 + w + 1)
        }
        EvmExpr::Top(op, a, b, c) => {
            let has_state = matches!(
                op,
                EvmTernaryOp::SStore
                    | EvmTernaryOp::TStore
                    | EvmTernaryOp::MStore
                    | EvmTernaryOp::MStore8
                    | EvmTernaryOp::Keccak256
            );
            let wa = inline_width(a)?;
            let wb = inline_width(b)?;
            if has_state {
                Some(format!("{op}").len() + 1 + wa + 2 + wb + 8)
            } else {
                let wc = inline_width(c)?;
                Some(format!("{op}").len() + 1 + wa + 2 + wb + 2 + wc + 1)
            }
        }
        EvmExpr::Get(inner, idx) => {
            let w = inline_width(inner)?;
            Some(w + 1 + format!("{idx}").len())
        }
        // These are never inlined (control flow, compound, or kept multi-line for clarity)
        EvmExpr::If(..)
        | EvmExpr::DoWhile(..)
        | EvmExpr::Concat(..)
        | EvmExpr::LetBind(..)
        | EvmExpr::Log(..)
        | EvmExpr::ExtCall(..)
        | EvmExpr::Function(..)
        | EvmExpr::StorageField(..)
        | EvmExpr::Revert(..)
        | EvmExpr::ReturnOp(..)
        | EvmExpr::Call(..)
        | EvmExpr::VarStore(..) => None,
    }
}

/// Can this expression be printed inline within the given budget?
fn fits_inline(expr: &RcExpr, budget: usize) -> bool {
    inline_width(expr).is_some_and(|w| w <= budget)
}

/// Print an expression inline (no indentation, no newlines).
fn pp_inline(expr: &RcExpr, buf: &mut String) {
    match expr.as_ref() {
        EvmExpr::Arg(ty, _) => buf.push_str(&format!("arg:{}", fmt_type(ty))),
        EvmExpr::Const(c, ty, _) => {
            let val = match c {
                EvmConstant::SmallInt(n) => format!("{n}"),
                EvmConstant::LargeInt(s) => format!("0x{s}"),
                EvmConstant::Bool(b) => format!("{b}"),
                EvmConstant::Addr(a) => format!("@{a}"),
            };
            buf.push_str(&format!("{val}:{}", fmt_type(ty)));
        }
        EvmExpr::Empty(_, _) => buf.push_str("empty"),
        EvmExpr::Var(name) => buf.push_str(&format!("${name}")),
        EvmExpr::Drop(name) => buf.push_str(&format!("drop ${name}")),
        EvmExpr::Selector(sig) => buf.push_str(&format!("selector(\"{sig}\")")),
        EvmExpr::EnvRead(op, _) => buf.push_str(&format!("{op}()")),
        EvmExpr::EnvRead1(op, arg, _) => {
            buf.push_str(&format!("{op}("));
            pp_inline(arg, buf);
            buf.push(')');
        }
        EvmExpr::Bop(op, lhs, rhs) => {
            buf.push_str(&format!("{op}("));
            pp_inline(lhs, buf);
            if op.has_state() {
                buf.push_str(", state)");
            } else {
                buf.push_str(", ");
                pp_inline(rhs, buf);
                buf.push(')');
            }
        }
        EvmExpr::Uop(op, inner) => {
            buf.push_str(&format!("{op}("));
            pp_inline(inner, buf);
            buf.push(')');
        }
        EvmExpr::Top(op, a, b, c) => {
            let has_state = matches!(
                op,
                EvmTernaryOp::SStore
                    | EvmTernaryOp::TStore
                    | EvmTernaryOp::MStore
                    | EvmTernaryOp::MStore8
                    | EvmTernaryOp::Keccak256
            );
            buf.push_str(&format!("{op}("));
            pp_inline(a, buf);
            buf.push_str(", ");
            pp_inline(b, buf);
            if has_state {
                buf.push_str(", state)");
            } else {
                buf.push_str(", ");
                pp_inline(c, buf);
                buf.push(')');
            }
        }
        EvmExpr::Get(inner, idx) => {
            pp_inline(inner, buf);
            buf.push_str(&format!(".{idx}"));
        }
        _ => {
            // Fallback for things that shouldn't be inlined but ended up here
            pp(expr, 0, buf);
        }
    }
}

/// Budget remaining for inline content at a given depth.
const fn budget(depth: usize) -> usize {
    MAX_INLINE.saturating_sub(depth * 2)
}

fn pp(expr: &RcExpr, depth: usize, buf: &mut String) {
    // Try inline first for non-statement expressions
    if fits_inline(expr, budget(depth)) {
        indent(depth, buf);
        pp_inline(expr, buf);
        return;
    }

    match expr.as_ref() {
        // ---- Leaves (always inline, handled above) ----
        EvmExpr::Arg(..)
        | EvmExpr::Const(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(..)
        | EvmExpr::Drop(..)
        | EvmExpr::Selector(..)
        | EvmExpr::EnvRead(..) => {
            // These always fit inline, but just in case:
            indent(depth, buf);
            pp_inline(expr, buf);
        }

        // ---- Operators ----
        EvmExpr::Bop(op, lhs, rhs) => {
            indent(depth, buf);
            if op.has_state() {
                if fits_inline(lhs, budget(depth + 1)) {
                    buf.push_str(&format!("{op}("));
                    pp_inline(lhs, buf);
                } else {
                    buf.push_str(&format!("{op}(\n"));
                    pp(lhs, depth + 1, buf);
                }
                buf.push_str(", state)");
            } else {
                buf.push_str(&format!("{op}(\n"));
                pp(lhs, depth + 1, buf);
                buf.push_str(",\n");
                pp(rhs, depth + 1, buf);
                buf.push(')');
            }
        }
        EvmExpr::Uop(op, inner) => {
            indent(depth, buf);
            buf.push_str(&format!("{op}(\n"));
            pp(inner, depth + 1, buf);
            buf.push(')');
        }
        EvmExpr::EnvRead1(op, arg, _state) => {
            indent(depth, buf);
            buf.push_str(&format!("{op}("));
            if fits_inline(arg, budget(depth + 1)) {
                pp_inline(arg, buf);
            } else {
                buf.push('\n');
                pp(arg, depth + 1, buf);
            }
            buf.push(')');
        }
        EvmExpr::Top(op, a, b, c) => {
            indent(depth, buf);
            let has_state = matches!(
                op,
                EvmTernaryOp::SStore
                    | EvmTernaryOp::TStore
                    | EvmTernaryOp::MStore
                    | EvmTernaryOp::MStore8
                    | EvmTernaryOp::Keccak256
            );
            buf.push_str(&format!("{op}(\n"));
            pp(a, depth + 1, buf);
            buf.push_str(",\n");
            pp(b, depth + 1, buf);
            if has_state {
                buf.push_str(", state)");
            } else {
                buf.push_str(",\n");
                pp(c, depth + 1, buf);
                buf.push(')');
            }
        }

        // ---- Tuple ----
        EvmExpr::Get(inner, idx) => {
            indent(depth, buf);
            buf.push_str(&format!("get.{idx}(\n"));
            pp(inner, depth + 1, buf);
            buf.push(')');
        }
        EvmExpr::Concat(_a, _b) => {
            let mut stmts = Vec::new();
            flatten_concat(expr, &mut stmts);
            for (i, stmt) in stmts.iter().enumerate() {
                pp(stmt, depth, buf);
                if i + 1 < stmts.len() {
                    buf.push('\n');
                }
            }
        }

        // ---- Control flow ----
        EvmExpr::If(pred, _inputs, then_body, else_body) => {
            indent(depth, buf);
            if fits_inline(pred, budget(depth).saturating_sub(5)) {
                // "if " + pred + " {"
                buf.push_str("if ");
                pp_inline(pred, buf);
            } else {
                buf.push_str("if (\n");
                pp(pred, depth + 1, buf);
                buf.push('\n');
                indent(depth, buf);
                buf.push(')');
            }
            buf.push_str(" {\n");
            pp(then_body, depth + 1, buf);
            buf.push('\n');
            indent(depth, buf);
            buf.push_str("} else {\n");
            pp(else_body, depth + 1, buf);
            buf.push('\n');
            indent(depth, buf);
            buf.push('}');
        }
        EvmExpr::DoWhile(_inputs, pred_and_body) => {
            indent(depth, buf);
            buf.push_str("do {\n");
            pp(pred_and_body, depth + 1, buf);
            buf.push('\n');
            indent(depth, buf);
            buf.push_str("} while(...)");
        }

        // ---- EVM ops ----
        EvmExpr::Log(n, topics, data_offset, data_size, _state) => {
            indent(depth, buf);
            buf.push_str(&format!("LOG{n}("));
            // Try to inline topics
            let all_topics_inline = topics.iter().all(|t| fits_inline(t, 40));
            if all_topics_inline && topics.len() <= 2 {
                for (i, t) in topics.iter().enumerate() {
                    pp_inline(t, buf);
                    if i + 1 < topics.len() {
                        buf.push_str(", ");
                    }
                }
                buf.push_str(",\n");
            } else {
                buf.push('\n');
                for t in topics {
                    pp(t, depth + 1, buf);
                    buf.push_str(",\n");
                }
            }
            indent(depth + 1, buf);
            buf.push_str("data=");
            pp_inline(data_offset, buf);
            buf.push_str(", ");
            pp_inline(data_size, buf);
            buf.push(')');
        }
        EvmExpr::Revert(offset, size, _state) => {
            indent(depth, buf);
            buf.push_str("revert(");
            pp_inline(offset, buf);
            buf.push_str(", ");
            pp_inline(size, buf);
            buf.push(')');
        }
        EvmExpr::ReturnOp(offset, size, _state) => {
            indent(depth, buf);
            buf.push_str("return(");
            pp_inline(offset, buf);
            buf.push_str(", ");
            pp_inline(size, buf);
            buf.push(')');
        }
        EvmExpr::ExtCall(target, value, args_off, args_len, ret_off, ret_len, _state) => {
            indent(depth, buf);
            buf.push_str("CALL(\n");
            let labels = [
                "target", "value", "args_off", "args_len", "ret_off", "ret_len",
            ];
            let args = [target, value, args_off, args_len, ret_off, ret_len];
            for (i, (label, arg)) in labels.iter().zip(args.iter()).enumerate() {
                indent(depth + 1, buf);
                buf.push_str(label);
                buf.push('=');
                if fits_inline(arg, budget(depth + 1).saturating_sub(label.len() + 1)) {
                    pp_inline(arg, buf);
                } else {
                    buf.push('\n');
                    pp(arg, depth + 2, buf);
                }
                if i + 1 < labels.len() {
                    buf.push(',');
                }
                buf.push('\n');
            }
            indent(depth, buf);
            buf.push(')');
        }
        EvmExpr::Call(name, args) => {
            indent(depth, buf);
            buf.push_str(&format!("call {name}("));
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    buf.push_str(", ");
                }
                pp_inline(arg, buf);
            }
            buf.push(')');
        }
        // ---- Let bindings ----
        EvmExpr::LetBind(name, init, body) => {
            indent(depth, buf);
            let prefix_len = 6 + name.len() + 3; // "let $" + name + " = "
            if fits_inline(init, budget(depth).saturating_sub(prefix_len)) {
                buf.push_str(&format!("let ${name} = "));
                pp_inline(init, buf);
            } else {
                buf.push_str(&format!("let ${name} =\n"));
                pp(init, depth + 1, buf);
            }
            buf.push('\n');
            pp(body, depth, buf);
        }
        EvmExpr::VarStore(name, val) => {
            indent(depth, buf);
            let prefix_len = 1 + name.len() + 3; // "$" + name + " = "
            if fits_inline(val, budget(depth).saturating_sub(prefix_len)) {
                buf.push_str(&format!("${name} = "));
                pp_inline(val, buf);
            } else {
                buf.push_str(&format!("${name} =\n"));
                pp(val, depth + 1, buf);
            }
        }

        // ---- Top-level ----
        EvmExpr::Function(name, _in_ty, _out_ty, body) => {
            indent(depth, buf);
            buf.push_str(&format!("fn {name}() {{\n"));
            pp(body, depth + 1, buf);
            buf.push('\n');
            indent(depth, buf);
            buf.push('}');
        }
        EvmExpr::StorageField(name, slot, ty) => {
            indent(depth, buf);
            buf.push_str(&format!("storage {name} @ slot {slot} : {}", fmt_type(ty)));
        }
    }
}

/// Print an expression inline, always producing a single line.
/// Unlike `pp_inline`, this is fully self-contained and handles every node,
/// recursively calling itself (never `pp` or `pp_inline`) to guarantee no newlines.
fn pp_oneline(expr: &RcExpr, buf: &mut String) {
    match expr.as_ref() {
        EvmExpr::Arg(ty, _) => buf.push_str(&format!("arg:{}", fmt_type(ty))),
        EvmExpr::Const(c, ty, _) => {
            let val = match c {
                EvmConstant::SmallInt(n) => format!("{n}"),
                EvmConstant::LargeInt(s) => format!("0x{s}"),
                EvmConstant::Bool(b) => format!("{b}"),
                EvmConstant::Addr(a) => format!("@{a}"),
            };
            buf.push_str(&format!("{val}:{}", fmt_type(ty)));
        }
        EvmExpr::Empty(_, _) => buf.push_str("empty"),
        EvmExpr::Var(name) => buf.push_str(&format!("${name}")),
        EvmExpr::Drop(name) => buf.push_str(&format!("drop ${name}")),
        EvmExpr::Selector(sig) => buf.push_str(&format!("selector(\"{sig}\")")),
        EvmExpr::EnvRead(op, _) => buf.push_str(&format!("{op}()")),
        EvmExpr::EnvRead1(op, arg, _) => {
            buf.push_str(&format!("{op}("));
            pp_oneline(arg, buf);
            buf.push(')');
        }
        EvmExpr::Bop(op, lhs, rhs) => {
            buf.push_str(&format!("{op}("));
            pp_oneline(lhs, buf);
            if op.has_state() {
                buf.push_str(", state)");
            } else {
                buf.push_str(", ");
                pp_oneline(rhs, buf);
                buf.push(')');
            }
        }
        EvmExpr::Uop(op, inner) => {
            buf.push_str(&format!("{op}("));
            pp_oneline(inner, buf);
            buf.push(')');
        }
        EvmExpr::Top(op, a, b, c) => {
            let has_state = matches!(
                op,
                EvmTernaryOp::SStore
                    | EvmTernaryOp::TStore
                    | EvmTernaryOp::MStore
                    | EvmTernaryOp::MStore8
                    | EvmTernaryOp::Keccak256
            );
            buf.push_str(&format!("{op}("));
            pp_oneline(a, buf);
            buf.push_str(", ");
            pp_oneline(b, buf);
            if has_state {
                buf.push_str(", state)");
            } else {
                buf.push_str(", ");
                pp_oneline(c, buf);
                buf.push(')');
            }
        }
        EvmExpr::Get(inner, idx) => {
            pp_oneline(inner, buf);
            buf.push_str(&format!(".{idx}"));
        }
        // Compound/control-flow nodes — abbreviate
        EvmExpr::If(cond, _, _, _) => {
            buf.push_str("if ");
            pp_oneline(cond, buf);
            buf.push_str(" { ... } else { ... }");
        }
        EvmExpr::DoWhile(..) => buf.push_str("do { ... } while(...)"),
        EvmExpr::Concat(..) => {
            let mut stmts = Vec::new();
            flatten_concat(expr, &mut stmts);
            buf.push_str(&format!("<{} stmts>", stmts.len()));
        }
        EvmExpr::LetBind(name, init, _) => {
            buf.push_str(&format!("let ${name} = "));
            pp_oneline(init, buf);
        }
        EvmExpr::VarStore(name, val) => {
            buf.push_str(&format!("${name} = "));
            pp_oneline(val, buf);
        }
        EvmExpr::Log(n, _, _, _, _) => buf.push_str(&format!("LOG{n}(...)")),
        EvmExpr::Revert(off, size, _) => {
            buf.push_str("revert(");
            pp_oneline(off, buf);
            buf.push_str(", ");
            pp_oneline(size, buf);
            buf.push(')');
        }
        EvmExpr::ReturnOp(off, size, _) => {
            buf.push_str("return(");
            pp_oneline(off, buf);
            buf.push_str(", ");
            pp_oneline(size, buf);
            buf.push(')');
        }
        EvmExpr::ExtCall(..) => buf.push_str("CALL(...)"),
        EvmExpr::Call(name, args) => {
            buf.push_str(&format!("call {name}("));
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    buf.push_str(", ");
                }
                pp_oneline(arg, buf);
            }
            buf.push(')');
        }
        EvmExpr::Function(name, _, _, _) => buf.push_str(&format!("fn {name}()")),
        EvmExpr::StorageField(name, slot, ty) => {
            buf.push_str(&format!("storage {name} @ slot {slot} : {}", fmt_type(ty)));
        }
    }
}

/// Produce a compact one-line IR summary for **statement-level** nodes.
///
/// Returns `None` for leaf/value expressions (Const, Var, Bop, Uop, etc.)
/// that don't merit their own comment in assembly output.
pub fn pretty_summary(expr: &EvmExpr) -> Option<String> {
    let mut buf = String::new();
    match expr {
        EvmExpr::LetBind(name, init, _) => {
            buf.push_str(&format!("let ${name} = "));
            pp_oneline(init, &mut buf);
        }
        EvmExpr::VarStore(name, val) => {
            buf.push_str(&format!("${name} = "));
            pp_oneline(val, &mut buf);
        }
        EvmExpr::Drop(name) => {
            buf.push_str(&format!("drop ${name}"));
        }
        EvmExpr::If(cond, _, _, _) => {
            buf.push_str("if ");
            pp_oneline(cond, &mut buf);
            buf.push_str(" { ... } else { ... }");
        }
        EvmExpr::DoWhile(..) => {
            buf.push_str("do { ... } while(...)");
        }
        EvmExpr::ReturnOp(off, size, _) => {
            buf.push_str("return(");
            pp_oneline(off, &mut buf);
            buf.push_str(", ");
            pp_oneline(size, &mut buf);
            buf.push(')');
        }
        EvmExpr::Revert(off, size, _) => {
            buf.push_str("revert(");
            pp_oneline(off, &mut buf);
            buf.push_str(", ");
            pp_oneline(size, &mut buf);
            buf.push(')');
        }
        EvmExpr::Log(n, topics, data_offset, data_size, _) => {
            buf.push_str(&format!("LOG{n}("));
            for (i, t) in topics.iter().enumerate() {
                pp_oneline(t, &mut buf);
                if i + 1 < topics.len() {
                    buf.push_str(", ");
                }
            }
            buf.push_str(", data=");
            pp_oneline(data_offset, &mut buf);
            buf.push_str(", ");
            pp_oneline(data_size, &mut buf);
            buf.push(')');
        }
        EvmExpr::ExtCall(target, value, args_off, args_len, _ret_off, _ret_len, _) => {
            buf.push_str("CALL(target=");
            pp_oneline(target, &mut buf);
            buf.push_str(", value=");
            pp_oneline(value, &mut buf);
            buf.push_str(", args_off=");
            pp_oneline(args_off, &mut buf);
            buf.push_str(", args_len=");
            pp_oneline(args_len, &mut buf);
            buf.push_str(", ...)");
        }
        EvmExpr::Call(name, args) => {
            buf.push_str(&format!("call {name}("));
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    buf.push_str(", ");
                }
                pp_oneline(arg, &mut buf);
            }
            buf.push(')');
        }
        EvmExpr::Function(name, _, _, _) => {
            buf.push_str(&format!("fn {name}()"));
        }
        EvmExpr::Top(op, a, b, _c) => match op {
            EvmTernaryOp::SStore
            | EvmTernaryOp::TStore
            | EvmTernaryOp::MStore
            | EvmTernaryOp::MStore8
            | EvmTernaryOp::Keccak256 => {
                buf.push_str(&format!("{op}("));
                pp_oneline(a, &mut buf);
                buf.push_str(", ");
                pp_oneline(b, &mut buf);
                buf.push(')');
            }
            _ => return None,
        },
        // Non-statement nodes: no comment
        _ => return None,
    }

    // Truncate at 120 chars
    if buf.len() > 120 {
        buf.truncate(117);
        buf.push_str("...");
    }

    Some(buf)
}

fn flatten_concat<'a>(expr: &'a RcExpr, out: &mut Vec<&'a RcExpr>) {
    if let EvmExpr::Concat(a, b) = expr.as_ref() {
        flatten_concat(a, out);
        flatten_concat(b, out);
    } else {
        out.push(expr);
    }
}
