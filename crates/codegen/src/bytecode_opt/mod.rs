//! Stage 2 bytecode-level peephole optimizer using egglog equality saturation.
//!
//! Splits codegen output into basic blocks, optimizes each block body through
//! egglog, and reassembles. Operates on `AsmInstruction` sequences.

mod convert;
mod costs;
mod rules;
mod schema;
mod schedule;

use edge_ir::OptimizeFor;

use crate::{assembler::AsmInstruction, opcode::Opcode};

/// Minimum block body size to bother optimizing (egglog overhead > benefit for tiny blocks).
const MIN_BLOCK_SIZE: usize = 3;

/// A basic block: optional label, body of ops/pushes, optional terminator jump.
struct BasicBlock {
    /// Entry label (if any).
    label: Option<AsmInstruction>,
    /// Instructions in the block body (Op and Push only).
    body: Vec<AsmInstruction>,
    /// Terminator instruction (JumpTo/JumpITo, or None for fallthrough/end).
    terminator: Option<AsmInstruction>,
}

/// Optimize a sequence of `AsmInstruction`s at the given optimization level.
///
/// - Level 0: no optimization (passthrough).
/// - Level 1: peepholes + dead push removal.
/// - Level 2: + constant folding + strength reduction.
/// - Level 3+: aggressive iteration.
///
/// `optimize_for` controls the cost model used for extraction:
/// - `Gas`: minimize estimated EVM execution gas.
/// - `Size`: minimize bytecode byte-size.
pub fn optimize(
    instructions: Vec<AsmInstruction>,
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<Vec<AsmInstruction>, crate::CodegenError> {
    if optimization_level == 0 {
        return Ok(instructions);
    }

    let schedule = match schedule::schedule_for_level(optimization_level) {
        Some(s) => s,
        None => return Ok(instructions),
    };

    // Pre-pass: eliminate dead code after RETURN/REVERT/STOP/INVALID
    let instructions = eliminate_dead_code(instructions);

    let schema = schema::generate_schema(optimize_for);
    let blocks = split_into_basic_blocks(instructions);
    let mut result = Vec::new();

    for block in blocks {
        if let Some(label) = block.label {
            result.push(label);
        }

        if block.body.len() >= MIN_BLOCK_SIZE {
            match optimize_block(&block.body, &schema, &schedule) {
                Ok(optimized) => result.extend(optimized),
                Err(_) => {
                    // Fall back to unoptimized block on egglog errors
                    tracing::warn!("bytecode optimizer: egglog error, falling back to unoptimized block");
                    result.extend(block.body);
                }
            }
        } else {
            result.extend(block.body);
        }

        if let Some(term) = block.terminator {
            result.push(term);
        }
    }

    // Post-pass: remove consecutive labels (redundant JUMPDESTs)
    let result = remove_consecutive_labels(result);

    Ok(result)
}

/// Eliminate unreachable instructions after RETURN/REVERT/STOP/INVALID.
///
/// Any Op/Push instructions between a terminating opcode and the next Label
/// are dead code and can be removed. Jump instructions are also dead if they
/// follow a terminator (they can't be reached).
fn eliminate_dead_code(instructions: Vec<AsmInstruction>) -> Vec<AsmInstruction> {
    let mut result = Vec::with_capacity(instructions.len());
    let mut dead = false;

    for inst in instructions {
        match &inst {
            AsmInstruction::Label(_) => {
                // Labels are always reachable (could be a jump target)
                dead = false;
                result.push(inst);
            }
            _ if dead => {
                // Skip dead instructions (between terminator and next label)
            }
            AsmInstruction::Op(op) if is_terminating_opcode(*op) => {
                result.push(inst);
                dead = true;
            }
            _ => {
                result.push(inst);
            }
        }
    }

    result
}

/// Returns true if this opcode unconditionally terminates execution
/// (no fallthrough to the next instruction).
fn is_terminating_opcode(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::Return | Opcode::Revert | Opcode::Stop | Opcode::Invalid | Opcode::SelfDestruct
    )
}

/// Remove consecutive Label instructions (redundant JUMPDESTs).
///
/// When Label(a) is immediately followed by Label(b) with no instructions
/// between them, alias a→b: rewrite all JumpTo(a)/JumpITo(a) to reference b
/// instead, then remove Label(a). This saves 1 byte per removed JUMPDEST.
///
/// Handles chains: Label(a), Label(b), Label(c) → keep only Label(c),
/// alias a→c and b→c.
fn remove_consecutive_labels(instructions: Vec<AsmInstruction>) -> Vec<AsmInstruction> {
    if instructions.is_empty() {
        return instructions;
    }

    // 1. Build alias map: for each label that's immediately followed by
    //    another label, map it to the final label in the chain.
    let mut aliases: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut i = 0;
    while i < instructions.len() {
        if let AsmInstruction::Label(_) = &instructions[i] {
            // Find the end of a chain of consecutive labels
            let mut j = i + 1;
            while j < instructions.len() {
                if let AsmInstruction::Label(_) = &instructions[j] {
                    j += 1;
                } else {
                    break;
                }
            }
            // If there are multiple labels in a row, alias all but the last
            if j > i + 1 {
                let last_label = if let AsmInstruction::Label(last) = &instructions[j - 1] {
                    last.clone()
                } else {
                    unreachable!()
                };
                for k in i..j - 1 {
                    if let AsmInstruction::Label(label) = &instructions[k] {
                        aliases.insert(label.clone(), last_label.clone());
                    }
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }

    if aliases.is_empty() {
        return instructions;
    }

    // 2. Rewrite jump targets and remove aliased labels
    instructions
        .into_iter()
        .filter_map(|inst| match inst {
            AsmInstruction::Label(ref label) if aliases.contains_key(label) => None,
            AsmInstruction::JumpTo(label) => {
                let target = aliases.get(&label).cloned().unwrap_or(label);
                Some(AsmInstruction::JumpTo(target))
            }
            AsmInstruction::JumpITo(label) => {
                let target = aliases.get(&label).cloned().unwrap_or(label);
                Some(AsmInstruction::JumpITo(target))
            }
            other => Some(other),
        })
        .collect()
}

/// Split instructions into basic blocks at label and jump boundaries.
fn split_into_basic_blocks(instructions: Vec<AsmInstruction>) -> Vec<BasicBlock> {
    let mut blocks = Vec::new();
    let mut current_label: Option<AsmInstruction> = None;
    let mut current_body: Vec<AsmInstruction> = Vec::new();

    for inst in instructions {
        match &inst {
            AsmInstruction::Label(_) => {
                // A label starts a new block. Flush the current block first.
                if current_label.is_some() || !current_body.is_empty() {
                    blocks.push(BasicBlock {
                        label: current_label.take(),
                        body: std::mem::take(&mut current_body),
                        terminator: None,
                    });
                }
                current_label = Some(inst);
            }
            AsmInstruction::JumpTo(_) | AsmInstruction::JumpITo(_) => {
                // Jump terminates the current block.
                blocks.push(BasicBlock {
                    label: current_label.take(),
                    body: std::mem::take(&mut current_body),
                    terminator: Some(inst),
                });
            }
            AsmInstruction::Op(_) | AsmInstruction::Push(_)
            | AsmInstruction::PushLabel(_) => {
                current_body.push(inst);
            }
        }
    }

    // Flush remaining instructions
    if current_label.is_some() || !current_body.is_empty() {
        blocks.push(BasicBlock {
            label: current_label,
            body: current_body,
            terminator: None,
        });
    }

    blocks
}

/// Run egglog optimization on a single basic block body.
fn optimize_block(
    body: &[AsmInstruction],
    schema: &str,
    schedule: &str,
) -> Result<Vec<AsmInstruction>, crate::CodegenError> {
    let input_sexp = convert::instructions_to_sexp(body);

    // Build the full egglog program
    let program = format!(
        "{schema}\n{dup_dedup}\n{cancel}\n{const_fold}\n{strength}\n{mod_and}\n{dead_push}\n\
         (let block {input_sexp})\n\
         {schedule}\n\
         (extract block)\n",
        dup_dedup = rules::DUP_DEDUP_RULES,
        cancel = rules::CANCELLATION_RULES,
        const_fold = rules::CONST_FOLD_RULES,
        strength = rules::STRENGTH_REDUCTION_RULES,
        mod_and = rules::MOD_TO_AND_RULES,
        dead_push = rules::DEAD_PUSH_RULES,
    );

    let mut egraph = egglog::EGraph::default();
    match egraph.parse_and_run_program(None, &program) {
        Ok(outputs) => {
            // The last output should be the extracted expression
            if let Some(output) = outputs.last() {
                let extracted = output.to_string();
                Ok(convert::sexp_to_instructions(&extracted))
            } else {
                // No output — return original
                Ok(body.to_vec())
            }
        }
        Err(e) => {
            tracing::warn!("bytecode optimizer egglog error: {e}");
            Err(crate::CodegenError::Internal(format!(
                "bytecode optimizer: {e}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::Opcode;

    #[test]
    fn test_split_basic_blocks() {
        let instrs = vec![
            AsmInstruction::Push(vec![0x01]),
            AsmInstruction::Op(Opcode::Add),
            AsmInstruction::JumpTo("foo".into()),
            AsmInstruction::Label("foo".into()),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let blocks = split_into_basic_blocks(instrs);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].label.is_none());
        assert_eq!(blocks[0].body.len(), 2);
        assert!(blocks[0].terminator.is_some());
        assert!(blocks[1].label.is_some());
        assert_eq!(blocks[1].body.len(), 1);
        assert!(blocks[1].terminator.is_none());
    }

    #[test]
    fn test_optimize_passthrough_o0() {
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs.clone(), 0, OptimizeFor::Size).unwrap();
        assert_eq!(result, instrs);
    }

    #[test]
    fn test_egglog_rules_fire() {
        // Minimal test: does egglog fire a simple rewrite?
        let program = r#"
(datatype Inst (IStop :cost 1) (IPop :cost 1))
(datatype PushVal (PushSmall i64 :cost 2))
(datatype ISeq
  (INil :cost 0)
  (ICons Inst ISeq :cost 0)
  (IPushCons PushVal ISeq :cost 0))

(ruleset test-rules)
(rewrite (IPushCons (PushSmall ?x) (ICons (IPop) ?rest))
         ?rest
  :ruleset test-rules)

(let block (IPushCons (PushSmall 42) (ICons (IPop) (ICons (IStop) (INil)))))
(run-schedule (repeat 3 (run test-rules)))
(extract block)
"#;
        let mut egraph = egglog::EGraph::default();
        let outputs = egraph.parse_and_run_program(None, program).unwrap();
        let extracted = outputs.last().unwrap().to_string();
        eprintln!("Extracted: {extracted}");
        assert!(extracted.contains("IStop"), "Expected rewrite to fire, got: {extracted}");
        assert!(!extracted.contains("PushSmall"), "PushSmall 42 should be eliminated, got: {extracted}");
    }

    #[test]
    fn test_optimize_dup_dedup() {
        // PUSH0 PUSH0 → PUSH0 DUP1
        // Note: For Push0, both forms have the same cost (1+1 vs 1+1),
        // so egglog may return either. The real benefit is for PUSH(data).
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs.clone(), 1, OptimizeFor::Size).unwrap();
        // Either form is valid since cost is the same
        let alt1 = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Add),
        ];
        let alt2 = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Dup1),
            AsmInstruction::Op(Opcode::Add),
        ];
        assert!(result == alt1 || result == alt2, "Expected one of the equivalent forms, got: {result:?}");
    }

    #[test]
    fn test_optimize_dup_dedup_with_data() {
        // PUSH 0x42 PUSH 0x42 ADD → PUSH 0x42 DUP1 ADD
        // PushSmall cost = 2, DUP1 cost = 1 → saves 1 byte
        let instrs = vec![
            AsmInstruction::Push(vec![0x42]),
            AsmInstruction::Push(vec![0x42]),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Push(vec![0x42]),
                AsmInstruction::Op(Opcode::Dup1),
                AsmInstruction::Op(Opcode::Add),
            ]
        );
    }

    #[test]
    fn test_optimize_dead_push_pop() {
        // PUSH 42 POP STOP → STOP (but only 3 instructions meet MIN_BLOCK_SIZE)
        let instrs = vec![
            AsmInstruction::Push(vec![42]),
            AsmInstruction::Op(Opcode::Pop),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    #[test]
    fn test_optimize_const_fold_add() {
        // PUSH 3, PUSH 4, ADD → PUSH 7
        let instrs = vec![
            AsmInstruction::Push(vec![3]),
            AsmInstruction::Push(vec![4]),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Push(vec![7])]);
    }

    #[test]
    fn test_optimize_preserves_labels_and_jumps() {
        let instrs = vec![
            AsmInstruction::Label("start".into()),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Add),
            AsmInstruction::JumpTo("end".into()),
            AsmInstruction::Label("end".into()),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        // Labels and jumps preserved
        assert_eq!(result[0], AsmInstruction::Label("start".into()));
        assert!(matches!(result.last(), Some(AsmInstruction::Op(Opcode::Stop))));
        // The jump should still be there
        assert!(result.iter().any(|i| matches!(i, AsmInstruction::JumpTo(_))));
    }

    #[test]
    fn test_optimize_swap_cancel() {
        // SWAP1 SWAP1 ADD → ADD
        let instrs = vec![
            AsmInstruction::Op(Opcode::Swap1),
            AsmInstruction::Op(Opcode::Swap1),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Add)]);
    }

    #[test]
    fn test_optimize_push0_add_identity() {
        // PUSH0 ADD STOP → STOP (PUSH0 + ADD is additive identity)
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Add),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    // --- Gas cost optimization tests ---

    #[test]
    fn test_gas_schema_parses() {
        // Verify the gas-mode schema is valid egglog
        let schema = crate::bytecode_opt::schema::generate_schema(OptimizeFor::Gas);
        let mut egraph = egglog::EGraph::default();
        egraph.parse_and_run_program(None, &schema)
            .expect("Gas schema should parse");
    }

    #[test]
    fn test_size_schema_parses() {
        // Verify the size-mode schema is valid egglog
        let schema = crate::bytecode_opt::schema::generate_schema(OptimizeFor::Size);
        let mut egraph = egglog::EGraph::default();
        egraph.parse_and_run_program(None, &schema)
            .expect("Size schema should parse");
    }

    #[test]
    fn test_gas_dead_push_pop() {
        // Dead push elimination works in gas mode too
        let instrs = vec![
            AsmInstruction::Push(vec![42]),
            AsmInstruction::Op(Opcode::Pop),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Gas).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    #[test]
    fn test_gas_const_fold_add() {
        // Constant folding in gas mode: PUSH 3 PUSH 4 ADD → PUSH 7
        // Saves 2 gas (extra PUSH + ADD) even though both are Gverylow
        let instrs = vec![
            AsmInstruction::Push(vec![3]),
            AsmInstruction::Push(vec![4]),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Gas).unwrap();
        assert_eq!(result, vec![AsmInstruction::Push(vec![7])]);
    }

    #[test]
    fn test_gas_swap_cancel() {
        // SWAP1 SWAP1 cancellation saves 6 gas (2 × Gverylow)
        let instrs = vec![
            AsmInstruction::Op(Opcode::Swap1),
            AsmInstruction::Op(Opcode::Swap1),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Gas).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Add)]);
    }

    #[test]
    fn test_gas_dup_dedup_with_data() {
        // PUSH 0x42 PUSH 0x42 → PUSH 0x42 DUP1
        // Gas mode: PushSmall=3, DUP=3, so same gas cost — either form valid
        // Size mode: PushSmall=2, DUP=1, so DUP is preferred
        let instrs = vec![
            AsmInstruction::Push(vec![0x42]),
            AsmInstruction::Push(vec![0x42]),
            AsmInstruction::Op(Opcode::Add),
        ];

        // Size mode should prefer DUP (saves 1 byte)
        let size_result = optimize(instrs.clone(), 1, OptimizeFor::Size).unwrap();
        assert_eq!(
            size_result,
            vec![
                AsmInstruction::Push(vec![0x42]),
                AsmInstruction::Op(Opcode::Dup1),
                AsmInstruction::Op(Opcode::Add),
            ]
        );

        // Gas mode: both forms cost the same (3+3+3 vs 3+3+3), either is valid
        let gas_result = optimize(instrs, 1, OptimizeFor::Gas).unwrap();
        let alt_dup = vec![
            AsmInstruction::Push(vec![0x42]),
            AsmInstruction::Op(Opcode::Dup1),
            AsmInstruction::Op(Opcode::Add),
        ];
        let alt_push = vec![
            AsmInstruction::Push(vec![0x42]),
            AsmInstruction::Push(vec![0x42]),
            AsmInstruction::Op(Opcode::Add),
        ];
        assert!(
            gas_result == alt_dup || gas_result == alt_push,
            "Gas mode: either form valid, got: {gas_result:?}"
        );
    }

    #[test]
    fn test_gas_preserves_expensive_ops() {
        // Ensure expensive ops (SSTORE, SLOAD) aren't incorrectly removed
        let instrs = vec![
            AsmInstruction::Push(vec![0x01]),
            AsmInstruction::Push(vec![0x00]),
            AsmInstruction::Op(Opcode::SStore),
        ];
        let result = optimize(instrs.clone(), 2, OptimizeFor::Gas).unwrap();
        // SStore should still be present (no rule removes it)
        assert!(
            result.iter().any(|i| matches!(i, AsmInstruction::Op(Opcode::SStore))),
            "SStore must be preserved, got: {result:?}"
        );
    }

    // --- New optimization rule tests ---

    #[test]
    fn test_push0_or_identity() {
        // PUSH0 OR STOP → STOP (OR with 0 is identity)
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Or),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    #[test]
    fn test_push0_xor_identity() {
        // PUSH0 XOR STOP → STOP (XOR with 0 is identity)
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Xor),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    #[test]
    fn test_push0_eq_to_iszero() {
        // PUSH0 EQ STOP → ISZERO STOP
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Eq),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Op(Opcode::IsZero),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_swap1_commutative_add() {
        // SWAP1 ADD STOP → ADD STOP
        let instrs = vec![
            AsmInstruction::Op(Opcode::Swap1),
            AsmInstruction::Op(Opcode::Add),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Op(Opcode::Add),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_swap1_commutative_mul() {
        // SWAP1 MUL STOP → MUL STOP
        let instrs = vec![
            AsmInstruction::Op(Opcode::Swap1),
            AsmInstruction::Op(Opcode::Mul),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Op(Opcode::Mul),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_dup_any_n_pop_cancel() {
        // DUP3 POP STOP → STOP (generalized dup-pop cancellation)
        let instrs = vec![
            AsmInstruction::Op(Opcode::Dup3),
            AsmInstruction::Op(Opcode::Pop),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    #[test]
    fn test_div_by_1_identity() {
        // PUSH 1 DIV STOP → STOP
        let instrs = vec![
            AsmInstruction::Push(vec![1]),
            AsmInstruction::Op(Opcode::Div),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    #[test]
    fn test_mul_by_2_to_shl() {
        // PUSH 2 MUL STOP → PUSH 1 SHL STOP (strength reduction)
        let instrs = vec![
            AsmInstruction::Push(vec![2]),
            AsmInstruction::Op(Opcode::Mul),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Gas).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Push(vec![1]),
                AsmInstruction::Op(Opcode::Shl),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_div_by_2_to_shr() {
        // PUSH 2 DIV STOP → PUSH 1 SHR STOP
        let instrs = vec![
            AsmInstruction::Push(vec![2]),
            AsmInstruction::Op(Opcode::Div),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Gas).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Push(vec![1]),
                AsmInstruction::Op(Opcode::Shr),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_dup1_swap1_cancel() {
        // DUP1 SWAP1 ADD → DUP1 ADD
        let instrs = vec![
            AsmInstruction::Op(Opcode::Dup1),
            AsmInstruction::Op(Opcode::Swap1),
            AsmInstruction::Op(Opcode::Add),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Op(Opcode::Dup1),
                AsmInstruction::Op(Opcode::Add),
            ]
        );
    }

    #[test]
    fn test_dead_code_after_return() {
        // Instructions after RETURN but before a Label are dead
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Return),
            // Dead code:
            AsmInstruction::Push(vec![0xFF]),
            AsmInstruction::Op(Opcode::Pop),
            AsmInstruction::JumpTo("end".into()),
            // Reachable (has a label):
            AsmInstruction::Label("end".into()),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        // Dead code should be removed; label and stop preserved
        assert!(!result.iter().any(|i| matches!(i, AsmInstruction::Push(d) if d == &[0xFF])),
            "Dead push after RETURN should be removed, got: {result:?}");
        assert!(result.iter().any(|i| matches!(i, AsmInstruction::Op(Opcode::Return))),
            "RETURN should be preserved");
        assert!(result.iter().any(|i| matches!(i, AsmInstruction::Label(_))),
            "Label should be preserved");
    }

    #[test]
    fn test_dead_code_after_revert() {
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Revert),
            AsmInstruction::Op(Opcode::Add), // dead
            AsmInstruction::Label("next".into()),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert!(!result.iter().any(|i| matches!(i, AsmInstruction::Op(Opcode::Add))),
            "Dead ADD after REVERT should be removed");
    }

    #[test]
    fn test_label_aliasing() {
        // Label(a) Label(b) → remove Label(a), rewrite jumps to a → b
        let instrs = vec![
            AsmInstruction::JumpTo("a".into()),
            AsmInstruction::Label("a".into()),
            AsmInstruction::Label("b".into()),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::JumpTo("b".into()),
                AsmInstruction::Label("b".into()),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_label_chain_aliasing() {
        // Label(a) Label(b) Label(c) → keep only Label(c), alias a→c, b→c
        let instrs = vec![
            AsmInstruction::JumpITo("a".into()),
            AsmInstruction::JumpTo("b".into()),
            AsmInstruction::Label("a".into()),
            AsmInstruction::Label("b".into()),
            AsmInstruction::Label("c".into()),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 1, OptimizeFor::Size).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::JumpITo("c".into()),
                AsmInstruction::JumpTo("c".into()),
                AsmInstruction::Label("c".into()),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_const_fold_shl() {
        // PUSH 3 PUSH 2 SHL → PUSH 12 (3 << 2 = 12)
        let instrs = vec![
            AsmInstruction::Push(vec![3]),
            AsmInstruction::Push(vec![2]),
            AsmInstruction::Op(Opcode::Shl),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Push(vec![12])]);
    }

    #[test]
    fn test_const_fold_shr() {
        // PUSH 16 PUSH 2 SHR → PUSH 4 (16 >> 2 = 4)
        let instrs = vec![
            AsmInstruction::Push(vec![16]),
            AsmInstruction::Push(vec![2]),
            AsmInstruction::Op(Opcode::Shr),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Push(vec![4])]);
    }

    #[test]
    fn test_const_fold_eq_same() {
        // PUSH 42 PUSH 42 EQ → PUSH 1
        let instrs = vec![
            AsmInstruction::Push(vec![42]),
            AsmInstruction::Push(vec![42]),
            AsmInstruction::Op(Opcode::Eq),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        assert_eq!(result, vec![AsmInstruction::Push(vec![1])]);
    }

    #[test]
    fn test_chained_optimization() {
        // PUSH 0 PUSH 0 ADD POP STOP
        // Step 1: PUSH 0 PUSH 0 → PUSH 0 DUP1 (dup dedup)
        // Step 2: PUSH 0 + DUP1 + ADD → various
        // Step 3: result POP → eliminated if dead
        // The chain should eventually collapse
        let instrs = vec![
            AsmInstruction::Push(vec![0]),
            AsmInstruction::Push(vec![0]),
            AsmInstruction::Op(Opcode::Add),
            AsmInstruction::Op(Opcode::Pop),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Size).unwrap();
        // After const fold: PUSH 0 ADD → POP remaining, eventually STOP
        assert_eq!(result, vec![AsmInstruction::Op(Opcode::Stop)]);
    }

    #[test]
    fn test_mod_4_to_and_3() {
        // PUSH 4 MOD STOP → PUSH 3 AND STOP (MOD 5 gas → AND 3 gas)
        let instrs = vec![
            AsmInstruction::Push(vec![4]),
            AsmInstruction::Op(Opcode::Mod),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Gas).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Push(vec![3]),
                AsmInstruction::Op(Opcode::And),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_mod_256_to_and_255() {
        // PUSH 256 MOD STOP → PUSH 255 AND STOP
        let instrs = vec![
            AsmInstruction::Push(vec![1, 0]), // 256
            AsmInstruction::Op(Opcode::Mod),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Gas).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Push(vec![255]),
                AsmInstruction::Op(Opcode::And),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }

    #[test]
    fn test_mul_256_to_shl_8() {
        // PUSH 256 MUL STOP → PUSH 8 SHL STOP (for gas mode)
        let instrs = vec![
            AsmInstruction::Push(vec![1, 0]), // 256 as big-endian
            AsmInstruction::Op(Opcode::Mul),
            AsmInstruction::Op(Opcode::Stop),
        ];
        let result = optimize(instrs, 2, OptimizeFor::Gas).unwrap();
        assert_eq!(
            result,
            vec![
                AsmInstruction::Push(vec![8]),
                AsmInstruction::Op(Opcode::Shl),
                AsmInstruction::Op(Opcode::Stop),
            ]
        );
    }
}
