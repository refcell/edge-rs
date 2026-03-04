//! Bytecode subroutine extraction.
//!
//! Detects repeated instruction sequences in the bytecode and extracts them
//! into JUMP-based subroutines, reducing code size at the cost of slight
//! gas overhead per call.
//!
//! Only runs at optimization level >= 2.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::assembler::AsmInstruction;
use crate::opcode::Opcode;

/// Minimum number of instructions in an extractable sequence.
const MIN_SEQ_LEN: usize = 5;

/// Maximum number of instructions in an extractable sequence.
const MAX_SEQ_LEN: usize = 50;

/// Minimum byte size of an extractable sequence.
const MIN_BYTE_SIZE: usize = 15;

/// Minimum number of occurrences for extraction to be profitable.
const MIN_OCCURRENCES: usize = 3;

/// Extract repeated instruction sequences into subroutines.
///
/// Returns the modified instruction list with subroutines appended
/// and call sites rewritten.
pub fn extract_subroutines(instructions: Vec<AsmInstruction>) -> Vec<AsmInstruction> {
    // Find straight-line regions (runs of Op/Push between labels/jumps)
    let regions = find_straight_line_regions(&instructions);

    if regions.is_empty() {
        return instructions;
    }

    // Find all repeated subsequences
    let candidates = find_candidates(&instructions, &regions);

    if candidates.is_empty() {
        return instructions;
    }

    // Greedy selection: pick most profitable non-overlapping candidates
    let selected = select_candidates(candidates);

    if selected.is_empty() {
        return instructions;
    }

    // Rewrite: replace inline occurrences with calls, append subroutines
    rewrite(instructions, selected)
}

/// A region of straight-line code (no labels or jumps).
struct Region {
    start: usize,
    len: usize,
}

/// A candidate for extraction.
struct Candidate {
    /// The instruction sequence (for comparison and inclusion in subroutine).
    instructions: Vec<AsmInstruction>,
    /// Positions where this sequence occurs in the original instruction list.
    positions: Vec<usize>,
    /// Byte size of the sequence.
    byte_size: usize,
    /// Stack inputs required from below.
    stack_inputs: usize,
    /// Stack outputs produced.
    stack_outputs: usize,
    /// Estimated byte savings from extraction.
    savings: i32,
}

/// Find maximal runs of Op/Push instructions between labels and jumps.
fn find_straight_line_regions(instructions: &[AsmInstruction]) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut start = None;

    for (i, inst) in instructions.iter().enumerate() {
        match inst {
            AsmInstruction::Op(_) | AsmInstruction::Push(_) => {
                if start.is_none() {
                    start = Some(i);
                }
            }
            _ => {
                if let Some(s) = start.take() {
                    if i - s >= MIN_SEQ_LEN {
                        regions.push(Region { start: s, len: i - s });
                    }
                }
            }
        }
    }

    // Handle trailing region
    if let Some(s) = start {
        let len = instructions.len() - s;
        if len >= MIN_SEQ_LEN {
            regions.push(Region { start: s, len });
        }
    }

    regions
}

/// Compute the byte size of a sequence of instructions.
fn compute_byte_size(instructions: &[AsmInstruction]) -> usize {
    instructions
        .iter()
        .map(|inst| match inst {
            AsmInstruction::Op(_) => 1,
            AsmInstruction::Push(data) => 1 + data.len(),
            _ => 0,
        })
        .sum()
}

/// Compute the stack effect of a sequence: (inputs_needed, outputs_produced).
///
/// `inputs_needed` accounts for both popped items AND items read by DUP/SWAP.
/// `outputs_produced` is the number of items on the stack above the consumed inputs.
fn compute_stack_effect(instructions: &[AsmInstruction]) -> (usize, usize) {
    let mut height: i32 = 0; // items above initial level
    let mut min_height: i32 = 0; // lowest height from pops
    let mut min_access: i32 = 0; // deepest stack position accessed

    for inst in instructions {
        match inst {
            AsmInstruction::Op(op) => {
                let consumed = op.stack_inputs() as i32;
                let produced = op.stack_outputs() as i32;

                // Track deepest access from consumed inputs
                if consumed > 0 {
                    let deepest_read = height - consumed;
                    min_access = min_access.min(deepest_read);
                }

                // DUP_n reads position (height - n) without popping
                let byte = op.byte();
                if (0x80..=0x8F).contains(&byte) {
                    let n = (byte - 0x80 + 1) as i32;
                    min_access = min_access.min(height - n);
                }
                // SWAP_n accesses position (height - 1 - n) and top
                if (0x90..=0x9F).contains(&byte) {
                    let n = (byte - 0x90 + 1) as i32;
                    min_access = min_access.min(height - 1 - n);
                }

                height -= consumed;
                min_height = min_height.min(height);
                height += produced;
            }
            AsmInstruction::Push(_) => {
                height += 1;
            }
            _ => {}
        }
    }

    // Inputs = max of pop depth and access depth
    let required_from_below = (-min_height).max(-min_access);
    let inputs = required_from_below as usize;
    let outputs = (height + required_from_below) as usize;
    (inputs, outputs)
}

/// Compute byte savings from extracting a sequence.
fn compute_savings(byte_size: usize, occurrences: usize, inputs: usize, outputs: usize) -> i32 {
    // Call site: PushLabel (3 bytes PUSH2) + JumpTo (4 bytes PUSH2+JUMP) + JUMPDEST (1) = 8
    let call_overhead = 8i32;

    // Subroutine: JUMPDEST (1) + entry SWAPs (inputs) + exit SWAPs (outputs) + JUMP (1)
    let sub_overhead = 2i32 + inputs as i32 + outputs as i32;

    // Savings = all inline removed - (one subroutine body + all call sites)
    let n = occurrences as i32;
    n * byte_size as i32 - (byte_size as i32 + sub_overhead) - n * call_overhead
}

/// Hash a sequence of instructions for deduplication.
fn hash_sequence(instructions: &[AsmInstruction]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for inst in instructions {
        inst.hash(&mut hasher);
    }
    hasher.finish()
}

/// Returns true if the instruction is a halting opcode.
fn is_halting(inst: &AsmInstruction) -> bool {
    matches!(
        inst,
        AsmInstruction::Op(
            Opcode::Return | Opcode::Revert | Opcode::Stop | Opcode::Invalid | Opcode::SelfDestruct
        )
    )
}

/// Find all candidate repeated subsequences.
fn find_candidates(
    instructions: &[AsmInstruction],
    regions: &[Region],
) -> Vec<Candidate> {
    // Hash all subsequences within regions, keyed by (hash, length)
    let mut hash_groups: HashMap<(u64, usize), Vec<usize>> = HashMap::new();

    for region in regions {
        for seq_len in MIN_SEQ_LEN..=MAX_SEQ_LEN.min(region.len) {
            for offset in 0..=(region.len - seq_len) {
                let start = region.start + offset;
                let seq = &instructions[start..start + seq_len];

                // Skip sequences ending with halting opcodes
                if is_halting(&seq[seq_len - 1]) {
                    continue;
                }

                let byte_size = compute_byte_size(seq);
                if byte_size < MIN_BYTE_SIZE {
                    continue;
                }

                let hash = hash_sequence(seq);
                hash_groups.entry((hash, seq_len)).or_default().push(start);
            }
        }
    }

    let mut candidates = Vec::new();

    for ((_hash, seq_len), positions) in &hash_groups {
        if positions.len() < MIN_OCCURRENCES {
            continue;
        }

        // Verify exact equality (handle hash collisions)
        let mut exact_groups: Vec<Vec<usize>> = Vec::new();

        for &start in positions {
            let seq = &instructions[start..start + *seq_len];
            let mut found = false;
            for group in &mut exact_groups {
                let ref_start = group[0];
                let ref_seq = &instructions[ref_start..ref_start + *seq_len];
                if seq == ref_seq {
                    group.push(start);
                    found = true;
                    break;
                }
            }
            if !found {
                exact_groups.push(vec![start]);
            }
        }

        for group in exact_groups {
            if group.len() < MIN_OCCURRENCES {
                continue;
            }

            let seq = &instructions[group[0]..group[0] + *seq_len];
            let byte_size = compute_byte_size(seq);
            let (inputs, outputs) = compute_stack_effect(seq);

            // SWAP depth limit
            if inputs > 16 || outputs > 16 {
                continue;
            }

            let savings = compute_savings(byte_size, group.len(), inputs, outputs);
            if savings <= 0 {
                continue;
            }

            candidates.push(Candidate {
                instructions: seq.to_vec(),
                positions: group,
                byte_size,
                stack_inputs: inputs,
                stack_outputs: outputs,
                savings,
            });
        }
    }

    candidates
}

/// Greedy selection: pick most profitable non-overlapping candidates.
fn select_candidates(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    // Sort by savings descending
    candidates.sort_by(|a, b| b.savings.cmp(&a.savings));

    let mut selected = Vec::new();
    let mut used_ranges: Vec<(usize, usize)> = Vec::new();

    for candidate in candidates {
        let seq_len = candidate.instructions.len();

        // Filter positions that don't overlap with already-used ranges
        let valid_positions: Vec<usize> = candidate
            .positions
            .iter()
            .filter(|&&pos| {
                let end = pos + seq_len;
                !used_ranges.iter().any(|&(us, ue)| pos < ue && end > us)
            })
            .copied()
            .collect();

        if valid_positions.len() < MIN_OCCURRENCES {
            continue;
        }

        // Recompute savings with remaining positions
        let savings = compute_savings(
            candidate.byte_size,
            valid_positions.len(),
            candidate.stack_inputs,
            candidate.stack_outputs,
        );

        if savings <= 0 {
            continue;
        }

        // Mark positions as used
        for &pos in &valid_positions {
            used_ranges.push((pos, pos + seq_len));
        }

        selected.push(Candidate {
            positions: valid_positions,
            savings,
            ..candidate
        });
    }

    selected
}

/// Rewrite instructions: replace inline occurrences with call stubs, append subroutines.
fn rewrite(
    instructions: Vec<AsmInstruction>,
    candidates: Vec<Candidate>,
) -> Vec<AsmInstruction> {
    // Build map: position -> (candidate_index, occurrence_index)
    let mut replacement_map: HashMap<usize, (usize, usize)> = HashMap::new();
    for (ci, candidate) in candidates.iter().enumerate() {
        for (oi, &pos) in candidate.positions.iter().enumerate() {
            replacement_map.insert(pos, (ci, oi));
        }
    }

    // Generate unique labels
    let sub_labels: Vec<String> = (0..candidates.len())
        .map(|i| format!("__sub_{i}"))
        .collect();

    let return_labels: Vec<Vec<String>> = candidates
        .iter()
        .enumerate()
        .map(|(ci, c)| {
            (0..c.positions.len())
                .map(|oi| format!("__sub_{ci}_ret_{oi}"))
                .collect()
        })
        .collect();

    // Rewrite main instruction stream
    let mut result = Vec::new();
    let mut skip_until = 0;

    for (i, inst) in instructions.iter().enumerate() {
        if i < skip_until {
            continue;
        }

        if let Some(&(ci, oi)) = replacement_map.get(&i) {
            let candidate = &candidates[ci];
            let seq_len = candidate.instructions.len();

            // Emit call stub: push return addr, jump to subroutine
            result.push(AsmInstruction::PushLabel(return_labels[ci][oi].clone()));
            result.push(AsmInstruction::JumpTo(sub_labels[ci].clone()));
            result.push(AsmInstruction::Label(return_labels[ci][oi].clone()));

            skip_until = i + seq_len;
        } else {
            result.push(inst.clone());
        }
    }

    // Append subroutines at the end
    for (ci, candidate) in candidates.iter().enumerate() {
        result.push(AsmInstruction::Label(sub_labels[ci].clone()));

        // Entry: move return address below inputs
        // Stack at entry: [..., input_N, ..., input_1, ret_addr]
        // Goal: [..., ret_addr, input_N, ..., input_1]
        // Method: SWAP(inputs), SWAP(inputs-1), ..., SWAP(1)
        let inputs = candidate.stack_inputs;
        if inputs > 0 {
            for n in (1..=inputs).rev() {
                result.push(AsmInstruction::Op(Opcode::swap_n(n as u8)));
            }
        }

        // Body
        result.extend(candidate.instructions.iter().cloned());

        // Exit: move return address above outputs
        // Stack after body: [..., ret_addr, out_M, ..., out_1]
        // Goal: [..., out_M, ..., out_1, ret_addr]
        // Method: SWAP(1), SWAP(2), ..., SWAP(outputs)
        let outputs = candidate.stack_outputs;
        if outputs > 0 {
            for n in 1..=outputs {
                result.push(AsmInstruction::Op(Opcode::swap_n(n as u8)));
            }
        }

        // Return
        result.push(AsmInstruction::Op(Opcode::Jump));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_straight_line_regions() {
        let instrs = vec![
            AsmInstruction::Label("start".into()),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::JumpTo("end".into()),
            AsmInstruction::Label("end".into()),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let regions = find_straight_line_regions(&instrs);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start, 1);
        assert_eq!(regions[0].len, 5);
    }

    #[test]
    fn test_compute_stack_effect_push_mstore() {
        // PUSH 0, MSTORE: consumes 1 from below (MSTORE needs 2, PUSH provides 1)
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::MStore),
        ];
        let (inputs, outputs) = compute_stack_effect(&instrs);
        assert_eq!(inputs, 1, "PUSH+MSTORE needs 1 from below");
        assert_eq!(outputs, 0, "PUSH+MSTORE produces nothing");
    }

    #[test]
    fn test_compute_stack_effect_push_sload() {
        // PUSH 0, SLOAD: consumes 0, produces 1
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::SLoad),
        ];
        let (inputs, outputs) = compute_stack_effect(&instrs);
        assert_eq!(inputs, 0);
        assert_eq!(outputs, 1);
    }

    #[test]
    fn test_compute_stack_effect_dup_access() {
        // DUP3, ADD: DUP3 reads 3 deep (needs 3 inputs), ADD pops 2 pushes 1
        let instrs = vec![
            AsmInstruction::Op(Opcode::Dup3),
            AsmInstruction::Op(Opcode::Add),
        ];
        let (inputs, outputs) = compute_stack_effect(&instrs);
        assert_eq!(inputs, 3, "DUP3 reads 3 deep");
        // After DUP3: height = 1 (duped item). After ADD: height = 0 (consumed dup + 1 from below)
        // But DUP3 read 3 deep, so inputs = 3. outputs = height + inputs = 0 + 3 = 3
        // Wait, let me trace: height=0, DUP3 reads (0-3)=-3 → min_access=-3, height becomes 1.
        // ADD: consumed=2, deepest_read=1-2=-1 → min_access still -3. height=1-2=-1, min_height=-1, height=-1+1=0.
        // inputs = max(1, 3) = 3, outputs = 0 + 3 = 3
        assert_eq!(outputs, 3);
    }

    #[test]
    fn test_compute_savings() {
        // 3 occurrences, 20 byte body, 0 inputs, 0 outputs
        // savings = 3*20 - (20 + 2) - 3*8 = 60 - 22 - 24 = 14
        assert_eq!(compute_savings(20, 3, 0, 0), 14);
    }

    #[test]
    fn test_no_extraction_below_threshold() {
        // Only 2 occurrences — below MIN_OCCURRENCES (3)
        let seq = vec![
            AsmInstruction::Push(vec![0x00]),
            AsmInstruction::Op(Opcode::MStore),
            AsmInstruction::Push(vec![0x01]),
            AsmInstruction::Push(vec![0x20]),
            AsmInstruction::Op(Opcode::MStore),
            AsmInstruction::Push(vec![0x40]),
            AsmInstruction::Push(vec![0x00]),
            AsmInstruction::Op(Opcode::Keccak256),
        ];
        let mut instrs = Vec::new();
        // Only 2 copies — not enough
        instrs.extend(seq.clone());
        instrs.push(AsmInstruction::Label("mid".into()));
        instrs.extend(seq);

        let result = extract_subroutines(instrs.clone());
        assert_eq!(result, instrs, "Should not extract with only 2 occurrences");
    }

    #[test]
    fn test_extraction_with_three_occurrences() {
        // Build a sequence that's long enough (>15 bytes) and repeat 3 times
        let seq: Vec<AsmInstruction> = vec![
            AsmInstruction::Push(vec![0x00]),                     // 2 bytes
            AsmInstruction::Push(vec![0x00]),                     // 2 bytes
            AsmInstruction::Op(Opcode::MStore),                   // 1 byte
            AsmInstruction::Push(vec![0x01]),                     // 2 bytes
            AsmInstruction::Push(vec![0x20]),                     // 2 bytes
            AsmInstruction::Op(Opcode::MStore),                   // 1 byte
            AsmInstruction::Push(vec![0x40]),                     // 2 bytes
            AsmInstruction::Push(vec![0x00]),                     // 2 bytes
            AsmInstruction::Op(Opcode::Keccak256),                // 1 byte = 15 bytes
        ];

        let mut instrs = Vec::new();
        instrs.extend(seq.clone());
        instrs.push(AsmInstruction::Label("a".into()));
        instrs.extend(seq.clone());
        instrs.push(AsmInstruction::Label("b".into()));
        instrs.extend(seq);

        let result = extract_subroutines(instrs.clone());

        // Should have subroutine labels
        let has_sub_label = result.iter().any(|i| matches!(i, AsmInstruction::Label(l) if l.starts_with("__sub_")));
        assert!(has_sub_label, "Should have subroutine labels after extraction");

        // Should have PushLabel for return addresses
        let has_push_label = result.iter().any(|i| matches!(i, AsmInstruction::PushLabel(_)));
        assert!(has_push_label, "Should have PushLabel for return addresses");
    }
}
