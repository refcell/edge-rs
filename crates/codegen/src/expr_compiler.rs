//! IR expression to EVM opcode compilation.
//!
//! Walks the `EvmExpr` tree and emits EVM opcodes into an `Assembler`.
//! Since the EVM is a stack machine, we compile in postorder: children
//! first, then the operator.

use std::collections::{HashMap, HashSet};

use edge_ir::schema::{
    EvmBinaryOp, EvmConstant, EvmEnvOp, EvmExpr, EvmTernaryOp, EvmUnaryOp, RcExpr,
};

use crate::{
    assembler::{AsmInstruction, Assembler},
    opcode::Opcode,
};

/// Base memory offset for LetBind scratch storage.
const LET_BIND_BASE_OFFSET: usize = 0x80;

/// Compiles IR expressions into EVM assembly instructions.
#[derive(Debug)]
pub struct ExprCompiler<'a> {
    /// The assembler to emit instructions into
    asm: &'a mut Assembler,
    /// Maps LetBind names to their memory offsets
    let_bindings: HashMap<String, usize>,
    /// Next available memory offset for LetBind storage
    next_let_offset: usize,
    /// Variables kept on stack (DUP instead of MLOAD)
    stack_bound_vars: HashSet<String>,
}

impl<'a> ExprCompiler<'a> {
    /// Create a new expression compiler targeting the given assembler.
    pub fn new(asm: &'a mut Assembler) -> Self {
        Self {
            asm,
            let_bindings: HashMap::new(),
            next_let_offset: LET_BIND_BASE_OFFSET,
            stack_bound_vars: HashSet::new(),
        }
    }

    /// Compile an IR expression, pushing its result onto the stack.
    pub fn compile_expr(&mut self, expr: &EvmExpr) {
        match expr {
            EvmExpr::Const(c, _, _) => self.compile_const(c),

            EvmExpr::Arg(_, _) => {
                // Function argument is already on the stack at entry.
                // In the dispatcher, we decode it from calldata.
                // For now, this is a no-op placeholder.
            }

            EvmExpr::Empty(_, _) => {
                // Empty tuple / unit — no value on stack
            }

            EvmExpr::Bop(op, lhs, rhs) => self.compile_binary_op(op, lhs, rhs),
            EvmExpr::Uop(op, expr) => self.compile_unary_op(op, expr),
            EvmExpr::Top(op, a, b, c) => self.compile_ternary_op(op, a, b, c),

            EvmExpr::Get(tuple, idx) => {
                self.compile_expr(tuple);
                let n = Self::count_stack_values(tuple);
                if n > 1 {
                    // Stack layout after compiling tuple: [e_0 ... e_{n-1}]
                    // e_{n-1} is on top, e_0 is deepest.
                    // Element idx is at depth (n - 1 - idx) from top.
                    let depth = n - 1 - idx;
                    if depth > 0 && depth <= 16 {
                        // SWAP the desired element to the top
                        self.asm.emit_op(Opcode::swap_n(depth as u8));
                    }
                    // POP the remaining n-1 values below
                    for _ in 0..(n - 1) {
                        self.asm.emit_op(Opcode::Pop);
                    }
                }
                // If n <= 1, the single value is already on top (no-op)
            }

            EvmExpr::Single(inner) => {
                self.compile_expr(inner);
            }

            EvmExpr::Concat(a, b) => {
                self.compile_expr(a);
                self.compile_expr(b);
            }

            EvmExpr::If(cond, _inputs, then_body, else_body) => {
                self.compile_if(cond, then_body, else_body);
            }

            EvmExpr::DoWhile(inputs, pred_and_body) => {
                self.compile_do_while(inputs, pred_and_body);
            }

            EvmExpr::EnvRead(op, _state) => self.compile_env_read(op),
            EvmExpr::EnvRead1(op, arg, _state) => self.compile_env_read1(op, arg),

            EvmExpr::Log(topic_count, topics, data, _state) => {
                self.compile_log(*topic_count, topics, data);
            }

            EvmExpr::Revert(offset, size, _state) => {
                self.compile_expr(size);
                self.compile_expr(offset);
                self.asm.emit_op(Opcode::Revert);
            }

            EvmExpr::ReturnOp(offset, size, _state) => {
                self.compile_expr(size);
                self.compile_expr(offset);
                self.asm.emit_op(Opcode::Return);
            }

            EvmExpr::ExtCall(target, value, args_offset, args_len, ret_offset, ret_len, _state) => {
                // CALL: gas, addr, value, argsOffset, argsLength, retOffset, retLength
                self.compile_expr(ret_len);
                self.compile_expr(ret_offset);
                self.compile_expr(args_len);
                self.compile_expr(args_offset);
                self.compile_expr(value);
                self.compile_expr(target);
                self.asm.emit_op(Opcode::Gas); // forward all gas
                self.asm.emit_op(Opcode::Call);
            }

            EvmExpr::Call(name, args) => {
                self.compile_expr(args);
                let label = format!("fn_{name}");
                let ret_label = self.asm.fresh_label(&format!("ret_{name}"));
                // Push return address, jump to function
                // The function will JUMP back to ret_label
                self.asm.emit(AsmInstruction::JumpTo(label));
                self.asm.emit(AsmInstruction::Label(ret_label));
            }

            EvmExpr::Selector(sig) => {
                // Compute keccak256 of the signature and take top 4 bytes
                let mut hash = [0u8; 32];
                edge_types::bytes::hash_bytes(&mut hash, &sig.to_owned());
                let selector = &hash[..4];
                self.asm.emit(AsmInstruction::Push(selector.to_vec()));
            }

            EvmExpr::LetBind(name, value, body) => {
                // Check if we can keep this value on the stack instead of
                // spilling to memory. This is safe when the body is a
                // terminating if-chain (each then-branch ends with
                // RETURN/REVERT/STOP) — the value stays at TOS through
                // the entire else-chain, and we just DUP1 before each use.
                if Self::is_terminating_if_chain(body) {
                    // Stack mode: compile value, leave on stack
                    self.compile_expr(value);
                    self.stack_bound_vars.insert(name.clone());
                    self.compile_expr(body);
                    self.stack_bound_vars.remove(name);
                    // The last else-branch (revert/stop) terminates, so
                    // we don't need to POP. But if somehow we fall through,
                    // clean up:
                    if !Self::is_terminating_expr(body) {
                        self.asm.emit_op(Opcode::Pop);
                    }
                } else {
                    // Memory mode: compile value onto stack, spill to memory
                    self.compile_expr(value);
                    let offset = self.next_let_offset;
                    self.next_let_offset += 32;
                    self.asm.emit_push_usize(offset);
                    self.asm.emit_op(Opcode::MStore);
                    let prev = self.let_bindings.insert(name.clone(), offset);
                    self.compile_expr(body);
                    if let Some(prev_offset) = prev {
                        self.let_bindings.insert(name.clone(), prev_offset);
                    } else {
                        self.let_bindings.remove(name);
                    }
                    self.next_let_offset -= 32;
                }
            }

            EvmExpr::Var(name) => {
                if self.stack_bound_vars.contains(name.as_str()) {
                    // Stack mode: value is at TOS, duplicate it
                    self.asm.emit_op(Opcode::Dup1);
                } else {
                    // Memory mode: load from the bound memory offset
                    let offset = self.let_bindings[name];
                    self.asm.emit_push_usize(offset);
                    self.asm.emit_op(Opcode::MLoad);
                }
            }

            EvmExpr::Function(name, _in_ty, _out_ty, body) => {
                let label = format!("fn_{name}");
                self.asm.emit(AsmInstruction::Label(label));
                self.compile_expr(body);
            }

            EvmExpr::StorageField(_, _, _) => {
                // Storage field declarations don't emit code
            }
        }
    }

    /// Compile a constant value.
    fn compile_const(&mut self, c: &EvmConstant) {
        match c {
            EvmConstant::SmallInt(0) => {
                self.asm.emit_op(Opcode::Push0);
            }
            EvmConstant::SmallInt(n) => {
                let val = *n;
                if val < 0 {
                    // Negative values: two's complement
                    // For simplicity, push the absolute value and negate
                    let abs_val = val.unsigned_abs();
                    let bytes = minimal_be_bytes_u64(abs_val);
                    self.asm.emit(AsmInstruction::Push(bytes));
                    // 0 - x = negate
                    self.asm.emit_op(Opcode::Push0);
                    self.asm.emit_op(Opcode::Sub);
                } else {
                    let bytes = minimal_be_bytes_u64(val as u64);
                    self.asm.emit(AsmInstruction::Push(bytes));
                }
            }
            EvmConstant::LargeInt(hex_str) => {
                let bytes = hex_string_to_bytes(hex_str);
                if bytes.is_empty() || bytes.iter().all(|&b| b == 0) {
                    self.asm.emit_op(Opcode::Push0);
                } else {
                    // Strip leading zeros
                    let start = bytes.iter().position(|&b| b != 0).unwrap_or(0);
                    self.asm.emit(AsmInstruction::Push(bytes[start..].to_vec()));
                }
            }
            EvmConstant::Bool(true) => {
                self.asm.emit(AsmInstruction::Push(vec![1]));
            }
            EvmConstant::Bool(false) => {
                self.asm.emit_op(Opcode::Push0);
            }
            EvmConstant::Addr(hex_str) => {
                let bytes = hex_string_to_bytes(hex_str);
                // Addresses are 20 bytes
                if bytes.len() > 20 {
                    self.asm
                        .emit(AsmInstruction::Push(bytes[bytes.len() - 20..].to_vec()));
                } else {
                    self.asm.emit(AsmInstruction::Push(bytes));
                }
            }
        }
    }

    /// Compile a binary operation.
    fn compile_binary_op(&mut self, op: &EvmBinaryOp, lhs: &RcExpr, rhs: &RcExpr) {
        match op {
            // Arithmetic & comparison: push operands, then opcode
            // EVM order: for ADD, stack top = a, next = b, result = a + b
            // But operand order varies by opcode. Most are commutative,
            // but SUB, DIV, etc. care about order.
            EvmBinaryOp::Add => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Add);
            }
            EvmBinaryOp::Sub => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Sub);
            }
            EvmBinaryOp::Mul => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Mul);
            }
            EvmBinaryOp::Div => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Div);
            }
            EvmBinaryOp::SDiv => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SDiv);
            }
            EvmBinaryOp::Mod => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Mod);
            }
            EvmBinaryOp::SMod => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SMod);
            }
            EvmBinaryOp::Exp => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Exp);
            }
            EvmBinaryOp::Lt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Lt);
            }
            EvmBinaryOp::Gt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Gt);
            }
            EvmBinaryOp::SLt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SLt);
            }
            EvmBinaryOp::SGt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SGt);
            }
            EvmBinaryOp::Eq => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Eq);
            }
            EvmBinaryOp::And => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::And);
            }
            EvmBinaryOp::Or => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Or);
            }
            EvmBinaryOp::Xor => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Xor);
            }
            EvmBinaryOp::Shl => {
                // EVM SHL: shift, value -> result
                self.compile_expr(rhs); // value
                self.compile_expr(lhs); // shift amount
                self.asm.emit_op(Opcode::Shl);
            }
            EvmBinaryOp::Shr => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Shr);
            }
            EvmBinaryOp::Sar => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Sar);
            }
            EvmBinaryOp::Byte => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Byte);
            }
            EvmBinaryOp::LogAnd => {
                // Short-circuit AND: if lhs is false, skip rhs
                let skip_label = self.asm.fresh_label("logand_skip");
                let end_label = self.asm.fresh_label("logand_end");
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Dup1);
                self.asm.emit_op(Opcode::IsZero);
                self.asm.emit(AsmInstruction::JumpITo(skip_label.clone()));
                self.asm.emit_op(Opcode::Pop); // pop lhs
                self.compile_expr(rhs);
                self.asm.emit(AsmInstruction::JumpTo(end_label.clone()));
                self.asm.emit(AsmInstruction::Label(skip_label));
                // lhs (false) is already on stack
                self.asm.emit(AsmInstruction::Label(end_label));
            }
            EvmBinaryOp::LogOr => {
                // Short-circuit OR: if lhs is true, skip rhs
                let skip_label = self.asm.fresh_label("logor_skip");
                let end_label = self.asm.fresh_label("logor_end");
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Dup1);
                self.asm.emit(AsmInstruction::JumpITo(skip_label.clone()));
                self.asm.emit_op(Opcode::Pop); // pop lhs
                self.compile_expr(rhs);
                self.asm.emit(AsmInstruction::JumpTo(end_label.clone()));
                self.asm.emit(AsmInstruction::Label(skip_label));
                // lhs (true) is already on stack
                self.asm.emit(AsmInstruction::Label(end_label));
            }
            EvmBinaryOp::SLoad => {
                // slot is lhs, state is rhs (state is implicit in codegen)
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SLoad);
            }
            EvmBinaryOp::TLoad => {
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::TLoad);
            }
            EvmBinaryOp::MLoad => {
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::MLoad);
            }
            EvmBinaryOp::CalldataLoad => {
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::CallDataLoad);
            }
        }
    }

    /// Compile a unary operation.
    fn compile_unary_op(&mut self, op: &EvmUnaryOp, expr: &RcExpr) {
        self.compile_expr(expr);
        match op {
            EvmUnaryOp::IsZero => self.asm.emit_op(Opcode::IsZero),
            EvmUnaryOp::Not => self.asm.emit_op(Opcode::Not),
            EvmUnaryOp::Neg => {
                // 0 - x
                self.asm.emit_op(Opcode::Push0);
                self.asm.emit_op(Opcode::Sub);
            }
            EvmUnaryOp::SignExtend => self.asm.emit_op(Opcode::SignExtend),
        }
    }

    /// Compile a ternary operation.
    fn compile_ternary_op(&mut self, op: &EvmTernaryOp, a: &RcExpr, b: &RcExpr, c: &RcExpr) {
        match op {
            EvmTernaryOp::SStore => {
                // SSTORE: key, value
                self.compile_expr(b); // value
                self.compile_expr(a); // key (slot)
                self.asm.emit_op(Opcode::SStore);
            }
            EvmTernaryOp::TStore => {
                self.compile_expr(b); // value
                self.compile_expr(a); // key
                self.asm.emit_op(Opcode::TStore);
            }
            EvmTernaryOp::MStore => {
                self.compile_expr(b); // value
                self.compile_expr(a); // offset
                self.asm.emit_op(Opcode::MStore);
            }
            EvmTernaryOp::MStore8 => {
                self.compile_expr(b); // value
                self.compile_expr(a); // offset
                self.asm.emit_op(Opcode::MStore8);
            }
            EvmTernaryOp::Keccak256 => {
                // Keccak256(offset, size, state) -> hash
                // a = offset, b = size, c = state (ignored — memory already written)
                self.compile_expr(b); // size
                self.compile_expr(a); // offset
                self.asm.emit_op(Opcode::Keccak256);
            }
            EvmTernaryOp::Select => {
                // Select(cond, true_val, false_val)
                // Implemented as: if cond then true_val else false_val
                let else_label = self.asm.fresh_label("select_else");
                let end_label = self.asm.fresh_label("select_end");
                self.compile_expr(a); // cond
                self.asm.emit_op(Opcode::IsZero);
                self.asm.emit(AsmInstruction::JumpITo(else_label.clone()));
                self.compile_expr(b); // true value
                self.asm.emit(AsmInstruction::JumpTo(end_label.clone()));
                self.asm.emit(AsmInstruction::Label(else_label));
                self.compile_expr(c); // false value
                self.asm.emit(AsmInstruction::Label(end_label));
            }
        }
        // State operand (c for SStore/TStore/MStore, ignored here as state is implicit)
        let _ = c;
    }

    /// Compile an if-then-else expression.
    fn compile_if(&mut self, cond: &RcExpr, then_body: &RcExpr, else_body: &RcExpr) {
        let else_label = self.asm.fresh_label("else");
        let end_label = self.asm.fresh_label("endif");

        self.compile_expr(cond);
        self.asm.emit_op(Opcode::IsZero);
        self.asm.emit(AsmInstruction::JumpITo(else_label.clone()));

        self.compile_expr(then_body);
        self.asm.emit(AsmInstruction::JumpTo(end_label.clone()));

        self.asm.emit(AsmInstruction::Label(else_label));
        self.compile_expr(else_body);

        self.asm.emit(AsmInstruction::Label(end_label));
    }

    /// Compile a do-while loop.
    fn compile_do_while(&mut self, inputs: &RcExpr, pred_and_body: &RcExpr) {
        let loop_label = self.asm.fresh_label("loop");

        self.compile_expr(inputs);
        self.asm.emit(AsmInstruction::Label(loop_label.clone()));
        self.compile_expr(pred_and_body);
        // The predicate should be on top of stack
        self.asm.emit(AsmInstruction::JumpITo(loop_label));
    }

    /// Compile a nullary environment read.
    fn compile_env_read(&mut self, op: &EvmEnvOp) {
        let opcode = match op {
            EvmEnvOp::Caller => Opcode::Caller,
            EvmEnvOp::CallValue => Opcode::CallValue,
            EvmEnvOp::CallDataSize => Opcode::CallDataSize,
            EvmEnvOp::Origin => Opcode::Origin,
            EvmEnvOp::GasPrice => Opcode::GasPrice,
            EvmEnvOp::Coinbase => Opcode::Coinbase,
            EvmEnvOp::Timestamp => Opcode::Timestamp,
            EvmEnvOp::Number => Opcode::Number,
            EvmEnvOp::GasLimit => Opcode::GasLimit,
            EvmEnvOp::ChainId => Opcode::ChainId,
            EvmEnvOp::SelfBalance => Opcode::SelfBalance,
            EvmEnvOp::BaseFee => Opcode::BaseFee,
            EvmEnvOp::Gas => Opcode::Gas,
            EvmEnvOp::Address => Opcode::Address,
            EvmEnvOp::CodeSize => Opcode::CodeSize,
            EvmEnvOp::ReturnDataSize => Opcode::ReturnDataSize,
            EvmEnvOp::BlockHash | EvmEnvOp::Balance => {
                // These are unary, shouldn't be called as nullary
                // Emit anyway for robustness
                Opcode::Invalid
            }
        };
        self.asm.emit_op(opcode);
    }

    /// Compile a unary environment read.
    fn compile_env_read1(&mut self, op: &EvmEnvOp, arg: &RcExpr) {
        self.compile_expr(arg);
        let opcode = match op {
            EvmEnvOp::Balance => Opcode::Balance,
            EvmEnvOp::BlockHash => Opcode::BlockHash,
            _ => {
                // Other env ops are nullary; compile as such
                self.compile_env_read(op);
                return;
            }
        };
        self.asm.emit_op(opcode);
    }

    /// Compile a LOG instruction.
    ///
    /// The data argument is `Concat(offset, size)` — both are compiled to
    /// produce the memory range for the log data. The caller is responsible
    /// for having already MSTORE'd the data into memory (via Concat side effects).
    fn compile_log(&mut self, topic_count: usize, topics: &[RcExpr], data: &RcExpr) {
        // EVM LOGn pops from the stack top in order: offset, size, topic0, topic1, ...topicN
        // So we push in reverse: topicN first (deepest), then ... topic0, size, offset (top).

        // Push topics in reverse order (last topic is deepest in the stack)
        for topic in topics.iter().rev() {
            self.compile_expr(topic);
        }

        // Push data offset and size (offset ends up on top)
        match data.as_ref() {
            EvmExpr::Concat(offset, size) => {
                self.compile_expr(size);
                self.compile_expr(offset);
            }
            _ => {
                // Fallback: single expr is size=32, data was stored at offset 0
                self.compile_expr(data);
                self.asm.emit_op(Opcode::Push0);
                self.asm.emit_op(Opcode::MStore);
                self.asm.emit_push_usize(32);
                self.asm.emit_op(Opcode::Push0);
            }
        }

        self.asm.emit_op(Opcode::log_n(topic_count as u8));
    }

    /// Check if an expression is a chain of If nodes where every then-branch
    /// terminates (RETURN/REVERT/STOP). This pattern is used by the dispatcher:
    /// ```text
    /// If(cond1, _, body1_terminates, If(cond2, _, body2_terminates, ... revert))
    /// ```
    /// When this is true, a LetBind value on the stack stays at TOS through
    /// the entire else-chain — each then-branch pops it (or terminates before
    /// needing to), so we can use DUP1 for each Var reference instead of
    /// MSTORE/MLOAD.
    fn is_terminating_if_chain(expr: &EvmExpr) -> bool {
        match expr {
            EvmExpr::If(_cond, _inputs, then_body, else_body) => {
                // The then-branch must terminate (so we don't fall through
                // with a dirty stack)
                if !Self::is_terminating_expr(then_body) {
                    return false;
                }
                // The else-branch must either be another terminating if-chain
                // or itself terminate
                Self::is_terminating_if_chain(else_body) || Self::is_terminating_expr(else_body)
            }
            // A single terminating expression (the final else: revert/stop)
            _ => Self::is_terminating_expr(expr),
        }
    }

    /// Check if an expression unconditionally terminates execution
    /// (RETURN, REVERT, STOP, INVALID). These never fall through.
    fn is_terminating_expr(expr: &EvmExpr) -> bool {
        match expr {
            EvmExpr::ReturnOp(_, _, _) => true,
            EvmExpr::Revert(_, _, _) => true,
            // Concat: check if the last expression terminates
            // (common pattern: side effects concat'd with a terminator)
            EvmExpr::Concat(_, b) => Self::is_terminating_expr(b),
            // If both branches terminate, the whole thing terminates
            EvmExpr::If(_, _, then_body, else_body) => {
                Self::is_terminating_expr(then_body) && Self::is_terminating_expr(else_body)
            }
            // LetBind terminates if its body terminates
            EvmExpr::LetBind(_, _, body) => Self::is_terminating_expr(body),
            _ => false,
        }
    }

    /// Estimate how many stack values an expression pushes.
    ///
    /// For `Single(x)` → 1, `Concat(a, b)` → count(a) + count(b),
    /// and most other expressions push exactly 1 value.
    fn count_stack_values(expr: &EvmExpr) -> usize {
        match expr {
            EvmExpr::Concat(a, b) => Self::count_stack_values(a) + Self::count_stack_values(b),
            EvmExpr::Single(inner) => Self::count_stack_values(inner),
            EvmExpr::Empty(_, _) => 0,
            EvmExpr::LetBind(_, _, body) => Self::count_stack_values(body),
            EvmExpr::Var(_) => 1,
            _ => 1,
        }
    }
}

/// Convert a u64 to minimal big-endian bytes.
fn minimal_be_bytes_u64(val: u64) -> Vec<u8> {
    if val == 0 {
        return vec![0];
    }
    let bytes = val.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[start..].to_vec()
}

/// Convert a hex string (without 0x prefix) to bytes.
fn hex_string_to_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    // Pad to even length
    let hex = if hex.len() % 2 != 0 {
        format!("0{hex}")
    } else {
        hex.to_owned()
    };
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_be_bytes() {
        assert_eq!(minimal_be_bytes_u64(0), vec![0]);
        assert_eq!(minimal_be_bytes_u64(1), vec![1]);
        assert_eq!(minimal_be_bytes_u64(255), vec![255]);
        assert_eq!(minimal_be_bytes_u64(256), vec![1, 0]);
        assert_eq!(minimal_be_bytes_u64(65535), vec![255, 255]);
    }

    #[test]
    fn test_hex_string_to_bytes() {
        assert_eq!(hex_string_to_bytes("ff"), vec![0xFF]);
        assert_eq!(hex_string_to_bytes("0xff"), vec![0xFF]);
        assert_eq!(hex_string_to_bytes("0100"), vec![0x01, 0x00]);
        assert_eq!(hex_string_to_bytes("a"), vec![0x0A]);
    }
}
