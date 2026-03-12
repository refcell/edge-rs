//! Pattern matching lowering: match statements, if-match, union patterns.

use std::rc::Rc;

use super::{AstToEgglog, VarBinding};
use crate::{
    ast_helpers,
    schema::{DataLocation, EvmBaseType, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// Lower a pattern match expression: `expr matches Type::Variant` → `EQ(expr, idx)`.
    pub(crate) fn lower_pattern_match(
        &mut self,
        expr: &edge_ast::Expr,
        pattern: &edge_ast::pattern::UnionPattern,
    ) -> Result<RcExpr, IrError> {
        let disc_ir = self.lower_expr(expr)?;
        let idx = self.variant_index(
            &pattern.union_name.name,
            &pattern.member_name.name,
            Some(&pattern.span),
        )?;
        let idx_ir = ast_helpers::const_int(idx as i64, self.current_ctx.clone());
        Ok(ast_helpers::eq(disc_ir, idx_ir))
    }

    /// Check if a union type has any data-carrying variants.
    /// Handles both concrete and generic (monomorphized) union types.
    pub(crate) fn union_has_data(&self, type_name: &str) -> bool {
        let variants = self.union_types.get(type_name).or_else(|| {
            self.resolve_generic_type_name(type_name)
                .and_then(|mangled| self.union_types.get(&mangled))
        });
        variants
            .map(|v| v.iter().any(|(_, has_data)| *has_data))
            .unwrap_or(false)
    }

    /// Lower a match statement to nested if-else chains.
    ///
    /// `match d { A::X => { body1 }, A::Y => { body2 }, _ => { body3 } }`
    /// becomes: `if (d == 0) { body1 } else if (d == 1) { body2 } else { body3 }`
    pub(crate) fn lower_match(
        &mut self,
        discriminant: &edge_ast::Expr,
        arms: &[edge_ast::pattern::MatchArm],
    ) -> Result<RcExpr, IrError> {
        let disc_ir = self.lower_expr(discriminant)?;

        // Determine if this is a data-carrying union by checking the first Union pattern
        let union_name = arms.iter().find_map(|arm| {
            if let edge_ast::pattern::MatchPattern::Union(up) = &arm.pattern {
                Some(up.union_name.name.clone())
            } else {
                None
            }
        });
        let is_data_union = union_name
            .as_ref()
            .map(|n| self.union_has_data(n))
            .unwrap_or(false);

        // For data unions, disc_ir is a memory base address.
        // Load the discriminant integer from memory.
        let disc_val = if is_data_union {
            ast_helpers::mload(Rc::clone(&disc_ir), Rc::clone(&self.current_state))
        } else {
            Rc::clone(&disc_ir)
        };

        // Separate concrete variant arms from wildcard/ident catch-all
        let mut variant_arms: Vec<(usize, &edge_ast::stmt::CodeBlock, Vec<String>)> = Vec::new();
        let mut default_arm: Option<&edge_ast::stmt::CodeBlock> = None;

        for arm in arms {
            match &arm.pattern {
                edge_ast::pattern::MatchPattern::Union(up) => {
                    let idx = self.variant_index(
                        &up.union_name.name,
                        &up.member_name.name,
                        Some(&up.span),
                    )?;
                    let bindings: Vec<String> =
                        up.bindings.iter().map(|b| b.name.clone()).collect();
                    variant_arms.push((idx, &arm.body, bindings));
                }
                edge_ast::pattern::MatchPattern::Wildcard
                | edge_ast::pattern::MatchPattern::Ident(_) => {
                    default_arm = Some(&arm.body);
                }
            }
        }

        // Build nested if-else from back to front
        let mut result = if let Some(body) = default_arm {
            self.lower_code_block(body)?
        } else {
            // No default arm: exhaustive match. Use revert as unreachable fallback
            // so halting-branch detection in codegen allows stack depth mismatch.
            ast_helpers::revert(
                ast_helpers::const_int(0, self.current_ctx.clone()),
                ast_helpers::const_int(0, self.current_ctx.clone()),
                Rc::clone(&self.current_state),
            )
        };

        for (idx, body, bindings) in variant_arms.into_iter().rev() {
            let idx_ir = ast_helpers::const_int(idx as i64, self.current_ctx.clone());
            let cond = ast_helpers::eq(Rc::clone(&disc_val), idx_ir);
            let inputs =
                ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());

            let has_bindings = is_data_union && !bindings.is_empty();
            let mut binding_vars: Vec<(String, String)> = Vec::new();

            // Add bindings to scope BEFORE lowering body so variables resolve
            if has_bindings {
                for binding_name in &bindings {
                    let var_name = format!("{}__local_{}", self.inline_prefix, binding_name);
                    binding_vars.push((binding_name.clone(), var_name.clone()));
                    self.scopes
                        .last_mut()
                        .expect("scope stack empty")
                        .bindings
                        .insert(
                            binding_name.clone(),
                            VarBinding {
                                value: ast_helpers::var(var_name.clone()),
                                location: DataLocation::Stack,
                                storage_slot: None,
                                _ty: EvmType::Base(EvmBaseType::UIntT(256)),
                                let_bind_name: Some(var_name),
                                composite_type: None,
                                composite_base: None,
                                composite_type_args: Vec::new(),
                                is_dynamic_memory: false,
                            },
                        );
                }
            }

            let mut then_body = self.lower_code_block(body)?;

            // Wrap body with LetBind for extracted data
            if has_bindings {
                let data_offset = ast_helpers::add(
                    Rc::clone(&disc_ir),
                    ast_helpers::const_int(32, self.current_ctx.clone()),
                );
                let data_val = ast_helpers::mload(data_offset, Rc::clone(&self.current_state));
                for (_binding_name, var_name) in binding_vars.iter().rev() {
                    then_body =
                        ast_helpers::concat(then_body, ast_helpers::drop_var(var_name.clone()));
                    then_body =
                        ast_helpers::let_bind(var_name.clone(), Rc::clone(&data_val), then_body);
                }
                // Remove bindings from scope (they're only valid inside this arm)
                for (binding_name, _) in &binding_vars {
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.bindings.swap_remove(binding_name);
                    }
                }
            }

            result = ast_helpers::if_then_else(cond, inputs, then_body, result);
        }

        Ok(result)
    }

    /// Lower `if expr matches Pattern { body }` statement.
    pub(crate) fn lower_if_match(
        &mut self,
        expr: &edge_ast::Expr,
        pattern: &edge_ast::pattern::UnionPattern,
        body: &edge_ast::stmt::CodeBlock,
    ) -> Result<RcExpr, IrError> {
        let disc_ir = self.lower_expr(expr)?;
        let is_data_union = self.union_has_data(&pattern.union_name.name);

        let disc_val = if is_data_union {
            ast_helpers::mload(Rc::clone(&disc_ir), Rc::clone(&self.current_state))
        } else {
            Rc::clone(&disc_ir)
        };

        let idx = self.variant_index(
            &pattern.union_name.name,
            &pattern.member_name.name,
            Some(&pattern.span),
        )?;
        let idx_ir = ast_helpers::const_int(idx as i64, self.current_ctx.clone());
        let cond = ast_helpers::eq(disc_val, idx_ir);
        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());

        let has_bindings = is_data_union && !pattern.bindings.is_empty();
        let mut binding_vars: Vec<(String, String)> = Vec::new();

        // Add bindings to scope BEFORE lowering body
        if has_bindings {
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
                            composite_type_args: Vec::new(),
                            is_dynamic_memory: false,
                        },
                    );
            }
        }

        let mut then_ir = self.lower_code_block(body)?;

        if has_bindings {
            let data_offset = ast_helpers::add(
                Rc::clone(&disc_ir),
                ast_helpers::const_int(32, self.current_ctx.clone()),
            );
            let data_val = ast_helpers::mload(data_offset, Rc::clone(&self.current_state));
            for (_binding_name, var_name) in binding_vars.iter().rev() {
                then_ir = ast_helpers::concat(then_ir, ast_helpers::drop_var(var_name.clone()));
                then_ir = ast_helpers::let_bind(var_name.clone(), Rc::clone(&data_val), then_ir);
            }
            // Remove bindings from scope
            for (binding_name, _) in &binding_vars {
                if let Some(scope) = self.scopes.last_mut() {
                    scope.bindings.swap_remove(binding_name);
                }
            }
        }

        let else_ir =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        Ok(ast_helpers::if_then_else(cond, inputs, then_ir, else_ir))
    }
}
