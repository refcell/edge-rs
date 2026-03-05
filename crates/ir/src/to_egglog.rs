//! AST to egglog IR lowering.
//!
//! Converts `edge_ast::Program` into `EvmProgram` by walking the AST
//! and producing IR nodes. This follows the pattern from eggcc's
//! `TreeToEgglog` but targets EVM-specific IR constructs.

use std::{collections::HashSet, rc::Rc};

use indexmap::IndexMap;

use crate::{
    ast_helpers,
    schema::{
        DataLocation, EvmBaseType, EvmBinaryOp, EvmConstant, EvmContext, EvmContract, EvmEnvOp,
        EvmExpr, EvmProgram, EvmTernaryOp, EvmType, EvmUnaryOp, RcExpr,
    },
    IrError,
};

/// Check if an expression references any variable from a set of names.
///
/// Used during lowering to ensure a `LetBind` init expression doesn't reference
/// variables whose `LetBinds` are inner (not yet allocated).
fn references_any_var(expr: &RcExpr, names: &HashSet<&str>) -> bool {
    match expr.as_ref() {
        EvmExpr::Var(n) => names.contains(n.as_str()),
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::Drop(_) => false,
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) | EvmExpr::DoWhile(a, b) => {
            references_any_var(a, names) || references_any_var(b, names)
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => references_any_var(a, names),
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            references_any_var(a, names)
                || references_any_var(b, names)
                || references_any_var(c, names)
        }
        EvmExpr::If(c, i, t, e) => {
            references_any_var(c, names)
                || references_any_var(i, names)
                || references_any_var(t, names)
                || references_any_var(e, names)
        }
        EvmExpr::VarStore(_, v) => references_any_var(v, names),
        EvmExpr::LetBind(_, init, body) => {
            references_any_var(init, names) || references_any_var(body, names)
        }
        EvmExpr::EnvRead(_, s) => references_any_var(s, names),
        EvmExpr::EnvRead1(_, a, s) => references_any_var(a, names) || references_any_var(s, names),
        EvmExpr::Log(_, topics, data, state) => {
            topics.iter().any(|t| references_any_var(t, names))
                || references_any_var(data, names)
                || references_any_var(state, names)
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => [a, b, c, d, e, f, g]
            .iter()
            .any(|x| references_any_var(x, names)),
        EvmExpr::Call(_, args) => references_any_var(args, names),
        EvmExpr::Function(_, _, _, body) => references_any_var(body, names),
    }
}

/// Tracks a variable binding during lowering.
#[derive(Debug, Clone)]
struct VarBinding {
    /// The current value expression (for storage/transient: the IR tree; for memory-backed: ignored)
    value: RcExpr,
    /// Where this variable lives
    location: DataLocation,
    /// For storage variables, the slot index
    storage_slot: Option<usize>,
    /// The type
    _ty: EvmType,
    /// For memory-backed local variables, the `LetBind` variable name
    let_bind_name: Option<String>,
}

/// Scope for variable resolution during lowering.
#[derive(Debug, Clone)]
struct Scope {
    /// Variable bindings: name -> binding
    bindings: IndexMap<String, VarBinding>,
}

impl Scope {
    fn new() -> Self {
        Self {
            bindings: IndexMap::new(),
        }
    }
}

/// A contract function: (name, params, body).
type ContractFunction = (
    String,
    Vec<(String, edge_ast::ty::TypeSig)>,
    edge_ast::CodeBlock,
);

/// Converts Edge AST to the egglog-based EVM IR.
#[derive(Debug)]
pub struct AstToEgglog {
    /// Scope stack (innermost last)
    scopes: Vec<Scope>,
    /// Current state expression (for threading side effects)
    current_state: RcExpr,
    /// Current context
    current_ctx: EvmContext,
    /// Persistent storage slot counter for the current contract
    next_storage_slot: usize,
    /// Transient storage slot counter for the current contract
    next_transient_slot: usize,
    /// Storage field IR nodes for the current contract
    storage_fields: Vec<RcExpr>,
    /// Internal functions available for inlining in the current contract
    /// Maps function name -> (`fn_decl` ref data, body)
    contract_functions: Vec<ContractFunction>,
    /// Events declared in the program (name -> (params with indexed info and type))
    events: IndexMap<String, Vec<(String, bool, edge_ast::ty::TypeSig)>>,
    /// Inline call depth — when > 0, `return` produces just the value (no RETURN opcode)
    inline_depth: usize,
}

impl Default for AstToEgglog {
    fn default() -> Self {
        Self::new()
    }
}

impl AstToEgglog {
    /// Create a new lowering context.
    pub fn new() -> Self {
        let dummy_ctx = EvmContext::InFunction("__init__".to_owned());
        Self {
            scopes: vec![Scope::new()],
            current_state: Rc::new(EvmExpr::Arg(
                EvmType::Base(EvmBaseType::StateT),
                dummy_ctx.clone(),
            )),
            current_ctx: dummy_ctx,
            next_storage_slot: 0,
            next_transient_slot: 0,
            storage_fields: Vec::new(),
            contract_functions: Vec::new(),
            events: IndexMap::new(),
            inline_depth: 0,
        }
    }

    /// Lower an entire program.
    pub fn lower_program(&mut self, program: &edge_ast::Program) -> Result<EvmProgram, IrError> {
        let mut contracts = Vec::new();
        let mut free_functions = Vec::new();

        // First pass: collect event declarations
        for stmt in &program.stmts {
            if let edge_ast::Stmt::EventDecl(event) = stmt {
                let params = event
                    .fields
                    .iter()
                    .map(|f| (f.name.name.clone(), f.indexed, f.ty.clone()))
                    .collect();
                self.events.insert(event.name.name.clone(), params);
            }
        }

        for stmt in &program.stmts {
            match stmt {
                edge_ast::Stmt::ContractDecl(contract) => {
                    let ir_contract = self.lower_contract(contract)?;
                    contracts.push(ir_contract);
                }
                edge_ast::Stmt::FnAssign(fn_decl, body) => {
                    let ir_fn = self.lower_function(fn_decl, body)?;
                    free_functions.push(ir_fn);
                }
                // Skip other top-level items for now (type aliases, traits, etc.)
                _ => {}
            }
        }

        Ok(EvmProgram {
            contracts,
            free_functions,
        })
    }

    /// Lower a contract declaration.
    fn lower_contract(
        &mut self,
        contract: &edge_ast::ContractDecl,
    ) -> Result<EvmContract, IrError> {
        // Reset storage layout for this contract
        self.next_storage_slot = 0;
        self.next_transient_slot = 0;
        self.storage_fields.clear();
        self.scopes = vec![Scope::new()];

        let contract_name = contract.name.name.clone();

        // Assign storage slots to fields
        for (ident, type_sig) in &contract.fields {
            let location = Self::extract_data_location(type_sig);
            let slot = match location {
                DataLocation::Transient => {
                    let s = self.next_transient_slot;
                    self.next_transient_slot += 1;
                    s
                }
                _ => {
                    let s = self.next_storage_slot;
                    self.next_storage_slot += 1;
                    s
                }
            };
            let ty = self.lower_type_sig(type_sig);

            // Create storage field IR node
            let field_ir = ast_helpers::storage_field(ident.name.clone(), slot, ty.clone());
            self.storage_fields.push(field_ir);

            // Register in scope with the correct location
            let binding = VarBinding {
                value: ast_helpers::const_int(
                    slot as i64,
                    EvmContext::InFunction(contract_name.clone()),
                ),
                location,
                storage_slot: Some(slot),
                _ty: ty,
                let_bind_name: None,
            };
            self.scopes
                .last_mut()
                .expect("scope stack empty")
                .bindings
                .insert(ident.name.clone(), binding);
        }

        // Collect internal functions for inlining
        self.contract_functions.clear();
        for fn_decl in &contract.functions {
            if let Some(body) = &fn_decl.body {
                let params = fn_decl
                    .params
                    .iter()
                    .map(|(id, ty)| (id.name.clone(), ty.clone()))
                    .collect();
                self.contract_functions
                    .push((fn_decl.name.name.clone(), params, body.clone()));
            }
        }

        // Lower contract function bodies
        let mut fn_bodies: Vec<(&edge_ast::ContractFnDecl, Option<RcExpr>)> = Vec::new();
        for fn_decl in &contract.functions {
            if let Some(body) = &fn_decl.body {
                let body_ir = self.lower_contract_fn_body(&contract_name, fn_decl, body)?;
                fn_bodies.push((fn_decl, Some(body_ir)));
            } else {
                fn_bodies.push((fn_decl, None));
            }
        }

        // Build dispatcher (runtime entry point) with inlined function bodies
        let runtime = self.build_dispatcher(&contract_name, &fn_bodies)?;

        // Constructor: initialize persistent storage fields to zero.
        // Transient fields are auto-zeroed per EIP-1153 at the start of each tx.
        let constructor_ctx = EvmContext::InFunction(format!("{contract_name}::constructor"));
        // Collect persistent storage slot indices
        let persistent_slots: Vec<usize> = contract
            .fields
            .iter()
            .filter_map(|(ident, type_sig)| {
                let loc = Self::extract_data_location(type_sig);
                if loc != DataLocation::Transient {
                    self.scopes
                        .last()
                        .and_then(|s| s.bindings.get(&ident.name))
                        .and_then(|b| b.storage_slot)
                } else {
                    None
                }
            })
            .collect();
        let constructor = if persistent_slots.is_empty() {
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), constructor_ctx)
        } else {
            let mut sstores: Vec<RcExpr> = Vec::new();
            let init_state = Rc::new(EvmExpr::Arg(
                EvmType::Base(EvmBaseType::StateT),
                constructor_ctx.clone(),
            ));
            let mut ctor_state = init_state;
            for &slot in &persistent_slots {
                let store = ast_helpers::sstore(
                    ast_helpers::const_int(slot as i64, constructor_ctx.clone()),
                    ast_helpers::const_int(0, constructor_ctx.clone()),
                    Rc::clone(&ctor_state),
                );
                ctor_state = Rc::clone(&store);
                sstores.push(store);
            }
            let mut result = Rc::clone(&sstores[0]);
            for store in &sstores[1..] {
                result = ast_helpers::concat(result, Rc::clone(store));
            }
            result
        };

        Ok(EvmContract {
            name: contract_name,
            storage_fields: self.storage_fields.clone(),
            constructor,
            runtime,
        })
    }

    /// Lower a contract function body into IR.
    fn lower_contract_fn_body(
        &mut self,
        contract_name: &str,
        fn_decl: &edge_ast::ContractFnDecl,
        body: &edge_ast::CodeBlock,
    ) -> Result<RcExpr, IrError> {
        let fn_name = format!("{contract_name}::{}", fn_decl.name.name);
        self.current_ctx = EvmContext::InFunction(fn_name);

        // Reset state for this function
        self.current_state = Rc::new(EvmExpr::Arg(
            EvmType::Base(EvmBaseType::StateT),
            self.current_ctx.clone(),
        ));

        // Push a new scope for function params
        self.scopes.push(Scope::new());

        // Bind parameters from calldata
        for (i, (ident, type_sig)) in fn_decl.params.iter().enumerate() {
            let ty = self.lower_type_sig(type_sig);
            let calldata_offset = 4 + i * 32; // After 4-byte selector
            let raw_val = Rc::new(EvmExpr::Bop(
                EvmBinaryOp::CalldataLoad,
                ast_helpers::const_int(calldata_offset as i64, self.current_ctx.clone()),
                Rc::clone(&self.current_state),
            ));
            // Mask address-typed params to 20 bytes to clean dirty upper bits
            let param_val = if ty == EvmType::Base(EvmBaseType::AddrT) {
                Rc::new(EvmExpr::Bop(
                    EvmBinaryOp::And,
                    raw_val,
                    ast_helpers::const_bigint(
                        "ffffffffffffffffffffffffffffffffffffffff".to_owned(),
                        self.current_ctx.clone(),
                    ),
                ))
            } else {
                raw_val
            };
            let binding = VarBinding {
                value: param_val,
                location: DataLocation::Stack,
                storage_slot: None,
                _ty: ty,
                let_bind_name: None,
            };
            self.scopes
                .last_mut()
                .expect("scope stack empty")
                .bindings
                .insert(ident.name.clone(), binding);
        }

        // Lower body
        let body_ir = self.lower_code_block(body)?;

        self.scopes.pop();

        // Append a STOP (RETURN with 0 size) after the body.
        // If the body already ends with RETURN, this is unreachable dead code.
        let stop = ast_helpers::return_op(
            ast_helpers::const_int(0, self.current_ctx.clone()),
            ast_helpers::const_int(0, self.current_ctx.clone()),
            Rc::clone(&self.current_state),
        );
        Ok(ast_helpers::concat(body_ir, stop))
    }

    /// Lower a standalone function.
    fn lower_function(
        &mut self,
        fn_decl: &edge_ast::FnDecl,
        body: &edge_ast::CodeBlock,
    ) -> Result<RcExpr, IrError> {
        let fn_name = fn_decl.name.name.clone();
        self.current_ctx = EvmContext::InFunction(fn_name.clone());

        let in_ty = self.params_to_type(&fn_decl.params);
        let out_ty = self.returns_to_type(&fn_decl.returns);

        // Reset state for this function
        self.current_state = Rc::new(EvmExpr::Arg(
            EvmType::Base(EvmBaseType::StateT),
            self.current_ctx.clone(),
        ));

        // Push a new scope for function params
        self.scopes.push(Scope::new());

        // Bind parameters
        let arg_expr = Rc::new(EvmExpr::Arg(in_ty.clone(), self.current_ctx.clone()));
        for (i, (ident, type_sig)) in fn_decl.params.iter().enumerate() {
            let ty = self.lower_type_sig(type_sig);
            let param_val = if fn_decl.params.len() == 1 {
                Rc::clone(&arg_expr)
            } else {
                ast_helpers::get(Rc::clone(&arg_expr), i)
            };
            let binding = VarBinding {
                value: param_val,
                location: DataLocation::Stack,
                storage_slot: None,
                _ty: ty,
                let_bind_name: None,
            };
            self.scopes
                .last_mut()
                .expect("scope stack empty")
                .bindings
                .insert(ident.name.clone(), binding);
        }

        // Lower body
        let body_ir = self.lower_code_block(body)?;

        self.scopes.pop();

        Ok(ast_helpers::function(fn_name, in_ty, out_ty, body_ir))
    }

    /// Lower a code block (sequence of statements).
    ///
    /// All statements are concatenated so that side effects (SSTORE, MSTORE,
    /// LOG, etc.) from every statement are preserved in the IR tree and will
    /// be compiled by codegen.
    fn lower_code_block(&mut self, block: &edge_ast::CodeBlock) -> Result<RcExpr, IrError> {
        // First pass: scan for VarDecl names to identify memory-backed locals.
        // We need this list BEFORE lowering to know which variables to wrap in LetBinds.
        let var_decl_names: Vec<String> = block
            .stmts
            .iter()
            .filter_map(|item| match item {
                edge_ast::BlockItem::Stmt(stmt) => match stmt.as_ref() {
                    edge_ast::Stmt::VarDecl(ident, _, _) => Some(ident.name.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();

        // Lower all statements
        let mut stmts: Vec<RcExpr> = Vec::new();
        for item in &block.stmts {
            let ir = match item {
                edge_ast::BlockItem::Stmt(stmt) => self.lower_stmt(stmt)?,
                edge_ast::BlockItem::Expr(expr) => self.lower_expr(expr)?,
            };
            stmts.push(ir);
        }

        if stmts.is_empty() {
            return Ok(ast_helpers::empty(
                EvmType::Base(EvmBaseType::UnitT),
                self.current_ctx.clone(),
            ));
        }

        // Store-forwarding at the lowering level: for each VarDecl, find
        // the first VarStore(var, val) in the statement list. If it's only
        // preceded by Empty nodes (from other VarDecls) and its value doesn't
        // reference any later-declared locals, use val directly as the
        // LetBind init instead of zero. This avoids generating
        // LetBind(x, 0, Concat(VarStore(x, real), ...)) in the first place.
        let local_var_names: Vec<String> = var_decl_names
            .iter()
            .map(|n| format!("__local_{n}"))
            .collect();
        let mut var_inits: std::collections::HashMap<String, RcExpr> =
            std::collections::HashMap::new();
        for (i, name) in var_decl_names.iter().enumerate() {
            let var_name = format!("__local_{name}");

            // Find the first VarStore for this variable
            let idx = stmts
                .iter()
                .position(|s| matches!(s.as_ref(), EvmExpr::VarStore(n, _) if n == &var_name));
            let Some(idx) = idx else { continue };

            // All preceding statements must be Empty (pure VarDecl placeholders).
            let preceding_ok = stmts[..idx]
                .iter()
                .all(|s| matches!(s.as_ref(), EvmExpr::Empty(..)));
            if !preceding_ok {
                continue;
            }

            // Extract the init value
            let init_val = match stmts[idx].as_ref() {
                EvmExpr::VarStore(_, val) => Rc::clone(val),
                _ => unreachable!(),
            };

            // The init must not reference any locals declared AFTER this one,
            // because those LetBinds are inner (haven't allocated yet when
            // the outer LetBind's init is evaluated).
            let inner_vars: std::collections::HashSet<&str> = local_var_names[i + 1..]
                .iter()
                .map(|s| s.as_str())
                .collect();
            if !inner_vars.is_empty() && references_any_var(&init_val, &inner_vars) {
                continue;
            }

            var_inits.insert(var_name, init_val);
            stmts.remove(idx);
        }

        let mut result = Rc::clone(&stmts[0]);
        for stmt in &stmts[1..] {
            result = ast_helpers::concat(result, Rc::clone(stmt));
        }

        // Wrap the result in LetBinds for memory-backed locals (innermost first).
        // Each variable gets: Drop(var) appended to body, then wrapped in LetBind.
        // Drop marks the end of the variable's lifetime for slot reclamation.
        for name in var_decl_names.iter().rev() {
            let var_name = format!("__local_{name}");
            // Append Drop at end of this variable's scope
            result = ast_helpers::concat(result, ast_helpers::drop_var(var_name.clone()));
            let init = var_inits
                .remove(&var_name)
                .unwrap_or_else(|| ast_helpers::const_int(0, self.current_ctx.clone()));
            result = ast_helpers::let_bind(var_name, init, result);
        }

        Ok(result)
    }

    /// Lower a statement.
    fn lower_stmt(&mut self, stmt: &edge_ast::Stmt) -> Result<RcExpr, IrError> {
        match stmt {
            edge_ast::Stmt::VarDecl(ident, type_sig, _span) => {
                let ty = type_sig
                    .as_ref()
                    .map(|ts| self.lower_type_sig(ts))
                    .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));
                let zero = ast_helpers::const_int(0, self.current_ctx.clone());
                let var_name = format!("__local_{}", ident.name);
                let binding = VarBinding {
                    value: zero,
                    location: DataLocation::Memory,
                    storage_slot: None,
                    _ty: ty,
                    let_bind_name: Some(var_name),
                };
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(ident.name.clone(), binding);
                // VarDecl itself produces no side effects; the LetBind wrapper
                // is added by lower_code_block
                Ok(ast_helpers::empty(
                    EvmType::Base(EvmBaseType::UnitT),
                    self.current_ctx.clone(),
                ))
            }

            edge_ast::Stmt::VarAssign(lhs, rhs, _span) => {
                let rhs_ir = self.lower_expr(rhs)?;
                self.lower_assignment(lhs, rhs_ir)
            }

            edge_ast::Stmt::ConstAssign(const_decl, expr, _span) => {
                let val = self.lower_expr(expr)?;
                let ty = const_decl
                    .ty
                    .as_ref()
                    .map(|ts| self.lower_type_sig(ts))
                    .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));
                let binding = VarBinding {
                    value: Rc::clone(&val),
                    location: DataLocation::Stack,
                    storage_slot: None,
                    _ty: ty,
                    let_bind_name: None,
                };
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(const_decl.name.name.clone(), binding);
                Ok(val)
            }

            edge_ast::Stmt::FnAssign(fn_decl, body) => self.lower_function(fn_decl, body),

            edge_ast::Stmt::Return(maybe_expr, _span) => {
                if self.inline_depth > 0 {
                    // Inside an inlined function — just produce the value, no RETURN opcode
                    if let Some(expr) = maybe_expr {
                        self.lower_expr(expr)
                    } else {
                        // Void return inside inlined function — produce empty/zero
                        Ok(ast_helpers::const_int(0, self.current_ctx.clone()))
                    }
                } else if let Some(expr) = maybe_expr {
                    let val = self.lower_expr(expr)?;
                    // ABI-encode the return value to memory and RETURN
                    let offset = ast_helpers::const_int(0, self.current_ctx.clone());
                    let size = ast_helpers::const_int(32, self.current_ctx.clone());
                    // Store value at memory offset 0 (as a separate compiled expression)
                    let mstore_expr = ast_helpers::mstore(
                        Rc::clone(&offset),
                        val,
                        Rc::clone(&self.current_state),
                    );
                    self.current_state = Rc::clone(&mstore_expr);
                    let ret = ast_helpers::return_op(offset, size, Rc::clone(&self.current_state));
                    // Emit MStore first, then RETURN — the codegen ignores
                    // state parameters, so MStore must be a separate expression
                    Ok(ast_helpers::concat(mstore_expr, ret))
                } else {
                    let offset = ast_helpers::const_int(0, self.current_ctx.clone());
                    let size = ast_helpers::const_int(0, self.current_ctx.clone());
                    Ok(ast_helpers::return_op(
                        offset,
                        size,
                        Rc::clone(&self.current_state),
                    ))
                }
            }

            edge_ast::Stmt::IfElse(branches, else_block) => {
                self.lower_if_else(branches, else_block.as_ref())
            }

            edge_ast::Stmt::WhileLoop(cond, loop_block) => self.lower_while_loop(cond, loop_block),

            edge_ast::Stmt::ForLoop(init, cond, update, loop_block) => self.lower_for_loop(
                init.as_deref(),
                cond.as_ref(),
                update.as_deref(),
                loop_block,
            ),

            edge_ast::Stmt::Loop(loop_block) => self.lower_infinite_loop(loop_block),

            edge_ast::Stmt::DoWhile(loop_block, cond) => self.lower_do_while(loop_block, cond),

            edge_ast::Stmt::Emit(event_name, args, _span) => self.lower_emit(event_name, args),

            edge_ast::Stmt::Expr(expr) => self.lower_expr(expr),

            edge_ast::Stmt::Break(_) | edge_ast::Stmt::Continue(_) => {
                // Break/continue need special handling within loop context
                // For now, return empty
                Ok(ast_helpers::empty(
                    EvmType::Base(EvmBaseType::UnitT),
                    self.current_ctx.clone(),
                ))
            }

            edge_ast::Stmt::CodeBlock(block) => {
                self.scopes.push(Scope::new());
                let result = self.lower_code_block(block)?;
                self.scopes.pop();
                Ok(result)
            }

            // TODO: implement remaining statement types
            other => Err(IrError::Unsupported(format!(
                "Statement type not yet supported: {other:?}"
            ))),
        }
    }

    /// Lower an expression.
    fn lower_expr(&mut self, expr: &edge_ast::Expr) -> Result<RcExpr, IrError> {
        match expr {
            edge_ast::Expr::Literal(lit) => self.lower_literal(lit),

            edge_ast::Expr::Ident(ident) => self.lower_ident(&ident.name),

            edge_ast::Expr::Binary(lhs, op, rhs, _span) => {
                let lhs_ir = self.lower_expr(lhs)?;
                let rhs_ir = self.lower_expr(rhs)?;
                self.lower_binary_op(op, lhs_ir, rhs_ir)
            }

            edge_ast::Expr::Unary(op, expr, _span) => {
                let expr_ir = self.lower_expr(expr)?;
                self.lower_unary_op(op, expr_ir)
            }

            edge_ast::Expr::Ternary(cond, true_expr, false_expr, _span) => {
                let cond_ir = self.lower_expr(cond)?;
                let true_ir = self.lower_expr(true_expr)?;
                let false_ir = self.lower_expr(false_expr)?;
                Ok(Rc::new(EvmExpr::Top(
                    EvmTernaryOp::Select,
                    cond_ir,
                    true_ir,
                    false_ir,
                )))
            }

            edge_ast::Expr::FunctionCall(callee, args, _span) => {
                self.lower_function_call(callee, args)
            }

            edge_ast::Expr::At(builtin_name, args, _span) => {
                self.lower_builtin(&builtin_name.name, args)
            }

            edge_ast::Expr::Assign(lhs, rhs, _span) => {
                let rhs_ir = self.lower_expr(rhs)?;
                self.lower_assignment(lhs, rhs_ir)
            }

            edge_ast::Expr::ArrayIndex(base, index, _end_index, _span) => {
                self.lower_mapping_read(base, index)
            }

            edge_ast::Expr::Paren(inner, _span) => self.lower_expr(inner),

            edge_ast::Expr::FieldAccess(obj, field, _span) => {
                // For now, treat as accessing a contract storage field
                let _obj_ir = self.lower_expr(obj)?;
                self.lower_ident(&field.name)
            }

            edge_ast::Expr::Path(components, _span) => {
                // Qualified path like Module::Item
                components.last().map_or_else(
                    || Err(IrError::Lowering("empty path".to_owned())),
                    |last| self.lower_ident(&last.name),
                )
            }

            // TODO: implement remaining expression types
            other => Err(IrError::Unsupported(format!(
                "Expression type not yet supported: {other:?}"
            ))),
        }
    }

    /// Lower a literal value.
    fn lower_literal(&self, lit: &edge_ast::Lit) -> Result<RcExpr, IrError> {
        match lit {
            edge_ast::Lit::Int(val, maybe_ty, _span) => {
                let ty = maybe_ty
                    .as_ref()
                    .map(|pt| self.lower_primitive_type(pt))
                    .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));
                Ok(Rc::new(EvmExpr::Const(
                    EvmConstant::SmallInt(*val as i64),
                    ty,
                    self.current_ctx.clone(),
                )))
            }
            edge_ast::Lit::Bool(val, _span) => {
                Ok(ast_helpers::const_bool(*val, self.current_ctx.clone()))
            }
            edge_ast::Lit::Hex(bytes, _span) => {
                let hex_str = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                Ok(ast_helpers::const_bigint(hex_str, self.current_ctx.clone()))
            }
            edge_ast::Lit::Bin(bytes, _span) => {
                let hex_str = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                Ok(ast_helpers::const_bigint(hex_str, self.current_ctx.clone()))
            }
            edge_ast::Lit::Str(s, _span) => {
                // Strings become their keccak256 hash in most EVM contexts
                // For now, store as BigInt of the raw bytes
                let hex_str = s
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                Ok(ast_helpers::const_bigint(hex_str, self.current_ctx.clone()))
            }
        }
    }

    /// Lower an identifier reference.
    fn lower_ident(&self, name: &str) -> Result<RcExpr, IrError> {
        // Search scopes from innermost to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                return match binding.location {
                    DataLocation::Storage => {
                        // Persistent storage variable: emit SLOAD
                        let slot = ast_helpers::const_int(
                            binding.storage_slot.unwrap_or(0) as i64,
                            self.current_ctx.clone(),
                        );
                        Ok(ast_helpers::sload(slot, Rc::clone(&self.current_state)))
                    }
                    DataLocation::Transient => {
                        // Transient storage variable: emit TLOAD
                        let slot = ast_helpers::const_int(
                            binding.storage_slot.unwrap_or(0) as i64,
                            self.current_ctx.clone(),
                        );
                        Ok(ast_helpers::tload(slot, Rc::clone(&self.current_state)))
                    }
                    _ => {
                        binding.let_bind_name.as_ref().map_or_else(
                            // Stack/compile-time variable: return the value directly
                            || Ok(Rc::clone(&binding.value)),
                            // Memory-backed local: emit Var(name) to read from memory
                            |var_name| Ok(ast_helpers::var(var_name.clone())),
                        )
                    }
                };
            }
        }
        Err(IrError::Lowering(format!("undefined variable: {name}")))
    }

    /// Lower an assignment expression.
    fn lower_assignment(
        &mut self,
        lhs: &edge_ast::Expr,
        rhs_ir: RcExpr,
    ) -> Result<RcExpr, IrError> {
        match lhs {
            edge_ast::Expr::Ident(ident) => {
                let name = &ident.name;
                // Find the binding
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(binding) = scope.bindings.get_mut(name) {
                        return match binding.location {
                            DataLocation::Storage => {
                                let slot = ast_helpers::const_int(
                                    binding.storage_slot.unwrap_or(0) as i64,
                                    self.current_ctx.clone(),
                                );
                                let new_state = ast_helpers::sstore(
                                    slot,
                                    rhs_ir,
                                    Rc::clone(&self.current_state),
                                );
                                self.current_state = Rc::clone(&new_state);
                                Ok(new_state)
                            }
                            DataLocation::Transient => {
                                let slot = ast_helpers::const_int(
                                    binding.storage_slot.unwrap_or(0) as i64,
                                    self.current_ctx.clone(),
                                );
                                let new_state = ast_helpers::tstore(
                                    slot,
                                    rhs_ir,
                                    Rc::clone(&self.current_state),
                                );
                                self.current_state = Rc::clone(&new_state);
                                Ok(new_state)
                            }
                            _ => {
                                if let Some(ref var_name) = binding.let_bind_name {
                                    // Memory-backed local: emit VarStore to write to memory
                                    Ok(ast_helpers::var_store(var_name.clone(), rhs_ir))
                                } else {
                                    // Compile-time variable (const/param): replace value
                                    binding.value = Rc::clone(&rhs_ir);
                                    Ok(rhs_ir)
                                }
                            }
                        };
                    }
                }
                Err(IrError::Lowering(format!(
                    "assignment to undefined variable: {name}"
                )))
            }
            edge_ast::Expr::ArrayIndex(base, index, _end_index, _span) => {
                self.lower_mapping_write(base, index, rhs_ir)
            }
            _ => Err(IrError::Unsupported(
                "complex assignment target not yet supported".to_owned(),
            )),
        }
    }

    /// Lower a binary operator.
    fn lower_binary_op(
        &self,
        op: &edge_ast::BinOp,
        lhs: RcExpr,
        rhs: RcExpr,
    ) -> Result<RcExpr, IrError> {
        let ir_op = match op {
            edge_ast::BinOp::Add | edge_ast::BinOp::AddAssign => EvmBinaryOp::CheckedAdd,
            edge_ast::BinOp::Sub | edge_ast::BinOp::SubAssign => EvmBinaryOp::CheckedSub,
            edge_ast::BinOp::Mul | edge_ast::BinOp::MulAssign => EvmBinaryOp::CheckedMul,
            edge_ast::BinOp::Div | edge_ast::BinOp::DivAssign => EvmBinaryOp::Div,
            edge_ast::BinOp::Mod | edge_ast::BinOp::ModAssign => EvmBinaryOp::Mod,
            edge_ast::BinOp::Exp | edge_ast::BinOp::ExpAssign => EvmBinaryOp::Exp,
            edge_ast::BinOp::BitwiseAnd | edge_ast::BinOp::BitwiseAndAssign => EvmBinaryOp::And,
            edge_ast::BinOp::BitwiseOr | edge_ast::BinOp::BitwiseOrAssign => EvmBinaryOp::Or,
            edge_ast::BinOp::BitwiseXor | edge_ast::BinOp::BitwiseXorAssign => EvmBinaryOp::Xor,
            edge_ast::BinOp::Shl | edge_ast::BinOp::ShlAssign => {
                // IR convention: Bop(Shl, shift_amount, value)
                // AST: value << shift → swap to (shift, value)
                return Ok(ast_helpers::bop(EvmBinaryOp::Shl, rhs, lhs));
            }
            edge_ast::BinOp::Shr | edge_ast::BinOp::ShrAssign => {
                // IR convention: Bop(Shr, shift_amount, value)
                // AST: value >> shift → swap to (shift, value)
                return Ok(ast_helpers::bop(EvmBinaryOp::Shr, rhs, lhs));
            }
            edge_ast::BinOp::LogicalAnd => EvmBinaryOp::LogAnd,
            edge_ast::BinOp::LogicalOr => EvmBinaryOp::LogOr,
            edge_ast::BinOp::Eq => EvmBinaryOp::Eq,
            edge_ast::BinOp::Neq => {
                // a != b -> IsZero(Eq(a, b))
                let eq_expr = ast_helpers::eq(lhs, rhs);
                return Ok(ast_helpers::iszero(eq_expr));
            }
            edge_ast::BinOp::Lt => EvmBinaryOp::Lt,
            edge_ast::BinOp::Lte => {
                // a <= b -> IsZero(Gt(a, b))
                let gt_expr = ast_helpers::bop(EvmBinaryOp::Gt, lhs, rhs);
                return Ok(ast_helpers::iszero(gt_expr));
            }
            edge_ast::BinOp::Gt => EvmBinaryOp::Gt,
            edge_ast::BinOp::Gte => {
                // a >= b -> IsZero(Lt(a, b))
                let lt_expr = ast_helpers::bop(EvmBinaryOp::Lt, lhs, rhs);
                return Ok(ast_helpers::iszero(lt_expr));
            }
        };
        Ok(ast_helpers::bop(ir_op, lhs, rhs))
    }

    /// Lower a unary operator.
    fn lower_unary_op(&self, op: &edge_ast::UnaryOp, expr: RcExpr) -> Result<RcExpr, IrError> {
        let ir_op = match op {
            edge_ast::UnaryOp::Neg => EvmUnaryOp::Neg,
            edge_ast::UnaryOp::BitwiseNot => EvmUnaryOp::Not,
            edge_ast::UnaryOp::LogicalNot => EvmUnaryOp::IsZero,
        };
        Ok(ast_helpers::uop(ir_op, expr))
    }

    /// Lower a function call.
    ///
    /// For internal contract functions, inlines the function body at the call site
    /// by binding the arguments in a new scope and lowering the body.
    fn lower_function_call(
        &mut self,
        callee: &edge_ast::Expr,
        args: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        // Get function name
        let fn_name = match callee {
            edge_ast::Expr::Ident(id) => id.name.clone(),
            edge_ast::Expr::Path(components, _) => components
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join("::"),
            _ => {
                return Err(IrError::Unsupported(
                    "dynamic function calls not yet supported".to_owned(),
                ));
            }
        };

        // Check if this is an internal contract function we can inline
        let internal_fn = self
            .contract_functions
            .iter()
            .find(|(name, _, _)| *name == fn_name)
            .cloned();

        if let Some((_name, params, body)) = internal_fn {
            // Lower arguments
            let args_ir: Vec<RcExpr> = args
                .iter()
                .map(|a| self.lower_expr(a))
                .collect::<Result<_, _>>()?;

            // Push a new scope and bind parameters
            self.scopes.push(Scope::new());
            for (i, (param_name, param_ty)) in params.iter().enumerate() {
                let ty = self.lower_type_sig(param_ty);
                let val = args_ir
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| ast_helpers::const_int(0, self.current_ctx.clone()));
                let binding = VarBinding {
                    value: val,
                    location: DataLocation::Stack,
                    storage_slot: None,
                    _ty: ty,
                    let_bind_name: None,
                };
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(param_name.clone(), binding);
            }

            // Lower the function body inline (return should produce value, not RETURN opcode)
            self.inline_depth += 1;
            let result = self.lower_code_block(&body)?;
            self.inline_depth -= 1;
            self.scopes.pop();
            return Ok(result);
        }

        // Handle builtin functions
        if fn_name == "revert" {
            let state =
                ast_helpers::arg(EvmType::Base(EvmBaseType::StateT), self.current_ctx.clone());
            return Ok(ast_helpers::revert(
                ast_helpers::const_int(0, self.current_ctx.clone()),
                ast_helpers::const_int(0, self.current_ctx.clone()),
                state,
            ));
        }

        // Not an internal function — emit a Call node
        let args_ir: Vec<RcExpr> = args
            .iter()
            .map(|a| self.lower_expr(a))
            .collect::<Result<_, _>>()?;

        let arg_tuple = match args_ir.len() {
            0 => ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone()),
            1 => args_ir.into_iter().next().expect("checked len"),
            _ => {
                let mut result = Rc::clone(&args_ir[0]);
                for arg in &args_ir[1..] {
                    result = ast_helpers::concat(result, Rc::clone(arg));
                }
                result
            }
        };

        Ok(ast_helpers::call(fn_name, arg_tuple))
    }

    /// Lower a builtin call (@caller, @callvalue, etc.).
    fn lower_builtin(&self, name: &str, _args: &[edge_ast::Expr]) -> Result<RcExpr, IrError> {
        let env_op = match name {
            "caller" => EvmEnvOp::Caller,
            "callvalue" | "value" => EvmEnvOp::CallValue,
            "calldatasize" => EvmEnvOp::CallDataSize,
            "origin" => EvmEnvOp::Origin,
            "gasprice" => EvmEnvOp::GasPrice,
            "coinbase" => EvmEnvOp::Coinbase,
            "timestamp" => EvmEnvOp::Timestamp,
            "number" => EvmEnvOp::Number,
            "gaslimit" => EvmEnvOp::GasLimit,
            "chainid" => EvmEnvOp::ChainId,
            "selfbalance" => EvmEnvOp::SelfBalance,
            "basefee" => EvmEnvOp::BaseFee,
            "gas" => EvmEnvOp::Gas,
            "address" => EvmEnvOp::Address,
            "codesize" => EvmEnvOp::CodeSize,
            "returndatasize" => EvmEnvOp::ReturnDataSize,
            _ => {
                return Err(IrError::Unsupported(format!("unknown builtin: @{name}")));
            }
        };
        Ok(Rc::new(EvmExpr::EnvRead(
            env_op,
            Rc::clone(&self.current_state),
        )))
    }

    /// Lower if/else chains.
    fn lower_if_else(
        &mut self,
        branches: &[(edge_ast::Expr, edge_ast::CodeBlock)],
        else_block: Option<&edge_ast::CodeBlock>,
    ) -> Result<RcExpr, IrError> {
        if branches.is_empty() {
            return if let Some(block) = else_block {
                self.lower_code_block(block)
            } else {
                Ok(ast_helpers::empty(
                    EvmType::Base(EvmBaseType::UnitT),
                    self.current_ctx.clone(),
                ))
            };
        }

        let (cond, body) = &branches[0];
        let cond_ir = self.lower_expr(cond)?;
        let then_ir = self.lower_code_block(body)?;

        let else_ir = if branches.len() > 1 {
            self.lower_if_else(&branches[1..], else_block)?
        } else if let Some(block) = else_block {
            self.lower_code_block(block)?
        } else {
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone())
        };

        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        Ok(ast_helpers::if_then_else(cond_ir, inputs, then_ir, else_ir))
    }

    /// Lower a while loop.
    fn lower_while_loop(
        &mut self,
        cond: &edge_ast::Expr,
        loop_block: &edge_ast::LoopBlock,
    ) -> Result<RcExpr, IrError> {
        let cond_ir = self.lower_expr(cond)?;
        let body_ir = self.lower_loop_block(loop_block)?;
        // while(cond) { body } -> if(cond) { do { body; cond } while(top) }
        // Body side effects (SSTORE) must run BEFORE condition is re-evaluated
        let pred_and_body = ast_helpers::concat(body_ir, Rc::clone(&cond_ir));
        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        let loop_ir = ast_helpers::do_while(Rc::clone(&inputs), pred_and_body);
        let empty = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        Ok(ast_helpers::if_then_else(cond_ir, inputs, loop_ir, empty))
    }

    /// Lower a for loop.
    fn lower_for_loop(
        &mut self,
        init: Option<&edge_ast::Stmt>,
        cond: Option<&edge_ast::Expr>,
        update: Option<&edge_ast::Stmt>,
        loop_block: &edge_ast::LoopBlock,
    ) -> Result<RcExpr, IrError> {
        self.scopes.push(Scope::new());

        // Lower init
        if let Some(init_stmt) = init {
            let _ = self.lower_stmt(init_stmt)?;
        }

        // Condition (default true if absent)
        let cond_ir = if let Some(cond_expr) = cond {
            self.lower_expr(cond_expr)?
        } else {
            ast_helpers::const_bool(true, self.current_ctx.clone())
        };

        // Body + update
        let body_ir = self.lower_loop_block(loop_block)?;
        let update_ir = if let Some(update_stmt) = update {
            self.lower_stmt(update_stmt)?
        } else {
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone())
        };

        // Combine: pred_and_body = (body, update, cond)
        // Body + update run BEFORE condition is re-evaluated
        let pred_and_body =
            ast_helpers::concat(ast_helpers::concat(body_ir, update_ir), Rc::clone(&cond_ir));

        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        let loop_ir = ast_helpers::do_while(Rc::clone(&inputs), pred_and_body);
        let empty = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());

        self.scopes.pop();

        Ok(ast_helpers::if_then_else(cond_ir, inputs, loop_ir, empty))
    }

    /// Lower an infinite loop.
    fn lower_infinite_loop(&mut self, loop_block: &edge_ast::LoopBlock) -> Result<RcExpr, IrError> {
        let body_ir = self.lower_loop_block(loop_block)?;
        let true_const = ast_helpers::const_bool(true, self.current_ctx.clone());
        // Body runs first, then always-true condition
        let pred_and_body = ast_helpers::concat(body_ir, true_const);
        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        Ok(ast_helpers::do_while(inputs, pred_and_body))
    }

    /// Lower a do-while loop.
    fn lower_do_while(
        &mut self,
        loop_block: &edge_ast::LoopBlock,
        cond: &edge_ast::Expr,
    ) -> Result<RcExpr, IrError> {
        let body_ir = self.lower_loop_block(loop_block)?;
        let cond_ir = self.lower_expr(cond)?;
        // Body runs first, then condition is evaluated
        let pred_and_body = ast_helpers::concat(body_ir, cond_ir);
        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        Ok(ast_helpers::do_while(inputs, pred_and_body))
    }

    /// Lower a loop block.
    fn lower_loop_block(&mut self, block: &edge_ast::LoopBlock) -> Result<RcExpr, IrError> {
        let mut result =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());

        for item in &block.items {
            let item_ir = match item {
                edge_ast::LoopItem::Stmt(stmt) => self.lower_stmt(stmt)?,
                edge_ast::LoopItem::Expr(expr) => self.lower_expr(expr)?,
                edge_ast::LoopItem::Break(_) | edge_ast::LoopItem::Continue(_) => {
                    // TODO: handle break/continue with control flow markers
                    continue;
                }
            };
            // Concatenate all statements — intermediate ones have critical side effects
            result = ast_helpers::concat(result, item_ir);
        }

        Ok(result)
    }

    /// Lower an emit statement.
    ///
    /// Generates LOG opcode with:
    /// - topic[0] = keccak256 of event signature
    /// - topic[1..] = indexed parameters
    /// - data = ABI-encoded non-indexed parameters (each MSTORE'd to memory)
    fn lower_emit(
        &mut self,
        event_name: &edge_ast::Ident,
        args: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        let ctx = self.current_ctx.clone();

        // Compute event signature for topic[0]
        // Build the event signature string: "EventName(type1,type2,...)"
        let event_info = self.events.get(&event_name.name).cloned();
        let sig = event_info.as_ref().map_or_else(
            || {
                // Fallback: build signature from arg count
                let types: Vec<&str> = args.iter().map(|_| "uint256").collect();
                format!("{}({})", event_name.name, types.join(","))
            },
            |fields| {
                let types: Vec<String> = fields
                    .iter()
                    .map(|(_, _, ty)| Self::type_sig_to_abi_string(ty))
                    .collect();
                format!("{}({})", event_name.name, types.join(","))
            },
        );
        // Event topic0 must be the full 32-byte keccak256 hash (not a 4-byte selector)
        let mut hash = [0u8; 32];
        edge_types::bytes::hash_bytes(&mut hash, &sig);
        let hash_hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let sig_topic = ast_helpers::const_bigint(hash_hex, ctx.clone());

        // Separate indexed and non-indexed args
        let mut topics = vec![sig_topic];
        let mut data_exprs = Vec::new();
        let mut side_effects: Vec<RcExpr> = Vec::new();

        for (i, arg) in args.iter().enumerate() {
            let arg_ir = self.lower_expr(arg)?;
            let is_indexed = event_info
                .as_ref()
                .and_then(|fields| fields.get(i))
                .map(|(_, indexed, _)| *indexed)
                .unwrap_or(false);

            if is_indexed {
                topics.push(arg_ir);
            } else {
                data_exprs.push(arg_ir);
            }
        }

        // MSTORE non-indexed data to memory
        let (data_offset, data_size) = if data_exprs.is_empty() {
            (
                ast_helpers::const_int(0, ctx.clone()),
                ast_helpers::const_int(0, ctx),
            )
        } else {
            for (i, data_expr) in data_exprs.iter().enumerate() {
                let offset = (i * 32) as i64;
                let mstore = ast_helpers::mstore(
                    ast_helpers::const_int(offset, ctx.clone()),
                    Rc::clone(data_expr),
                    Rc::clone(&self.current_state),
                );
                self.current_state = Rc::clone(&mstore);
                side_effects.push(mstore);
            }
            (
                ast_helpers::const_int(0, ctx.clone()),
                ast_helpers::const_int((data_exprs.len() * 32) as i64, ctx),
            )
        };

        let topic_count = topics.len();
        let log = Rc::new(EvmExpr::Log(
            topic_count,
            topics,
            ast_helpers::concat(data_offset, data_size),
            Rc::clone(&self.current_state),
        ));
        self.current_state = Rc::clone(&log);

        // Build concat of side effects + log
        if side_effects.is_empty() {
            Ok(log)
        } else {
            let mut result = Rc::clone(&side_effects[0]);
            for effect in &side_effects[1..] {
                result = ast_helpers::concat(result, Rc::clone(effect));
            }
            Ok(ast_helpers::concat(result, log))
        }
    }

    /// Compute the storage slot for a mapping access.
    ///
    /// For `mapping[key]` at base slot `s`, Solidity uses:
    ///   `keccak256(abi.encode(key, s))` where key is left-padded to 32 bytes
    ///   at memory[0..32] and s is at memory[32..64].
    ///
    /// Returns `(side_effects_expr, computed_slot_expr)` where `side_effects_expr`
    /// is a Concat of MSTOREs that must be emitted before the slot is used.
    fn compute_mapping_slot(&mut self, key: RcExpr, base_slot: i64) -> (RcExpr, RcExpr) {
        let ctx = self.current_ctx.clone();
        // MSTORE(0, key)
        let mstore_key = ast_helpers::mstore(
            ast_helpers::const_int(0, ctx.clone()),
            key,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_key);
        // MSTORE(32, base_slot)
        let mstore_slot = ast_helpers::mstore(
            ast_helpers::const_int(32, ctx.clone()),
            ast_helpers::const_int(base_slot, ctx.clone()),
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_slot);
        // KECCAK256(0, 64, state) — state captures the memory contents
        let computed_slot = ast_helpers::keccak256(
            ast_helpers::const_int(0, ctx.clone()),
            ast_helpers::const_int(64, ctx),
            Rc::clone(&self.current_state),
        );
        let side_effects = ast_helpers::concat(mstore_key, mstore_slot);
        (side_effects, computed_slot)
    }

    /// Compute the storage slot for a nested mapping access.
    ///
    /// For `mapping[key1][key2]`, uses `keccak256(key2 . keccak256(key1 . base_slot))`.
    ///
    /// Uses memory[0..64] for the first level and memory[64..128] for the second
    /// to avoid the second level's MSTORE overwriting the first level's data before
    /// KECCAK256 reads it.
    fn compute_nested_mapping_slot(
        &mut self,
        outer_key: RcExpr,
        inner_key: RcExpr,
        base_slot: i64,
    ) -> (RcExpr, RcExpr) {
        let ctx = self.current_ctx.clone();
        // First level: keccak256(key1 . base_slot) at memory[0..64]
        let mstore_key1 = ast_helpers::mstore(
            ast_helpers::const_int(0, ctx.clone()),
            outer_key,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_key1);
        let mstore_slot1 = ast_helpers::mstore(
            ast_helpers::const_int(32, ctx.clone()),
            ast_helpers::const_int(base_slot, ctx.clone()),
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_slot1);
        // inner_slot — KECCAK256(0, 64, state) reads memory[0..64]
        let inner_slot = ast_helpers::keccak256(
            ast_helpers::const_int(0, ctx.clone()),
            ast_helpers::const_int(64, ctx.clone()),
            Rc::clone(&self.current_state),
        );
        // Second level: keccak256(key2 . inner_slot) at memory[64..128]
        // Using offset 64 avoids overwriting memory[0..64] before KECCAK256 reads it
        let mstore_key2 = ast_helpers::mstore(
            ast_helpers::const_int(64, ctx.clone()),
            inner_key,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_key2);
        let mstore_slot2 = ast_helpers::mstore(
            ast_helpers::const_int(96, ctx.clone()),
            inner_slot,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_slot2);
        let computed_slot = ast_helpers::keccak256(
            ast_helpers::const_int(64, ctx.clone()),
            ast_helpers::const_int(64, ctx),
            Rc::clone(&self.current_state),
        );
        let side_effects = ast_helpers::concat(
            ast_helpers::concat(mstore_key1, mstore_slot1),
            ast_helpers::concat(mstore_key2, mstore_slot2),
        );
        (side_effects, computed_slot)
    }

    /// Lower a mapping read: `field[key]` or `field[key1][key2]`.
    fn lower_mapping_read(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
    ) -> Result<RcExpr, IrError> {
        // Check for nested mapping: base is itself an ArrayIndex
        if let edge_ast::Expr::ArrayIndex(outer_base, outer_index, _, _) = base {
            // nested: outer_base[outer_index][index]
            let field_name = match &**outer_base {
                edge_ast::Expr::Ident(id) => &id.name,
                _ => {
                    return Err(IrError::Unsupported(
                        "nested mapping on non-identifier".to_owned(),
                    ));
                }
            };
            let (base_slot, location) = self.find_storage_slot(field_name)?;
            let outer_key = self.lower_expr(outer_index)?;
            let inner_key = self.lower_expr(index)?;
            let (side_effects, computed_slot) =
                self.compute_nested_mapping_slot(outer_key, inner_key, base_slot as i64);
            let load = match location {
                DataLocation::Transient => {
                    ast_helpers::tload(computed_slot, Rc::clone(&self.current_state))
                }
                _ => ast_helpers::sload(computed_slot, Rc::clone(&self.current_state)),
            };
            return Ok(ast_helpers::concat(side_effects, load));
        }

        // Simple mapping: field[key]
        let field_name = match base {
            edge_ast::Expr::Ident(id) => &id.name,
            _ => {
                return Err(IrError::Unsupported(
                    "mapping on non-identifier base".to_owned(),
                ));
            }
        };
        let (base_slot, location) = self.find_storage_slot(field_name)?;
        let key = self.lower_expr(index)?;
        let (side_effects, computed_slot) = self.compute_mapping_slot(key, base_slot as i64);
        let load = match location {
            DataLocation::Transient => {
                ast_helpers::tload(computed_slot, Rc::clone(&self.current_state))
            }
            _ => ast_helpers::sload(computed_slot, Rc::clone(&self.current_state)),
        };
        Ok(ast_helpers::concat(side_effects, load))
    }

    /// Lower a mapping write: `field[key] = value` or `field[key1][key2] = value`.
    fn lower_mapping_write(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
        value: RcExpr,
    ) -> Result<RcExpr, IrError> {
        // Check for nested mapping
        if let edge_ast::Expr::ArrayIndex(outer_base, outer_index, _, _) = base {
            let field_name = match &**outer_base {
                edge_ast::Expr::Ident(id) => &id.name,
                _ => {
                    return Err(IrError::Unsupported(
                        "nested mapping on non-identifier".to_owned(),
                    ));
                }
            };
            let (base_slot, location) = self.find_storage_slot(field_name)?;
            let outer_key = self.lower_expr(outer_index)?;
            let inner_key = self.lower_expr(index)?;
            let (side_effects, computed_slot) =
                self.compute_nested_mapping_slot(outer_key, inner_key, base_slot as i64);
            let store = match location {
                DataLocation::Transient => {
                    ast_helpers::tstore(computed_slot, value, Rc::clone(&self.current_state))
                }
                _ => ast_helpers::sstore(computed_slot, value, Rc::clone(&self.current_state)),
            };
            self.current_state = Rc::clone(&store);
            return Ok(ast_helpers::concat(side_effects, store));
        }

        // Simple mapping write
        let field_name = match base {
            edge_ast::Expr::Ident(id) => &id.name,
            _ => {
                return Err(IrError::Unsupported(
                    "mapping on non-identifier base".to_owned(),
                ));
            }
        };
        let (base_slot, location) = self.find_storage_slot(field_name)?;
        let key = self.lower_expr(index)?;
        let (side_effects, computed_slot) = self.compute_mapping_slot(key, base_slot as i64);
        let store = match location {
            DataLocation::Transient => {
                ast_helpers::tstore(computed_slot, value, Rc::clone(&self.current_state))
            }
            _ => ast_helpers::sstore(computed_slot, value, Rc::clone(&self.current_state)),
        };
        self.current_state = Rc::clone(&store);
        Ok(ast_helpers::concat(side_effects, store))
    }

    /// Find the storage slot index and data location for a named field.
    fn find_storage_slot(&self, name: &str) -> Result<(usize, DataLocation), IrError> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                if let Some(slot) = binding.storage_slot {
                    return Ok((slot, binding.location));
                }
            }
        }
        Err(IrError::Lowering(format!(
            "storage field not found: {name}"
        )))
    }

    /// Build the function dispatcher for a contract.
    ///
    /// Inlines function bodies directly in the dispatcher. For contracts with
    /// fewer than 4 public functions, uses a linear if-else chain. For 4+
    /// functions, builds a balanced binary search tree sorted by selector value
    /// for O(log N) dispatch instead of O(N).
    ///
    /// Uses `LetBind` to compute the calldata selector once, then Var references
    /// in each condition to avoid redundant CALLDATALOAD+SHR per branch.
    fn build_dispatcher(
        &self,
        contract_name: &str,
        fn_bodies: &[(&edge_ast::ContractFnDecl, Option<RcExpr>)],
    ) -> Result<RcExpr, IrError> {
        let ctx = EvmContext::InFunction(format!("{contract_name}::dispatcher"));

        // Fallback: REVERT if no selector matches
        let fallback: RcExpr = ast_helpers::revert(
            ast_helpers::const_int(0, ctx.clone()),
            ast_helpers::const_int(0, ctx.clone()),
            Rc::new(EvmExpr::Arg(
                EvmType::Base(EvmBaseType::StateT),
                ctx.clone(),
            )),
        );

        // Collect dispatchable functions with their selector values
        let mut entries: Vec<(u32, String, RcExpr)> = Vec::new();
        for (fn_decl, body_ir) in fn_bodies {
            if !fn_decl.is_ext && !fn_decl.is_pub {
                continue;
            }
            let body = match body_ir {
                Some(b) => Rc::clone(b),
                None => continue,
            };
            let sig = self.compute_function_signature(&fn_decl.name.name, &fn_decl.params);
            let sel_val = Self::compute_selector_value(&sig);
            entries.push((sel_val, sig, body));
        }

        if entries.is_empty() {
            return Ok(fallback);
        }

        // Sort by selector value for binary search
        entries.sort_by_key(|(sel, _, _)| *sel);

        let selector_var = ast_helpers::var("__selector".to_string());

        let result = if entries.len() >= 4 {
            // Binary search dispatch for 4+ functions
            Self::build_bst_dispatch(&entries, &selector_var, &fallback, &ctx)
        } else {
            // Linear dispatch for few functions
            Self::build_linear_dispatch(&entries, &selector_var, &fallback, &ctx)
        };

        // Wrap in LetBind that computes the selector once
        // Load first 4 bytes of calldata as selector
        let calldataload = Rc::new(EvmExpr::Bop(
            EvmBinaryOp::CalldataLoad,
            ast_helpers::const_int(0, ctx.clone()),
            Rc::new(EvmExpr::Arg(
                EvmType::Base(EvmBaseType::StateT),
                ctx.clone(),
            )),
        ));
        // Shift right by 224 bits to get top 4 bytes
        // IR convention: Bop(Shr, shift_amount, value)
        let shifted = ast_helpers::bop(
            EvmBinaryOp::Shr,
            ast_helpers::const_int(224, ctx),
            calldataload,
        );

        Ok(ast_helpers::let_bind(
            "__selector".to_string(),
            shifted,
            result,
        ))
    }

    /// Compute the numeric 4-byte selector value for a function signature.
    fn compute_selector_value(sig: &str) -> u32 {
        let mut hash = [0u8; 32];
        edge_types::bytes::hash_bytes(&mut hash, sig);
        u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]])
    }

    /// Build a linear if-else dispatch chain (for < 4 functions).
    fn build_linear_dispatch(
        entries: &[(u32, String, RcExpr)],
        selector_var: &RcExpr,
        fallback: &RcExpr,
        ctx: &EvmContext,
    ) -> RcExpr {
        let mut result = Rc::clone(fallback);
        for (_sel_val, sig, body) in entries.iter().rev() {
            let selector_expr = ast_helpers::selector(sig.clone());
            let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
            let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
            result = ast_helpers::if_then_else(cond, inputs, Rc::clone(body), result);
        }
        result
    }

    /// Build a balanced binary search tree dispatch (for 4+ functions).
    ///
    /// At each node: check EQ with pivot selector. If no match, use GT
    /// to decide which subtree to recurse into.
    fn build_bst_dispatch(
        entries: &[(u32, String, RcExpr)],
        selector_var: &RcExpr,
        fallback: &RcExpr,
        ctx: &EvmContext,
    ) -> RcExpr {
        match entries.len() {
            0 => Rc::clone(fallback),
            1 => {
                // Leaf: single EQ check
                let (_, sig, body) = &entries[0];
                let selector_expr = ast_helpers::selector(sig.clone());
                let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
                let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
                ast_helpers::if_then_else(cond, inputs, Rc::clone(body), Rc::clone(fallback))
            }
            2 => {
                // Two entries: linear chain (no benefit from GT)
                let right = {
                    let (_, sig, body) = &entries[1];
                    let selector_expr = ast_helpers::selector(sig.clone());
                    let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
                    let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
                    ast_helpers::if_then_else(cond, inputs, Rc::clone(body), Rc::clone(fallback))
                };
                let (_, sig, body) = &entries[0];
                let selector_expr = ast_helpers::selector(sig.clone());
                let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
                let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
                ast_helpers::if_then_else(cond, inputs, Rc::clone(body), right)
            }
            _ => {
                // Split at midpoint
                let mid = entries.len() / 2;
                let (pivot_val, pivot_sig, pivot_body) = &entries[mid];

                // EQ check with pivot
                let pivot_selector = ast_helpers::selector(pivot_sig.clone());
                let eq_cond = ast_helpers::eq(Rc::clone(selector_var), pivot_selector);

                // GT comparison for branching
                let pivot_const = ast_helpers::const_int(*pivot_val as i64, ctx.clone());
                let gt_cond =
                    ast_helpers::bop(EvmBinaryOp::Gt, Rc::clone(selector_var), pivot_const);

                let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());

                // Recurse on left (selectors < pivot) and right (selectors > pivot)
                let left_tree =
                    Self::build_bst_dispatch(&entries[..mid], selector_var, fallback, ctx);
                let right_tree =
                    Self::build_bst_dispatch(&entries[mid + 1..], selector_var, fallback, ctx);

                // If GT(sel, pivot) then right_tree else left_tree
                let gt_branch =
                    ast_helpers::if_then_else(gt_cond, Rc::clone(&inputs), right_tree, left_tree);

                // If EQ(sel, pivot) then pivot_body else gt_branch
                ast_helpers::if_then_else(eq_cond, inputs, Rc::clone(pivot_body), gt_branch)
            }
        }
    }

    /// Compute the ABI function signature string.
    fn compute_function_signature(
        &self,
        name: &str,
        params: &[(edge_ast::Ident, edge_ast::ty::TypeSig)],
    ) -> String {
        let param_types: Vec<String> = params
            .iter()
            .map(|(_, ty)| Self::type_sig_to_abi_string(ty))
            .collect();
        format!("{name}({})", param_types.join(","))
    }

    /// Convert a type signature to its ABI string representation.
    fn type_sig_to_abi_string(ty: &edge_ast::ty::TypeSig) -> String {
        match ty {
            edge_ast::ty::TypeSig::Primitive(prim) => match prim {
                edge_ast::ty::PrimitiveType::UInt(bits) => format!("uint{bits}"),
                edge_ast::ty::PrimitiveType::Int(bits) => format!("int{bits}"),
                edge_ast::ty::PrimitiveType::FixedBytes(bytes) => format!("bytes{bytes}"),
                edge_ast::ty::PrimitiveType::Address => "address".to_owned(),
                edge_ast::ty::PrimitiveType::Bool | edge_ast::ty::PrimitiveType::Bit => {
                    "bool".to_owned()
                }
            },
            edge_ast::ty::TypeSig::Pointer(_, inner) => Self::type_sig_to_abi_string(inner),
            _ => "uint256".to_owned(), // fallback
        }
    }

    // ---- Type lowering helpers ----

    /// Extract the data location from a contract field's type signature.
    /// `&s T` → Storage (persistent), `&t T` → Transient, bare `T` → Storage (default).
    const fn extract_data_location(ty: &edge_ast::ty::TypeSig) -> DataLocation {
        match ty {
            // &t T → Transient storage
            edge_ast::ty::TypeSig::Pointer(edge_ast::ty::Location::Transient, _) => {
                DataLocation::Transient
            }
            // &s T or bare T → persistent storage
            _ => DataLocation::Storage,
        }
    }

    /// Lower a type signature to an EVM IR type.
    fn lower_type_sig(&self, ty: &edge_ast::ty::TypeSig) -> EvmType {
        match ty {
            edge_ast::ty::TypeSig::Primitive(prim) => {
                EvmType::Base(self.lower_primitive_base_type(prim))
            }
            edge_ast::ty::TypeSig::Pointer(_, inner) => self.lower_type_sig(inner),
            edge_ast::ty::TypeSig::Tuple(types) => {
                let base_types: Vec<EvmBaseType> = types
                    .iter()
                    .map(|t| match self.lower_type_sig(t) {
                        EvmType::Base(b) => b,
                        EvmType::TupleT(_) => EvmBaseType::UIntT(256), // flatten nested tuples
                    })
                    .collect();
                EvmType::TupleT(base_types)
            }
            _ => EvmType::Base(EvmBaseType::UIntT(256)), // fallback for unhandled types
        }
    }

    /// Lower a primitive type to an EVM base type.
    const fn lower_primitive_type(&self, prim: &edge_ast::ty::PrimitiveType) -> EvmType {
        EvmType::Base(self.lower_primitive_base_type(prim))
    }

    /// Lower a primitive type to an EVM base type.
    const fn lower_primitive_base_type(&self, prim: &edge_ast::ty::PrimitiveType) -> EvmBaseType {
        match prim {
            edge_ast::ty::PrimitiveType::UInt(bits) => EvmBaseType::UIntT(*bits),
            edge_ast::ty::PrimitiveType::Int(bits) => EvmBaseType::IntT(*bits),
            edge_ast::ty::PrimitiveType::FixedBytes(bytes) => EvmBaseType::BytesT(*bytes),
            edge_ast::ty::PrimitiveType::Address => EvmBaseType::AddrT,
            edge_ast::ty::PrimitiveType::Bool | edge_ast::ty::PrimitiveType::Bit => {
                EvmBaseType::BoolT
            }
        }
    }

    /// Build input type from function parameters.
    fn params_to_type(&self, params: &[(edge_ast::Ident, edge_ast::ty::TypeSig)]) -> EvmType {
        match params.len() {
            0 => EvmType::Base(EvmBaseType::UnitT),
            1 => self.lower_type_sig(&params[0].1),
            _ => {
                let base_types: Vec<EvmBaseType> = params
                    .iter()
                    .map(|(_, ty)| match self.lower_type_sig(ty) {
                        EvmType::Base(b) => b,
                        EvmType::TupleT(_) => EvmBaseType::UIntT(256),
                    })
                    .collect();
                EvmType::TupleT(base_types)
            }
        }
    }

    /// Build output type from return types.
    fn returns_to_type(&self, returns: &[edge_ast::ty::TypeSig]) -> EvmType {
        match returns.len() {
            0 => EvmType::Base(EvmBaseType::UnitT),
            1 => self.lower_type_sig(&returns[0]),
            _ => {
                let base_types: Vec<EvmBaseType> = returns
                    .iter()
                    .map(|ty| match self.lower_type_sig(ty) {
                        EvmType::Base(b) => b,
                        EvmType::TupleT(_) => EvmBaseType::UIntT(256),
                    })
                    .collect();
                EvmType::TupleT(base_types)
            }
        }
    }
}
