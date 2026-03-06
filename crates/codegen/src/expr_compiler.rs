//! IR expression to EVM opcode compilation.
//!
//! Walks the `EvmExpr` tree and emits EVM opcodes into an `Assembler`.
//! Since the EVM is a stack machine, we compile in postorder: children
//! first, then the operator.

use std::collections::HashMap;

use edge_ir::{
    schema::{
        EvmBinaryOp, EvmConstant, EvmEnvOp, EvmExpr, EvmTernaryOp, EvmType, EvmUnaryOp, RcExpr,
    },
    var_opt::{AllocationMode, VarAllocation},
};

use crate::{
    assembler::{AsmInstruction, Assembler},
    opcode::Opcode,
};

/// Base memory offset for `LetBind` scratch storage.
const LET_BIND_BASE_OFFSET: usize = 0x80;

/// Compiles IR expressions into EVM assembly instructions.
#[derive(Debug)]
pub struct ExprCompiler<'a> {
    /// The assembler to emit instructions into
    asm: &'a mut Assembler,
    /// Maps `LetBind` names to their memory offsets (memory-mode variables)
    let_bindings: HashMap<String, usize>,
    /// Next available memory offset for `LetBind` storage (high-water mark)
    next_let_offset: usize,
    /// Free-list of memory slots reclaimed by `Drop` (available for reuse)
    free_slots: Vec<usize>,
    /// Per-variable allocation info (Stack vs Memory + read count)
    allocation_modes: HashMap<String, VarAllocation>,
    /// Maps stack-allocated variable names to their stack position (depth when pushed)
    stack_vars: HashMap<String, usize>,
    /// Current EVM stack depth (number of values on the stack, tracked for DUP indexing)
    stack_depth: usize,
    /// Label for shared overflow revert trampoline (lazily created)
    overflow_revert_label: Option<String>,
    /// Inner function metadata: name -> (`param_count`, `return_count`)
    /// Populated by a pre-pass over the IR tree before compilation.
    fn_info: HashMap<String, (usize, usize)>,
}

impl<'a> ExprCompiler<'a> {
    /// Create a new expression compiler targeting the given assembler.
    pub fn new(asm: &'a mut Assembler) -> Self {
        Self::with_allocations(asm, HashMap::new())
    }

    /// Create an expression compiler with pre-computed allocation modes.
    pub fn with_allocations(
        asm: &'a mut Assembler,
        allocation_modes: HashMap<String, VarAllocation>,
    ) -> Self {
        Self::with_allocations_and_base(asm, allocation_modes, LET_BIND_BASE_OFFSET)
    }

    /// Create an expression compiler with pre-computed allocation modes and
    /// a custom base offset for `LetBind` memory slots.
    ///
    /// Use `memory_base` to avoid colliding with IR-allocated memory regions
    /// (arrays, structs, etc. placed during lowering).
    pub fn with_allocations_and_base(
        asm: &'a mut Assembler,
        allocation_modes: HashMap<String, VarAllocation>,
        memory_base: usize,
    ) -> Self {
        Self {
            asm,
            let_bindings: HashMap::new(),
            next_let_offset: memory_base,
            free_slots: Vec::new(),
            allocation_modes,
            stack_vars: HashMap::new(),
            stack_depth: 0,
            overflow_revert_label: None,
            fn_info: HashMap::new(),
        }
    }

    /// Look up the allocation mode for a variable (defaults to Memory).
    fn alloc_mode(&self, name: &str) -> AllocationMode {
        self.allocation_modes
            .get(name)
            .map(|a| a.mode)
            .unwrap_or(AllocationMode::Memory)
    }

    /// Pre-pass: collect Function metadata (param count, return count) from the IR tree.
    /// Must be called before `compile_expr` so that `Call` nodes can look up stack info.
    pub fn collect_fn_info(&mut self, expr: &EvmExpr) {
        match expr {
            EvmExpr::Function(name, in_ty, _out_ty, body) => {
                let param_count = Self::type_slot_count(in_ty);
                let ret_count = Self::count_stack_values(body);
                self.fn_info.insert(name.clone(), (param_count, ret_count));
                self.collect_fn_info(body);
            }
            EvmExpr::Concat(a, b) => {
                self.collect_fn_info(a);
                self.collect_fn_info(b);
            }
            EvmExpr::If(_, _, t, e) => {
                self.collect_fn_info(t);
                self.collect_fn_info(e);
            }
            EvmExpr::LetBind(_, v, b) => {
                self.collect_fn_info(v);
                self.collect_fn_info(b);
            }
            _ => {}
        }
    }

    /// Compile an IR expression, pushing its result onto the stack.
    pub fn compile_expr(&mut self, expr: &EvmExpr) {
        // Emit pretty-IR comment for statement-level nodes
        if let Some(summary) = edge_ir::pretty::pretty_summary(expr) {
            self.asm.emit_comment(summary);
        }

        match expr {
            EvmExpr::Const(c, _, _) => {
                self.compile_const(c);
                // All const paths push exactly 1 value
                self.stack_depth += 1;
            }

            EvmExpr::Arg(ty, _) => {
                // Function argument(s) already on the stack.
                // For single-param functions, DUP the arg (non-destructive).
                let n = Self::type_slot_count(ty);
                if n == 1 {
                    // Single arg: DUP from its position
                    let dup_depth = self.stack_depth; // arg0 is at position 0
                    debug_assert!(
                        (1..=16).contains(&dup_depth),
                        "Arg DUP depth {dup_depth} out of range"
                    );
                    self.asm.emit_op(Opcode::dup_n(dup_depth as u8));
                    self.stack_depth += 1;
                }
                // Multi-arg Arg should be accessed via Get(Arg, i), not bare.
            }
            EvmExpr::Empty(_, _) | EvmExpr::StorageField(_, _, _) => {
                // Empty: unit — no value on stack.
                // StorageField: declarations don't emit code.
                // No-ops.
            }

            EvmExpr::Bop(op, lhs, rhs) => {
                self.compile_binary_op(op, lhs, rhs);
            }
            EvmExpr::Uop(op, expr) => self.compile_unary_op(op, expr),
            EvmExpr::Top(op, a, b, c) => {
                self.compile_ternary_op(op, a, b, c);
            }

            EvmExpr::Get(tuple, idx) => {
                if let EvmExpr::Arg(ty, _) = tuple.as_ref() {
                    // Function parameter access: DUP from known stack position.
                    // At function entry, args are on the stack: [arg0, ..., argN-1]
                    // arg0 is deepest, argN-1 is closest to TOS.
                    // Arg at index i is at stack_depth - (param_count - i) from TOS.
                    let param_count = Self::type_slot_count(ty);
                    let dup_depth = self.stack_depth - *idx;
                    debug_assert!(
                        (1..=16).contains(&dup_depth),
                        "Arg DUP depth {dup_depth} out of range (param_count={param_count}, idx={idx}, stack_depth={})",
                        self.stack_depth
                    );
                    self.asm.emit_op(Opcode::dup_n(dup_depth as u8));
                    self.stack_depth += 1;
                } else {
                    self.compile_expr(tuple);
                    let n = Self::count_stack_values(tuple);
                    if n > 1 {
                        let depth = n - 1 - idx;
                        if depth > 0 && depth <= 16 {
                            self.asm.emit_op(Opcode::swap_n(depth as u8));
                        }
                        for _ in 0..(n - 1) {
                            self.asm.emit_op(Opcode::Pop);
                            self.stack_depth -= 1;
                        }
                    }
                    // Net: count(tuple) - (n-1) = 1
                }
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

            EvmExpr::EnvRead(op, _state) => {
                self.compile_env_read(op);
                self.stack_depth += 1;
            }
            EvmExpr::EnvRead1(op, arg, _state) => {
                self.compile_env_read1(op, arg);
                // compile_env_read1 handles depth tracking internally
            }

            EvmExpr::Log(topic_count, topics, data_offset, data_size, _state) => {
                self.compile_log(*topic_count, topics, data_offset, data_size);
            }

            EvmExpr::Revert(offset, size, _state) => {
                self.compile_expr(size);
                self.compile_expr(offset);
                self.asm.emit_op(Opcode::Revert);
                self.stack_depth -= 2; // REVERT pops offset + size
            }

            EvmExpr::ReturnOp(offset, size, _state) => {
                self.compile_expr(size);
                self.compile_expr(offset);
                self.asm.emit_op(Opcode::Return);
                self.stack_depth -= 2; // RETURN pops offset + size
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
                self.stack_depth += 1;
                self.asm.emit_op(Opcode::Call);
                self.stack_depth -= 6; // CALL pops 7, pushes 1
            }

            EvmExpr::Call(name, args) => {
                // Internal function call.
                // Calling convention:
                //   Push return address, push args, JUMP to function.
                //   Function returns: ret_addr consumed, return values on stack.
                let ret_label = self.asm.fresh_label(&format!("ret_fn_{name}"));
                let fn_label = format!("fn_{name}");

                let (_param_count, ret_count) = self.fn_info.get(name).copied().unwrap_or((0, 1));

                self.asm.emit(AsmInstruction::PushLabel(ret_label.clone()));
                self.stack_depth += 1;
                let mut arg_count = 0;
                for arg in args {
                    self.compile_expr(arg);
                    arg_count += Self::count_stack_values(arg);
                }

                self.asm.emit(AsmInstruction::JumpTo(fn_label));

                // Stack accounting:
                //   Before jump: stack has [... ret_addr arg0..argN]
                //   After return: stack has [... ret0..retM]
                //   Delta: -(1 + arg_count) + ret_count
                self.stack_depth -= 1 + arg_count;
                self.stack_depth += ret_count;

                self.asm.emit(AsmInstruction::Label(ret_label));
            }

            EvmExpr::Selector(sig) => {
                let mut hash = [0u8; 32];
                edge_types::bytes::hash_bytes(&mut hash, &sig.to_owned());
                let selector = &hash[..4];
                self.asm.emit(AsmInstruction::Push(selector.to_vec()));
                self.stack_depth += 1;
            }

            EvmExpr::LetBind(name, value, body) => {
                self.compile_let_bind(name, value, body);
            }

            EvmExpr::Var(name) => {
                self.compile_var(name);
            }

            EvmExpr::VarStore(name, value) => {
                self.compile_var_store(name, value);
            }

            EvmExpr::Drop(name) => {
                self.compile_drop(name);
            }

            EvmExpr::Function(name, in_ty, _out_ty, body) => {
                // Emit function body as a labeled subroutine.
                // Calling convention:
                //   Stack on entry: [...caller_stack, ret_addr, arg0, ..., argN-1]
                //   Body compiles, producing return values.
                //   Stack on exit:  [...caller_stack, ret0, ..., retM]
                //   Return: SWAP(ret_count) to bring ret_addr to TOS, JUMP.
                let skip_label = self.asm.fresh_label(&format!("skip_fn_{name}"));
                let fn_label = format!("fn_{name}");

                // Jump over the function body during linear execution
                self.asm.emit(AsmInstruction::JumpTo(skip_label.clone()));
                self.asm.emit(AsmInstruction::Label(fn_label));

                // Save compiler state — the function body compiles in its own
                // context. The EVM stack is shared at runtime, but the compiler's
                // depth tracking must be isolated (this code runs at call time,
                // not definition time).
                let saved_depth = self.stack_depth;
                let saved_let_bindings = self.let_bindings.clone();
                let saved_stack_vars = self.stack_vars.clone();
                let saved_free_slots = self.free_slots.clone();
                let saved_next_let_offset = self.next_let_offset;

                // Count params from in_ty
                let param_count = Self::type_slot_count(in_ty);

                // At call time, stack has: [ret_addr, arg0, ..., argN-1]
                // We track only the args (ret_addr is below, handled by SWAP+JUMP).
                self.stack_depth = param_count;
                self.let_bindings.clear();
                self.stack_vars.clear();

                self.compile_expr(body);

                // After body, stack is: [ret_addr, arg0..argN-1, retval0..retvalM]
                // Need to remove the N args from under the return values.
                let ret_count = Self::count_stack_values(body);

                if ret_count == 0 {
                    // No return values — just pop all args and JUMP
                    for _ in 0..param_count {
                        self.asm.emit_op(Opcode::Pop);
                    }
                    self.asm.emit_op(Opcode::Jump);
                } else if ret_count == 1 {
                    // Single return value — SWAP1+POP for each arg, then SWAP1+JUMP
                    for _ in 0..param_count {
                        self.asm.emit_op(Opcode::Swap1);
                        self.asm.emit_op(Opcode::Pop);
                    }
                    self.asm.emit_op(Opcode::Swap1);
                    self.asm.emit_op(Opcode::Jump);
                } else {
                    // Multiple return values — swap past all args + ret_addr
                    let total_below = param_count + 1; // args + ret_addr
                    assert!(
                        ret_count + total_below <= 17,
                        "Function {name}: too many values on stack for SWAP"
                    );
                    // For each return value (bottom to top), swap it past the args
                    // This is complex; for now just handle common cases
                    // TODO: handle multi-return + multi-param properly
                    self.asm
                        .emit_op(Opcode::swap_n((ret_count + param_count) as u8));
                    self.asm.emit_op(Opcode::Jump);
                    // Clean up remaining args
                    for _ in 0..param_count {
                        self.asm.emit_op(Opcode::Pop);
                    }
                }

                // Restore compiler state
                self.stack_depth = saved_depth;
                self.let_bindings = saved_let_bindings;
                self.stack_vars = saved_stack_vars;
                self.free_slots = saved_free_slots;
                self.next_let_offset = saved_next_let_offset;

                self.asm.emit(AsmInstruction::Label(skip_label));
            }
        }
    }

    /// Compile a `LetBind`: allocate variable, compile body, clean up.
    fn compile_let_bind(&mut self, name: &str, value: &RcExpr, body: &RcExpr) {
        match self.alloc_mode(name) {
            AllocationMode::Stack => {
                // Stack mode: leave value on stack, use DUP to read
                self.compile_expr(value);
                // Value is now on top of stack; record its position
                let var_pos = self.stack_depth - 1;
                let prev_stack = self.stack_vars.insert(name.to_owned(), var_pos);

                self.compile_expr(body);

                // Only clean up if the variable wasn't already dropped by an
                // early Drop node (e.g. in a halting branch).
                // Also skip cleanup if the body definitely halts — the cleanup
                // code is unreachable and the stack accounting would be wrong.
                let body_halts = Self::expr_definitely_halts(body.as_ref());
                if self.stack_vars.contains_key(name) && !body_halts {
                    // Clean up: remove variable from under body's results
                    let body_count = Self::count_stack_values(body);
                    if body_count == 0 {
                        self.asm.emit_op(Opcode::Pop);
                        self.stack_depth -= 1;
                    } else if body_count <= 16 {
                        self.asm.emit_op(Opcode::swap_n(body_count as u8));
                        self.asm.emit_op(Opcode::Pop);
                        self.stack_depth -= 1;
                    }
                    // else: body_count > 16, variable leaks (shouldn't happen with eligibility criteria)
                    self.stack_vars.remove(name);
                }

                // Restore previous stack binding if shadowed
                if let Some(prev) = prev_stack {
                    self.stack_vars.insert(name.to_owned(), prev);
                } else {
                    self.stack_vars.remove(name);
                }
            }
            AllocationMode::Memory => {
                // Memory mode: compile value, spill to memory
                self.compile_expr(value);
                // Allocate a memory slot: reuse a freed slot or bump the high-water mark
                let offset = if let Some(reused) = self.free_slots.pop() {
                    reused
                } else {
                    let off = self.next_let_offset;
                    self.next_let_offset += 32;
                    off
                };
                self.asm.emit_push_usize(offset);
                self.stack_depth += 1;
                self.asm.emit_op(Opcode::MStore);
                self.stack_depth -= 2; // MSTORE pops value + offset

                let prev = self.let_bindings.insert(name.to_owned(), offset);
                self.compile_expr(body);

                // Free the slot if Drop didn't already reclaim it
                if self.let_bindings.get(name) == Some(&offset) {
                    self.free_slots.push(offset);
                }
                // Restore previous binding (for shadowed names)
                if let Some(prev_offset) = prev {
                    self.let_bindings.insert(name.to_owned(), prev_offset);
                } else {
                    self.let_bindings.remove(name);
                }
            }
        }
    }

    /// Compile a variable read.
    fn compile_var(&mut self, name: &str) {
        if let Some(&var_pos) = self.stack_vars.get(name) {
            // Stack mode: DUP from the correct position
            let dup_index = self.stack_depth - var_pos;
            debug_assert!(
                (1..=16).contains(&dup_index),
                "DUP index {dup_index} out of range for variable {name} (depth={}, pos={var_pos})",
                self.stack_depth
            );
            self.asm.emit_op(Opcode::dup_n(dup_index as u8));
            self.stack_depth += 1;
        } else {
            // Memory mode: PUSH offset, MLOAD
            let offset = *self.let_bindings.get(name).unwrap_or_else(|| {
                panic!(
                    "no entry found for key: {name}; let_bindings keys: {:?}, stack_vars keys: {:?}",
                    self.let_bindings.keys().collect::<Vec<_>>(),
                    self.stack_vars.keys().collect::<Vec<_>>()
                )
            });
            self.asm.emit_push_usize(offset);
            self.stack_depth += 1;
            self.asm.emit_op(Opcode::MLoad);
            // MLOAD: pops offset, pushes value → net 0
        }
    }

    /// Compile a variable store.
    fn compile_var_store(&mut self, name: &str, value: &RcExpr) {
        // VarStore only applies to memory-mode variables (stack vars can't be reassigned)
        self.compile_expr(value);
        let offset = self.let_bindings[name];
        self.asm.emit_push_usize(offset);
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::MStore);
        self.stack_depth -= 2;
    }

    /// Compile a drop (lifetime end marker).
    ///
    /// For stack-allocated variables, emits POP (or SWAP+POP) to remove the
    /// variable from the stack. For memory-allocated variables, reclaims the
    /// slot for reuse.
    fn compile_drop(&mut self, name: &str) {
        if let Some(var_pos) = self.stack_vars.remove(name) {
            // Stack mode: actually emit POP to remove the variable
            let depth = self.stack_depth - var_pos;
            if depth == 1 {
                // Variable is at TOS: just POP
                self.asm.emit_op(Opcode::Pop);
            } else if depth <= 16 {
                // Variable is buried: SWAP to TOS then POP
                self.asm.emit_op(Opcode::swap_n((depth - 1) as u8));
                self.asm.emit_op(Opcode::Pop);
            }
            // else: depth > 16, can't reach (shouldn't happen with eligibility criteria)

            if (1..=16).contains(&depth) {
                // SWAP+POP moved the old TOS into the variable's slot.
                // Update any stack var that was at TOS position.
                if depth > 1 {
                    let old_tos = self.stack_depth - 1;
                    for pos in self.stack_vars.values_mut() {
                        if *pos == old_tos {
                            *pos = var_pos;
                            break;
                        }
                    }
                }
                self.stack_depth -= 1;
            }
        } else {
            // Memory mode: reclaim the slot for reuse
            if let Some(offset) = self.let_bindings.remove(name) {
                self.free_slots.push(offset);
            }
        }
    }

    /// Compile a constant value.
    fn compile_const(&mut self, c: &EvmConstant) {
        match c {
            EvmConstant::SmallInt(0) | EvmConstant::Bool(false) => {
                self.asm.emit_op(Opcode::Push0);
            }
            EvmConstant::SmallInt(n) => {
                let val = *n;
                if val < 0 {
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
                    let start = bytes.iter().position(|&b| b != 0).unwrap_or(0);
                    self.asm.emit(AsmInstruction::Push(bytes[start..].to_vec()));
                }
            }
            EvmConstant::Bool(true) => {
                self.asm.emit(AsmInstruction::Push(vec![1]));
            }
            EvmConstant::Addr(hex_str) => {
                let bytes = hex_string_to_bytes(hex_str);
                if bytes.len() > 20 {
                    self.asm
                        .emit(AsmInstruction::Push(bytes[bytes.len() - 20..].to_vec()));
                } else {
                    self.asm.emit(AsmInstruction::Push(bytes));
                }
            }
        }
        // Note: stack_depth += 1 is handled by the caller (compile_expr)
    }

    /// Compile a binary operation.
    fn compile_binary_op(&mut self, op: &EvmBinaryOp, lhs: &RcExpr, rhs: &RcExpr) {
        match op {
            EvmBinaryOp::Add => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Add);
                self.stack_depth -= 1; // pops 2, pushes 1
            }
            EvmBinaryOp::Sub => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Sub);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Mul => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Mul);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Div => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Div);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::SDiv => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SDiv);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Mod => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Mod);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::SMod => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SMod);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Exp => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Exp);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Lt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Lt);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Gt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Gt);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::SLt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SLt);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::SGt => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::SGt);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Eq => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Eq);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::And => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::And);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Or => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Or);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Xor => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Xor);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Shl => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Shl);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Shr => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Shr);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Sar => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Sar);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::Byte => {
                self.compile_expr(rhs);
                self.compile_expr(lhs);
                self.asm.emit_op(Opcode::Byte);
                self.stack_depth -= 1;
            }
            EvmBinaryOp::LogAnd => {
                // Short-circuit AND: if lhs is false, skip rhs
                let skip_label = self.asm.fresh_label("logand_skip");
                let end_label = self.asm.fresh_label("logand_end");
                self.compile_expr(lhs); // depth += 1
                self.asm.emit_op(Opcode::Dup1);
                self.stack_depth += 1;
                self.asm.emit_op(Opcode::IsZero); // 0 net
                self.asm.emit(AsmInstruction::JumpITo(skip_label.clone()));
                self.stack_depth -= 1; // JumpITo: PUSH(+1) JUMPI(-2) = net -1
                self.asm.emit_op(Opcode::Pop); // pop lhs copy
                self.stack_depth -= 1;
                self.compile_expr(rhs); // depth += 1
                self.asm.emit(AsmInstruction::JumpTo(end_label.clone())); // net 0
                self.asm.emit(AsmInstruction::Label(skip_label));
                // On skip path: lhs (false) is on stack — same depth as fall-through
                self.asm.emit(AsmInstruction::Label(end_label));
                // Both paths end with 1 value. Net from start: +1
            }
            EvmBinaryOp::LogOr => {
                // Short-circuit OR: if lhs is true, skip rhs
                let skip_label = self.asm.fresh_label("logor_skip");
                let end_label = self.asm.fresh_label("logor_end");
                self.compile_expr(lhs); // depth += 1
                self.asm.emit_op(Opcode::Dup1);
                self.stack_depth += 1;
                self.asm.emit(AsmInstruction::JumpITo(skip_label.clone()));
                self.stack_depth -= 1; // JumpITo: net -1
                self.asm.emit_op(Opcode::Pop); // pop lhs copy
                self.stack_depth -= 1;
                self.compile_expr(rhs); // depth += 1
                self.asm.emit(AsmInstruction::JumpTo(end_label.clone())); // net 0
                self.asm.emit(AsmInstruction::Label(skip_label));
                // On skip path: lhs (true) is on stack — same depth
                self.asm.emit(AsmInstruction::Label(end_label));
                // Both paths: +1 net
            }
            EvmBinaryOp::SLoad => {
                self.compile_expr(lhs); // slot; depth += 1
                self.asm.emit_op(Opcode::SLoad); // pops 1, pushes 1 → net 0
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
            EvmBinaryOp::CheckedAdd => {
                self.compile_checked_add(lhs, rhs);
            }
            EvmBinaryOp::CheckedSub => {
                self.compile_checked_sub(lhs, rhs);
            }
            EvmBinaryOp::CheckedMul => {
                self.compile_checked_mul(lhs, rhs);
            }
        }
    }

    /// Get or create the shared overflow revert label.
    fn get_overflow_revert_label(&mut self) -> String {
        if let Some(ref label) = self.overflow_revert_label {
            label.clone()
        } else {
            let label = self.asm.fresh_label("overflow_revert");
            self.overflow_revert_label = Some(label.clone());
            label
        }
    }

    /// Emit the shared overflow revert trampoline (if any checked op was compiled).
    /// Call this after all expressions have been compiled.
    pub fn emit_overflow_revert_trampoline(&mut self) {
        if let Some(label) = self.overflow_revert_label.take() {
            self.asm.emit(AsmInstruction::Label(label));
            self.asm.emit_op(Opcode::Push0);
            self.asm.emit_op(Opcode::Push0);
            self.asm.emit_op(Opcode::Revert);
        }
    }

    /// Compile checked addition: a + b, revert if overflow.
    /// Stack: rhs, lhs → [b, a] → DUP2 → [b, a, b] → ADD → [r, b]
    ///   → DUP1 → [r, r, b] → SWAP2 → [b, r, r] → GT → [b>r, r]
    ///   → JUMPI(revert) → [r]
    fn compile_checked_add(&mut self, lhs: &RcExpr, rhs: &RcExpr) {
        let revert_label = self.get_overflow_revert_label();
        self.compile_expr(rhs); // [b]
        self.compile_expr(lhs); // [a, b]
        self.asm.emit_op(Opcode::Dup2); // [b, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Add); // [r, b]  (r = a+b wrapping)
        self.stack_depth -= 1;
        self.asm.emit_op(Opcode::Dup1); // [r, r, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Swap2); // [b, r, r]
        self.asm.emit_op(Opcode::Gt); // [b>r, r]  overflow iff b > result
        self.stack_depth -= 1;
        self.asm.emit(AsmInstruction::JumpITo(revert_label));
        self.stack_depth -= 1; // JUMPI consumes condition
                               // Net: pushed 2 (lhs, rhs), +1 DUP2, -1 ADD, +1 DUP1, -1 GT, -1 JUMPI = -1 from initial 2 = 1 value
                               // But we already tracked lhs (+1) and rhs (+1) via compile_expr. So net from here is -1.
    }

    /// Compile checked subtraction: a - b, revert if a < b.
    /// Stack: rhs, lhs → [b, a] → DUP2 → [b, a, b] → DUP2 → [a, b, a, b]
    ///   → LT → [a<b, a, b] → JUMPI(revert) → [a, b] → SUB → [a-b]
    fn compile_checked_sub(&mut self, lhs: &RcExpr, rhs: &RcExpr) {
        let revert_label = self.get_overflow_revert_label();
        self.compile_expr(rhs); // [b]
        self.compile_expr(lhs); // [a, b]
        self.asm.emit_op(Opcode::Dup2); // [b, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Dup2); // [a, b, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Lt); // [a<b, a, b]  underflow iff a < b
        self.stack_depth -= 1;
        self.asm.emit(AsmInstruction::JumpITo(revert_label));
        self.stack_depth -= 1; // JUMPI consumes condition
        self.asm.emit_op(Opcode::Sub); // [a-b]
        self.stack_depth -= 1;
    }

    /// Compile checked multiplication: a * b, revert if overflow.
    /// Uses: if a == 0, result is 0 (no overflow possible).
    /// Otherwise: result = a*b (wrapping), check result/a == b.
    fn compile_checked_mul(&mut self, lhs: &RcExpr, rhs: &RcExpr) {
        let revert_label = self.get_overflow_revert_label();
        let mul_ok_label = self.asm.fresh_label("mul_ok");

        self.compile_expr(rhs); // [b]
        self.compile_expr(lhs); // [a, b]
        self.asm.emit_op(Opcode::Dup2); // [b, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Dup2); // [a, b, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Mul); // [r, a, b]  r = a*b wrapping
        self.stack_depth -= 1;

        // Check: if a == 0, skip overflow check (0 * anything = 0)
        self.asm.emit_op(Opcode::Dup2); // [a, r, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::IsZero); // [a==0, r, a, b]
        self.asm.emit(AsmInstruction::JumpITo(mul_ok_label.clone()));
        self.stack_depth -= 1; // JUMPI consumes condition

        // a != 0: check r/a == b
        self.asm.emit_op(Opcode::Dup1); // [r, r, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Dup3); // [a, r, r, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Swap1); // [r, a, r, a, b]
        self.asm.emit_op(Opcode::Div); // [r/a, r, a, b]
        self.stack_depth -= 1;
        self.asm.emit_op(Opcode::Dup4); // [b, r/a, r, a, b]
        self.stack_depth += 1;
        self.asm.emit_op(Opcode::Eq); // [b==r/a, r, a, b]
        self.stack_depth -= 1;
        self.asm.emit_op(Opcode::IsZero); // [b!=r/a, r, a, b]
        self.asm.emit(AsmInstruction::JumpITo(revert_label));
        self.stack_depth -= 1; // JUMPI consumes condition

        // mul_ok: [r, a, b]
        self.asm.emit(AsmInstruction::Label(mul_ok_label));
        self.asm.emit_op(Opcode::Swap2); // [b, a, r]
        self.asm.emit_op(Opcode::Pop); // [a, r]
        self.stack_depth -= 1;
        self.asm.emit_op(Opcode::Pop); // [r]
        self.stack_depth -= 1;
    }

    /// Compile a unary operation.
    fn compile_unary_op(&mut self, op: &EvmUnaryOp, expr: &RcExpr) {
        self.compile_expr(expr); // depth += 1
        match op {
            EvmUnaryOp::IsZero => self.asm.emit_op(Opcode::IsZero), // 0 net
            EvmUnaryOp::Not => self.asm.emit_op(Opcode::Not),       // 0 net
            EvmUnaryOp::Neg => {
                // 0 - x: Push0 (+1), Sub (-1) → net 0
                self.asm.emit_op(Opcode::Push0);
                self.stack_depth += 1;
                self.asm.emit_op(Opcode::Sub);
                self.stack_depth -= 1;
            }
            EvmUnaryOp::SignExtend => self.asm.emit_op(Opcode::SignExtend), // 0 net (our Uop convention)
        }
        // Total net: +1
    }

    /// Compile a ternary operation.
    fn compile_ternary_op(&mut self, op: &EvmTernaryOp, a: &RcExpr, b: &RcExpr, c: &RcExpr) {
        match op {
            EvmTernaryOp::SStore => {
                self.compile_expr(b); // value
                self.compile_expr(a); // key (slot)
                self.asm.emit_op(Opcode::SStore);
                self.stack_depth -= 2;
            }
            EvmTernaryOp::TStore => {
                self.compile_expr(b); // value
                self.compile_expr(a); // key
                self.asm.emit_op(Opcode::TStore);
                self.stack_depth -= 2;
            }
            EvmTernaryOp::MStore => {
                self.compile_expr(b); // value
                self.compile_expr(a); // offset
                self.asm.emit_op(Opcode::MStore);
                self.stack_depth -= 2;
            }
            EvmTernaryOp::MStore8 => {
                self.compile_expr(b); // value
                self.compile_expr(a); // offset
                self.asm.emit_op(Opcode::MStore8);
                self.stack_depth -= 2;
            }
            EvmTernaryOp::Keccak256 => {
                self.compile_expr(b); // size
                self.compile_expr(a); // offset
                self.asm.emit_op(Opcode::Keccak256);
                self.stack_depth -= 1; // pops 2, pushes 1
            }
            EvmTernaryOp::CalldataCopy => {
                // CalldataCopy(dest_offset, cd_offset, size)
                // EVM stack order: CALLDATACOPY(destOffset, offset, size) — pops 3, pushes 0
                self.compile_expr(c); // size
                self.compile_expr(b); // cd_offset
                self.compile_expr(a); // dest_offset
                self.asm.emit_op(Opcode::CallDataCopy);
                self.stack_depth -= 3; // pops 3, pushes 0
            }
            EvmTernaryOp::Select => {
                // Select(cond, true_val, false_val) → if cond then true_val else false_val
                let else_label = self.asm.fresh_label("select_else");
                let end_label = self.asm.fresh_label("select_end");

                self.compile_expr(a); // cond; depth += 1
                self.asm.emit_op(Opcode::IsZero); // 0 net
                self.asm.emit(AsmInstruction::JumpITo(else_label.clone()));
                self.stack_depth -= 1; // JumpITo: net -1 (cond consumed)

                let depth_before_branches = self.stack_depth;

                self.compile_expr(b); // true value
                self.asm.emit(AsmInstruction::JumpTo(end_label.clone())); // net 0

                // Reset depth for else path
                let depth_after_then = self.stack_depth;
                self.stack_depth = depth_before_branches;

                self.asm.emit(AsmInstruction::Label(else_label));
                self.compile_expr(c); // false value

                debug_assert_eq!(
                    self.stack_depth, depth_after_then,
                    "Select branches produce different stack depths"
                );

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

        self.compile_expr(cond); // depth += 1
        self.asm.emit_op(Opcode::IsZero); // 0 net
        self.asm.emit(AsmInstruction::JumpITo(else_label.clone()));
        self.stack_depth -= 1; // JumpITo: net -1 (cond consumed)

        let depth_before_branches = self.stack_depth;
        // Save all mutable state before branching, since
        // Drop in one branch must not affect the other.
        let stack_vars_before = self.stack_vars.clone();
        let let_bindings_before = self.let_bindings.clone();
        let free_slots_before = self.free_slots.clone();

        let then_halts = Self::expr_definitely_halts(then_body);
        let else_halts = Self::expr_definitely_halts(else_body);

        self.compile_expr(then_body);
        self.asm.emit(AsmInstruction::JumpTo(end_label.clone())); // net 0

        let depth_after_then = self.stack_depth;
        let stack_vars_after_then = self.stack_vars.clone();
        let let_bindings_after_then = self.let_bindings.clone();
        let free_slots_after_then = self.free_slots.clone();

        // Restore all compiler state for the else path
        self.stack_depth = depth_before_branches;
        self.stack_vars = stack_vars_before;
        self.let_bindings = let_bindings_before;
        self.free_slots = free_slots_before;

        self.asm.emit(AsmInstruction::Label(else_label));
        self.compile_expr(else_body);

        let depth_after_else = self.stack_depth;

        // Reconcile stack depths and variable state across branches:
        // - If one branch halts, its state is irrelevant — use the other's.
        // - If neither halts, they must match.
        if then_halts && !else_halts {
            // Use else branch's state (then never reaches end label)
            self.stack_depth = depth_after_else;
        } else if else_halts && !then_halts {
            // Use then branch's state (else never reaches end label)
            self.stack_depth = depth_after_then;
            self.stack_vars = stack_vars_after_then;
            self.let_bindings = let_bindings_after_then;
            self.free_slots = free_slots_after_then;
        } else if !then_halts && !else_halts {
            debug_assert_eq!(
                depth_after_else, depth_after_then,
                "If branches produce different stack depths"
            );
        }
        // else: both halt — state is irrelevant, keep current

        self.asm.emit(AsmInstruction::Label(end_label));
    }

    /// Check if an expression is guaranteed to halt (ends with RETURN or REVERT).
    fn expr_definitely_halts(expr: &EvmExpr) -> bool {
        match expr {
            EvmExpr::ReturnOp(_, _, _) | EvmExpr::Revert(_, _, _) => true,
            EvmExpr::Concat(_, b) => Self::expr_definitely_halts(b),
            EvmExpr::If(_, _, then_body, else_body) => {
                Self::expr_definitely_halts(then_body) && Self::expr_definitely_halts(else_body)
            }
            EvmExpr::LetBind(_, _, body) => Self::expr_definitely_halts(body),
            _ => false,
        }
    }

    /// Compile a do-while loop.
    fn compile_do_while(&mut self, inputs: &RcExpr, pred_and_body: &RcExpr) {
        let loop_label = self.asm.fresh_label("loop");

        self.compile_expr(inputs);
        self.asm.emit(AsmInstruction::Label(loop_label.clone()));
        self.compile_expr(pred_and_body);
        self.asm.emit(AsmInstruction::JumpITo(loop_label));
        self.stack_depth -= 1; // JumpITo: net -1 (condition consumed)
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
            EvmEnvOp::BlockHash | EvmEnvOp::Balance => Opcode::Invalid,
        };
        self.asm.emit_op(opcode);
        // Note: depth increment handled by caller (compile_expr)
    }

    /// Compile a unary environment read.
    fn compile_env_read1(&mut self, op: &EvmEnvOp, arg: &RcExpr) {
        self.compile_expr(arg); // depth += 1
        let opcode = match op {
            EvmEnvOp::Balance => Opcode::Balance,
            EvmEnvOp::BlockHash => Opcode::BlockHash,
            _ => {
                // Other env ops are nullary; compile as such
                self.compile_env_read(op);
                self.stack_depth += 1; // env read pushes 1
                return;
            }
        };
        self.asm.emit_op(opcode); // pops 1, pushes 1 → net 0
                                  // Total: +1 from arg compile
    }

    /// Compile a LOG instruction.
    fn compile_log(
        &mut self,
        topic_count: usize,
        topics: &[RcExpr],
        data_offset: &RcExpr,
        data_size: &RcExpr,
    ) {
        // Push topics in reverse order
        for topic in topics.iter().rev() {
            self.compile_expr(topic);
        }

        // Push data size then offset (EVM stack order: offset on top, size below)
        self.compile_expr(data_size);
        self.compile_expr(data_offset);

        self.asm.emit_op(Opcode::log_n(topic_count as u8));
        // LOGn pops: offset + size + n topics = 2 + topic_count
        self.stack_depth -= 2 + topic_count;
    }

    /// How many EVM stack slots a type occupies.
    fn type_slot_count(ty: &EvmType) -> usize {
        use edge_ir::schema::EvmBaseType;
        match ty {
            EvmType::TupleT(elems) => elems.len(),
            EvmType::Base(EvmBaseType::UnitT) | EvmType::Base(EvmBaseType::StateT) => 0,
            EvmType::Base(_) => 1,
        }
    }

    /// Estimate how many stack values an expression pushes.
    ///
    /// Must be accurate for stack-mode `LetBind` cleanup (SWAP+POP).
    fn count_stack_values(expr: &EvmExpr) -> usize {
        match expr {
            EvmExpr::Concat(a, b) => Self::count_stack_values(a) + Self::count_stack_values(b),
            EvmExpr::Empty(_, _)
            | EvmExpr::VarStore(_, _)
            | EvmExpr::Drop(_)
            | EvmExpr::Revert(_, _, _)
            | EvmExpr::ReturnOp(_, _, _)
            | EvmExpr::Log(_, _, _, _, _)
            | EvmExpr::Function(_, _, _, _)
            | EvmExpr::DoWhile(_, _)
            | EvmExpr::StorageField(_, _, _) => 0,
            EvmExpr::Arg(ty, _) => Self::type_slot_count(ty),
            EvmExpr::LetBind(_, _, body) => Self::count_stack_values(body),
            // Side-effect ternary ops push nothing onto the stack
            EvmExpr::Top(op, _, _, _) => match op {
                EvmTernaryOp::SStore
                | EvmTernaryOp::TStore
                | EvmTernaryOp::MStore
                | EvmTernaryOp::MStore8
                | EvmTernaryOp::CalldataCopy => 0,
                EvmTernaryOp::Keccak256 | EvmTernaryOp::Select => 1,
            },
            // If: both branches should push the same count
            EvmExpr::If(_, _, then_body, _) => Self::count_stack_values(then_body),
            // Everything else pushes 1 value (Var, Bop, Uop, Const, etc.)
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
