//! Control flow lowering: if/else, loops.

use std::rc::Rc;

use super::{references_any_var, AstToEgglog, Scope, VarBinding};
use crate::{
    ast_helpers,
    schema::{DataLocation, EvmBaseType, EvmExpr, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// Lower if/else chains.
    pub(crate) fn lower_if_else(
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

        // Check if condition is a PatternMatch with data bindings
        let pattern_bindings = if let edge_ast::Expr::PatternMatch(pm_expr, pattern, _) = cond {
            let is_data = self.union_has_data(&pattern.union_name.name);
            if is_data && !pattern.bindings.is_empty() {
                Some((pm_expr.as_ref().clone(), pattern.clone()))
            } else {
                None
            }
        } else {
            None
        };

        let mut binding_vars: Vec<(String, String)> = Vec::new();
        let disc_ir_for_bindings;

        if let Some((ref pm_expr, ref pattern)) = pattern_bindings {
            // For data unions, lower the discriminant expression to get memory base
            disc_ir_for_bindings = Some(self.lower_expr(pm_expr)?);

            // Add bindings to scope before lowering body
            for binding in &pattern.bindings {
                let var_name = format!("{}__local_{}", self.inline_prefix, binding.name);
                binding_vars.push((binding.name.clone(), var_name.clone()));
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(
                        binding.name.clone(),
                        VarBinding {
                            value: ast_helpers::var(var_name.clone()),
                            location: DataLocation::Stack,
                            storage_slot: None,
                            _ty: EvmType::Base(EvmBaseType::UIntT(256)),
                            let_bind_name: Some(var_name),
                            composite_type: None,
                            composite_base: None,
                        },
                    );
            }
        } else {
            disc_ir_for_bindings = None;
        }

        let cond_ir = self.lower_expr(cond)?;
        let mut then_ir = self.lower_code_block(body)?;

        // Wrap then-body with LetBind for pattern bindings
        if !binding_vars.is_empty() {
            if let Some(ref disc_base) = disc_ir_for_bindings {
                let data_offset = ast_helpers::add(
                    Rc::clone(disc_base),
                    ast_helpers::const_int(32, self.current_ctx.clone()),
                );
                let data_val = ast_helpers::mload(data_offset, Rc::clone(&self.current_state));
                for (_binding_name, var_name) in binding_vars.iter().rev() {
                    then_ir = ast_helpers::concat(then_ir, ast_helpers::drop_var(var_name.clone()));
                    then_ir =
                        ast_helpers::let_bind(var_name.clone(), Rc::clone(&data_val), then_ir);
                }
            }
            // Remove bindings from scope
            for (binding_name, _) in &binding_vars {
                if let Some(scope) = self.scopes.last_mut() {
                    scope.bindings.swap_remove(binding_name);
                }
            }
        }

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
    pub(crate) fn lower_while_loop(
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
    pub(crate) fn lower_for_loop(
        &mut self,
        init: Option<&edge_ast::Stmt>,
        cond: Option<&edge_ast::Expr>,
        update: Option<&edge_ast::Stmt>,
        loop_block: &edge_ast::LoopBlock,
    ) -> Result<RcExpr, IrError> {
        self.scopes.push(Scope::new());

        // Extract VarDecl names from the init statement (which may be a CodeBlock)
        // so we can wrap the entire for-loop in LetBind for those variables.
        let mut init_var_names: Vec<String> = Vec::new();
        let mut init_stmts: Vec<RcExpr> = Vec::new();

        if let Some(init_stmt) = init {
            // Flatten the init CodeBlock: process VarDecl and other stmts individually
            let items: Vec<&edge_ast::Stmt> = match init_stmt {
                edge_ast::Stmt::CodeBlock(cb) => cb
                    .stmts
                    .iter()
                    .filter_map(|item| match item {
                        edge_ast::BlockItem::Stmt(s) => Some(s.as_ref()),
                        _ => None,
                    })
                    .collect(),
                other => vec![other],
            };

            for stmt in &items {
                match stmt {
                    edge_ast::Stmt::VarDecl(ident, type_sig, _init, _span) => {
                        let ty = type_sig
                            .as_ref()
                            .map(|ts| self.lower_type_sig(ts))
                            .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));
                        let zero = ast_helpers::const_int(0, self.current_ctx.clone());
                        let prefix = &self.inline_prefix;
                        let var_name = format!("{prefix}__local_{}", ident.name);
                        let binding = VarBinding {
                            value: zero,
                            location: DataLocation::Memory,
                            storage_slot: None,
                            _ty: ty,
                            let_bind_name: Some(var_name.clone()),
                            composite_type: None,
                            composite_base: None,
                        };
                        self.scopes
                            .last_mut()
                            .expect("scope stack empty")
                            .bindings
                            .insert(ident.name.clone(), binding);
                        init_var_names.push(var_name);
                    }
                    other => {
                        init_stmts.push(self.lower_stmt(other)?);
                    }
                }
            }
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

        // Build: init_stmts ; if (cond) { do-while } else { empty }
        let mut result = ast_helpers::if_then_else(cond_ir, inputs, loop_ir, empty);

        // Prepend any non-VarDecl init statements (e.g., the initial assignment i = 0)
        for init_ir in init_stmts.into_iter().rev() {
            result = ast_helpers::concat(init_ir, result);
        }

        // Wrap in LetBind/Drop for each init VarDecl (outermost first = reversed)
        for var_name in init_var_names.iter().rev() {
            result = ast_helpers::concat(result, ast_helpers::drop_var(var_name.clone()));
            let init = ast_helpers::const_int(0, self.current_ctx.clone());
            result = ast_helpers::let_bind(var_name.clone(), init, result);
        }

        self.scopes.pop();

        Ok(result)
    }

    /// Lower an infinite loop.
    pub(crate) fn lower_infinite_loop(
        &mut self,
        loop_block: &edge_ast::LoopBlock,
    ) -> Result<RcExpr, IrError> {
        let body_ir = self.lower_loop_block(loop_block)?;
        let true_const = ast_helpers::const_bool(true, self.current_ctx.clone());
        // Body runs first, then always-true condition
        let pred_and_body = ast_helpers::concat(body_ir, true_const);
        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        Ok(ast_helpers::do_while(inputs, pred_and_body))
    }

    /// Lower a do-while loop.
    pub(crate) fn lower_do_while(
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
    ///
    /// Like `lower_code_block`, scans for `VarDecl` items and wraps them in
    /// `LetBind`/`Drop` so that codegen can allocate memory slots for loop-local
    /// variables.
    pub(crate) fn lower_loop_block(
        &mut self,
        block: &edge_ast::LoopBlock,
    ) -> Result<RcExpr, IrError> {
        // First pass: scan for VarDecl names
        let var_decl_names: Vec<String> = block
            .items
            .iter()
            .filter_map(|item| match item {
                edge_ast::LoopItem::Stmt(stmt) => match stmt.as_ref() {
                    edge_ast::Stmt::VarDecl(ident, _, _, _) => Some(ident.name.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();

        // Lower all items
        let mut stmts: Vec<RcExpr> = Vec::new();
        for item in &block.items {
            let item_ir = match item {
                edge_ast::LoopItem::Stmt(stmt) => self.lower_stmt(stmt)?,
                edge_ast::LoopItem::Expr(expr) => self.lower_expr(expr)?,
                edge_ast::LoopItem::Break(span) => {
                    self.warnings.push(
                        edge_diagnostics::Diagnostic::warning(
                            "`break` is not yet implemented and will be ignored",
                        )
                        .with_label(span.clone(), "has no effect"),
                    );
                    continue;
                }
                edge_ast::LoopItem::Continue(span) => {
                    self.warnings.push(
                        edge_diagnostics::Diagnostic::warning(
                            "`continue` is not yet implemented and will be ignored",
                        )
                        .with_label(span.clone(), "has no effect"),
                    );
                    continue;
                }
            };
            stmts.push(item_ir);
        }

        if stmts.is_empty() {
            return Ok(ast_helpers::empty(
                EvmType::Base(EvmBaseType::UnitT),
                self.current_ctx.clone(),
            ));
        }

        // Store-forwarding for loop-local VarDecls (same logic as lower_code_block)
        let prefix = &self.inline_prefix;
        let local_var_names: Vec<String> = var_decl_names
            .iter()
            .map(|n| format!("{prefix}__local_{n}"))
            .collect();
        let mut var_inits: std::collections::HashMap<String, RcExpr> =
            std::collections::HashMap::new();
        for (i, name) in var_decl_names.iter().enumerate() {
            let var_name = format!("{prefix}__local_{name}");
            let idx = stmts
                .iter()
                .position(|s| matches!(s.as_ref(), EvmExpr::VarStore(n, _) if n == &var_name));
            let Some(idx) = idx else { continue };
            let preceding_ok = stmts[..idx]
                .iter()
                .all(|s| matches!(s.as_ref(), EvmExpr::Empty(..)));
            if !preceding_ok {
                continue;
            }
            let init_val = match stmts[idx].as_ref() {
                EvmExpr::VarStore(_, val) => Rc::clone(val),
                _ => unreachable!(),
            };
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

        // Wrap in LetBinds for loop-local variables
        for name in var_decl_names.iter().rev() {
            let var_name = format!("{prefix}__local_{name}");
            result = ast_helpers::concat(result, ast_helpers::drop_var(var_name.clone()));
            let init = var_inits
                .remove(&var_name)
                .unwrap_or_else(|| ast_helpers::const_int(0, self.current_ctx.clone()));
            result = ast_helpers::let_bind(var_name, init, result);
        }

        Ok(result)
    }
}
