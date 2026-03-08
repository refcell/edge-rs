//! Function call lowering: call resolution, inlining, builtin calls.

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
                    return self.lower_union_instantiation_expr(type_name, variant_name, args);
                }
                // Check for generic union types (e.g., Result::Ok(42) where Result<T> was monomorphized)
                if self.generic_type_templates.contains_key(type_name) {
                    // First try to find an already-monomorphized version
                    if let Some(mangled) = self.resolve_generic_type_name(type_name) {
                        return self.lower_union_instantiation_expr(&mangled, variant_name, args);
                    }
                    // No monomorphized version yet — try to infer type params from
                    // the constructor argument and monomorphize on the fly.
                    if let Some(mangled) =
                        self.try_monomorphize_union_from_constructor(type_name, variant_name, args)?
                    {
                        return self.lower_union_instantiation_expr(&mangled, variant_name, args);
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
                let type_or_trait = &components[0].name;
                let method_name = &components[1].name;

                let method_span = &components[1].span;

                // Check inherent methods: Type::method(receiver, args...)
                if self.inherent_methods.contains_key(type_or_trait) {
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

        if let Some(ref type_name) = receiver_type {
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
                return self.inline_function_call(&params, &body, &all_args);
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
                return self.inline_function_call(&params, &body, &all_args);
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

        // Built-in UnsafeAdd/UnsafeSub/UnsafeMul for primitives
        let unsafe_op = match (trait_name, method_name) {
            ("UnsafeAdd", "unsafe_add") => Some(EvmBinaryOp::Add),
            ("UnsafeSub", "unsafe_sub") => Some(EvmBinaryOp::Sub),
            ("UnsafeMul", "unsafe_mul") => Some(EvmBinaryOp::Mul),
            _ => None,
        };
        if let Some(op) = unsafe_op {
            // Check if receiver is a primitive (not a user-defined type)
            let receiver_type = self.infer_receiver_type(&args[0]);
            if receiver_type.is_none() {
                // Primitive type — emit unchecked op directly
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
            // User-defined type — fall through to trait impl lookup
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
    pub(crate) fn infer_receiver_type(&self, expr: &edge_ast::Expr) -> Option<String> {
        match expr {
            edge_ast::Expr::Ident(ident) => {
                // Check scope for composite type info
                for scope in self.scopes.iter().rev() {
                    if let Some(binding) = scope.bindings.get(&ident.name) {
                        if let Some(ref ct) = binding.composite_type {
                            return Some(ct.clone());
                        }
                    }
                }
                None
            }
            edge_ast::Expr::StructInstantiation(_, type_name, _, _) => Some(type_name.name.clone()),
            _ => None,
        }
    }

    /// Infer the `EvmType` of an expression (best-effort, defaults to u256).
    pub(crate) fn infer_expr_type(&self, expr: &edge_ast::Expr) -> EvmType {
        match expr {
            edge_ast::Expr::Literal(lit) => match lit.as_ref() {
                edge_ast::Lit::Bool(_, _) => EvmType::Base(EvmBaseType::BoolT),
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
        // (needed for method calls where `self` refers to a struct variable)
        let mut arg_composite: Vec<Option<(String, Option<usize>)>> = Vec::new();
        for arg in args {
            if let edge_ast::Expr::Ident(ident) = arg {
                let info = self.lookup_composite_info(&ident.name);
                arg_composite.push(info.map(|(ct, cb)| (ct, Some(cb))));
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
            let (mut composite_type, composite_base) = arg_composite
                .get(i)
                .and_then(|c| c.as_ref())
                .map(|(ct, cb)| (Some(ct.clone()), *cb))
                .unwrap_or((None, None));

            // If composite_type is still None, check if the param type sig names
            // a known struct/union type — this enables trait method dispatch on
            // generic parameters after monomorphization substitutes concrete types.
            if composite_type.is_none() {
                if let edge_ast::ty::TypeSig::Named(ref name, _) = param_ty {
                    if self.struct_types.contains_key(&name.name)
                        || self.union_types.contains_key(&name.name)
                    {
                        composite_type = Some(name.name.clone());
                    }
                }
            }
            let binding = VarBinding {
                value: val,
                location: DataLocation::Stack,
                storage_slot: None,
                _ty: ty,
                let_bind_name: None,
                composite_type,
                composite_base,
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
