//! AST to IR lowering

use alloy_primitives::Selector;
use edge_ast::{
    expr::Expr,
    lit::Lit,
    op::{BinOp, UnaryOp},
    pattern::MatchPattern,
    stmt::{BlockItem, CodeBlock, LoopItem, Stmt},
    Program,
};
use indexmap::IndexMap;
use tiny_keccak::{Hasher, Keccak};

use crate::{
    instruction::IrInstruction,
    program::{IrContract, IrFunction, IrProgram},
};

/// Error type for lowering failures
#[derive(Debug, thiserror::Error)]
pub enum LowerError {
    /// Unsupported expression
    #[error("unsupported expression: {0}")]
    UnsupportedExpr(String),
    /// Unsupported statement
    #[error("unsupported statement: {0}")]
    UnsupportedStmt(String),
    /// Undefined variable
    #[error("undefined variable: {0}")]
    UndefinedVariable(String),
}

/// Storage slot information passed into the lowerer
#[derive(Debug, Clone)]
pub struct StorageSlots {
    /// Map from field name to u32 storage slot index
    pub slots: IndexMap<String, u32>,
}

/// Function metadata passed into the lowerer
#[derive(Debug, Clone)]
pub struct FnMeta {
    /// Function name
    pub name: String,
    /// 4-byte EVM ABI selector
    pub selector: Selector,
    /// Whether function is publicly callable
    pub is_pub: bool,
    /// Parameter names in order
    pub params: Vec<String>,
}

/// Event metadata for emit → LOG lowering
#[derive(Debug, Clone)]
pub struct EventMeta {
    /// Event name
    pub name: String,
    /// keccak256 of the event signature string, e.g. "Transfer(address,address,uint256)"
    pub sig_hash: [u8; 32],
    /// Number of indexed fields (topics after the sig hash)
    pub indexed_count: u8,
    /// Total number of fields
    pub total_fields: usize,
}

/// Context for lowering a single function
struct FnContext {
    /// Storage slots: `field_name` → slot number
    storage_slots: IndexMap<String, u32>,
    /// Local variables: `var_name` → memory offset (multiple of 32)
    locals: IndexMap<String, u64>,
    /// Next available memory offset for a new local
    next_mem_offset: u64,
    /// Emitted instructions so far
    instructions: Vec<IrInstruction>,
    /// Counter for generating unique labels within this function
    label_counter: u32,
    /// Function name prefix used to namespace labels across functions
    fn_name: String,
    /// Stack of (`loop_start_label`, `loop_end_label`) for break/continue resolution
    loop_stack: Vec<(String, String)>,
}

impl FnContext {
    /// Create a new function context.  `fn_name` is used to namespace labels so
    /// that identical label names (e.g. `if_skip_0`) in different functions do
    /// not collide when the global assembler resolves all labels at once.
    fn new(storage_slots: IndexMap<String, u32>, fn_name: &str) -> Self {
        Self {
            storage_slots,
            locals: IndexMap::new(),
            next_mem_offset: 0,
            instructions: Vec::new(),
            label_counter: 0,
            fn_name: fn_name.to_string(),
            loop_stack: Vec::new(),
        }
    }

    /// Allocate a memory slot for a local variable
    fn alloc_local(&mut self, name: &str) -> u64 {
        let offset = self.next_mem_offset;
        self.locals.insert(name.to_string(), offset);
        self.next_mem_offset += 32;
        offset
    }

    /// Emit an instruction
    fn emit(&mut self, instr: IrInstruction) {
        self.instructions.push(instr);
    }

    /// Emit a PUSH for a u32 value (slot number, memory offset, etc.)
    fn emit_push_u32(&mut self, value: u32) {
        let bytes = if value == 0 {
            vec![0u8]
        } else {
            let b = value.to_be_bytes();
            let skip = b.iter().position(|&x| x != 0).unwrap_or(3);
            b[skip..].to_vec()
        };
        self.emit(IrInstruction::Push(bytes));
    }

    /// Emit a PUSH for a u64 value
    fn emit_push_u64(&mut self, value: u64) {
        let bytes = if value == 0 {
            vec![0u8]
        } else {
            let b = value.to_be_bytes();
            let skip = b.iter().position(|&x| x != 0).unwrap_or(7);
            b[skip..].to_vec()
        };
        self.emit(IrInstruction::Push(bytes));
    }

    /// Generate a unique label namespaced to this function to avoid collisions
    /// across functions in the global assembler label map.
    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}_{}", self.fn_name, prefix, self.label_counter);
        self.label_counter += 1;
        label
    }
}

/// The IR lowerer
#[derive(Debug)]
pub struct Lowerer {
    /// Storage slot assignments per field name
    pub storage_slots: IndexMap<String, u32>,
    /// Function metadata for selector computation
    pub fn_metas: Vec<FnMeta>,
    /// Event metadata for emit → LOG lowering
    pub event_metas: Vec<EventMeta>,
}

impl Lowerer {
    /// Create a new lowerer
    pub const fn new(
        storage_slots: IndexMap<String, u32>,
        fn_metas: Vec<FnMeta>,
        event_metas: Vec<EventMeta>,
    ) -> Self {
        Self {
            storage_slots,
            fn_metas,
            event_metas,
        }
    }

    /// Compute keccak256 of a byte slice (for event sig hashes at compile time)
    fn keccak256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Keccak::v256();
        hasher.update(data);
        let mut out = [0u8; 32];
        hasher.finalize(&mut out);
        out
    }

    /// Lower an entire program to IR
    pub fn lower(&self, program: &Program) -> Result<IrProgram, LowerError> {
        let mut contracts = Vec::new();

        for stmt in &program.stmts {
            if let Stmt::ContractDecl(contract) = stmt {
                contracts.push(self.lower_contract(contract)?);
            }
        }

        // If no contracts found, lower top-level functions into a synthetic contract
        if contracts.is_empty() {
            if let Some(synthetic_contract) = self.lower_toplevel_functions(program)? {
                contracts.push(synthetic_contract);
            }
        }

        Ok(IrProgram { contracts })
    }

    /// Lower only a single named contract from the program to IR.
    ///
    /// Use this instead of [`Self::lower`] when each contract has its own storage
    /// slots and `fn_metas` — avoids cross-contamination in multi-contract files.
    pub fn lower_one(&self, program: &Program, target_name: &str) -> Result<IrProgram, LowerError> {
        let mut contracts = Vec::new();

        for stmt in &program.stmts {
            if let Stmt::ContractDecl(contract) = stmt {
                if contract.name.name == target_name {
                    contracts.push(self.lower_contract(contract)?);
                    break;
                }
            }
        }

        // Fall back to top-level functions when the target isn't a contract
        if contracts.is_empty() {
            if let Some(synthetic_contract) = self.lower_toplevel_functions(program)? {
                contracts.push(synthetic_contract);
            }
        }

        Ok(IrProgram { contracts })
    }

    /// Lower top-level functions into a synthetic __module__ contract
    fn lower_toplevel_functions(
        &self,
        program: &Program,
    ) -> Result<Option<IrContract>, LowerError> {
        let mut functions = Vec::new();

        for stmt in &program.stmts {
            if let Stmt::FnAssign(fn_decl, body) = stmt {
                let fn_name = &fn_decl.name.name;

                // Find metadata for this function
                let meta = self
                    .fn_metas
                    .iter()
                    .find(|m| &m.name == fn_name)
                    .ok_or_else(|| {
                        LowerError::UndefinedVariable(format!(
                            "function not in metadata: {fn_name}"
                        ))
                    })?;

                let ir_fn = self.lower_fn_body_with_params(
                    fn_name,
                    meta.selector,
                    meta.is_pub,
                    body,
                    &meta.params,
                )?;
                functions.push(ir_fn);
            }
        }

        if functions.is_empty() {
            return Ok(None);
        }

        Ok(Some(IrContract {
            name: "__module__".to_string(),
            functions,
        }))
    }

    fn lower_contract(&self, contract: &edge_ast::ContractDecl) -> Result<IrContract, LowerError> {
        let mut functions = Vec::new();

        for fn_decl in &contract.functions {
            let fn_name = &fn_decl.name.name;

            // Find metadata for this function. Private functions may not appear
            // in the typechecked fn_metas list (no ABI selector assigned); fall
            // back to a zero selector and is_pub=false so compilation continues.
            let (selector, is_pub) = self
                .fn_metas
                .iter()
                .find(|m| &m.name == fn_name)
                .map(|m| (m.selector, m.is_pub))
                .unwrap_or((Selector::ZERO, false));

            let ir_fn =
                self.lower_fn_body(fn_name, selector, is_pub, &fn_decl.body, Some(fn_decl))?;
            functions.push(ir_fn);
        }

        Ok(IrContract {
            name: contract.name.name.clone(),
            functions,
        })
    }

    fn lower_fn_body(
        &self,
        _fn_name: &str,
        _selector: Selector,
        _is_pub: bool,
        body: &CodeBlock,
        fn_decl: Option<&edge_ast::ContractFnDecl>,
    ) -> Result<IrFunction, LowerError> {
        let mut ctx = FnContext::new(self.storage_slots.clone(), _fn_name);

        // Load function parameters from calldata at the start of the function.
        // Calldata layout: [0:4] = selector, [4:36] = arg0, [36:68] = arg1, etc.
        if let Some(fn_decl) = fn_decl {
            for (i, (param_ident, _param_type)) in fn_decl.params.iter().enumerate() {
                let param_name = &param_ident.name;
                // Allocate memory slot for this parameter
                let mem_offset = ctx.alloc_local(param_name);

                // Load from calldata at offset 4 + 32*i
                let calldata_offset = 4 + 32 * i as u32;
                ctx.emit_push_u32(calldata_offset);
                ctx.emit(IrInstruction::CallDataLoad);

                // Store to memory
                ctx.emit_push_u64(mem_offset);
                ctx.emit(IrInstruction::MStore);
            }
        }

        for item in &body.stmts {
            self.lower_block_item(&mut ctx, item)?;
        }

        Ok(IrFunction {
            name: _fn_name.to_string(),
            selector: _selector,
            is_pub: _is_pub,
            body: ctx.instructions,
            local_mem_size: ctx.next_mem_offset,
        })
    }

    /// Like `lower_fn_body` but takes param names directly (for top-level fns).
    fn lower_fn_body_with_params(
        &self,
        fn_name: &str,
        selector: Selector,
        is_pub: bool,
        body: &CodeBlock,
        params: &[String],
    ) -> Result<IrFunction, LowerError> {
        let mut ctx = FnContext::new(self.storage_slots.clone(), fn_name);

        for (i, param_name) in params.iter().enumerate() {
            let mem_offset = ctx.alloc_local(param_name);
            let calldata_offset = 4 + 32 * i as u32;
            ctx.emit_push_u32(calldata_offset);
            ctx.emit(IrInstruction::CallDataLoad);
            ctx.emit_push_u64(mem_offset);
            ctx.emit(IrInstruction::MStore);
        }

        for item in &body.stmts {
            self.lower_block_item(&mut ctx, item)?;
        }

        Ok(IrFunction {
            name: fn_name.to_string(),
            selector,
            is_pub,
            body: ctx.instructions,
            local_mem_size: ctx.next_mem_offset,
        })
    }

    fn lower_stmt(&self, ctx: &mut FnContext, stmt: &Stmt) -> Result<(), LowerError> {
        match stmt {
            Stmt::VarDecl(ident, _ty, _span) => {
                ctx.alloc_local(&ident.name);
                Ok(())
            }
            Stmt::VarAssign(lhs, rhs, _span) => {
                self.lower_expr(ctx, rhs)?;
                self.lower_assign_lhs(ctx, lhs)
            }
            Stmt::Return(Some(expr), _span) => {
                self.lower_expr(ctx, expr)?;
                ctx.emit_push_u64(0);
                ctx.emit(IrInstruction::MStore);
                ctx.emit_push_u64(32);
                ctx.emit_push_u64(0);
                ctx.emit(IrInstruction::Return);
                Ok(())
            }
            Stmt::Return(None, _span) => {
                ctx.emit(IrInstruction::Stop);
                Ok(())
            }
            Stmt::Break(_) => {
                if let Some((_, end_label)) = ctx.loop_stack.last() {
                    let lbl = end_label.clone();
                    ctx.emit(IrInstruction::PushLabel(lbl));
                    ctx.emit(IrInstruction::Jump);
                }
                Ok(())
            }
            Stmt::Continue(_) => {
                if let Some((start_label, _)) = ctx.loop_stack.last() {
                    let lbl = start_label.clone();
                    ctx.emit(IrInstruction::PushLabel(lbl));
                    ctx.emit(IrInstruction::Jump);
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.lower_expr(ctx, expr)?;
                ctx.emit(IrInstruction::Pop);
                Ok(())
            }
            Stmt::ConstAssign(_, _, _)
            | Stmt::TypeAssign(_, _, _)
            | Stmt::EventDecl(_)
            | Stmt::TraitDecl(_, _)
            | Stmt::ImplBlock(_)
            | Stmt::AbiDecl(_)
            | Stmt::ModuleDecl(_)
            | Stmt::ModuleImport(_)
            | Stmt::ComptimeBranch(_)
            | Stmt::ComptimeFn(_, _)
            // ContractDecl / ContractImpl / FnAssign are handled at program level
            | Stmt::ContractDecl(_)
            | Stmt::ContractImpl(_)
            | Stmt::FnAssign(_, _) => Ok(()),
            Stmt::Emit(name, args, _span) => self.lower_emit(ctx, &name.name, args),
            Stmt::IfElse(branches, else_block) => {
                let end_label = ctx.fresh_label("if_end");

                for (cond, body) in branches {
                    let skip_label = ctx.fresh_label("if_skip");

                    self.lower_expr(ctx, cond)?;
                    ctx.emit(IrInstruction::IsZero);
                    ctx.emit(IrInstruction::PushLabel(skip_label.clone()));
                    ctx.emit(IrInstruction::JumpI);

                    for item in &body.stmts {
                        self.lower_block_item(ctx, item)?;
                    }
                    ctx.emit(IrInstruction::PushLabel(end_label.clone()));
                    ctx.emit(IrInstruction::Jump);
                    ctx.emit(IrInstruction::JumpDest(skip_label));
                }

                if let Some(else_body) = else_block {
                    for item in &else_body.stmts {
                        self.lower_block_item(ctx, item)?;
                    }
                }
                ctx.emit(IrInstruction::JumpDest(end_label));
                Ok(())
            }
            Stmt::WhileLoop(cond, body) => {
                let start_label = ctx.fresh_label("while_start");
                let end_label = ctx.fresh_label("while_end");

                ctx.emit(IrInstruction::JumpDest(start_label.clone()));
                ctx.loop_stack
                    .push((start_label.clone(), end_label.clone()));

                // Skip body if condition is false
                self.lower_expr(ctx, cond)?;
                ctx.emit(IrInstruction::IsZero);
                ctx.emit(IrInstruction::PushLabel(end_label.clone()));
                ctx.emit(IrInstruction::JumpI);

                for item in &body.items {
                    self.lower_loop_item(ctx, item)?;
                }

                // Jump back to condition
                ctx.emit(IrInstruction::PushLabel(start_label));
                ctx.emit(IrInstruction::Jump);
                ctx.emit(IrInstruction::JumpDest(end_label));
                ctx.loop_stack.pop();
                Ok(())
            }
            Stmt::ForLoop(init, cond, update, body) => {
                // Init
                if let Some(init_stmt) = init {
                    self.lower_stmt(ctx, init_stmt)?;
                }

                let start_label = ctx.fresh_label("for_start");
                let end_label = ctx.fresh_label("for_end");

                ctx.emit(IrInstruction::JumpDest(start_label.clone()));
                ctx.loop_stack
                    .push((start_label.clone(), end_label.clone()));

                // Condition
                if let Some(cond_expr) = cond {
                    self.lower_expr(ctx, cond_expr)?;
                    ctx.emit(IrInstruction::IsZero);
                    ctx.emit(IrInstruction::PushLabel(end_label.clone()));
                    ctx.emit(IrInstruction::JumpI);
                }

                // Body
                for item in &body.items {
                    self.lower_loop_item(ctx, item)?;
                }

                // Update
                if let Some(update_stmt) = update {
                    self.lower_stmt(ctx, update_stmt)?;
                }

                ctx.emit(IrInstruction::PushLabel(start_label));
                ctx.emit(IrInstruction::Jump);
                ctx.emit(IrInstruction::JumpDest(end_label));
                ctx.loop_stack.pop();
                Ok(())
            }
            Stmt::Loop(body) => {
                let start_label = ctx.fresh_label("loop_start");
                let end_label = ctx.fresh_label("loop_end");

                ctx.emit(IrInstruction::JumpDest(start_label.clone()));
                ctx.loop_stack
                    .push((start_label.clone(), end_label.clone()));

                for item in &body.items {
                    self.lower_loop_item(ctx, item)?;
                }

                ctx.emit(IrInstruction::PushLabel(start_label));
                ctx.emit(IrInstruction::Jump);
                ctx.emit(IrInstruction::JumpDest(end_label));
                ctx.loop_stack.pop();
                Ok(())
            }
            Stmt::DoWhile(body, cond) => {
                let start_label = ctx.fresh_label("dowhile_start");
                let end_label = ctx.fresh_label("dowhile_end");

                ctx.emit(IrInstruction::JumpDest(start_label.clone()));
                ctx.loop_stack
                    .push((start_label.clone(), end_label.clone()));

                for item in &body.items {
                    self.lower_loop_item(ctx, item)?;
                }

                // Loop if condition is true
                self.lower_expr(ctx, cond)?;
                ctx.emit(IrInstruction::PushLabel(start_label));
                ctx.emit(IrInstruction::JumpI);
                ctx.emit(IrInstruction::JumpDest(end_label));
                ctx.loop_stack.pop();
                Ok(())
            }
            Stmt::Match(scrutinee, arms, _) => self.lower_match(ctx, scrutinee, arms),
            Stmt::CodeBlock(block) => {
                for item in &block.stmts {
                    self.lower_block_item(ctx, item)?;
                }
                Ok(())
            }
            Stmt::IfMatch(_, _, _) => {
                // if-matches: simplified — always fall through for now
                Ok(())
            }
        }
    }

    /// Emit the store for an assignment lhs (identifier or mapping index)
    fn lower_assign_lhs(&self, ctx: &mut FnContext, lhs: &Expr) -> Result<(), LowerError> {
        match lhs {
            Expr::Ident(id) => {
                if let Some(&slot) = ctx.storage_slots.get(&id.name) {
                    ctx.emit_push_u32(slot);
                    ctx.emit(IrInstruction::SStore);
                } else if let Some(&offset) = ctx.locals.get(&id.name) {
                    ctx.emit_push_u64(offset);
                    ctx.emit(IrInstruction::MStore);
                } else {
                    return Err(LowerError::UndefinedVariable(id.name.clone()));
                }
                Ok(())
            }
            Expr::ArrayIndex(base, key, None, _) => {
                // mapping[key] = value  → keccak256(abi.encode(key, slot))
                // Stack going in: [value]
                // We need to compute the storage slot and call SSTORE.
                if let Expr::Ident(map_ident) = base.as_ref() {
                    if let Some(&base_slot) = ctx.storage_slots.get(&map_ident.name) {
                        // Dup value (we'll consume it with SSTORE)
                        ctx.emit(IrInstruction::Dup(1));
                        // Compute slot = keccak256(pad32(key) ++ pad32(base_slot))
                        self.emit_mapping_slot(ctx, key, base_slot)?;
                        // Stack: [slot, value, value]  → SSTORE(slot, value)
                        ctx.emit(IrInstruction::SStore);
                        // Stack: [value] — discard remaining copy
                        ctx.emit(IrInstruction::Pop);
                        return Ok(());
                    }
                }
                // Fallback: pop value
                ctx.emit(IrInstruction::Pop);
                Ok(())
            }
            _ => Err(LowerError::UnsupportedExpr(format!(
                "unsupported assignment lhs: {lhs:?}"
            ))),
        }
    }

    /// Emit keccak256(abi.encode(key, `base_slot`)) and leave the storage slot on the stack.
    fn emit_mapping_slot(
        &self,
        ctx: &mut FnContext,
        key: &Expr,
        base_slot: u32,
    ) -> Result<(), LowerError> {
        // Memory layout for keccak: [0..32] = key, [32..64] = slot number
        // Use scratch memory starting at a high offset to avoid clobbering locals.
        // Convention: use the top of free memory area. For safety, use offsets 0x80 and 0xa0
        // (the EVM free-memory-pointer convention).
        let key_mem = 0x80u64;
        let slot_mem = 0xa0u64;

        // Store key at key_mem
        self.lower_expr(ctx, key)?;
        ctx.emit_push_u64(key_mem);
        ctx.emit(IrInstruction::MStore);

        // Store base_slot at slot_mem
        ctx.emit_push_u32(base_slot);
        ctx.emit_push_u64(slot_mem);
        ctx.emit(IrInstruction::MStore);

        // keccak256(key_mem, 64)
        ctx.emit_push_u64(64);
        ctx.emit_push_u64(key_mem);
        ctx.emit(IrInstruction::Keccak256);
        Ok(())
    }

    /// Lower an emit statement into LOG instructions.
    fn lower_emit(
        &self,
        ctx: &mut FnContext,
        event_name: &str,
        args: &[Expr],
    ) -> Result<(), LowerError> {
        // Look up event metadata
        let meta = self.event_metas.iter().find(|e| e.name == event_name);

        let (sig_hash, indexed_count) = meta.map_or_else(
            || {
                // Unknown event: compute sig hash from name alone, treat all args as non-indexed
                let hash = Self::keccak256(event_name.as_bytes());
                (hash, 0usize)
            },
            |m| (m.sig_hash, m.indexed_count as usize),
        );

        // Separate indexed and non-indexed args
        let indexed_args = &args[..indexed_count.min(args.len())];
        let data_args = &args[indexed_count.min(args.len())..];

        // ABI-encode non-indexed args into memory at offset 0
        let mut mem_offset = 0u64;
        for arg in data_args {
            self.lower_expr(ctx, arg)?;
            ctx.emit_push_u64(mem_offset);
            ctx.emit(IrInstruction::MStore);
            mem_offset += 32;
        }

        // EVM LOGn stack layout (top = first popped):
        //   [top] mem_offset, mem_size, topic0, topic1, topic2, ... [bottom]
        //
        // So push in reverse order (first pushed = deepest = last topic):
        //   push last indexed arg, ..., first indexed arg, sig_hash, size, offset
        //
        // indexed_args[0] becomes topic1, indexed_args[1] becomes topic2, etc.
        // To get topic1 closer to the top than topic2, push topic2 first (deepest).
        for arg in indexed_args.iter().rev() {
            self.lower_expr(ctx, arg)?;
        }
        // Sig hash is topic0 — sits just below size/offset on the stack
        ctx.emit(IrInstruction::Push(sig_hash.to_vec()));

        // Size and offset go on top (offset is topmost = first popped by LOG)
        ctx.emit_push_u64(mem_offset); // data length (bytes of non-indexed args)
        ctx.emit_push_u64(0); // data memory offset

        // Total topics = sig_hash (1) + indexed args
        let n_topics = (1 + indexed_count).min(4) as u8;
        ctx.emit(IrInstruction::Log(n_topics));
        Ok(())
    }

    /// Lower a loop item (expr, stmt, break, continue)
    fn lower_loop_item(&self, ctx: &mut FnContext, item: &LoopItem) -> Result<(), LowerError> {
        match item {
            LoopItem::Stmt(stmt) => self.lower_stmt(ctx, stmt),
            LoopItem::Expr(expr) => {
                self.lower_expr(ctx, expr)?;
                ctx.emit(IrInstruction::Pop);
                Ok(())
            }
            LoopItem::Break(_) => {
                if let Some((_, end_label)) = ctx.loop_stack.last() {
                    let lbl = end_label.clone();
                    ctx.emit(IrInstruction::PushLabel(lbl));
                    ctx.emit(IrInstruction::Jump);
                }
                Ok(())
            }
            LoopItem::Continue(_) => {
                if let Some((start_label, _)) = ctx.loop_stack.last() {
                    let lbl = start_label.clone();
                    ctx.emit(IrInstruction::PushLabel(lbl));
                    ctx.emit(IrInstruction::Jump);
                }
                Ok(())
            }
        }
    }

    /// Lower a match statement.
    fn lower_match(
        &self,
        ctx: &mut FnContext,
        scrutinee: &Expr,
        arms: &[edge_ast::pattern::MatchArm],
    ) -> Result<(), LowerError> {
        let end_label = ctx.fresh_label("match_end");

        // Evaluate scrutinee once; keep a copy on the stack for each arm comparison.
        self.lower_expr(ctx, scrutinee)?;
        // Stack: [discriminant]

        let arm_labels: Vec<String> = arms
            .iter()
            .enumerate()
            .map(|(i, _)| ctx.fresh_label(&format!("match_arm_{i}")))
            .collect();

        let wildcard_label = ctx.fresh_label("match_wildcard");

        for (i, arm) in arms.iter().enumerate() {
            match &arm.pattern {
                MatchPattern::Wildcard | MatchPattern::Ident(_) => {
                    // Wildcard / catch-all: always jump to this arm's body
                    ctx.emit(IrInstruction::PushLabel(arm_labels[i].clone()));
                    ctx.emit(IrInstruction::Jump);
                    break;
                }
                MatchPattern::Union(up) => {
                    // Compare discriminant against the variant index (implicit integer index).
                    // Each member of the union gets an integer tag equal to its position.
                    // For now we use the arm index as the discriminant value.
                    ctx.emit(IrInstruction::Dup(1)); // dup discriminant
                    ctx.emit_push_u32(i as u32); // expected tag
                    ctx.emit(IrInstruction::Eq);
                    ctx.emit(IrInstruction::PushLabel(arm_labels[i].clone()));
                    ctx.emit(IrInstruction::JumpI);

                    // Bind inner value if the pattern has bindings (pop discriminant into local)
                    let _ = up; // binding extraction handled inside arm body for now
                }
            }
        }

        // No match → jump past all arms
        ctx.emit(IrInstruction::PushLabel(wildcard_label.clone()));
        ctx.emit(IrInstruction::Jump);

        // Emit arm bodies
        for (i, arm) in arms.iter().enumerate() {
            ctx.emit(IrInstruction::JumpDest(arm_labels[i].clone()));
            // Pop the discriminant before executing the arm body
            ctx.emit(IrInstruction::Pop);

            // For union patterns with bindings, extract inner value (future work)

            for item in &arm.body.stmts {
                self.lower_block_item(ctx, item)?;
            }
            ctx.emit(IrInstruction::PushLabel(end_label.clone()));
            ctx.emit(IrInstruction::Jump);
        }

        ctx.emit(IrInstruction::JumpDest(wildcard_label));
        ctx.emit(IrInstruction::Pop); // pop discriminant
        ctx.emit(IrInstruction::JumpDest(end_label));
        Ok(())
    }

    fn lower_block_item(&self, ctx: &mut FnContext, item: &BlockItem) -> Result<(), LowerError> {
        match item {
            BlockItem::Stmt(stmt) => self.lower_stmt(ctx, stmt),
            BlockItem::Expr(expr) => {
                self.lower_expr(ctx, expr)?;
                ctx.emit(IrInstruction::Pop);
                Ok(())
            }
        }
    }

    fn lower_expr(&self, ctx: &mut FnContext, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Literal(lit) => match lit.as_ref() {
                Lit::Int(n, _, _) => {
                    let bytes = if *n == 0 {
                        vec![0u8]
                    } else {
                        let b = n.to_be_bytes();
                        let skip = b.iter().position(|&x| x != 0).unwrap_or(7);
                        b[skip..].to_vec()
                    };
                    ctx.emit(IrInstruction::Push(bytes));
                    Ok(())
                }
                Lit::Bool(b, _) => {
                    ctx.emit(IrInstruction::Push(vec![if *b { 1 } else { 0 }]));
                    Ok(())
                }
                Lit::Hex(bytes, _) | Lit::Bin(bytes, _) => {
                    ctx.emit(IrInstruction::Push(bytes.clone()));
                    Ok(())
                }
                Lit::Str(_, _) => {
                    ctx.emit(IrInstruction::Push(vec![0]));
                    Ok(())
                }
            },
            Expr::Ident(id) => {
                if let Some(&slot) = ctx.storage_slots.get(&id.name) {
                    ctx.emit_push_u32(slot);
                    ctx.emit(IrInstruction::SLoad);
                } else if let Some(&offset) = ctx.locals.get(&id.name) {
                    ctx.emit_push_u64(offset);
                    ctx.emit(IrInstruction::MLoad);
                } else {
                    ctx.emit(IrInstruction::Push(vec![0]));
                }
                Ok(())
            }
            Expr::Unary(op, inner, _) => {
                self.lower_expr(ctx, inner)?;
                match op {
                    UnaryOp::Neg => {
                        // 0 - inner
                        ctx.emit(IrInstruction::Push(vec![0]));
                        ctx.emit(IrInstruction::Sub);
                    }
                    UnaryOp::BitwiseNot => {
                        ctx.emit(IrInstruction::Not);
                    }
                    UnaryOp::LogicalNot => {
                        ctx.emit(IrInstruction::IsZero);
                    }
                }
                Ok(())
            }
            Expr::Binary(lhs, op, rhs, _) => {
                // Compound assignment operators: x += y  →  x = x + y
                match op {
                    BinOp::AddAssign
                    | BinOp::SubAssign
                    | BinOp::MulAssign
                    | BinOp::DivAssign
                    | BinOp::ModAssign
                    | BinOp::ExpAssign
                    | BinOp::BitwiseAndAssign
                    | BinOp::BitwiseOrAssign
                    | BinOp::BitwiseXorAssign
                    | BinOp::ShlAssign
                    | BinOp::ShrAssign => {
                        // EVM pops the top of stack first.  For commutative ops (Add,
                        // Mul, And, Or, Xor) and shift ops (Shl, Shr — which take
                        // shift=top, value=second) the current push order is correct.
                        // For non-commutative ops (Sub, Div, Mod, Exp) we need lhs on
                        // top so the EVM computes lhs op rhs, so push rhs first.
                        let needs_swap = matches!(
                            op,
                            BinOp::SubAssign
                                | BinOp::DivAssign
                                | BinOp::ModAssign
                                | BinOp::ExpAssign
                        );
                        if needs_swap {
                            self.lower_expr(ctx, rhs)?;
                            self.lower_expr(ctx, lhs)?;
                        } else {
                            self.lower_expr(ctx, lhs)?;
                            self.lower_expr(ctx, rhs)?;
                        }
                        match op {
                            BinOp::AddAssign => ctx.emit(IrInstruction::Add),
                            BinOp::SubAssign => ctx.emit(IrInstruction::Sub),
                            BinOp::MulAssign => ctx.emit(IrInstruction::Mul),
                            BinOp::DivAssign => ctx.emit(IrInstruction::Div),
                            BinOp::ModAssign => ctx.emit(IrInstruction::Mod),
                            BinOp::ExpAssign => ctx.emit(IrInstruction::Exp),
                            BinOp::BitwiseAndAssign => ctx.emit(IrInstruction::And),
                            BinOp::BitwiseOrAssign => ctx.emit(IrInstruction::Or),
                            BinOp::BitwiseXorAssign => ctx.emit(IrInstruction::Xor),
                            BinOp::ShlAssign => ctx.emit(IrInstruction::Shl),
                            BinOp::ShrAssign => ctx.emit(IrInstruction::Shr),
                            _ => unreachable!(),
                        }
                        // Stack: [result] — store into lhs
                        // DUP so the expression also leaves a value (assign returns rhs)
                        ctx.emit(IrInstruction::Dup(1));
                        self.lower_assign_lhs(ctx, lhs)?;
                        return Ok(());
                    }
                    _ => {}
                }

                // EVM pops the top of stack first.  For Sub/Div/Mod/Lt/Gt and the
                // derived Lte/Gte we need lhs on top so EVM computes lhs op rhs.
                // Push rhs first (deepest) for those ops; keep lhs-first for the rest.
                let needs_swap = matches!(
                    op,
                    BinOp::Sub
                        | BinOp::Div
                        | BinOp::Mod
                        | BinOp::Exp
                        | BinOp::Lt
                        | BinOp::Gt
                        | BinOp::Lte
                        | BinOp::Gte
                );
                if needs_swap {
                    self.lower_expr(ctx, rhs)?;
                    self.lower_expr(ctx, lhs)?;
                } else {
                    self.lower_expr(ctx, lhs)?;
                    self.lower_expr(ctx, rhs)?;
                }

                match op {
                    BinOp::Add => ctx.emit(IrInstruction::Add),
                    BinOp::Sub => ctx.emit(IrInstruction::Sub),
                    BinOp::Mul => ctx.emit(IrInstruction::Mul),
                    BinOp::Div => ctx.emit(IrInstruction::Div),
                    BinOp::Mod => ctx.emit(IrInstruction::Mod),
                    BinOp::Lt => ctx.emit(IrInstruction::Lt),
                    BinOp::Gt => ctx.emit(IrInstruction::Gt),
                    BinOp::Eq => ctx.emit(IrInstruction::Eq),
                    BinOp::Neq => {
                        ctx.emit(IrInstruction::Eq);
                        ctx.emit(IrInstruction::IsZero);
                    }
                    BinOp::Lte => {
                        ctx.emit(IrInstruction::Gt);
                        ctx.emit(IrInstruction::IsZero);
                    }
                    BinOp::Gte => {
                        ctx.emit(IrInstruction::Lt);
                        ctx.emit(IrInstruction::IsZero);
                    }
                    BinOp::BitwiseAnd | BinOp::LogicalAnd => ctx.emit(IrInstruction::And),
                    BinOp::BitwiseOr | BinOp::LogicalOr => ctx.emit(IrInstruction::Or),
                    BinOp::BitwiseXor => ctx.emit(IrInstruction::Xor),
                    BinOp::Shl => ctx.emit(IrInstruction::Shl),
                    BinOp::Shr => ctx.emit(IrInstruction::Shr),
                    BinOp::Exp => ctx.emit(IrInstruction::Exp),
                    _ => {
                        return Err(LowerError::UnsupportedExpr(format!(
                            "unsupported binary op: {op:?}"
                        )));
                    }
                }
                Ok(())
            }
            Expr::Ternary(cond, then_expr, else_expr, _) => {
                let else_label = ctx.fresh_label("ternary_else");
                let end_label = ctx.fresh_label("ternary_end");

                self.lower_expr(ctx, cond)?;
                ctx.emit(IrInstruction::IsZero);
                ctx.emit(IrInstruction::PushLabel(else_label.clone()));
                ctx.emit(IrInstruction::JumpI);

                self.lower_expr(ctx, then_expr)?;
                ctx.emit(IrInstruction::PushLabel(end_label.clone()));
                ctx.emit(IrInstruction::Jump);

                ctx.emit(IrInstruction::JumpDest(else_label));
                self.lower_expr(ctx, else_expr)?;
                ctx.emit(IrInstruction::JumpDest(end_label));
                Ok(())
            }
            Expr::At(name, args, _) => {
                match name.name.as_str() {
                    "caller" => ctx.emit(IrInstruction::Caller),
                    "callvalue" | "value" => ctx.emit(IrInstruction::CallValue),
                    "timestamp" => ctx.emit(IrInstruction::Timestamp),
                    "blocknumber" | "number" => ctx.emit(IrInstruction::Number),
                    "calldatasize" => ctx.emit(IrInstruction::CallDataSize),
                    _ => ctx.emit(IrInstruction::Push(vec![0])),
                }
                for _ in args {
                    // @builtins take no runtime arguments in the lowered form
                }
                Ok(())
            }
            Expr::Assign(lhs, rhs, _) => {
                self.lower_expr(ctx, rhs)?;
                // DUP so assign-expression leaves value on stack for caller
                ctx.emit(IrInstruction::Dup(1));
                self.lower_assign_lhs(ctx, lhs)?;
                Ok(())
            }
            Expr::ArrayIndex(base, index, None, _) => {
                // mapping[key] read → keccak256 slot then SLOAD
                if let Expr::Ident(map_ident) = base.as_ref() {
                    if let Some(&base_slot) = ctx.storage_slots.get(&map_ident.name) {
                        self.emit_mapping_slot(ctx, index, base_slot)?;
                        ctx.emit(IrInstruction::SLoad);
                        return Ok(());
                    }
                }
                // Local array or unsupported: push dummy
                self.lower_expr(ctx, base)?;
                ctx.emit(IrInstruction::Pop);
                self.lower_expr(ctx, index)?;
                ctx.emit(IrInstruction::Pop);
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::ArrayIndex(base, index, Some(end), _) => {
                // Slice: evaluate for side effects, push dummy
                self.lower_expr(ctx, base)?;
                ctx.emit(IrInstruction::Pop);
                self.lower_expr(ctx, index)?;
                ctx.emit(IrInstruction::Pop);
                self.lower_expr(ctx, end)?;
                ctx.emit(IrInstruction::Pop);
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::FieldAccess(base, _field, _) => {
                // For storage structs or simple objects: push the base value.
                // Full struct field offset computation is future work.
                self.lower_expr(ctx, base)?;
                Ok(())
            }
            Expr::TupleFieldAccess(base, _idx, _) => {
                self.lower_expr(ctx, base)?;
                Ok(())
            }
            Expr::FunctionCall(func, args, _) => {
                // Evaluate args (for side effects); result is a JUMP-based call (future).
                for arg in args {
                    self.lower_expr(ctx, arg)?;
                    ctx.emit(IrInstruction::Pop);
                }
                // For now emit a placeholder 0 return value.
                // Full internal call ABI requires a call frame protocol to be designed.
                let _ = func;
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::Paren(inner, _) | Expr::Comptime(inner, _) => self.lower_expr(ctx, inner),
            Expr::Path(_, _) | Expr::UnionInstantiation(_, _, _, _) => {
                // Union/path expressions in value position push their discriminant tag.
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::StructInstantiation(_, _, fields, _) => {
                // Evaluate field expressions for side effects; push 0 as placeholder value.
                for (_, field_expr) in fields {
                    self.lower_expr(ctx, field_expr)?;
                    ctx.emit(IrInstruction::Pop);
                }
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::TupleInstantiation(_, elems, _) | Expr::ArrayInstantiation(_, elems, _) => {
                for elem in elems {
                    self.lower_expr(ctx, elem)?;
                    ctx.emit(IrInstruction::Pop);
                }
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::ArrowFunction(_, _, _) => {
                // Closures are not directly lowerable to EVM; push 0 placeholder.
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::PatternMatch(scrutinee, _pattern, _) => {
                // `if expr matches Pattern` — evaluate the scrutinee, push 1 (always matches for MVP)
                self.lower_expr(ctx, scrutinee)?;
                ctx.emit(IrInstruction::Pop);
                ctx.emit(IrInstruction::Push(vec![1]));
                Ok(())
            }
        }
    }
}
