//! EVM bytecode assembler with label resolution.
//!
//! Converts a sequence of `AsmInstruction` items into final bytecode,
//! resolving symbolic labels to concrete byte offsets.

use indexmap::IndexMap;

use crate::opcode::Opcode;

/// An instruction in the assembly buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AsmInstruction {
    /// A raw opcode (no immediate data).
    Op(Opcode),
    /// PUSH with immediate data (1-32 bytes).
    Push(Vec<u8>),
    /// A label target (emitted as JUMPDEST).
    Label(String),
    /// Unconditional jump to a label (emits PUSH addr + JUMP).
    JumpTo(String),
    /// Conditional jump to a label (emits PUSH addr + JUMPI).
    JumpITo(String),
    /// Push a label's address onto the stack (emits PUSH addr, no JUMP).
    /// Used by subroutine extraction for return addresses.
    PushLabel(String),
}

impl AsmInstruction {
    /// Compute the byte size of this instruction.
    ///
    /// `short_jumps`: if true, use PUSH1 (2 bytes) for jump addresses;
    /// if false, use PUSH2 (3 bytes). Short jumps work for contracts < 256 bytes.
    fn byte_size(&self, short_jumps: bool) -> usize {
        match self {
            Self::Op(_) => 1,
            Self::Push(data) => 1 + data.len(), // PUSHn + n bytes
            Self::Label(_) => 1,                // JUMPDEST
            Self::JumpTo(_) => {
                if short_jumps {
                    3
                } else {
                    4
                }
            } // PUSH1/PUSH2 + JUMP
            Self::JumpITo(_) => {
                if short_jumps {
                    3
                } else {
                    4
                }
            } // PUSH1/PUSH2 + JUMPI
            Self::PushLabel(_) => {
                if short_jumps {
                    2
                } else {
                    3
                }
            } // PUSH1/PUSH2 (no JUMP)
        }
    }
}

/// Assembler that builds EVM bytecode from instructions.
#[derive(Debug)]
pub struct Assembler {
    /// Instruction buffer
    instructions: Vec<AsmInstruction>,
    /// Label counter for generating unique labels
    label_counter: usize,
}

impl Assembler {
    /// Create a new empty assembler.
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            label_counter: 0,
        }
    }

    /// Generate a fresh unique label with the given prefix.
    pub fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}_{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    /// Emit an instruction.
    pub fn emit(&mut self, inst: AsmInstruction) {
        self.instructions.push(inst);
    }

    /// Emit a raw opcode.
    pub fn emit_op(&mut self, op: Opcode) {
        self.emit(AsmInstruction::Op(op));
    }

    /// Emit PUSH with the minimal encoding for a `usize` value.
    pub fn emit_push_usize(&mut self, value: usize) {
        if value == 0 {
            self.emit_op(Opcode::Push0);
            return;
        }
        let bytes = value.to_be_bytes();
        // Find first non-zero byte
        let start = bytes
            .iter()
            .position(|&b| b != 0)
            .unwrap_or(bytes.len() - 1);
        self.emit(AsmInstruction::Push(bytes[start..].to_vec()));
    }

    /// Emit PUSH with raw bytes (1-32 bytes).
    pub fn emit_push_bytes(&mut self, data: Vec<u8>) {
        assert!(
            !data.is_empty() && data.len() <= 32,
            "PUSH data must be 1-32 bytes"
        );
        self.emit(AsmInstruction::Push(data));
    }

    /// Emit a PUSH for a 256-bit value (32 bytes).
    pub fn emit_push_u256(&mut self, value: &[u8; 32]) {
        // Find first non-zero byte for minimal encoding
        let start = value.iter().position(|&b| b != 0).unwrap_or(31);
        if start == 32 || value.iter().all(|&b| b == 0) {
            self.emit_op(Opcode::Push0);
        } else {
            self.emit(AsmInstruction::Push(value[start..].to_vec()));
        }
    }

    /// Take all instructions out of the assembler, leaving it empty.
    pub fn take_instructions(&mut self) -> Vec<AsmInstruction> {
        std::mem::take(&mut self.instructions)
    }

    /// Create an assembler from a pre-built instruction list.
    pub fn from_instructions(instructions: Vec<AsmInstruction>) -> Self {
        Self {
            instructions,
            label_counter: 0,
        }
    }

    /// Get the current number of instructions.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if the assembler has no instructions.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Assemble into final bytecode, resolving all labels to offsets.
    ///
    /// Tries PUSH1 (short) jumps first. If any label offset >= 256,
    /// falls back to PUSH2 (long) jumps. This saves 1 byte per jump
    /// for contracts under 256 bytes.
    pub fn assemble(&self) -> Vec<u8> {
        // Try short jumps first (optimistic)
        let short_positions = self.compute_label_positions(true);
        let use_short = short_positions.values().all(|&pos| pos < 256);

        // If short jumps don't fit, recompute with long jumps
        let (label_positions, short_jumps) = if use_short {
            (short_positions, true)
        } else {
            (self.compute_label_positions(false), false)
        };

        // Emit bytes
        let mut bytecode = Vec::new();
        for inst in &self.instructions {
            match inst {
                AsmInstruction::Op(op) => {
                    bytecode.push(op.byte());
                }
                AsmInstruction::Push(data) => {
                    let n = data.len() as u8;
                    bytecode.push(Opcode::push_n(n).byte());
                    bytecode.extend_from_slice(data);
                }
                AsmInstruction::Label(_) => {
                    bytecode.push(Opcode::JumpDest.byte());
                }
                AsmInstruction::JumpTo(label) => {
                    let target = label_positions
                        .get(label)
                        .unwrap_or_else(|| panic!("undefined label: {label}"));
                    if short_jumps {
                        bytecode.push(Opcode::Push1.byte());
                        bytecode.push(*target as u8);
                    } else {
                        let addr_bytes = (*target as u16).to_be_bytes();
                        bytecode.push(Opcode::Push2.byte());
                        bytecode.extend_from_slice(&addr_bytes);
                    }
                    bytecode.push(Opcode::Jump.byte());
                }
                AsmInstruction::JumpITo(label) => {
                    let target = label_positions
                        .get(label)
                        .unwrap_or_else(|| panic!("undefined label: {label}"));
                    if short_jumps {
                        bytecode.push(Opcode::Push1.byte());
                        bytecode.push(*target as u8);
                    } else {
                        let addr_bytes = (*target as u16).to_be_bytes();
                        bytecode.push(Opcode::Push2.byte());
                        bytecode.extend_from_slice(&addr_bytes);
                    }
                    bytecode.push(Opcode::JumpI.byte());
                }
                AsmInstruction::PushLabel(label) => {
                    let target = label_positions
                        .get(label)
                        .unwrap_or_else(|| panic!("undefined label: {label}"));
                    if short_jumps {
                        bytecode.push(Opcode::Push1.byte());
                        bytecode.push(*target as u8);
                    } else {
                        let addr_bytes = (*target as u16).to_be_bytes();
                        bytecode.push(Opcode::Push2.byte());
                        bytecode.extend_from_slice(&addr_bytes);
                    }
                }
            }
        }

        bytecode
    }

    /// Compute the byte offset for each label.
    fn compute_label_positions(&self, short_jumps: bool) -> IndexMap<String, usize> {
        let mut positions = IndexMap::new();
        let mut offset = 0;

        for inst in &self.instructions {
            if let AsmInstruction::Label(label) = inst {
                positions.insert(label.clone(), offset);
            }
            offset += inst.byte_size(short_jumps);
        }

        positions
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_assembler() {
        let asm = Assembler::new();
        assert!(asm.is_empty());
        assert_eq!(asm.assemble(), Vec::<u8>::new());
    }

    #[test]
    fn test_simple_opcodes() {
        let mut asm = Assembler::new();
        asm.emit_op(Opcode::Push0);
        asm.emit_op(Opcode::Push0);
        asm.emit_op(Opcode::Add);
        let bytecode = asm.assemble();
        assert_eq!(bytecode, vec![0x5F, 0x5F, 0x01]);
    }

    #[test]
    fn test_push_values() {
        let mut asm = Assembler::new();
        asm.emit_push_usize(0); // PUSH0
        asm.emit_push_usize(1); // PUSH1 0x01
        asm.emit_push_usize(255); // PUSH1 0xFF
        asm.emit_push_usize(256); // PUSH2 0x01 0x00
        let bytecode = asm.assemble();
        assert_eq!(
            bytecode,
            vec![
                0x5F, // PUSH0
                0x60, 0x01, // PUSH1 1
                0x60, 0xFF, // PUSH1 255
                0x61, 0x01, 0x00, // PUSH2 256
            ]
        );
    }

    #[test]
    fn test_label_resolution() {
        let mut asm = Assembler::new();
        // Jump forward to a label
        asm.emit(AsmInstruction::JumpTo("target".to_owned()));
        asm.emit_op(Opcode::Stop);
        asm.emit(AsmInstruction::Label("target".to_owned()));
        asm.emit_op(Opcode::Stop);

        let bytecode = asm.assemble();
        // Small program → short jumps (PUSH1)
        // JumpTo = PUSH1 0x04 + JUMP = 3 bytes (offset 0-2)
        // STOP = 1 byte (offset 3)
        // JUMPDEST at offset 4
        // STOP
        assert_eq!(
            bytecode,
            vec![
                0x60, 0x04, // PUSH1 4
                0x56, // JUMP
                0x00, // STOP
                0x5B, // JUMPDEST
                0x00, // STOP
            ]
        );
    }

    #[test]
    fn test_conditional_jump() {
        let mut asm = Assembler::new();
        asm.emit_push_usize(1); // push condition (true)
        asm.emit(AsmInstruction::JumpITo("target".to_owned()));
        asm.emit_op(Opcode::Stop);
        asm.emit(AsmInstruction::Label("target".to_owned()));
        asm.emit_op(Opcode::Stop);

        let bytecode = asm.assemble();
        // Small program → short jumps (PUSH1)
        // PUSH1 1 = 2 bytes (offset 0-1)
        // JumpITo = PUSH1 0x06 + JUMPI = 3 bytes (offset 2-4)
        // STOP = 1 byte (offset 5)
        // JUMPDEST at offset 6
        assert_eq!(
            bytecode,
            vec![
                0x60, 0x01, // PUSH1 1
                0x60, 0x06, // PUSH1 6
                0x57, // JUMPI
                0x00, // STOP
                0x5B, // JUMPDEST
                0x00, // STOP
            ]
        );
    }

    #[test]
    fn test_fresh_labels() {
        let mut asm = Assembler::new();
        let l1 = asm.fresh_label("loop");
        let l2 = asm.fresh_label("loop");
        let l3 = asm.fresh_label("end");
        assert_eq!(l1, "loop_0");
        assert_eq!(l2, "loop_1");
        assert_eq!(l3, "end_2");
    }
}
