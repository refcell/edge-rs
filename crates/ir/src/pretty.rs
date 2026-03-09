//! Pretty-printer for `EvmExpr` IR trees.
//!
//! Produces a human-readable, indented representation of the post-egglog
//! optimized IR.

use crate::schema::{EvmConstant, EvmContract, EvmExpr, EvmTernaryOp, EvmType, RcExpr};

/// Max inline width before we break to multi-line.
const MAX_INLINE: usize = 80;

/// Disassemble a hex-encoded bytecode string into EVM mnemonics.
/// E.g. "02600101" → "MUL PUSH1(0x01) ADD"
fn disassemble_hex(hex: &str) -> String {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .filter_map(|i| {
            hex.get(i..i + 2)
                .and_then(|s| u8::from_str_radix(s, 16).ok())
        })
        .collect();
    let mut parts = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let op = bytes[i];
        let name = opcode_mnemonic(op);
        if (0x60..=0x7f).contains(&op) {
            let n = (op - 0x5f) as usize;
            let data_end = (i + 1 + n).min(bytes.len());
            let data: String = bytes[i + 1..data_end]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            parts.push(format!("{name}(0x{data})"));
            i = data_end;
        } else {
            parts.push(name.to_string());
            i += 1;
        }
    }
    parts.join(" ")
}

const fn opcode_mnemonic(op: u8) -> &'static str {
    match op {
        0x00 => "STOP",
        0x01 => "ADD",
        0x02 => "MUL",
        0x03 => "SUB",
        0x04 => "DIV",
        0x05 => "SDIV",
        0x06 => "MOD",
        0x07 => "SMOD",
        0x08 => "ADDMOD",
        0x09 => "MULMOD",
        0x0a => "EXP",
        0x0b => "SIGNEXTEND",
        0x10 => "LT",
        0x11 => "GT",
        0x12 => "SLT",
        0x13 => "SGT",
        0x14 => "EQ",
        0x15 => "ISZERO",
        0x16 => "AND",
        0x17 => "OR",
        0x18 => "XOR",
        0x19 => "NOT",
        0x1a => "BYTE",
        0x1b => "SHL",
        0x1c => "SHR",
        0x1d => "SAR",
        0x20 => "KECCAK256",
        0x30 => "ADDRESS",
        0x31 => "BALANCE",
        0x32 => "ORIGIN",
        0x33 => "CALLER",
        0x34 => "CALLVALUE",
        0x35 => "CALLDATALOAD",
        0x36 => "CALLDATASIZE",
        0x37 => "CALLDATACOPY",
        0x38 => "CODESIZE",
        0x39 => "CODECOPY",
        0x3a => "GASPRICE",
        0x3b => "EXTCODESIZE",
        0x3c => "EXTCODECOPY",
        0x3d => "RETURNDATASIZE",
        0x3e => "RETURNDATACOPY",
        0x3f => "EXTCODEHASH",
        0x40 => "BLOCKHASH",
        0x41 => "COINBASE",
        0x42 => "TIMESTAMP",
        0x43 => "NUMBER",
        0x44 => "PREVRANDAO",
        0x45 => "GASLIMIT",
        0x46 => "CHAINID",
        0x47 => "SELFBALANCE",
        0x48 => "BASEFEE",
        0x49 => "BLOBHASH",
        0x4a => "BLOBBASEFEE",
        0x50 => "POP",
        0x51 => "MLOAD",
        0x52 => "MSTORE",
        0x53 => "MSTORE8",
        0x54 => "SLOAD",
        0x55 => "SSTORE",
        0x56 => "JUMP",
        0x57 => "JUMPI",
        0x58 => "PC",
        0x59 => "MSIZE",
        0x5a => "GAS",
        0x5b => "JUMPDEST",
        0x5c => "TLOAD",
        0x5d => "TSTORE",
        0x5e => "MCOPY",
        0x5f => "PUSH0",
        0x60..=0x7f => {
            const PUSH_NAMES: [&str; 32] = [
                "PUSH1", "PUSH2", "PUSH3", "PUSH4", "PUSH5", "PUSH6", "PUSH7", "PUSH8", "PUSH9",
                "PUSH10", "PUSH11", "PUSH12", "PUSH13", "PUSH14", "PUSH15", "PUSH16", "PUSH17",
                "PUSH18", "PUSH19", "PUSH20", "PUSH21", "PUSH22", "PUSH23", "PUSH24", "PUSH25",
                "PUSH26", "PUSH27", "PUSH28", "PUSH29", "PUSH30", "PUSH31", "PUSH32",
            ];
            PUSH_NAMES[(op - 0x60) as usize]
        }
        0x80..=0x8f => {
            const DUP_NAMES: [&str; 16] = [
                "DUP1", "DUP2", "DUP3", "DUP4", "DUP5", "DUP6", "DUP7", "DUP8", "DUP9", "DUP10",
                "DUP11", "DUP12", "DUP13", "DUP14", "DUP15", "DUP16",
            ];
            DUP_NAMES[(op - 0x80) as usize]
        }
        0x90..=0x9f => {
            const SWAP_NAMES: [&str; 16] = [
                "SWAP1", "SWAP2", "SWAP3", "SWAP4", "SWAP5", "SWAP6", "SWAP7", "SWAP8", "SWAP9",
                "SWAP10", "SWAP11", "SWAP12", "SWAP13", "SWAP14", "SWAP15", "SWAP16",
            ];
            SWAP_NAMES[(op - 0x90) as usize]
        }
        0xa0 => "LOG0",
        0xa1 => "LOG1",
        0xa2 => "LOG2",
        0xa3 => "LOG3",
        0xa4 => "LOG4",
        0xf0 => "CREATE",
        0xf1 => "CALL",
        0xf2 => "CALLCODE",
        0xf3 => "RETURN",
        0xf4 => "DELEGATECALL",
        0xf5 => "CREATE2",
        0xfa => "STATICCALL",
        0xfd => "REVERT",
        0xfe => "INVALID",
        0xff => "SELFDESTRUCT",
        _ => "UNKNOWN",
    }
}

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
        | EvmExpr::VarStore(..)
        | EvmExpr::InlineAsm(..) => None,
        EvmExpr::MemRegion(id, sz) => Some(format!("region({id}, {sz})").len()),
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
        EvmExpr::MemRegion(id, sz) => buf.push_str(&format!("region({id}, {sz})")),
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
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            indent(depth, buf);
            let disasm = disassemble_hex(hex);
            if inputs.is_empty() {
                buf.push_str(&format!("asm() -> ({num_outputs}) {{ {disasm} }}"));
            } else {
                let args: Vec<String> = inputs
                    .iter()
                    .map(|e| {
                        let mut s = String::new();
                        pp_oneline(e, &mut s);
                        s
                    })
                    .collect();
                buf.push_str(&format!(
                    "asm({}) -> ({num_outputs}) {{ {disasm} }}",
                    args.join(", ")
                ));
            }
        }
        EvmExpr::MemRegion(id, sz) => {
            indent(depth, buf);
            buf.push_str(&format!("region({id}, {sz})"));
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
        EvmExpr::InlineAsm(_inputs, hex, num_outputs) => {
            let disasm = disassemble_hex(hex);
            buf.push_str(&format!("asm({num_outputs}){{ {disasm} }}"));
        }
        EvmExpr::MemRegion(id, sz) => buf.push_str(&format!("region({id}, {sz})")),
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
        EvmExpr::InlineAsm(_inputs, hex, num_outputs) => {
            let disasm = disassemble_hex(hex);
            buf.push_str(&format!("asm({num_outputs}) {{ {disasm} }}"));
        }
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
