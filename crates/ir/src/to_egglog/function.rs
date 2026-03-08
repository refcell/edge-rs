//! Function lowering: contract function bodies, standalone functions, code blocks.

use std::rc::Rc;

use super::{references_any_var, AstToEgglog, Scope, VarBinding};
use crate::{
    ast_helpers,
    schema::{DataLocation, EvmBaseType, EvmBinaryOp, EvmContext, EvmExpr, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// Lower a contract function body into IR.
    pub(crate) fn lower_contract_fn_body(
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
        let mut calldata_offset: usize = 4; // After 4-byte selector
        let mut array_param_prefix = ast_helpers::empty(
            EvmType::Base(EvmBaseType::UIntT(256)),
            self.current_ctx.clone(),
        );
        for (ident, type_sig) in &fn_decl.params {
            let ty = self.lower_type_sig(type_sig);

            // Check if this is an array type (needs multi-word calldata decoding)
            let resolved = self.resolve_type_alias(type_sig).clone();
            let array_len = match &resolved {
                edge_ast::ty::TypeSig::Array(_, len_expr)
                | edge_ast::ty::TypeSig::PackedArray(_, len_expr) => {
                    Self::extract_array_length(len_expr)
                }
                _ => None,
            };

            if let Some(n) = array_len {
                // Array parameter: allocate memory, copy N elements from calldata
                let base = self.next_memory_offset;
                self.next_memory_offset += n * 32;

                // Use a single CALLDATACOPY to bulk-copy array data from calldata to memory
                let dest_off = ast_helpers::const_int(base as i64, self.current_ctx.clone());
                let cd_off =
                    ast_helpers::const_int(calldata_offset as i64, self.current_ctx.clone());
                let size = ast_helpers::const_int((n * 32) as i64, self.current_ctx.clone());
                let copy = ast_helpers::calldatacopy(dest_off, cd_off, size);
                array_param_prefix = ast_helpers::concat(array_param_prefix, copy);

                let binding = VarBinding {
                    value: ast_helpers::const_int(base as i64, self.current_ctx.clone()),
                    location: DataLocation::Stack,
                    storage_slot: None,
                    _ty: ty,
                    let_bind_name: None,
                    composite_type: Some("__array__".to_string()),
                    composite_base: Some(base),
                };
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(ident.name.clone(), binding);
                calldata_offset += n * 32;
            } else {
                // Scalar parameter: single 32-byte calldataload
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
                    composite_type: None,
                    composite_base: None,
                };
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(ident.name.clone(), binding);
                calldata_offset += 32;
            }
        }

        // Lower body
        let body_ir = self.lower_code_block(body)?;

        self.scopes.pop();

        // Prepend array parameter loading before body
        let full_body = ast_helpers::concat(array_param_prefix, body_ir);

        // Append a STOP (RETURN with 0 size) after the body.
        // If the body already ends with RETURN, this is unreachable dead code.
        let stop = ast_helpers::return_op(
            ast_helpers::const_int(0, self.current_ctx.clone()),
            ast_helpers::const_int(0, self.current_ctx.clone()),
            Rc::clone(&self.current_state),
        );
        Ok(ast_helpers::concat(full_body, stop))
    }

    /// Lower a standalone function.
    pub(crate) fn lower_function(
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

            // Check if this is an array-typed parameter
            let resolved = self.resolve_type_alias(type_sig).clone();
            let is_array = matches!(
                &resolved,
                edge_ast::ty::TypeSig::Array(..) | edge_ast::ty::TypeSig::PackedArray(..)
            );

            let binding = VarBinding {
                value: param_val,
                location: DataLocation::Stack,
                storage_slot: None,
                _ty: ty,
                let_bind_name: None,
                // Mark array params so element access uses dynamic base
                composite_type: if is_array {
                    Some("__array_param__".to_string())
                } else {
                    None
                },
                composite_base: None, // dynamic base — resolved at element access
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

    /// Check if a code block ends with a `Return` statement (possibly nested
    /// inside inner blocks/if-else chains).
    pub(crate) fn block_ends_with_return(block: &edge_ast::CodeBlock) -> bool {
        block.stmts.last().is_some_and(|item| match item {
            edge_ast::BlockItem::Stmt(stmt) => match stmt.as_ref() {
                edge_ast::Stmt::Return(_, _) => true,
                edge_ast::Stmt::CodeBlock(inner) => Self::block_ends_with_return(inner),
                _ => false,
            },
            _ => false,
        })
    }

    /// Restructure inline if-return patterns in a code block.
    ///
    /// Transforms `if (cond) { ...return val; } stmt1; stmt2; ...` into
    /// `if (cond) { ...return val; } else { stmt1; stmt2; ... }` so that
    /// when `return` is lowered as a value (`inline_depth` > 0), both branches
    /// of the `If` produce a value, avoiding stack depth mismatch.
    ///
    /// Only transforms `IfElse` with no else block where the then-block
    /// contains an explicit `Return` statement. Normal if-without-else blocks
    /// (like `if (val >= 128) { val >>= 128; }`) are left untouched.
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn restructure_inline_returns<'a>(
        &self,
        block: &'a edge_ast::CodeBlock,
    ) -> std::borrow::Cow<'a, edge_ast::CodeBlock> {
        // Scan for the first IfElse(branches, None) where the then-block ends with Return
        let idx = block.stmts.iter().position(|item| {
            if let edge_ast::BlockItem::Stmt(stmt) = item {
                if let edge_ast::Stmt::IfElse(branches, None) = stmt.as_ref() {
                    if let Some((_, then_block)) = branches.first() {
                        return Self::block_ends_with_return(then_block);
                    }
                }
            }
            false
        });

        let Some(idx) = idx else {
            return std::borrow::Cow::Borrowed(block);
        };

        // No remaining statements after the if — nothing to fold
        if idx + 1 >= block.stmts.len() {
            return std::borrow::Cow::Borrowed(block);
        }

        // Build the else block from remaining statements
        let remaining: Vec<edge_ast::BlockItem> = block.stmts[idx + 1..].to_vec();
        let else_block = edge_ast::CodeBlock {
            stmts: remaining,
            span: block.span.clone(),
        };

        // Recursively restructure the else block (there may be more if-return patterns)
        let else_block = match self.restructure_inline_returns(&else_block) {
            std::borrow::Cow::Borrowed(_) => else_block,
            std::borrow::Cow::Owned(restructured) => restructured,
        };

        // Replace the IfElse with one that has the else block
        let original_stmt = &block.stmts[idx];
        let new_stmt = if let edge_ast::BlockItem::Stmt(stmt) = original_stmt {
            if let edge_ast::Stmt::IfElse(branches, None) = stmt.as_ref() {
                edge_ast::BlockItem::Stmt(Box::new(edge_ast::Stmt::IfElse(
                    branches.clone(),
                    Some(else_block),
                )))
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        };

        // Build new stmts: everything before idx + the restructured if-else
        let mut new_stmts = block.stmts[..idx].to_vec();
        new_stmts.push(new_stmt);

        std::borrow::Cow::Owned(edge_ast::CodeBlock {
            stmts: new_stmts,
            span: block.span.clone(),
        })
    }

    /// Lower a code block (sequence of statements).
    ///
    /// All statements are concatenated so that side effects (SSTORE, MSTORE,
    /// LOG, etc.) from every statement are preserved in the IR tree and will
    /// be compiled by codegen.
    pub(crate) fn lower_code_block(
        &mut self,
        block: &edge_ast::CodeBlock,
    ) -> Result<RcExpr, IrError> {
        // When inlining, restructure `if (cond) { return val; } rest...` into
        // `if (cond) { val } else { rest... }`. This avoids stack depth mismatch
        // in codegen since both branches produce values. Only applies when the
        // then-block ends with a Return (AST-level check, not IR-level).
        let block = if self.inline_depth > 0 {
            self.restructure_inline_returns(block)
        } else {
            std::borrow::Cow::Borrowed(block)
        };
        let block = block.as_ref();

        // First pass: scan for VarDecl names to identify memory-backed locals.
        // We need this list BEFORE lowering to know which variables to wrap in LetBinds.
        let var_decl_names: Vec<String> = block
            .stmts
            .iter()
            .filter_map(|item| match item {
                edge_ast::BlockItem::Stmt(stmt) => match stmt.as_ref() {
                    edge_ast::Stmt::VarDecl(ident, _, _, _) => Some(ident.name.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();

        // Lower all statements
        let mut stmts: Vec<RcExpr> = Vec::new();
        for item in &block.stmts {
            let ir = match item {
                edge_ast::BlockItem::Stmt(stmt) => {
                    // Check for expression-statements with unused return values
                    if let edge_ast::Stmt::Expr(expr) = stmt.as_ref() {
                        self.check_unused_return_value(expr);
                    }
                    self.lower_stmt(stmt)?
                }
                edge_ast::BlockItem::Expr(expr) => {
                    self.check_unused_return_value(expr);
                    self.lower_expr(expr)?
                }
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
        let prefix = &self.inline_prefix;
        let local_var_names: Vec<String> = var_decl_names
            .iter()
            .map(|n| format!("{prefix}__local_{n}"))
            .collect();
        let mut var_inits: std::collections::HashMap<String, RcExpr> =
            std::collections::HashMap::new();
        for (i, name) in var_decl_names.iter().enumerate() {
            let var_name = format!("{prefix}__local_{name}");

            // Find the first VarStore for this variable
            let idx = stmts
                .iter()
                .position(|s| matches!(s.as_ref(), EvmExpr::VarStore(n, _) if n == &var_name));
            let Some(idx) = idx else { continue };

            // All preceding statements must be Empty (uninit VarDecl) or VarStore
            // (init VarDecl / earlier assignment) — no side effects that could depend
            // on memory state.
            let preceding_ok = stmts[..idx]
                .iter()
                .all(|s| matches!(s.as_ref(), EvmExpr::Empty(..) | EvmExpr::VarStore(..)));
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

        let mut result = if stmts.is_empty() {
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone())
        } else {
            let mut r = Rc::clone(&stmts[0]);
            for stmt in &stmts[1..] {
                r = ast_helpers::concat(r, Rc::clone(stmt));
            }
            r
        };

        // Wrap the result in LetBinds for memory-backed locals (innermost first).
        // Each variable gets: Drop(var) appended to body, then wrapped in LetBind.
        // Drop marks the end of the variable's lifetime for slot reclamation.
        //
        // When inlining (inline_depth > 0), the last expression in the block is
        // the return value. We must not append Drop after it (that would make
        // Drop the "result" of the Concat, losing the return value). Instead,
        // we insert Drops between the side-effect prefix and the return value.
        for name in var_decl_names.iter().rev() {
            let var_name = format!("{prefix}__local_{name}");
            // When inlining, don't append Drop — the last expression is the
            // return value and Concat(result, Drop) would lose it (Drop pushes
            // nothing). LetBind's codegen already cleans up if Drop wasn't reached.
            if self.inline_depth == 0 {
                result = ast_helpers::concat(result, ast_helpers::drop_var(var_name.clone()));
            }
            let init = var_inits
                .remove(&var_name)
                .unwrap_or_else(|| ast_helpers::const_int(0, self.current_ctx.clone()));
            result = ast_helpers::let_bind(var_name, init, result);
        }

        Ok(result)
    }

    /// Lower an internal function body once as a Function node.
    /// Uses `inline_depth` > 0 so `return` produces just the value (not `ReturnOp`).
    /// Parameters are bound via Arg/Get so the body works as a standalone subroutine.
    pub(crate) fn lower_internal_function_body(
        &mut self,
        name: &str,
        params: &[(String, edge_ast::ty::TypeSig)],
    ) -> Result<(), IrError> {
        let saved_ctx = self.current_ctx.clone();
        let saved_state = Rc::clone(&self.current_state);

        self.current_ctx = EvmContext::InFunction(name.to_string());
        self.current_state = Rc::new(EvmExpr::Arg(
            EvmType::Base(EvmBaseType::StateT),
            self.current_ctx.clone(),
        ));

        let in_ty = self.params_to_type(
            &params
                .iter()
                .map(|(n, t)| {
                    (
                        edge_ast::Ident {
                            name: n.clone(),
                            span: edge_types::span::Span::default(),
                        },
                        t.clone(),
                    )
                })
                .collect::<Vec<_>>(),
        );
        let out_ty = EvmType::Base(EvmBaseType::UIntT(256)); // TODO: derive from return type

        // Bind parameters via Arg/Get
        self.scopes.push(Scope::new());
        let arg_expr = Rc::new(EvmExpr::Arg(in_ty.clone(), self.current_ctx.clone()));
        for (i, (param_name, param_ty)) in params.iter().enumerate() {
            let ty = self.lower_type_sig(param_ty);
            let param_val = if params.len() == 1 {
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
                composite_type: None,
                composite_base: None,
            };
            self.scopes
                .last_mut()
                .expect("scope stack empty")
                .bindings
                .insert(param_name.clone(), binding);
        }

        // Lower body with inline_depth > 0 so return produces value, not ReturnOp
        self.inline_depth += 1;
        // Find the body from contract_functions or free_fn_bodies
        let body = self
            .contract_functions
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, _, b)| b.clone())
            .or_else(|| {
                self.free_fn_bodies
                    .iter()
                    .find(|f| f.name == name)
                    .map(|f| f.body.clone())
            })
            .ok_or_else(|| IrError::Unsupported(format!("internal function not found: {name}")))?;
        let body_ir = self.lower_code_block(&body)?;
        self.inline_depth -= 1;
        self.scopes.pop();

        self.current_ctx = saved_ctx;
        self.current_state = saved_state;

        let func_node = ast_helpers::function(name.to_string(), in_ty, out_ty, body_ir);
        self.lowered_functions.push(func_node);
        Ok(())
    }
}
