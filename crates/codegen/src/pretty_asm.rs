//! Pretty-printer for post-optimization assembly instructions.
//!
//! Produces a human-readable disassembly with labeled blocks, formatted
//! PUSH values, and indented instruction bodies.

use crate::{assembler::AsmInstruction, opcode::Opcode, AsmOutput};

/// Pretty-print an `AsmOutput` (constructor + runtime).
pub fn pretty_print_asm(output: &AsmOutput, contract_name: &str) -> String {
    let mut buf = String::new();

    if !output.constructor.is_empty() {
        buf.push_str(&format!("=== {contract_name} constructor ===\n"));
        pp_instructions(&output.constructor, &mut buf);
        buf.push('\n');
    }

    buf.push_str(&format!("=== {contract_name} runtime ===\n"));
    pp_instructions(&output.runtime, &mut buf);

    buf
}

fn pp_instructions(instructions: &[AsmInstruction], buf: &mut String) {
    // Track whether we're inside a block (after a label) for indentation
    let mut in_block = false;
    let mut byte_offset: usize = 0;
    // Estimate if short jumps (< 256 bytes total)
    let total_size: usize = instructions.iter().map(est_size).sum();
    let short = total_size < 256;

    for (i, inst) in instructions.iter().enumerate() {
        match inst {
            AsmInstruction::Label(name) => {
                // Blank line before label (unless first instruction)
                if i > 0 {
                    buf.push('\n');
                }
                buf.push_str(&format!("{name}:  ; {byte_offset:04x} JUMPDEST\n"));
                in_block = true;
                byte_offset += 1; // JUMPDEST
            }
            AsmInstruction::Op(op) => {
                let prefix = if in_block { "    " } else { "  " };
                buf.push_str(&format!(
                    "{prefix}{:04x}  {}\n",
                    byte_offset,
                    format_op(*op)
                ));
                byte_offset += 1;
            }
            AsmInstruction::Push(data) => {
                let prefix = if in_block { "    " } else { "  " };
                let val = format_push_value(data);
                buf.push_str(&format!(
                    "{prefix}{:04x}  PUSH{} {val}\n",
                    byte_offset,
                    data.len()
                ));
                byte_offset += 1 + data.len();
            }
            AsmInstruction::JumpTo(label) => {
                let prefix = if in_block { "    " } else { "  " };
                buf.push_str(&format!("{prefix}{byte_offset:04x}  JUMP -> {label}\n"));
                byte_offset += if short { 3 } else { 4 };
            }
            AsmInstruction::JumpITo(label) => {
                let prefix = if in_block { "    " } else { "  " };
                buf.push_str(&format!("{prefix}{byte_offset:04x}  JUMPI -> {label}\n"));
                byte_offset += if short { 3 } else { 4 };
            }
            AsmInstruction::PushLabel(label) => {
                let prefix = if in_block { "    " } else { "  " };
                buf.push_str(&format!("{prefix}{byte_offset:04x}  PUSH @{label}\n"));
                byte_offset += if short { 2 } else { 3 };
            }
            AsmInstruction::Comment(msg) => {
                let prefix = if in_block { "    " } else { "  " };
                buf.push_str(&format!("{prefix}      ; {msg}\n"));
            }
            AsmInstruction::Raw(data) => {
                let prefix = if in_block { "    " } else { "  " };
                let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
                buf.push_str(&format!(
                    "{prefix}{byte_offset:04x}  ASM [{} bytes] {hex}\n",
                    data.len()
                ));
                byte_offset += data.len();
            }
        }
    }
}

fn est_size(inst: &AsmInstruction) -> usize {
    match inst {
        AsmInstruction::Op(_) | AsmInstruction::Label(_) => 1,
        AsmInstruction::Push(data) => 1 + data.len(),
        AsmInstruction::JumpTo(_) | AsmInstruction::JumpITo(_) => 4, // conservative
        AsmInstruction::PushLabel(_) => 3,
        AsmInstruction::Comment(_) => 0,
        AsmInstruction::Raw(data) => data.len(),
    }
}

fn format_op(op: Opcode) -> String {
    format!("{op:?}")
}

fn format_push_value(data: &[u8]) -> String {
    if data.is_empty() {
        return "0x00".to_string();
    }

    // Try to show as decimal if it's small enough
    if data.len() <= 8 {
        let mut val: u64 = 0;
        for &b in data {
            val = (val << 8) | u64::from(b);
        }
        if val <= 0xFFFF {
            return format!("{val} (0x{val:x})");
        }
    }

    // Otherwise hex
    let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
    format!("0x{hex}")
}
