//! Function call lowering: call resolution, inlining, builtin calls.

use std::collections::HashMap;
use std::rc::Rc;

use super::{AstToEgglog, FreeFnInfo, Scope, VarBinding};
use crate::{
    ast_helpers,
    schema::{DataLocation, EvmBaseType, EvmBinaryOp, EvmExpr, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// For internal contract functions, inlines the function body at the call site
    /// by binding the arguments in a new scope and lowering the body.
    pub(crate) fn lower_function_call(
        &mut self,
        callee: &edge_ast::Expr,
        args: &[edge_ast::Expr],
        explicit_type_args: &[edge_ast::ty::TypeSig],
        call_span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        // Check if this is a union constructor call (e.g., Result::Ok(42))
        if let edge_ast::Expr::Path(components, span) = callee {
            if components.len() == 2 {
                let type_name = &components[0].name;
                let variant_name = &components[1].name;
                if self.union_types.contains_key(type_name) {
                    return self.lower_union_instantiation_expr(type_name, variant_name, args, Some(span));
                }
                // Check for generic union types (e.g., Result::Ok(42) where Result<T> was monomorphized)
                if self.generic_type_templates.contains_key(type_name) {
                    // First try to find an already-monomorphized version
                    if let Some(mangled) = self.resolve_generic_type_name(type_name) {
                        return self.lower_union_instantiation_expr(&mangled, variant_name, args, Some(span));
                    }
                    // No monomorphized version yet — try to infer type params from
                    // the constructor argument and monomorphize on the fly.
                    if let Some(mangled) =
                        self.try_monomorphize_union_from_constructor(type_name, variant_name, args)?
                    {
                        return self.lower_union_instantiation_expr(&mangled, variant_name, args, Some(span));
                    }
                    return Err(IrError::Diagnostic(
                        edge_diagnostics::Diagnostic::error(format!(
                            "cannot infer type parameters for generic type `{type_name}`",
                        ))
                        .with_label(
                            span.clone(),
                            format!("cannot infer type arguments from `{type_name}::{variant_name}(...)`"),
                        )
                        .with_note("provide explicit type arguments, e.g. `{type_name}<u256>::{variant_name}(...)`".to_string()),
                    ));
                }
            }
        }

        // Check for method call: FunctionCall(FieldAccess(obj, method), args)
        if let edge_ast::Expr::FieldAccess(obj, method, span) = callee {
            return self.lower_method_call(obj, &method.name, args, span);
        }

        // Check for qualified trait/type call: Path(["Type", "method"])
        if let edge_ast::Expr::Path(components, _) = callee {
            if components.len() == 2 {
                // Resolve type parameter substitutions (e.g., V → u256 inside Map<K, V> methods)
                let resolved_type = self.type_param_subst
                    .get(&components[0].name)
                    .cloned()
                    .unwrap_or_else(|| components[0].name.clone());
                let type_or_trait = &resolved_type;
                let method_name = &components[1].name;

                let method_span = &components[1].span;

                // Check inherent methods: Type::method(receiver, args...)
                if self.find_inherent_method(type_or_trait, method_name).is_some() {
                    return self.lower_qualified_method_call(
                        type_or_trait,
                        method_name,
                        args,
                        method_span,
                    );
                }

                // Check trait methods: Trait::method(receiver, args...)
                if self.trait_registry.contains_key(type_or_trait)
                    || self.std_ops_traits.contains(type_or_trait)
                {
                    return self.lower_qualified_trait_call(
                        type_or_trait,
                        method_name,
                        args,
                        method_span,
                    );
                }

                // Check primitive type qualified calls: u256::sload(slot), etc.
                // This handles resolved type parameters like V::sload() where V = u256.
                if Self::is_primitive_type(type_or_trait) {
                    return self.lower_qualified_trait_call(
                        type_or_trait,
                        method_name,
                        args,
                        method_span,
                    );
                }

                // Check trait impls for non-primitive types: Map::sload(slot), etc.
                // Directly look up and inline the method from the type's trait impls.
                if let Some((fn_decl, body)) = self.find_trait_method_for_type(type_or_trait, method_name) {
                    let params: Vec<(String, edge_ast::ty::TypeSig)> = fn_decl
                        .params
                        .iter()
                        .map(|(id, ty)| (id.name.clone(), ty.clone()))
                        .collect();
                    return self.inline_function_call(&params, &body, args);
                }
                // Also check inherent methods on the type
                if let Some(method) = self.find_inherent_method(type_or_trait, method_name) {
                    let fn_decl = method.fn_decl.clone();
                    let body = method.body;
                    let params: Vec<(String, edge_ast::ty::TypeSig)> = fn_decl
                        .params
                        .iter()
                        .map(|(id, ty)| (id.name.clone(), ty.clone()))
                        .collect();
                    return self.inline_function_call(&params, &body, args);
                }
            }
        }

        // Get function name, resolving module-prefixed paths.
        let fn_name = match callee {
            edge_ast::Expr::Ident(id) => id.name.clone(),
            edge_ast::Expr::Path(components, _) => self.resolve_path_to_name(components)?,
            _ => {
                return Err(IrError::Unsupported(
                    "dynamic function calls not yet supported".to_owned(),
                ));
            }
        };

        // Check comptime free functions first — always inline these
        if let Some(info) = self
            .free_fn_bodies
            .iter()
            .find(|f| f.name == fn_name && f.is_comptime)
            .cloned()
        {
            return self.inline_function_call(&info.params, &info.body, args);
        }

        // Check contract functions — emit Call (not inline)
        if let Some((name, params, returns, _body)) = self
            .contract_functions
            .iter()
            .find(|(name, _, _, _)| *name == fn_name)
            .cloned()
        {
            return self.emit_call(&name, &params, &returns, args);
        }

        // Check non-comptime free functions — emit Call (not inline)
        if let Some(info) = self
            .free_fn_bodies
            .iter()
            .find(|f| f.name == fn_name && !f.is_comptime)
            .cloned()
        {
            return self.emit_call(&info.name, &info.params, &info.returns, args);
        }

        // Check generic function templates
        if let Some(template) = self.generic_fn_templates.get(&fn_name).cloned() {
            return self.lower_generic_function_call(
                &template,
                args,
                explicit_type_args,
                call_span,
            );
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

        Ok(ast_helpers::call(fn_name, args_ir))
    }

    /// Lower a method call: obj.method(args...)
    /// Resolves to inherent methods or trait methods.
    pub(crate) fn lower_method_call(
        &mut self,
        receiver: &edge_ast::Expr,
        method_name: &str,
        args: &[edge_ast::Expr],
        span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        // Determine receiver type from scope bindings
        let receiver_type = self.infer_receiver_type(receiver);
        let receiver_type_args = self.infer_receiver_type_args(receiver);
        tracing::trace!(
            "lower_method_call: .{}(), receiver_type={:?}",
            method_name,
            receiver_type
        );

        if let Some(ref type_name) = receiver_type {
            // Build type param substitution map for generic types
            let type_param_subst = self.build_type_param_subst(type_name, &receiver_type_args);

            // Check inherent methods first
            if let Some(method) = self.find_inherent_method(type_name, method_name) {
                let fn_decl = method.fn_decl.clone();
                let body = method.body;
                // Prepend receiver to args and inline
                let mut all_args = vec![receiver.clone()];
                all_args.extend_from_slice(args);
                let params: Vec<(String, edge_ast::ty::TypeSig)> = fn_decl
                    .params
                    .iter()
                    .map(|(id, ty)| (id.name.clone(), ty.clone()))
                    .collect();
                // Set type param substitutions for generic method bodies
                let old_subst = std::mem::replace(&mut self.type_param_subst, type_param_subst.clone());
                let result = self.inline_function_call(&params, &body, &all_args);
                self.type_param_subst = old_subst;
                return result;
            }

            // Check trait impls
            if let Some((fn_decl, body)) = self.find_trait_method_for_type(type_name, method_name) {
                let mut all_args = vec![receiver.clone()];
                all_args.extend_from_slice(args);
                let params: Vec<(String, edge_ast::ty::TypeSig)> = fn_decl
                    .params
                    .iter()
                    .map(|(id, ty)| (id.name.clone(), ty.clone()))
                    .collect();
                let old_subst = std::mem::replace(&mut self.type_param_subst, type_param_subst);
                let result = self.inline_function_call(&params, &body, &all_args);
                self.type_param_subst = old_subst;
                return result;
            }

            // Check compiler-provided trait methods for primitive types
            if Self::is_primitive_type(type_name) {
                if let Some(op) = self.compiler_provided_method(method_name) {
                    if args.len() != 1 {
                        return Err(IrError::Diagnostic(
                            edge_diagnostics::Diagnostic::error(format!(
                                "`.{method_name}()` expects exactly 1 argument",
                            ))
                            .with_label(span.clone(), "expected 1 argument"),
                        ));
                    }
                    let lhs = self.lower_expr(receiver)?;
                    let rhs = self.lower_expr(&args[0])?;
                    return Ok(ast_helpers::bop(op, lhs, rhs));
                }

                // Check compiler-provided stateful methods (derive_slot, sload, sstore)
                if let Some(result) =
                    self.try_compiler_stateful_dispatch(receiver, method_name, args)?
                {
                    return Ok(result);
                }
            }

            // Default derive_slot for struct types without explicit UniqueSlot impl.
            // Chains keccak256 over each field like Solidity nested mappings:
            //   slot = keccak256(field_0 . base_slot)
            //   slot = keccak256(field_1 . slot)
            //   ...
            if method_name == "derive_slot"
                && self.std_ops_traits.contains("UniqueSlot")
                && args.len() == 1
            {
                if let Some(struct_info) = self.struct_types.get(type_name).cloned() {
                    let recv_ir = self.lower_expr(receiver)?;
                    let base_slot = self.lower_expr(&args[0])?;
                    let result = self.default_struct_derive_slot(
                        &recv_ir,
                        &base_slot,
                        &struct_info.fields,
                    );
                    return Ok(result);
                }
            }
        }

        // If receiver type is known but no method found, give a clear error
        if let Some(ref type_name) = receiver_type {
            let mut diag = edge_diagnostics::Diagnostic::error(format!(
                "no method named `{method_name}` found for type `{type_name}`",
            ))
            .with_label(span.clone(), format!("method not found in `{type_name}`"));
            // Check if there are any inherent methods to suggest
            if let Some(methods) = self.inherent_methods.get(type_name) {
                let available: Vec<&str> = methods
                    .iter()
                    .map(|m| m.fn_decl.name.name.as_str())
                    .collect();
                if !available.is_empty() {
                    diag = diag.with_note(format!(
                        "available methods for `{type_name}`: {}",
                        available.join(", "),
                    ));
                }
            }
            return Err(IrError::Diagnostic(diag));
        }

        // When receiver type is unknown, try compiler-provided trait methods
        // (handles chained calls like `a.unsafe_add(b).unsafe_sub(c)`,
        // paren expressions, and other cases where type inference fails)
        if receiver_type.is_none() {
            if let Some(op) = self.compiler_provided_method(method_name) {
                if args.len() != 1 {
                    return Err(IrError::Diagnostic(
                        edge_diagnostics::Diagnostic::error(format!(
                            "`.{method_name}()` expects exactly 1 argument",
                        ))
                        .with_label(span.clone(), "expected 1 argument"),
                    ));
                }
                let lhs = self.lower_expr(receiver)?;
                let rhs = self.lower_expr(&args[0])?;
                return Ok(ast_helpers::bop(op, lhs, rhs));
            }

            // Also check stateful methods for unknown receiver
            if let Some(result) =
                self.try_compiler_stateful_dispatch(receiver, method_name, args)?
            {
                return Ok(result);
            }
        }

        // Fallback: treat as FunctionCall(FieldAccess(...), args) — lower normally
        let _field_access = self.lower_field_access(receiver, method_name)?;
        let args_ir: Vec<RcExpr> = args
            .iter()
            .map(|a| self.lower_expr(a))
            .collect::<Result<_, _>>()?;
        Ok(ast_helpers::call(method_name.to_string(), args_ir))
    }

    /// Lower a qualified inherent method call: `Type::method(receiver`, args...)
    pub(crate) fn lower_qualified_method_call(
        &mut self,
        type_name: &str,
        method_name: &str,
        args: &[edge_ast::Expr],
        span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        if let Some(method) = self.find_inherent_method(type_name, method_name) {
            let fn_decl = method.fn_decl.clone();
            let body = method.body;
            let params: Vec<(String, edge_ast::ty::TypeSig)> = fn_decl
                .params
                .iter()
                .map(|(id, ty)| (id.name.clone(), ty.clone()))
                .collect();
            return self.inline_function_call(&params, &body, args);
        }
        Err(IrError::Diagnostic(
            edge_diagnostics::Diagnostic::error(format!(
                "no method named `{method_name}` found for type `{type_name}`",
            ))
            .with_label(
                span.clone(),
                format!("`{type_name}` does not have a method named `{method_name}`"),
            ),
        ))
    }

    /// Lower a qualified trait call: `Trait::method(receiver`, args...)
    pub(crate) fn lower_qualified_trait_call(
        &mut self,
        trait_name: &str,
        method_name: &str,
        args: &[edge_ast::Expr],
        span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        // Determine the concrete type from the first argument
        if args.is_empty() {
            return Err(IrError::Diagnostic(
                edge_diagnostics::Diagnostic::error(format!(
                    "`{trait_name}::{method_name}` requires at least one argument (the receiver)",
                ))
                .with_label(span.clone(), "expected at least one argument")
                .with_note(format!(
                    "qualified trait calls pass the receiver explicitly: `{trait_name}::{method_name}(value, ...)`",
                )),
            ));
        }

        // Compiler-provided trait methods for primitive types
        {
            let receiver_type = self.infer_receiver_type(&args[0]);
            let is_primitive = receiver_type
                .as_ref()
                .map_or(true, |t| Self::is_primitive_type(t));
            if is_primitive {
                if let Some(op) = self.compiler_provided_method(method_name) {
                    if args.len() != 2 {
                        return Err(IrError::Diagnostic(
                            edge_diagnostics::Diagnostic::error(format!(
                                "`{trait_name}::{method_name}` expects exactly 2 arguments",
                            ))
                            .with_label(span.clone(), "expected 2 arguments"),
                        ));
                    }
                    let lhs = self.lower_expr(&args[0])?;
                    let rhs = self.lower_expr(&args[1])?;
                    return Ok(ast_helpers::bop(op, lhs, rhs));
                }

                // Compiler-provided stateful methods (sload, sstore, derive_slot)
                // For qualified calls: Sload::sload(slot) has no receiver (first arg is slot)
                // Sstore::sstore(value, slot) has receiver as first arg
                {
                    let args_ir: Vec<RcExpr> = args
                        .iter()
                        .map(|a| self.lower_expr(a))
                        .collect::<Result<_, _>>()?;
                    // For static methods like sload: no receiver, all args
                    if let Some(result) =
                        self.compiler_provided_stateful_method(method_name, None, &args_ir)
                    {
                        return Ok(result);
                    }
                    // For instance methods like sstore/derive_slot: first arg is receiver
                    if args_ir.len() >= 2 {
                        let recv = args_ir[0].clone();
                        if let Some(result) = self.compiler_provided_stateful_method(
                            method_name,
                            Some(recv),
                            &args_ir[1..],
                        ) {
                            return Ok(result);
                        }
                    }
                }
            }
        }

        // Try to infer receiver type
        let receiver_type = self.infer_receiver_type(&args[0]);
        if let Some(ref type_name) = receiver_type {
            if let Some((fn_decl, body)) =
                self.find_trait_impl_method(type_name, trait_name, method_name)
            {
                let params: Vec<(String, edge_ast::ty::TypeSig)> = fn_decl
                    .params
                    .iter()
                    .map(|(id, ty)| (id.name.clone(), ty.clone()))
                    .collect();
                return self.inline_function_call(&params, &body, args);
            }
        }

        // If we can't determine the type or no impl found
        let msg = receiver_type.as_ref().map_or_else(
            || format!("cannot resolve `{trait_name}::{method_name}`: could not determine receiver type"),
            |type_name| format!("type `{type_name}` does not implement trait `{trait_name}`"),
        );
        Err(IrError::Diagnostic(
            edge_diagnostics::Diagnostic::error(&msg).with_label(span.clone(), msg),
        ))
    }

    /// Lower a generic function call by monomorphizing with inferred types.
    pub(crate) fn lower_generic_function_call(
        &mut self,
        template: &FreeFnInfo,
        args: &[edge_ast::Expr],
        explicit_type_args: &[edge_ast::ty::TypeSig],
        call_span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        // If explicit type args provided (turbofish), use them directly
        let inferred = if !explicit_type_args.is_empty() {
            if explicit_type_args.len() != template.type_params.len() {
                return Err(IrError::Diagnostic(
                    edge_diagnostics::Diagnostic::error(format!(
                        "wrong number of type arguments: expected {}, found {}",
                        template.type_params.len(),
                        explicit_type_args.len(),
                    ))
                    .with_label(
                        call_span.clone(),
                        format!(
                            "expected {} type argument{}",
                            template.type_params.len(),
                            if template.type_params.len() == 1 {
                                ""
                            } else {
                                "s"
                            },
                        ),
                    )
                    .with_note(format!(
                        "function `{}` has {} type parameter(s)",
                        template.name,
                        template.type_params.len(),
                    )),
                ));
            }
            template
                .type_params
                .iter()
                .zip(explicit_type_args.iter())
                .map(|(tp, ts)| (tp.name.name.clone(), ts.clone()))
                .collect::<std::collections::HashMap<_, _>>()
        } else {
            // Infer type params from argument types + return type hint
            let arg_types: Vec<EvmType> = args.iter().map(|a| self.infer_expr_type(a)).collect();
            self.infer_type_params_from_args_and_return(
                &template.type_params,
                &template.params,
                &arg_types,
                &template.returns,
            )?
        };

        // Validate trait bounds on type parameters
        for tp in &template.type_params {
            if !tp.constraints.is_empty() {
                let concrete_sig = inferred.get(&tp.name.name).unwrap();
                let concrete_name = Self::type_sig_display(concrete_sig);
                for constraint in &tp.constraints {
                    let key = (concrete_name.clone(), constraint.name.clone());
                    if !self.trait_impls.contains_key(&key) {
                        return Err(IrError::Diagnostic(
                            edge_diagnostics::Diagnostic::error(format!(
                                "the trait bound `{}: {}` is not satisfied",
                                concrete_name, constraint.name,
                            ))
                            .with_label(
                                call_span.clone(),
                                format!(
                                    "`{}` does not implement `{}`",
                                    concrete_name, constraint.name,
                                ),
                            )
                            .with_note(format!("required by a bound in `{}`", template.name,)),
                        ));
                    }
                }
            }
        }

        // Build mangled name using source-level type names (not lowered EvmType)
        // to distinguish struct types that lower to the same EVM representation.
        let type_name_strs: Vec<String> = template
            .type_params
            .iter()
            .map(|tp| {
                let sig = inferred.get(&tp.name.name).unwrap();
                Self::type_sig_display(sig)
            })
            .collect();
        let mangled = format!("{}__{}", template.name, type_name_strs.join("_"));

        // Check if already monomorphized
        if let Some(mono_info) = self.monomorphized_fns.get(&mangled).cloned() {
            return self.inline_function_call(&mono_info.params, &mono_info.body, args);
        }

        // Substitute type params in the function's param types and body
        let new_params: Vec<(String, edge_ast::ty::TypeSig)> = template
            .params
            .iter()
            .map(|(name, ty)| (name.clone(), Self::substitute_type_params(ty, &inferred)))
            .collect();

        // Cache the monomorphized function
        let mono_info = FreeFnInfo {
            name: mangled.clone(),
            params: new_params.clone(),
            returns: template.returns.clone(),
            body: template.body.clone(),
            is_comptime: template.is_comptime,
            type_params: Vec::new(),
        };
        self.monomorphized_fns.insert(mangled, mono_info);

        // Inline with substituted params
        self.inline_function_call(&new_params, &template.body, args)
    }

    /// Infer the type of a receiver expression (best-effort).
    /// Look up the scope binding for an expression (Ident or self.field).
    /// Returns the variable name and binding reference if found.
    fn lookup_binding_for_expr<'a>(&'a self, expr: &edge_ast::Expr) -> Option<&'a super::VarBinding> {
        let var_name = match expr {
            edge_ast::Expr::Ident(ident) => &ident.name,
            edge_ast::Expr::FieldAccess(obj, field, _) => {
                if let edge_ast::Expr::Ident(ident) = obj.as_ref() {
                    if ident.name == "self" {
                        &field.name
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(var_name) {
                return Some(binding);
            }
        }
        None
    }

    pub(crate) fn infer_receiver_type(&self, expr: &edge_ast::Expr) -> Option<String> {
        // Try direct binding lookup first
        if let Some(binding) = self.lookup_binding_for_expr(expr) {
            if let Some(ref ct) = binding.composite_type {
                return Some(ct.clone());
            }
            return Self::evm_type_to_name(&binding._ty);
        }

        match expr {
            edge_ast::Expr::StructInstantiation(_, type_name, _, _) => Some(type_name.name.clone()),
            edge_ast::Expr::Literal(lit) => match lit.as_ref() {
                edge_ast::Lit::Bool(_, _) => Some("bool".to_string()),
                edge_ast::Lit::Int(_, Some(pt), _) => {
                    Some(Self::primitive_type_to_name(pt))
                }
                edge_ast::Lit::Int(_, None, _) => Some("u256".to_string()),
                _ => None,
            },
            // ArrayIndex: base[index] — if base is a Map, the result type is the value type (V)
            edge_ast::Expr::ArrayIndex(base, _, _, _) => {
                let base_type = self.infer_receiver_type(base);
                let base_args = self.infer_receiver_type_args(base);
                if let Some(ref bt) = base_type {
                    if bt.starts_with("Map") && base_args.len() == 2 {
                        return Some(Self::type_sig_mangle(&base_args[1]));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get the concrete type arguments for a receiver's generic composite type.
    pub(crate) fn infer_receiver_type_args(&self, expr: &edge_ast::Expr) -> Vec<edge_ast::ty::TypeSig> {
        if let Some(binding) = self.lookup_binding_for_expr(expr) {
            return binding.composite_type_args.clone();
        }

        match expr {
            // ArrayIndex: base[index] — if base is a Map, the result's type args come from V
            edge_ast::Expr::ArrayIndex(base, _, _, _) => {
                let base_args = self.infer_receiver_type_args(base);
                if base_args.len() == 2 {
                    if let edge_ast::ty::TypeSig::Named(_, inner_args) = &base_args[1] {
                        return inner_args.clone();
                    }
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Build a type parameter substitution map from a generic type's type params and concrete args.
    /// E.g., for Map<K, V> with args [addr, u256], returns {"K": "addr", "V": "u256"}.
    /// For nested generics like Map<addr, Map<addr, u256>>, V maps to "Map" (base name only).
    fn build_type_param_subst(
        &self,
        type_name: &str,
        type_args: &[edge_ast::ty::TypeSig],
    ) -> HashMap<String, String> {
        if type_args.is_empty() {
            return HashMap::new();
        }
        // Try exact match first, then strip mangled suffix to find base template.
        // E.g., "Map__CustomHash_CustomSStore" → try "Map" if exact lookup fails.
        let template = self.generic_type_templates.get(type_name).or_else(|| {
            let base = type_name.split("__").next().unwrap_or(type_name);
            self.generic_type_templates.get(base)
        });
        if let Some(template) = template {
            template
                .type_params
                .iter()
                .zip(type_args.iter())
                .map(|(param, arg)| {
                    let name = Self::type_sig_mangle(arg);
                    (param.name.name.clone(), name)
                })
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Convert an EvmType to a type name string (for primitives).
    fn evm_type_to_name(ty: &EvmType) -> Option<String> {
        match ty {
            EvmType::Base(base) => match base {
                EvmBaseType::UIntT(256) => Some("u256".to_string()),
                EvmBaseType::UIntT(w) => Some(format!("u{w}")),
                EvmBaseType::IntT(256) => Some("i256".to_string()),
                EvmBaseType::IntT(w) => Some(format!("i{w}")),
                EvmBaseType::BoolT => Some("bool".to_string()),
                EvmBaseType::AddrT => Some("address".to_string()),
                EvmBaseType::BytesT(n) => Some(format!("bytes{n}")),
                EvmBaseType::UnitT | EvmBaseType::StateT => None,
            },
            _ => None,
        }
    }

    /// Convert a PrimitiveType to a type name string.
    fn primitive_type_to_name(pt: &edge_ast::ty::PrimitiveType) -> String {
        use edge_ast::ty::PrimitiveType;
        match pt {
            PrimitiveType::UInt(256) => "u256".to_string(),
            PrimitiveType::UInt(w) => format!("u{w}"),
            PrimitiveType::Int(256) => "i256".to_string(),
            PrimitiveType::Int(w) => format!("i{w}"),
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::Address => "address".to_string(),
            PrimitiveType::FixedBytes(n) => format!("bytes{n}"),
            PrimitiveType::Bit => "bit".to_string(),
        }
    }

    /// Check if a type name refers to a primitive type (not a user-defined composite).
    pub(crate) fn is_primitive_type(type_name: &str) -> bool {
        type_name == "u256"
            || type_name == "i256"
            || type_name == "bool"
            || type_name == "address"
            || type_name == "b32"
            || type_name.starts_with("u")
                && type_name[1..].parse::<u16>().is_ok()
            || type_name.starts_with("i")
                && type_name[1..].parse::<u16>().is_ok()
            || type_name.starts_with("bytes")
                && type_name[5..].parse::<u8>().is_ok()
    }

    /// Look up a compiler-provided trait method for a primitive type.
    /// Returns the binary op if the method matches an imported std::ops trait.
    fn compiler_provided_method(&self, method_name: &str) -> Option<EvmBinaryOp> {
        match method_name {
            "unsafe_add" if self.std_ops_traits.contains("UnsafeAdd") => Some(EvmBinaryOp::Add),
            "unsafe_sub" if self.std_ops_traits.contains("UnsafeSub") => Some(EvmBinaryOp::Sub),
            "unsafe_mul" if self.std_ops_traits.contains("UnsafeMul") => Some(EvmBinaryOp::Mul),
            _ => None,
        }
    }

    /// Compiler-provided complex trait methods for primitive types.
    /// Unlike `compiler_provided_method` (simple binary ops), these produce
    /// Lower receiver + args and try compiler-provided stateful method dispatch.
    /// Used for `.derive_slot()`, `.sload()`, `.sstore()` on primitives.
    fn try_compiler_stateful_dispatch(
        &mut self,
        receiver: &edge_ast::Expr,
        method_name: &str,
        args: &[edge_ast::Expr],
    ) -> Result<Option<RcExpr>, IrError> {
        let recv_ir = self.lower_expr(receiver)?;
        let args_ir: Vec<RcExpr> = args
            .iter()
            .map(|a| self.lower_expr(a))
            .collect::<Result<_, _>>()?;
        Ok(self.compiler_provided_stateful_method(method_name, Some(recv_ir), &args_ir))
    }

    /// full IR expression trees with state threading.
    ///
    /// Returns `Some(ir_expr)` if the method was handled, `None` otherwise.
    fn compiler_provided_stateful_method(
        &mut self,
        method_name: &str,
        receiver_ir: Option<RcExpr>,
        args_ir: &[RcExpr],
    ) -> Option<RcExpr> {
        use std::rc::Rc;

        match method_name {
            // UniqueSlot::derive_slot(self, base_slot) → keccak256(key . base_slot)
            "derive_slot" if self.std_ops_traits.contains("UniqueSlot") => {
                let key = receiver_ir?;
                let base_slot = args_ir.first()?;
                let scratch = self.alloc_region(2);
                // MSTORE(scratch, key)
                let mstore_key = ast_helpers::mstore(
                    Rc::clone(&scratch),
                    key,
                    Rc::clone(&self.current_state),
                );
                self.current_state = Rc::clone(&mstore_key);
                // MSTORE(scratch+32, base_slot)
                let slot_offset = ast_helpers::add(
                    Rc::clone(&scratch),
                    ast_helpers::const_int(32, self.current_ctx.clone()),
                );
                let mstore_slot = ast_helpers::mstore(
                    slot_offset,
                    Rc::clone(base_slot),
                    Rc::clone(&self.current_state),
                );
                self.current_state = Rc::clone(&mstore_slot);
                // KECCAK256(scratch, 64, state)
                let computed_slot = ast_helpers::keccak256(
                    scratch,
                    ast_helpers::const_int(64, self.current_ctx.clone()),
                    Rc::clone(&self.current_state),
                );
                let side_effects = ast_helpers::concat(mstore_key, mstore_slot);
                Some(ast_helpers::concat(side_effects, computed_slot))
            }

            // Sload::sload(slot) → SLOAD(slot, state) — static method (no receiver)
            "sload" if self.std_ops_traits.contains("Sload") => {
                let slot = if let Some(recv) = receiver_ir {
                    // Called as receiver.sload() — receiver is the slot
                    recv
                } else {
                    // Called as Type::sload(slot) — first arg is the slot
                    args_ir.first()?.clone()
                };
                Some(ast_helpers::sload(slot, Rc::clone(&self.current_state)))
            }

            // Sstore::sstore(self, slot) → SSTORE(slot, value, state)
            "sstore" if self.std_ops_traits.contains("Sstore") => {
                let value = receiver_ir?;
                let slot = args_ir.first()?;
                let store = ast_helpers::sstore(
                    Rc::clone(slot),
                    value,
                    Rc::clone(&self.current_state),
                );
                self.current_state = Rc::clone(&store);
                Some(store)
            }

            _ => None,
        }
    }

    /// Default `derive_slot` for struct types without an explicit `UniqueSlot` impl.
    ///
    /// Follows Solidity's nested mapping convention — each field is chained
    /// through keccak256 as if it were a separate mapping level:
    ///
    /// ```text
    /// slot = keccak256(field_0 . base_slot)
    /// slot = keccak256(field_1 . slot)
    /// slot = keccak256(field_2 . slot)
    /// ...
    /// ```
    fn default_struct_derive_slot(
        &mut self,
        receiver_ir: &RcExpr,
        base_slot: &RcExpr,
        fields: &[(String, EvmType)],
    ) -> RcExpr {
        let scratch = self.alloc_region(2);
        let mut current_slot = Rc::clone(base_slot);
        let mut side_effects = ast_helpers::empty(
            EvmType::Base(EvmBaseType::UnitT),
            self.current_ctx.clone(),
        );

        for (i, (_name, _ty)) in fields.iter().enumerate() {
            // Load field value: MLOAD(receiver + i*32)
            let field_offset = ast_helpers::add(
                Rc::clone(receiver_ir),
                ast_helpers::const_int((i * 32) as i64, self.current_ctx.clone()),
            );
            let field_val = ast_helpers::mload(field_offset, Rc::clone(&self.current_state));

            // MSTORE(scratch, field_value)
            let mstore_field = ast_helpers::mstore(
                Rc::clone(&scratch),
                field_val,
                Rc::clone(&self.current_state),
            );
            self.current_state = Rc::clone(&mstore_field);
            side_effects = ast_helpers::concat(side_effects, mstore_field);

            // MSTORE(scratch+32, current_slot)
            let slot_offset = ast_helpers::add(
                Rc::clone(&scratch),
                ast_helpers::const_int(32, self.current_ctx.clone()),
            );
            let mstore_slot = ast_helpers::mstore(
                slot_offset,
                current_slot,
                Rc::clone(&self.current_state),
            );
            self.current_state = Rc::clone(&mstore_slot);
            side_effects = ast_helpers::concat(side_effects, mstore_slot);

            // slot = keccak256(scratch, 64)
            current_slot = ast_helpers::keccak256(
                Rc::clone(&scratch),
                ast_helpers::const_int(64, self.current_ctx.clone()),
                Rc::clone(&self.current_state),
            );
        }

        ast_helpers::concat(side_effects, current_slot)
    }

    /// Infer the `EvmType` of an expression (best-effort, defaults to u256).
    pub(crate) fn infer_expr_type(&self, expr: &edge_ast::Expr) -> EvmType {
        match expr {
            edge_ast::Expr::Literal(lit) => match lit.as_ref() {
                edge_ast::Lit::Bool(_, _) => EvmType::Base(EvmBaseType::BoolT),
                edge_ast::Lit::Int(_, Some(pt), _) => self.lower_primitive_type(pt),
                _ => EvmType::Base(EvmBaseType::UIntT(256)),
            },
            edge_ast::Expr::Ident(ident) => {
                for scope in self.scopes.iter().rev() {
                    if let Some(binding) = scope.bindings.get(&ident.name) {
                        return binding._ty.clone();
                    }
                }
                EvmType::Base(EvmBaseType::UIntT(256))
            }
            edge_ast::Expr::Cast(_, target_type, _) => self.lower_type_sig(target_type),
            edge_ast::Expr::Paren(inner, _) => self.infer_expr_type(inner),
            edge_ast::Expr::At(name, _, _) => match name.name.as_str() {
                "caller" | "origin" | "coinbase" | "address" => EvmType::Base(EvmBaseType::AddrT),
                _ => EvmType::Base(EvmBaseType::UIntT(256)),
            },
            _ => EvmType::Base(EvmBaseType::UIntT(256)),
        }
    }

    /// Find an inherent method for a type.
    fn find_inherent_method(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<super::InherentMethod> {
        self.inherent_methods
            .get(type_name)?
            .iter()
            .find(|m| m.fn_decl.name.name == method_name)
            .cloned()
    }

    /// Find a trait method implementation for a type by searching all trait impls.
    fn find_trait_method_for_type(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<(edge_ast::item::FnDecl, edge_ast::CodeBlock)> {
        for ((impl_type, _trait_name), impl_info) in &self.trait_impls {
            if impl_type == type_name {
                if let Some((fn_decl, body)) = impl_info.methods.get(method_name) {
                    return Some((fn_decl.clone(), body.clone()));
                }
            }
        }
        None
    }

    /// Find a specific trait impl method for a type.
    pub(crate) fn find_trait_impl_method(
        &self,
        type_name: &str,
        trait_name: &str,
        method_name: &str,
    ) -> Option<(edge_ast::item::FnDecl, edge_ast::CodeBlock)> {
        let key = (type_name.to_string(), trait_name.to_string());
        if let Some(impl_info) = self.trait_impls.get(&key) {
            if let Some((fn_decl, body)) = impl_info.methods.get(method_name) {
                return Some((fn_decl.clone(), body.clone()));
            }
        }
        None
    }

    /// Inline a function call by re-lowering its AST body with params bound in scope.
    /// Used for comptime functions that must always be inlined.
    pub(crate) fn inline_function_call(
        &mut self,
        params: &[(String, edge_ast::ty::TypeSig)],
        body: &edge_ast::CodeBlock,
        args: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        let args_ir: Vec<RcExpr> = args
            .iter()
            .map(|a| self.lower_expr(a))
            .collect::<Result<_, _>>()?;

        // Before pushing a new scope, look up composite info for args that are identifiers
        // (needed for method calls where `self` refers to a struct variable or generic type)
        tracing::trace!(
            "inline_function_call: params={:?}, args={}",
            params.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
            args.len()
        );
        let mut arg_composite: Vec<Option<(String, Option<RcExpr>, Vec<edge_ast::ty::TypeSig>)>> = Vec::new();
        for arg in args {
            if let edge_ast::Expr::Ident(ident) = arg {
                let info = self.lookup_composite_info(&ident.name);
                if let Some((ct, cb)) = info {
                    arg_composite.push(Some((ct, Some(cb), Vec::new())));
                } else {
                    // Check for composite_type without composite_base (e.g., Map type aliases)
                    let mut found = false;
                    for scope in self.scopes.iter().rev() {
                        if let Some(binding) = scope.bindings.get(&ident.name) {
                            if let Some(ref ct) = binding.composite_type {
                                arg_composite.push(Some((ct.clone(), None, binding.composite_type_args.clone())));
                                found = true;
                            }
                            break;
                        }
                    }
                    if !found {
                        arg_composite.push(None);
                    }
                }
            } else if let edge_ast::Expr::ArrayIndex(base, _, _, _) = arg {
                // For ArrayIndex args (e.g., map[key] as self parameter),
                // infer the value type from the base Map's type args.
                let base_type = self.infer_receiver_type(base);
                let base_args = self.infer_receiver_type_args(base);
                if let Some(ref bt) = base_type {
                    if bt.starts_with("Map") && base_args.len() == 2 {
                        let value_mangled = Self::type_sig_mangle(&base_args[1]);
                        // Extract inner type args if V is a generic type
                        let inner_args = if let edge_ast::ty::TypeSig::Named(_, inner) = &base_args[1] {
                            inner.clone()
                        } else {
                            Vec::new()
                        };
                        arg_composite.push(Some((value_mangled, None, inner_args)));
                    } else {
                        arg_composite.push(None);
                    }
                } else {
                    arg_composite.push(None);
                }
            } else {
                arg_composite.push(None);
            }
        }

        self.scopes.push(Scope::new());
        for (i, (param_name, param_ty)) in params.iter().enumerate() {
            let ty = self.lower_type_sig(param_ty);
            let val = args_ir
                .get(i)
                .cloned()
                .unwrap_or_else(|| ast_helpers::const_int(0, self.current_ctx.clone()));
            let (mut composite_type, mut composite_base, composite_type_args) = arg_composite
                .get(i)
                .and_then(|c| c.as_ref())
                .map(|(ct, cb, ta)| (Some(ct.clone()), cb.clone(), ta.clone()))
                .unwrap_or((None, None, Vec::new()));

            // If the parameter has a primitive type annotation, don't inherit
            // composite_type from the argument — prevents Map type leaking through
            // when Map.get passes `self` (Map) to derive_slot(base_slot: u256).
            if matches!(param_ty, edge_ast::ty::TypeSig::Primitive(_)) && composite_type.is_some() {
                // Only clear if the composite type doesn't match a known struct/union
                // (the argument may be a struct disguised as u256 in the EVM)
                if let Some(ref ct) = composite_type {
                    if !self.struct_types.contains_key(ct) && !self.union_types.contains_key(ct) {
                        composite_type = None;
                    }
                }
            }

            // If composite_type is still None, check if the param type sig names
            // a known struct/union type — this enables trait method dispatch on
            // generic parameters after monomorphization substitutes concrete types.
            // Also resolve generic type parameters (K, V, etc.) through type_param_subst.
            if composite_type.is_none() {
                if let edge_ast::ty::TypeSig::Named(ref name, ref type_args) = param_ty {
                    let resolved_name = self
                        .type_param_subst
                        .get(&name.name)
                        .cloned()
                        .unwrap_or_else(|| name.name.clone());
                    if self.struct_types.contains_key(&resolved_name)
                        || self.union_types.contains_key(&resolved_name)
                    {
                        composite_type = Some(resolved_name);
                    } else if type_args.is_empty() {
                        // Check if resolved name is a generic type that was
                        // monomorphized (e.g., Result__u256)
                        let mangled = Self::type_sig_mangle(param_ty);
                        if self.struct_types.contains_key(&mangled)
                            || self.union_types.contains_key(&mangled)
                        {
                            composite_type = Some(mangled);
                        }
                    }
                }
            }

            // If we inferred composite_type from the type sig but have no
            // composite_base, set it to the param value — for struct types
            // the value IS the memory base address.
            if composite_type.is_some() && composite_base.is_none() {
                if let Some(ref ct) = composite_type {
                    if self.struct_types.contains_key(ct) {
                        composite_base = Some(Rc::clone(&val));
                    }
                }
            }
            tracing::trace!(
                "  param={}, composite_type={:?}, has_base={}",
                param_name,
                composite_type,
                composite_base.is_some()
            );
            let binding = VarBinding {
                value: val,
                location: DataLocation::Stack,
                storage_slot: None,
                _ty: ty,
                let_bind_name: None,
                composite_type,
                composite_base,
                composite_type_args,
            };
            self.scopes
                .last_mut()
                .expect("scope stack empty")
                .bindings
                .insert(param_name.clone(), binding);
        }

        let new_prefix = format!("_i{}_{}", self.inline_counter, self.inline_prefix);
        let old_prefix = std::mem::replace(&mut self.inline_prefix, new_prefix);
        self.inline_counter += 1;
        self.inline_depth += 1;
        let result = self.lower_code_block(body)?;
        self.inline_depth -= 1;
        self.inline_prefix = old_prefix;
        self.scopes.pop();
        Ok(result)
    }

    /// Emit a Call(name, args) node for an internal function.
    /// The function body is lowered separately as a Function node.
    pub(crate) fn emit_call(
        &mut self,
        name: &str,
        params: &[(String, edge_ast::ty::TypeSig)],
        returns: &[edge_ast::ty::TypeSig],
        args: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        let args_ir: Vec<RcExpr> = args
            .iter()
            .map(|a| self.lower_expr(a))
            .collect::<Result<_, _>>()?;

        // Ensure the function body has been lowered once
        if !self
            .lowered_functions
            .iter()
            .any(|f| matches!(f.as_ref(), EvmExpr::Function(n, _, _, _) if n == name))
        {
            self.lower_internal_function_body(name, params, returns)?;
        }

        Ok(ast_helpers::call(name.to_string(), args_ir))
    }

    /// Check if an expression used as a statement is a function call whose
    /// return value is being discarded, and emit a warning if so.
    pub(crate) fn check_unused_return_value(&mut self, expr: &edge_ast::Expr) {
        let (fn_name, span) = match expr {
            edge_ast::Expr::FunctionCall(callee, _, _, span) => {
                match callee.as_ref() {
                    edge_ast::Expr::Ident(id) => (Some(id.name.clone()), span.clone()),
                    edge_ast::Expr::Path(components, _) if components.len() == 2 => {
                        // Qualified call like Trait::method or Type::method
                        let method = &components[1].name;
                        (Some(method.clone()), span.clone())
                    }
                    edge_ast::Expr::FieldAccess(_, method, _) => {
                        // Method call like obj.method()
                        (Some(method.name.clone()), span.clone())
                    }
                    _ => return,
                }
            }
            _ => return,
        };

        let Some(name) = fn_name else { return };

        // Look up the function's return type
        let has_return = self.fn_has_return_value(&name);
        if has_return {
            self.warnings.push(
                edge_diagnostics::Diagnostic::warning(format!("unused return value of `{name}`",))
                    .with_label(span, "return value unused"),
            );
        }
    }

    /// Check if a function by name has a non-void return type.
    fn fn_has_return_value(&self, name: &str) -> bool {
        // Check free functions
        if let Some(info) = self.free_fn_bodies.iter().find(|f| f.name == name) {
            return !info.returns.is_empty();
        }
        // Check generic function templates
        if let Some(info) = self.generic_fn_templates.get(name) {
            return !info.returns.is_empty();
        }
        // Check trait methods — search all trait impls for a method with this name
        for ((_type_name, _trait_name), impl_info) in &self.trait_impls {
            if let Some((fn_decl, _body)) = impl_info.methods.get(name) {
                return !fn_decl.returns.is_empty();
            }
        }
        // Check inherent methods
        for (_type_name, methods) in &self.inherent_methods {
            if let Some(m) = methods.iter().find(|m| m.fn_decl.name.name == name) {
                return !m.fn_decl.returns.is_empty();
            }
        }
        // Check trait registry (for default methods)
        for (_trait_name, trait_info) in &self.trait_registry {
            if let Some((_, fn_decl)) = trait_info.required_methods.iter().find(|(n, _)| n == name)
            {
                return !fn_decl.returns.is_empty();
            }
            if let Some((_, fn_decl, _)) = trait_info
                .default_methods
                .iter()
                .find(|(n, _, _)| n == name)
            {
                return !fn_decl.returns.is_empty();
            }
        }
        // Unknown function — don't warn
        false
    }
}
