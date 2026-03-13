//! IR expression to EVM opcode compilation.
//!
//! Walks the `EvmExpr` tree and emits EVM opcodes into an `Assembler`.
//! Since the EVM is a stack machine, we compile in postorder: children
//! first, then the operator.

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

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
    /// Remaining reads for stack-allocated variables (for last-use consume optimization)
    remaining_reads: HashMap<String, usize>,
    /// True when compiling code after a RETURN/REVERT (unreachable dead code).
    /// Disables consume optimization since dead-code stack accounting is fragile.
    in_dead_code: bool,
    /// True when the current expression is followed by a halting instruction
    /// (RETURN/REVERT). Drops for stack vars skip SWAP+POP since the EVM will
    /// halt anyway — no point cleaning up a stack that's about to be discarded.
    halting_context: bool,
    /// Current EVM stack depth (number of values on the stack, tracked for DUP indexing)
    stack_depth: usize,
    /// Label for shared overflow revert trampoline (lazily created)
    overflow_revert_label: Option<String>,
    /// Label for shared revert(0,0) trampoline (lazily created)
    revert_trampoline_label: Option<String>,
    /// Inner function metadata: name -> (`param_count`, `return_count`)
    /// Populated by a pre-pass over the IR tree before compilation.
    fn_info: HashMap<String, (usize, usize)>,
    /// Minimum address that `DynAlloc` may return.
    /// Ensures `DynAlloc` pointers don't overlap with `LetBind` memory slots.
    /// Set to `memory_high_water + num_memory_vars * 32` when `DynAlloc` is used.
    dyn_alloc_floor: usize,
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
        Self::with_allocations_base_and_floor(asm, allocation_modes, memory_base, 0)
    }

    /// Create an expression compiler with allocation modes, a custom base offset,
    /// and a `DynAlloc` floor (minimum address for dynamic memory allocation).
    pub fn with_allocations_base_and_floor(
        asm: &'a mut Assembler,
        allocation_modes: HashMap<String, VarAllocation>,
        memory_base: usize,
        dyn_alloc_floor: usize,
    ) -> Self {
        Self {
            asm,
            let_bindings: HashMap::new(),
            next_let_offset: memory_base,
            free_slots: Vec::new(),
            allocation_modes,
            stack_vars: HashMap::new(),
            remaining_reads: HashMap::new(),
            in_dead_code: false,
            halting_context: false,
            stack_depth: 0,
            overflow_revert_label: None,
            revert_trampoline_label: None,
            fn_info: HashMap::new(),
            dyn_alloc_floor,
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
            EvmExpr::MemRegion(id, _size) => {
                // MemRegion should have been resolved to a concrete offset by
                // assign_memory_offsets(). If we get here, it's a bug.
                panic!(
                    "MemRegion({id}) reached codegen without being resolved to a concrete offset. \
                     Run assign_memory_offsets() after egglog extraction."
                );
            }

            EvmExpr::DynAlloc(size) => {
                // Dynamic memory allocation using max(MSIZE, floor).
                //
                // The floor ensures DynAlloc pointers don't overlap with
                // LetBind memory slots whose MSTORE hasn't happened yet.
                // If MSIZE is already past the floor (common case), MSIZE wins.
                //
                // When floor > 0, we emit:
                //   MSIZE                    → [ms]
                //   DUP1                     → [ms, ms]
                //   PUSH floor               → [ms, ms, floor]
                //   GT                       → [ms, ms>floor]
                //   PUSH skip                → [ms, ms>floor, skip]
                //   JUMPI                    → [ms]  -- if ms>floor, keep ms
                //   POP                      → []
                //   PUSH floor               → [floor]
                //   skip: JUMPDEST           → [base]  = max(ms, floor)
                //
                // Then allocate:
                //   DUP1                     → [base, base]
                //   compile(size)            → [base, base, size]
                //   ADD                      → [base, base+size]
                //   PUSH0                    → [base, base+size, 0]
                //   SWAP1                    → [base, 0, base+size]
                //   MSTORE                   → [base]  (expands memory)
                //
                // Net stack effect: +1 (the base pointer)
                if self.dyn_alloc_floor > 0 {
                    // Emit max(MSIZE, floor)
                    //
                    // Stack sequence:
                    //   MSIZE            → [ms]
                    //   DUP1             → [ms, ms]
                    //   PUSH floor       → [floor, ms, ms]
                    //   GT  (EVM: a > b where a=TOS=floor, b=ms)
                    //                    → [floor>ms, ms]
                    //   ...
                    //
                    // We want: if ms >= floor, keep ms (skip).
                    //          if ms < floor, replace ms with floor.
                    //
                    // EVM GT(floor, ms) = floor > ms.
                    // When floor > ms (need floor): GT=1, JUMPI takes jump.
                    //   But we want to REPLACE ms, not skip!
                    // So we must NOT jump when floor > ms.
                    //
                    // Fix: use ISZERO to invert, or just use LT instead.
                    // With GT: floor > ms means we need floor. So jump should
                    // go to the replacement path, not the skip path.
                    //
                    // Simplest fix: use GT but jump means "ms is big enough, skip".
                    // GT(floor, ms) = floor > ms → ms is NOT big enough.
                    // So: ISZERO + JUMPI to skip when ms IS big enough.
                    let skip_label = self.asm.fresh_label("dyn_alloc_skip");
                    self.asm.emit_op(Opcode::MSize);
                    self.stack_depth += 1;
                    self.asm.emit_op(Opcode::Dup1);
                    self.stack_depth += 1;
                    self.asm.emit_push_usize(self.dyn_alloc_floor);
                    self.stack_depth += 1;
                    // GT: floor > ms?
                    self.asm.emit_op(Opcode::Gt);
                    self.stack_depth -= 1;
                    // ISZERO: !(floor > ms) = ms >= floor
                    self.asm.emit_op(Opcode::IsZero);
                    // stack: [ms, ms >= floor]
                    self.asm.emit(AsmInstruction::JumpITo(skip_label.clone()));
                    self.stack_depth -= 1; // JumpITo: PUSH(+1) JUMPI(-2) = net -1
                                           // Fall-through: floor > ms, use floor instead
                    self.asm.emit_op(Opcode::Pop);
                    self.stack_depth -= 1;
                    self.asm.emit_push_usize(self.dyn_alloc_floor);
                    self.stack_depth += 1;
                    self.asm.emit(AsmInstruction::Label(skip_label));
                    // stack: [base] where base = max(MSIZE, floor)
                } else {
                    // No floor needed — MSIZE is sufficient
                    self.asm.emit_op(Opcode::MSize);
                    self.stack_depth += 1;
                }

                // Expand memory: MSTORE(base + size, 0)
                self.asm.emit_op(Opcode::Dup1);
                self.stack_depth += 1;
                self.compile_expr(size);
                // stack: [base, base, size]
                self.asm.emit_op(Opcode::Add);
                self.stack_depth -= 1;
                // stack: [base, base+size]
                self.asm.emit_op(Opcode::Push0);
                self.stack_depth += 1;
                self.asm.emit_op(Opcode::Swap1);
                // stack: [base, 0, base+size]
                self.asm.emit_op(Opcode::MStore);
                self.stack_depth -= 2;
                // stack: [base] — the returned pointer
            }

            EvmExpr::AllocRegion(id, _, _) => {
                panic!(
                    "AllocRegion({id}) reached codegen without being resolved. \
                     Run resolve_regions() after egglog extraction."
                );
            }

            EvmExpr::RegionStore(id, field, _, _) => {
                panic!(
                    "RegionStore({id}, {field}) reached codegen without being resolved to MStore. \
                     Run resolve_regions() after egglog extraction."
                );
            }

            EvmExpr::RegionLoad(id, field, _) => {
                panic!(
                    "RegionLoad({id}, {field}) reached codegen without being resolved to MLoad. \
                     Run resolve_regions() after egglog extraction."
                );
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
                // Last-use DUP elision: Concat(Var(x), Drop(x)) or
                // Concat(Var(x), Concat(Drop(x), rest)) where x is a stack
                // var at depth 1 (TOS).  Instead of DUP1 + SWAP1 + POP,
                // emit nothing — x stays in place.
                if let EvmExpr::Var(var_name) = a.as_ref() {
                    let drop_name = match b.as_ref() {
                        EvmExpr::Drop(n) => Some(n.as_str()),
                        EvmExpr::Concat(drop_expr, _) => match drop_expr.as_ref() {
                            EvmExpr::Drop(n) => Some(n.as_str()),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(dn) = drop_name {
                        if dn == var_name && !self.in_dead_code {
                            if let Some(&var_pos) = self.stack_vars.get(var_name.as_str()) {
                                let depth = self.stack_depth - var_pos;
                                if depth == 1 {
                                    self.stack_vars.remove(var_name.as_str());
                                    // Compile rest if Concat(Drop, rest)
                                    if let EvmExpr::Concat(_, rest) = b.as_ref() {
                                        self.compile_expr(rest);
                                    }
                                    return;
                                }
                            }
                        }
                    }
                }
                // If `b` will halt, set halting_context so that Drops in `a`
                // skip stack cleanup — no point SWAP+POP'ing when the EVM is
                // about to RETURN/REVERT and discard the entire stack.
                let was_halting = self.halting_context;
                if Self::expr_definitely_halts(b.as_ref()) {
                    self.halting_context = true;
                }
                self.compile_expr(a);
                self.halting_context = was_halting;
                // Mark dead code after a halting expression
                if Self::expr_definitely_halts(a.as_ref()) {
                    self.in_dead_code = true;
                }
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
                // revert(0, 0) is extremely common (bounds checks, dispatch fallback).
                // Share a single trampoline instead of emitting Push0+Push0+Revert each time.
                if Self::is_const_zero(offset) && Self::is_const_zero(size) {
                    let label = self.get_revert_trampoline_label();
                    self.asm.emit(AsmInstruction::JumpTo(label));
                } else {
                    self.compile_expr(size);
                    self.compile_expr(offset);
                    self.asm.emit_op(Opcode::Revert);
                    self.stack_depth -= 2; // REVERT pops offset + size
                }
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

            EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
                // Compile input expressions — they push values to stack
                for input in inputs {
                    self.compile_expr(input);
                }
                // Decode hex string to raw bytes and emit directly
                let bytes: Vec<u8> = (0..hex.len())
                    .step_by(2)
                    .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
                    .collect();
                self.asm.emit(AsmInstruction::Raw(bytes));
                // Adjust stack depth: inputs consumed, outputs produced
                // Net delta = num_outputs - num_inputs
                let num_inputs = inputs.len() as i32;
                self.stack_depth = (self.stack_depth as i32 + num_outputs - num_inputs) as usize;
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

    /// Compute the peak `next_let_offset` by simulating `LetBind` allocation.
    ///
    /// Walks the IR tree in the same order as `compile_expr`, tracking
    /// memory-mode `LetBind` slot allocation/deallocation. Returns the
    /// highest `next_let_offset` reached during the traversal.
    ///
    /// This mirrors the actual codegen behavior:
    /// - `If` saves/restores `let_bindings` and `free_slots` but NOT
    ///   `next_let_offset` (branches get non-overlapping slots)
    /// - `Function` saves/restores everything including `next_let_offset`
    /// - `Drop` reclaims slots to the free list for reuse
    pub fn compute_peak_let_offset(
        allocation_modes: &HashMap<String, VarAllocation>,
        memory_base: usize,
        exprs: &[&RcExpr],
    ) -> usize {
        let mut state = LetOffsetSim {
            let_bindings: HashMap::new(),
            free_slots: Vec::new(),
            next_let_offset: memory_base,
            peak: memory_base,
            allocation_modes,
            stack_var_count: 0,
            visited: HashSet::new(),
        };
        for expr in exprs {
            state.walk(expr);
        }
        state.peak
    }

    /// Compile a `LetBind`: allocate variable, compile body, clean up.
    fn compile_let_bind(&mut self, name: &str, value: &RcExpr, body: &RcExpr) {
        // Decide allocation mode with stack depth safety check:
        // Cap concurrent stack vars to avoid DUP/SWAP overflow (max depth 16).
        // With 14 stack vars + ~2 expression temporaries, peak depth ≈ 16.
        let mode = if self.alloc_mode(name) == AllocationMode::Stack && self.stack_vars.len() < 14 {
            AllocationMode::Stack
        } else {
            AllocationMode::Memory
        };
        match mode {
            AllocationMode::Stack => {
                // Stack mode: leave value on stack, use DUP to read
                let was_dead = self.in_dead_code;
                self.compile_expr(value);
                // If init halts, body is dead code
                if Self::expr_definitely_halts(value.as_ref()) {
                    self.in_dead_code = true;
                }
                // Value is now on top of stack; record its position
                let var_pos = self.stack_depth - 1;
                let prev_stack = self.stack_vars.insert(name.to_owned(), var_pos);
                // Count reads in body for last-use consume optimization.
                // Disable consume when body halts or we're in dead code:
                // cleanup will be skipped so the var must "leak" on stack
                // to maintain consistent depth accounting.
                let body_halts = Self::expr_definitely_halts(body.as_ref());
                let reads = if body_halts || self.in_dead_code {
                    usize::MAX
                } else {
                    Self::count_var_reads(name, body)
                };
                let prev_reads = self.remaining_reads.insert(name.to_owned(), reads);

                self.compile_expr(body);
                if self.stack_vars.contains_key(name) && !body_halts && !self.halting_context {
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
                if let Some(prev) = prev_reads {
                    self.remaining_reads.insert(name.to_owned(), prev);
                } else {
                    self.remaining_reads.remove(name);
                }
                self.in_dead_code = was_dead;
            }
            AllocationMode::Memory => {
                // Memory mode: compile value, spill to memory
                let was_dead = self.in_dead_code;
                self.compile_expr(value);
                if Self::expr_definitely_halts(value.as_ref()) {
                    self.in_dead_code = true;
                }
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
                self.in_dead_code = was_dead;
            }
        }
    }

    /// Compile a variable read.
    fn compile_var(&mut self, name: &str) {
        if let Some(&var_pos) = self.stack_vars.get(name) {
            let depth = self.stack_depth - var_pos;
            debug_assert!(
                (1..=16).contains(&depth),
                "DUP/SWAP index {depth} out of range for variable {name} (depth={}, pos={var_pos})",
                self.stack_depth
            );

            // Check if this is the last read — if so and var is at TOS,
            // consume it directly instead of DUP + later SWAP+POP.
            let is_last_use = self.remaining_reads.get_mut(name).is_some_and(|remaining| {
                *remaining = remaining.saturating_sub(1);
                *remaining == 0
            });

            if is_last_use && depth == 1 && !self.in_dead_code {
                // Last use and var is at TOS: consume in-place.
                self.stack_vars.remove(name);
            } else {
                self.asm.emit_op(Opcode::dup_n(depth as u8));
                self.stack_depth += 1;
            }
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
        if let Some(&var_pos) = self.stack_vars.get(name) {
            // Stack mode: evaluate new value, swap with old, pop old
            self.compile_expr(value);
            let depth = self.stack_depth - var_pos;
            debug_assert!(
                (2..=16).contains(&depth),
                "SWAP index {} out of range for VarStore of {name} (depth={}, pos={var_pos})",
                depth - 1,
                self.stack_depth
            );
            self.asm.emit_op(Opcode::swap_n((depth - 1) as u8));
            self.asm.emit_op(Opcode::Pop);
            self.stack_depth -= 1;
        } else {
            // Memory mode: compile value, push offset, MSTORE
            self.compile_expr(value);
            let offset = *self.let_bindings.get(name).unwrap_or_else(|| {
                panic!(
                    "VarStore: variable {name:?} not found in stack_vars or let_bindings. \
                     stack_vars={:?}, let_bindings={:?}",
                    self.stack_vars.keys().collect::<Vec<_>>(),
                    self.let_bindings.keys().collect::<Vec<_>>()
                )
            });
            self.asm.emit_push_usize(offset);
            self.stack_depth += 1;
            self.asm.emit_op(Opcode::MStore);
            self.stack_depth -= 2;
        }
    }

    /// Compile a drop (lifetime end marker).
    ///
    /// For stack-allocated variables, emits POP (or SWAP+POP) to remove the
    /// variable from the stack. For memory-allocated variables, reclaims the
    /// slot for reuse.
    fn compile_drop(&mut self, name: &str) {
        if let Some(var_pos) = self.stack_vars.remove(name) {
            if self.halting_context || self.in_dead_code {
                // About to halt (RETURN/REVERT) or already past a halt —
                // skip SWAP+POP cleanup. The EVM stack will be discarded anyway.
                return;
            }
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
            // Memory mode: reclaim the slot for reuse (skip if halting — no point)
            if !self.halting_context {
                if let Some(offset) = self.let_bindings.remove(name) {
                    self.free_slots.push(offset);
                }
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

    /// Get or create the shared revert(0,0) trampoline label.
    fn get_revert_trampoline_label(&mut self) -> String {
        if let Some(ref label) = self.revert_trampoline_label {
            return label.clone();
        }
        let label = self.asm.fresh_label("revert_trampoline");
        self.revert_trampoline_label = Some(label.clone());
        label
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

    /// Emit shared revert trampolines. Call after all expressions are compiled.
    pub fn emit_overflow_revert_trampoline(&mut self) {
        if let Some(label) = self.overflow_revert_label.take() {
            self.asm.emit(AsmInstruction::Label(label));
            self.asm.emit_op(Opcode::Push0);
            self.asm.emit_op(Opcode::Push0);
            self.asm.emit_op(Opcode::Revert);
        }
        if let Some(label) = self.revert_trampoline_label.take() {
            self.asm.emit(AsmInstruction::Label(label));
            self.asm.emit_op(Opcode::Push0);
            self.asm.emit_op(Opcode::Push0);
            self.asm.emit_op(Opcode::Revert);
        }
    }

    /// Check if an expression is a constant zero.
    fn is_const_zero(expr: &RcExpr) -> bool {
        matches!(
            expr.as_ref(),
            EvmExpr::Const(EvmConstant::SmallInt(0), _, _)
                | EvmExpr::Const(EvmConstant::Bool(false), _, _)
        )
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
            EvmUnaryOp::Clz => self.asm.emit_op(Opcode::Clz),       // 0 net
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
            EvmTernaryOp::Mcopy => {
                // Mcopy(dest, src, size)
                // EVM stack order: MCOPY(dest, src, length) — pops 3, pushes 0
                self.compile_expr(c); // size
                self.compile_expr(b); // src
                self.compile_expr(a); // dest
                self.asm.emit_op(Opcode::MCopy);
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
    ///
    /// Optimizes condition compilation to avoid redundant `IsZero`:
    /// - `if IsZero(x)`: compile x, JUMPI to else directly (double-negation cancel,
    ///   saves 3 gas per branch).
    fn compile_if(&mut self, cond: &RcExpr, then_body: &RcExpr, else_body: &RcExpr) {
        let else_label = self.asm.fresh_label("else");
        let end_label = self.asm.fresh_label("endif");

        match cond.as_ref() {
            // IsZero(inner): compile inner, JUMPI→else directly (cancel double negation)
            EvmExpr::Uop(EvmUnaryOp::IsZero, inner) => {
                self.compile_expr(inner);
                self.asm.emit(AsmInstruction::JumpITo(else_label.clone()));
                self.stack_depth -= 1;
            }
            // Default: emit IsZero + JUMPI
            _ => {
                self.compile_expr(cond);
                self.asm.emit_op(Opcode::IsZero);
                self.asm.emit(AsmInstruction::JumpITo(else_label.clone()));
                self.stack_depth -= 1;
            }
        };

        let (first_body, second_body) = (then_body, else_body);

        let depth_before_branches = self.stack_depth;
        // Save all mutable state before branching, since
        // Drop in one branch must not affect the other.
        let stack_vars_before = self.stack_vars.clone();
        let let_bindings_before = self.let_bindings.clone();
        let free_slots_before = self.free_slots.clone();
        let remaining_reads_before = self.remaining_reads.clone();
        let was_dead = self.in_dead_code;

        // Prevent consume of outer stack vars inside branches.
        for name in stack_vars_before.keys() {
            if let Some(count) = self.remaining_reads.get_mut(name) {
                *count = usize::MAX;
            }
        }

        let first_halts = Self::expr_definitely_halts(first_body);
        let second_halts = Self::expr_definitely_halts(second_body);

        // Don't propagate halting_context into branches: inner LetBinds that skip
        // cleanup leak stack slots, inflating depth so outer vars exceed DUP16.
        // The continuation after the if already benefits from halting_context.
        let outer_halting = self.halting_context;
        self.halting_context = false;
        self.compile_expr(first_body);
        self.halting_context = outer_halting;
        self.asm.emit(AsmInstruction::JumpTo(end_label.clone())); // net 0

        let depth_after_first = self.stack_depth;
        let stack_vars_after_first = self.stack_vars.clone();
        let let_bindings_after_first = self.let_bindings.clone();
        let free_slots_after_first = self.free_slots.clone();
        let remaining_reads_after_first = self.remaining_reads.clone();

        // Restore all compiler state for the second branch
        self.stack_depth = depth_before_branches;
        self.stack_vars = stack_vars_before.clone();
        self.let_bindings = let_bindings_before;
        self.free_slots = free_slots_before;
        self.remaining_reads = remaining_reads_before;
        self.in_dead_code = was_dead;
        // Re-apply MAX protection for outer vars in second branch
        for name in stack_vars_before.keys() {
            if let Some(count) = self.remaining_reads.get_mut(name) {
                *count = usize::MAX;
            }
        }

        self.asm.emit(AsmInstruction::Label(else_label));
        self.halting_context = false;
        self.compile_expr(second_body);
        self.halting_context = outer_halting;

        let depth_after_second = self.stack_depth;

        // Reconcile stack depths and variable state across branches:
        // - If one branch halts, its state is irrelevant — use the other's.
        // - If neither halts, they must match.
        if first_halts && !second_halts {
            // Use second branch's state (first never reaches end label)
            self.stack_depth = depth_after_second;
        } else if second_halts && !first_halts {
            // Use first branch's state (second never reaches end label)
            self.stack_depth = depth_after_first;
            self.stack_vars = stack_vars_after_first;
            self.let_bindings = let_bindings_after_first;
            self.free_slots = free_slots_after_first;
            self.remaining_reads = remaining_reads_after_first;
        } else if !first_halts && !second_halts {
            debug_assert_eq!(
                depth_after_second, depth_after_first,
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
            EvmExpr::Concat(a, b) => {
                Self::expr_definitely_halts(a) || Self::expr_definitely_halts(b)
            }
            EvmExpr::If(_, _, then_body, else_body) => {
                Self::expr_definitely_halts(then_body) && Self::expr_definitely_halts(else_body)
            }
            EvmExpr::LetBind(_, init, body) => {
                Self::expr_definitely_halts(init) || Self::expr_definitely_halts(body)
            }
            EvmExpr::VarStore(_, val) => Self::expr_definitely_halts(val),
            _ => false,
        }
    }

    /// Count how many times `Var(name)` appears in an expression tree.
    /// Skips state parameters (same positions codegen skips) for accuracy.
    fn count_var_reads(name: &str, expr: &RcExpr) -> usize {
        match expr.as_ref() {
            EvmExpr::Var(n) => {
                if n == name {
                    1
                } else {
                    0
                }
            }
            EvmExpr::Concat(a, b) | EvmExpr::DoWhile(a, b) => {
                Self::count_var_reads(name, a) + Self::count_var_reads(name, b)
            }
            EvmExpr::Bop(op, a, b) => {
                use EvmBinaryOp::*;
                let b_is_state = matches!(op, SLoad | TLoad | MLoad | CalldataLoad);
                Self::count_var_reads(name, a)
                    + if b_is_state {
                        0
                    } else {
                        Self::count_var_reads(name, b)
                    }
            }
            EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => Self::count_var_reads(name, a),
            EvmExpr::Top(op, a, b, c) => {
                use EvmTernaryOp::*;
                let c_is_state = matches!(op, SStore | TStore | MStore | MStore8 | Keccak256);
                Self::count_var_reads(name, a)
                    + Self::count_var_reads(name, b)
                    + if c_is_state {
                        0
                    } else {
                        Self::count_var_reads(name, c)
                    }
            }
            EvmExpr::Revert(a, b, _s) | EvmExpr::ReturnOp(a, b, _s) => {
                Self::count_var_reads(name, a) + Self::count_var_reads(name, b)
            }
            EvmExpr::If(c, _i, t, e) => {
                Self::count_var_reads(name, c)
                    + Self::count_var_reads(name, t)
                    + Self::count_var_reads(name, e)
            }
            EvmExpr::LetBind(n, init, body) => {
                Self::count_var_reads(name, init)
                    + if n == name {
                        0
                    } else {
                        Self::count_var_reads(name, body)
                    }
            }
            EvmExpr::VarStore(_, val) => Self::count_var_reads(name, val),
            EvmExpr::Log(_, topics, data_offset, data_size, _state) => {
                topics
                    .iter()
                    .map(|t| Self::count_var_reads(name, t))
                    .sum::<usize>()
                    + Self::count_var_reads(name, data_offset)
                    + Self::count_var_reads(name, data_size)
            }
            EvmExpr::EnvRead1(_, arg, _) => Self::count_var_reads(name, arg),
            EvmExpr::Call(_, args) => args.iter().map(|a| Self::count_var_reads(name, a)).sum(),
            EvmExpr::Function(_, _, _, body) => Self::count_var_reads(name, body),
            EvmExpr::InlineAsm(inputs, ..) => {
                inputs.iter().map(|i| Self::count_var_reads(name, i)).sum()
            }
            EvmExpr::ExtCall(a, b, c, d, e, f, _g) => [a, b, c, d, e, f]
                .iter()
                .map(|x| Self::count_var_reads(name, x))
                .sum(),
            _ => 0,
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
            EvmType::Base(_) | EvmType::ArrayT(..) => 1,
        }
    }

    /// Estimate how many stack values an expression pushes.
    ///
    /// Must be accurate for stack-mode `LetBind` cleanup (SWAP+POP).
    /// Uses signed arithmetic internally to handle `InlineAsm`'s negative deltas
    /// (it consumes sibling values pushed by Concat).
    fn count_stack_values(expr: &EvmExpr) -> usize {
        Self::count_stack_values_signed(expr).max(0) as usize
    }

    fn count_stack_values_signed(expr: &EvmExpr) -> i32 {
        match expr {
            EvmExpr::Concat(a, b) => {
                Self::count_stack_values_signed(a) + Self::count_stack_values_signed(b)
            }
            EvmExpr::Empty(_, _)
            | EvmExpr::VarStore(_, _)
            | EvmExpr::Drop(_)
            | EvmExpr::Revert(_, _, _)
            | EvmExpr::ReturnOp(_, _, _)
            | EvmExpr::Log(_, _, _, _, _)
            | EvmExpr::Function(_, _, _, _)
            | EvmExpr::DoWhile(_, _)
            | EvmExpr::StorageField(_, _, _) => 0,
            EvmExpr::Arg(ty, _) => Self::type_slot_count(ty) as i32,
            EvmExpr::LetBind(_, _, body) => Self::count_stack_values_signed(body),
            // Side-effect ternary ops push nothing onto the stack
            EvmExpr::Top(op, _, _, _) => match op {
                EvmTernaryOp::SStore
                | EvmTernaryOp::TStore
                | EvmTernaryOp::MStore
                | EvmTernaryOp::MStore8
                | EvmTernaryOp::CalldataCopy
                | EvmTernaryOp::Mcopy => 0,
                EvmTernaryOp::Keccak256 | EvmTernaryOp::Select => 1,
            },
            // If: both branches should push the same count
            EvmExpr::If(_, _, then_body, _) => Self::count_stack_values_signed(then_body),
            // InlineAsm: net delta = num_outputs - num_inputs
            EvmExpr::InlineAsm(inputs, _, num_outputs) => *num_outputs - inputs.len() as i32,
            // Everything else pushes 1 value (Var, Bop, Uop, Const, etc.)
            _ => 1,
        }
    }
}

/// Simulates `LetBind` memory slot allocation to compute peak `next_let_offset`.
///
/// Mirrors the allocation behavior in `ExprCompiler::compile_let_bind` and
/// the save/restore behavior in `compile_if` (does NOT restore `next_let_offset`
/// across if branches) and `compile_expr` for `Function` (DOES restore).
struct LetOffsetSim<'a> {
    let_bindings: HashMap<String, usize>,
    free_slots: Vec<usize>,
    next_let_offset: usize,
    peak: usize,
    allocation_modes: &'a HashMap<String, VarAllocation>,
    stack_var_count: usize,
    visited: HashSet<usize>,
}

impl<'a> LetOffsetSim<'a> {
    fn alloc_mode(&self, name: &str) -> AllocationMode {
        self.allocation_modes
            .get(name)
            .map(|a| a.mode)
            .unwrap_or(AllocationMode::Memory)
    }

    fn is_memory_mode(&self, name: &str) -> bool {
        self.alloc_mode(name) == AllocationMode::Memory || self.stack_var_count >= 14
    }

    fn walk(&mut self, expr: &RcExpr) {
        if !self.visited.insert(Rc::as_ptr(expr) as usize) {
            return;
        }
        match expr.as_ref() {
            EvmExpr::LetBind(name, init, body) => {
                self.walk(init);
                if self.is_memory_mode(name) {
                    // Allocate a memory slot
                    let offset = if let Some(reused) = self.free_slots.pop() {
                        tracing::trace!(
                            "LetOffsetSim: {name} → reused slot {reused}, next={}, peak={}",
                            self.next_let_offset,
                            self.peak
                        );
                        reused
                    } else {
                        let off = self.next_let_offset;
                        self.next_let_offset += 32;
                        if self.next_let_offset > self.peak {
                            self.peak = self.next_let_offset;
                        }
                        tracing::trace!(
                            "LetOffsetSim: {name} → new slot {off}, next={}, peak={}",
                            self.next_let_offset,
                            self.peak
                        );
                        off
                    };
                    self.let_bindings.insert(name.clone(), offset);
                    self.walk(body);
                    // Free slot if not already freed by Drop
                    if self.let_bindings.get(name) == Some(&offset) {
                        self.free_slots.push(offset);
                    }
                    self.let_bindings.remove(name);
                } else {
                    self.stack_var_count += 1;
                    self.walk(body);
                    self.stack_var_count = self.stack_var_count.saturating_sub(1);
                }
            }
            EvmExpr::Drop(name) => {
                if let Some(offset) = self.let_bindings.remove(name) {
                    self.free_slots.push(offset);
                } else {
                    // Stack mode drop — decrement count
                    self.stack_var_count = self.stack_var_count.saturating_sub(1);
                }
            }
            EvmExpr::If(_, cond, then_body, else_body) => {
                self.walk(cond);
                // Save state for branches (matching compile_if behavior)
                let saved_bindings = self.let_bindings.clone();
                let saved_free = self.free_slots.clone();
                let saved_stack_count = self.stack_var_count;
                self.walk(then_body);
                // Restore for else branch (but NOT next_let_offset!)
                self.let_bindings = saved_bindings;
                self.free_slots = saved_free;
                self.stack_var_count = saved_stack_count;
                self.walk(else_body);
            }
            EvmExpr::Function(_, _, _, body) => {
                // Functions save/restore everything including next_let_offset
                let saved_bindings = self.let_bindings.clone();
                let saved_free = self.free_slots.clone();
                let saved_offset = self.next_let_offset;
                let saved_stack_count = self.stack_var_count;
                self.let_bindings.clear();
                self.stack_var_count = 0;
                self.walk(body);
                self.let_bindings = saved_bindings;
                self.free_slots = saved_free;
                self.next_let_offset = saved_offset;
                self.stack_var_count = saved_stack_count;
            }
            // Recurse into children for everything else
            EvmExpr::Concat(a, b)
            | EvmExpr::Bop(_, a, b)
            | EvmExpr::DoWhile(a, b)
            | EvmExpr::EnvRead1(_, a, b) => {
                self.walk(a);
                self.walk(b);
            }
            EvmExpr::VarStore(_, val) => self.walk(val),
            EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
                self.walk(a);
                self.walk(b);
                self.walk(c);
            }
            EvmExpr::Uop(_, a)
            | EvmExpr::DynAlloc(a)
            | EvmExpr::AllocRegion(_, a, _)
            | EvmExpr::Get(a, _)
            | EvmExpr::EnvRead(_, a) => self.walk(a),
            EvmExpr::RegionStore(_, _, val, state) => {
                self.walk(val);
                self.walk(state);
            }
            EvmExpr::RegionLoad(_, _, state) => self.walk(state),
            EvmExpr::Log(_, topics, offset, size, state) => {
                for t in topics {
                    self.walk(t);
                }
                self.walk(offset);
                self.walk(size);
                self.walk(state);
            }
            EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
                self.walk(a);
                self.walk(b);
                self.walk(c);
                self.walk(d);
                self.walk(e);
                self.walk(f);
                self.walk(g);
            }
            EvmExpr::Call(_, args) => {
                for a in args {
                    self.walk(a);
                }
            }
            EvmExpr::InlineAsm(inputs, _, _) => {
                for i in inputs {
                    self.walk(i);
                }
            }
            // Leaf nodes — nothing to recurse into
            EvmExpr::Const(..)
            | EvmExpr::Var(_)
            | EvmExpr::Arg(_, _)
            | EvmExpr::Empty(_, _)
            | EvmExpr::StorageField(_, _, _)
            | EvmExpr::MemRegion(_, _)
            | EvmExpr::Selector(_) => {}
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
