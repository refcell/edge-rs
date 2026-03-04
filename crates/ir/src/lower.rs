//! AST to IR lowering

use alloy_primitives::Selector;
use edge_ast::{
    expr::Expr,
    lit::Lit,
    op::BinOp,
    stmt::{BlockItem, CodeBlock, Stmt},
    Program,
};
use indexmap::IndexMap;

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
}

/// Context for lowering a single function
#[allow(dead_code)]
struct FnContext {
    /// Storage slots: `field_name` → slot number
    storage_slots: IndexMap<String, u32>,
    /// Local variables: `var_name` → memory offset (multiple of 32)
    locals: IndexMap<String, u64>,
    /// Next available memory offset for a new local
    next_mem_offset: u64,
    /// Emitted instructions so far
    instructions: Vec<IrInstruction>,
    /// Counter for generating unique labels
    label_counter: u32,
}

#[allow(dead_code)]
impl FnContext {
    /// Create a new function context
    fn new(storage_slots: IndexMap<String, u32>) -> Self {
        Self {
            storage_slots,
            locals: IndexMap::new(),
            next_mem_offset: 0,
            instructions: Vec::new(),
            label_counter: 0,
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

    /// Generate a unique label
    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_counter);
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
}

#[allow(dead_code)]
impl Lowerer {
    /// Create a new lowerer
    pub const fn new(storage_slots: IndexMap<String, u32>, fn_metas: Vec<FnMeta>) -> Self {
        Self {
            storage_slots,
            fn_metas,
        }
    }

    /// Lower an entire program to IR
    pub fn lower(&self, program: &Program) -> Result<IrProgram, LowerError> {
        let mut contracts = Vec::new();

        for stmt in &program.stmts {
            if let Stmt::ContractDecl(contract) = stmt {
                contracts.push(self.lower_contract(contract)?);
            }
        }

        Ok(IrProgram { contracts })
    }

    fn lower_contract(&self, contract: &edge_ast::ContractDecl) -> Result<IrContract, LowerError> {
        let mut functions = Vec::new();

        for fn_decl in &contract.functions {
            let fn_name = &fn_decl.name.name;

            // Find metadata for this function
            let meta = self
                .fn_metas
                .iter()
                .find(|m| &m.name == fn_name)
                .ok_or_else(|| {
                    LowerError::UndefinedVariable(format!("function not in metadata: {fn_name}"))
                })?;

            let ir_fn = self.lower_fn_body(fn_name, meta.selector, meta.is_pub, &fn_decl.body)?;
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
    ) -> Result<IrFunction, LowerError> {
        let mut ctx = FnContext::new(self.storage_slots.clone());

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

    fn lower_stmt(&self, ctx: &mut FnContext, stmt: &Stmt) -> Result<(), LowerError> {
        match stmt {
            Stmt::VarDecl(ident, _ty, _span) => {
                ctx.alloc_local(&ident.name);
                Ok(())
            }
            Stmt::VarAssign(lhs, rhs, _span) => {
                // Lower rhs to put value on stack
                self.lower_expr(ctx, rhs)?;

                // Store to lhs
                match lhs {
                    Expr::Ident(id) => {
                        if let Some(&slot) = ctx.storage_slots.get(&id.name) {
                            // Storage variable
                            ctx.emit_push_u32(slot);
                            ctx.emit(IrInstruction::SStore);
                        } else if let Some(&offset) = ctx.locals.get(&id.name) {
                            // Local variable
                            ctx.emit_push_u64(offset);
                            ctx.emit(IrInstruction::MStore);
                        } else {
                            return Err(LowerError::UndefinedVariable(id.name.clone()));
                        }
                        Ok(())
                    }
                    Expr::ArrayIndex(base, index, None, _) => {
                        // Storage mapping access: map[key] = value
                        // For MVP: just pop and continue
                        ctx.emit(IrInstruction::Pop);
                        self.lower_expr(ctx, base)?;
                        self.lower_expr(ctx, index)?;
                        ctx.emit(IrInstruction::Pop);
                        ctx.emit(IrInstruction::Pop);
                        Ok(())
                    }
                    _ => Err(LowerError::UnsupportedExpr(format!(
                        "unsupported lhs: {lhs:?}"
                    ))),
                }
            }
            Stmt::Return(Some(expr), _span) => {
                // Lower expr to get return value on stack
                self.lower_expr(ctx, expr)?;

                // Store at mem[0]
                ctx.emit_push_u64(0);
                ctx.emit(IrInstruction::MStore);

                // Return 32 bytes from mem[0]
                ctx.emit_push_u64(32);
                ctx.emit_push_u64(0);
                ctx.emit(IrInstruction::Return);
                Ok(())
            }
            Stmt::Return(None, _span) => {
                ctx.emit(IrInstruction::Stop);
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.lower_expr(ctx, expr)?;
                ctx.emit(IrInstruction::Pop);
                Ok(())
            }
            Stmt::ConstAssign(_, _, _) => {
                // Skip constants (handled by typeck)
                Ok(())
            }
            Stmt::Emit(_name, args, _span) => {
                // Evaluate all arguments then discard them (full LOG encoding is future work)
                for arg in args {
                    self.lower_expr(ctx, arg)?;
                    ctx.emit(IrInstruction::Pop);
                }
                Ok(())
            }
            Stmt::IfElse(branches, else_block) => {
                // Generate if/else if/else chain with jump labels
                let end_label = ctx.fresh_label("if_end");

                for (cond, body) in branches {
                    let skip_label = ctx.fresh_label("if_skip");

                    // EVM JUMPI pops: top = destination, second = condition.
                    // We want to skip the body if cond == 0, so invert cond first.
                    // Stack after: [skip_label (top), !cond] → JUMPI skips if cond==0.
                    self.lower_expr(ctx, cond)?;
                    ctx.emit(IrInstruction::IsZero);
                    ctx.emit(IrInstruction::PushLabel(skip_label.clone()));
                    ctx.emit(IrInstruction::JumpI);

                    // True body
                    for item in &body.stmts {
                        self.lower_block_item(ctx, item)?;
                    }
                    // Jump to end of entire if chain
                    ctx.emit(IrInstruction::PushLabel(end_label.clone()));
                    ctx.emit(IrInstruction::Jump);

                    // Skip label destination
                    ctx.emit(IrInstruction::JumpDest(skip_label));
                }

                // Else block (optional)
                if let Some(else_body) = else_block {
                    for item in &else_body.stmts {
                        self.lower_block_item(ctx, item)?;
                    }
                }

                ctx.emit(IrInstruction::JumpDest(end_label));
                Ok(())
            }
            _ => Err(LowerError::UnsupportedStmt(format!(
                "unsupported statement: {stmt:?}"
            ))),
        }
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
            Expr::Literal(lit) => {
                match lit.as_ref() {
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
                        // Not supported for MVP
                        ctx.emit(IrInstruction::Push(vec![0]));
                        Ok(())
                    }
                }
            }
            Expr::Ident(id) => {
                // Load variable
                if let Some(&slot) = ctx.storage_slots.get(&id.name) {
                    // Storage variable
                    ctx.emit_push_u32(slot);
                    ctx.emit(IrInstruction::SLoad);
                } else if let Some(&offset) = ctx.locals.get(&id.name) {
                    // Local variable
                    ctx.emit_push_u64(offset);
                    ctx.emit(IrInstruction::MLoad);
                } else {
                    // Fallback: push zero
                    ctx.emit(IrInstruction::Push(vec![0]));
                }
                Ok(())
            }
            Expr::Binary(lhs, op, rhs, _) => {
                // Lower lhs, then rhs
                self.lower_expr(ctx, lhs)?;
                self.lower_expr(ctx, rhs)?;

                // Emit the operator
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
                    BinOp::Exp => {
                        // EXP not in our instruction set yet; emit placeholder
                        ctx.emit(IrInstruction::Push(vec![0]));
                    }
                    _ => {
                        // Compound assignment operators handled elsewhere
                        return Err(LowerError::UnsupportedExpr(format!(
                            "unsupported binary op: {op:?}"
                        )));
                    }
                }
                Ok(())
            }
            Expr::At(name, args, _) => {
                // Builtin calls like @caller(), @value(), etc.
                match name.name.as_str() {
                    "caller" => ctx.emit(IrInstruction::Caller),
                    "value" => ctx.emit(IrInstruction::CallValue),
                    "timestamp" => ctx.emit(IrInstruction::Timestamp),
                    "blocknumber" => ctx.emit(IrInstruction::Number),
                    _ => {
                        ctx.emit(IrInstruction::Push(vec![0]));
                    }
                }
                // Pop any arguments (not supported in MVP)
                for _ in args {
                    ctx.emit(IrInstruction::Pop);
                }
                Ok(())
            }
            Expr::Assign(lhs, rhs, _) => {
                // Lower rhs — value on stack
                self.lower_expr(ctx, rhs)?;

                // Assign-expressions return their RHS value (like C's `a = b`).
                // DUP the value so one copy can be stored while one remains on stack
                // for the caller (e.g. Stmt::Expr will Pop it).
                match lhs.as_ref() {
                    Expr::Ident(id) => {
                        if let Some(&slot) = ctx.storage_slots.get(&id.name) {
                            // Stack: [val] → DUP → [val, val] → PUSH slot → [slot, val, val]
                            // SSTORE(key=slot, val) → [val]
                            ctx.emit(IrInstruction::Dup(1));
                            ctx.emit_push_u32(slot);
                            ctx.emit(IrInstruction::SStore);
                        } else if let Some(&offset) = ctx.locals.get(&id.name) {
                            // Stack: [val] → DUP → [val, val] → PUSH offset → [offset, val, val]
                            // MSTORE(offset, val) → [val]
                            ctx.emit(IrInstruction::Dup(1));
                            ctx.emit_push_u64(offset);
                            ctx.emit(IrInstruction::MStore);
                        } else {
                            return Err(LowerError::UndefinedVariable(id.name.clone()));
                        }
                        Ok(())
                    }
                    Expr::ArrayIndex(_, _, _, _) => {
                        // Map write: full keccak256 slot computation is future work.
                        // Pop the value and push dummy so the caller can always Pop.
                        ctx.emit(IrInstruction::Pop);
                        ctx.emit(IrInstruction::Push(vec![0]));
                        Ok(())
                    }
                    _ => Err(LowerError::UnsupportedExpr(
                        "unsupported assignment target".to_string(),
                    )),
                }
            }
            Expr::ArrayIndex(base, index, None, _) => {
                // Storage mapping read
                // For MVP: just push dummy value
                self.lower_expr(ctx, base)?;
                self.lower_expr(ctx, index)?;
                ctx.emit(IrInstruction::Pop);
                ctx.emit(IrInstruction::Pop);
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::FunctionCall(_func, args, _) => {
                // Internal function calls (complex; placeholder for MVP)
                for arg in args {
                    self.lower_expr(ctx, arg)?;
                    ctx.emit(IrInstruction::Pop);
                }
                ctx.emit(IrInstruction::Push(vec![0]));
                Ok(())
            }
            Expr::Paren(inner, _) => self.lower_expr(ctx, inner),
            _ => Err(LowerError::UnsupportedExpr(format!(
                "unsupported expression: {expr:?}"
            ))),
        }
    }
}
