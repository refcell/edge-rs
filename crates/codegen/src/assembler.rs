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
    /// A comment (zero-width, no bytecode emitted). Used for IR provenance.
    Comment(String),
    /// Raw bytecode bytes (inline assembly). Not subject to label resolution.
    Raw(Vec<u8>),
}

impl AsmInstruction {
    /// Compute the byte size of this instruction.
    ///
    /// `short_jumps`: if true, use PUSH1 (2 bytes) for jump addresses;
    /// if false, use PUSH2 (3 bytes). Short jumps work for contracts < 256 bytes.
    fn byte_size(&self, short_jumps: bool) -> usize {
        match self {
            Self::Op(_) | Self::Label(_) => 1,  // Op or JUMPDEST
            Self::Push(data) => 1 + data.len(), // PUSHn + n bytes
            Self::JumpTo(_) | Self::JumpITo(_) => {
                if short_jumps {
                    3
                } else {
                    4
                }
            } // PUSH1/PUSH2 + JUMP/JUMPI
            Self::PushLabel(_) => {
                if short_jumps {
                    2
                } else {
                    3
                }
            } // PUSH1/PUSH2 (no JUMP)
            Self::Comment(_) => 0,
            Self::Raw(data) => data.len(),
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
    pub const fn new() -> Self {
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

    /// Emit a comment (zero-width, for IR provenance in asm output).
    pub fn emit_comment(&mut self, msg: impl Into<String>) {
        self.emit(AsmInstruction::Comment(msg.into()));
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
    pub const fn from_instructions(instructions: Vec<AsmInstruction>) -> Self {
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

    /// Jump threading: if Label(X) is immediately followed by JumpTo(Y)
    /// (skipping Comments), rewrite any JumpTo(X)/JumpITo(X)/PushLabel(X) to
    /// target Y instead. Iterates to a fixed point for chains.
    pub fn thread_jumps(&mut self) {
        use std::collections::HashMap;
        loop {
            // Build redirect map: label X → label Y when Label(X) is followed by JumpTo(Y)
            let mut redirects: HashMap<String, String> = HashMap::new();
            let mut i = 0;
            while i < self.instructions.len() {
                if let AsmInstruction::Label(ref label) = self.instructions[i] {
                    // Find next non-comment instruction after this label
                    let mut j = i + 1;
                    while j < self.instructions.len() {
                        if matches!(self.instructions[j], AsmInstruction::Comment(_)) {
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    if j < self.instructions.len() {
                        if let AsmInstruction::JumpTo(ref target) = self.instructions[j] {
                            if target != label {
                                redirects.insert(label.clone(), target.clone());
                            }
                        }
                    }
                }
                i += 1;
            }

            if redirects.is_empty() {
                break;
            }

            // Apply redirects
            let mut changed = false;
            for inst in &mut self.instructions {
                let target = match inst {
                    AsmInstruction::JumpTo(ref t)
                    | AsmInstruction::JumpITo(ref t)
                    | AsmInstruction::PushLabel(ref t) => redirects.get(t).cloned(),
                    _ => None,
                };
                if let Some(new_target) = target {
                    match inst {
                        AsmInstruction::JumpTo(ref mut t)
                        | AsmInstruction::JumpITo(ref mut t)
                        | AsmInstruction::PushLabel(ref mut t) => {
                            *t = new_target;
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }

            if !changed {
                break;
            }
        }

        // Remove dead labels: Label(X) that no jump/push references anymore.
        // Also remove the JumpTo that follows a dead label (and intervening comments).
        use std::collections::HashSet;
        let referenced: HashSet<String> = self
            .instructions
            .iter()
            .filter_map(|inst| match inst {
                AsmInstruction::JumpTo(t)
                | AsmInstruction::JumpITo(t)
                | AsmInstruction::PushLabel(t) => Some(t.clone()),
                _ => None,
            })
            .collect();

        let mut keep = vec![true; self.instructions.len()];
        let mut i = 0;
        while i < self.instructions.len() {
            if let AsmInstruction::Label(ref label) = self.instructions[i] {
                if !referenced.contains(label) {
                    // Mark label and subsequent comments + JumpTo for removal
                    keep[i] = false;
                    let mut j = i + 1;
                    while j < self.instructions.len() {
                        match &self.instructions[j] {
                            AsmInstruction::Comment(_) => {
                                keep[j] = false;
                                j += 1;
                            }
                            AsmInstruction::JumpTo(_) => {
                                keep[j] = false;
                                break;
                            }
                            _ => break,
                        }
                    }
                }
            }
            i += 1;
        }

        let mut idx = 0;
        self.instructions.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }

    /// Eliminate SWAP1+POP cleanup chains that precede a halting sequence.
    ///
    /// Within a basic block (between labels), if a contiguous chain of SWAP1+POP
    /// pairs is followed by a "clean terminal" sequence (only Push/MSTORE/RETURN/
    /// REVERT/STOP, no DUPs or SWAPs), the chain can be removed. The SWAP1+POP
    /// chain preserves TOS while removing elements below it; since the terminal
    /// sequence only uses TOS and freshly pushed values, the removed elements
    /// are never accessed. RETURN/REVERT/STOP halts execution so leftover stack
    /// junk is harmless.
    pub fn eliminate_pre_halt_cleanup(&mut self) {
        use std::collections::HashSet;

        // Phase 1: Identify "halting labels" — labels whose basic block ends
        // with RETURN/REVERT/STOP (or JUMP to another halting label).
        // Iterate to a fixed point for chains.
        let mut halting_labels: HashSet<String> = HashSet::new();
        loop {
            let mut changed = false;
            let mut current_label: Option<String> = None;
            for inst in &self.instructions {
                match inst {
                    AsmInstruction::Label(name) => {
                        current_label = Some(name.clone());
                    }
                    AsmInstruction::Op(Opcode::Return)
                    | AsmInstruction::Op(Opcode::Revert)
                    | AsmInstruction::Op(Opcode::Stop)
                    | AsmInstruction::Op(Opcode::Invalid) => {
                        if let Some(ref label) = current_label {
                            if halting_labels.insert(label.clone()) {
                                changed = true;
                            }
                        }
                    }
                    AsmInstruction::JumpTo(target) => {
                        if halting_labels.contains(target) {
                            if let Some(ref label) = current_label {
                                if halting_labels.insert(label.clone()) {
                                    changed = true;
                                }
                            }
                        }
                    }
                    // JumpITo doesn't unconditionally halt — skip
                    _ => {}
                }
            }
            if !changed {
                break;
            }
        }

        // Phase 2: For each basic block, check if it ends in a halt or
        // unconditional jump to a halting label. If so, remove SWAP1+POP
        // chains that immediately precede the terminal sequence.
        let len = self.instructions.len();
        let mut keep = vec![true; len];

        // Find block boundaries and terminal instructions
        let mut i = 0;
        while i < len {
            // Find the end of this basic block: next Label, or end of instructions
            let block_start = i;
            let mut block_end = i;
            let mut terminal_idx = None;

            let mut j = if matches!(&self.instructions[i], AsmInstruction::Label(_)) {
                i + 1
            } else {
                i
            };

            while j < len {
                match &self.instructions[j] {
                    AsmInstruction::Label(name) => {
                        // Fallthrough to next block — if the target is halting,
                        // use the label position as the boundary so the backward
                        // walk finds the SWAP1+POP chain at the end of this block.
                        if halting_labels.contains(name) {
                            terminal_idx = Some(j);
                        }
                        block_end = j;
                        break;
                    }
                    AsmInstruction::Op(Opcode::Return)
                    | AsmInstruction::Op(Opcode::Revert)
                    | AsmInstruction::Op(Opcode::Stop)
                    | AsmInstruction::Op(Opcode::Invalid) => {
                        terminal_idx = Some(j);
                        // Mark everything after the halt in this block as dead
                        let mut k = j + 1;
                        while k < len && !matches!(&self.instructions[k], AsmInstruction::Label(_))
                        {
                            keep[k] = false;
                            k += 1;
                        }
                        block_end = k;
                        break;
                    }
                    AsmInstruction::JumpTo(target) => {
                        if halting_labels.contains(target) {
                            terminal_idx = Some(j);
                        }
                        block_end = j + 1;
                        break;
                    }
                    AsmInstruction::JumpITo(_) => {
                        // Conditional jump — not a clean terminal
                        block_end = j + 1;
                        break;
                    }
                    _ => {
                        j += 1;
                    }
                }
            }
            if j >= len {
                block_end = len;
            }

            // If we found a terminal, walk backward to find the "clean terminal"
            // start, then further backward to find SWAP1+POP chain
            if let Some(term) = terminal_idx {
                // Walk backward past the clean terminal sequence
                // (Push, Push0, MStore, MStore8, Comments — no DUP/SWAP/other)
                let mut setup_start = term;
                while setup_start > block_start {
                    let prev = setup_start - 1;
                    match &self.instructions[prev] {
                        AsmInstruction::Op(Opcode::MStore | Opcode::MStore8 | Opcode::Push0)
                        | AsmInstruction::Push(_)
                        | AsmInstruction::Comment(_) => {
                            setup_start = prev;
                        }
                        _ => break,
                    }
                }

                // Check for redundant DUP1 before the clean terminal: DUP1 copies
                // TOS for MStore consumption, but if RETURN follows the original
                // copy is never used and the DUP1 can be removed.
                if setup_start > block_start {
                    let prev = setup_start - 1;
                    if matches!(&self.instructions[prev], AsmInstruction::Op(Opcode::Dup1)) {
                        keep[prev] = false;
                        setup_start = prev;
                    }
                }

                // Walk backward past SWAP1+POP chain
                let mut chain_start = setup_start;
                while chain_start >= block_start + 2 {
                    let pop_idx = chain_start - 1;
                    let swap_idx = chain_start - 2;
                    // Skip comments between swap and pop
                    if matches!(&self.instructions[pop_idx], AsmInstruction::Op(Opcode::Pop))
                        && matches!(
                            &self.instructions[swap_idx],
                            AsmInstruction::Op(Opcode::Swap1)
                        )
                    {
                        chain_start = swap_idx;
                    } else if matches!(&self.instructions[pop_idx], AsmInstruction::Comment(_)) {
                        // Skip comment and try again
                        chain_start = pop_idx;
                    } else {
                        break;
                    }
                }

                // Mark SWAP1+POP pairs in the chain for removal
                if chain_start < setup_start {
                    let mut k = chain_start;
                    while k < setup_start {
                        match &self.instructions[k] {
                            AsmInstruction::Op(Opcode::Swap1) | AsmInstruction::Op(Opcode::Pop) => {
                                keep[k] = false;
                            }
                            _ => {} // keep comments
                        }
                        k += 1;
                    }
                }
            }

            i = if block_end > block_start {
                block_end
            } else {
                block_start + 1
            };
        }

        let mut idx = 0;
        self.instructions.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
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
                AsmInstruction::Comment(_) => {
                    // Comments are zero-width; no bytecode emitted.
                }
                AsmInstruction::Raw(data) => {
                    bytecode.extend_from_slice(data);
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
