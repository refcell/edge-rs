//! Function call lowering: call resolution, inlining, builtin calls.

use super::{AstToEgglog, FreeFnInfo, Scope, VarBinding};
use crate::{
    ast_helpers,
    schema::{DataLocation, EvmBaseType, EvmExpr, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// For internal contract functions, inlines the function body at the call site
    /// by binding the arguments in a new scope and lowering the body.
    pub(crate) fn lower_function_call(
        &mut self,
        callee: &edge_ast::Expr,
        args: &[edge_ast::Expr],
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
                    return Err(IrError::LoweringSpanned {
                        message: format!(
                            "cannot infer type parameters for generic type `{type_name}` \
                             from `{type_name}::{variant_name}(...)` — provide explicit type arguments",
                        ),
                        span: span.clone(),
                    });
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
                if self.trait_registry.contains_key(type_or_trait) {
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
        if let Some((name, params, _body)) = self
            .contract_functions
            .iter()
            .find(|(name, _, _)| *name == fn_name)
            .cloned()
        {
            return self.emit_call(&name, &params, args);
        }

        // Check non-comptime free functions — emit Call (not inline)
        if let Some(info) = self
            .free_fn_bodies
            .iter()
            .find(|f| f.name == fn_name && !f.is_comptime)
            .cloned()
        {
            return self.emit_call(&info.name, &info.params, args);
        }

        // Check generic function templates
        if let Some(template) = self.generic_fn_templates.get(&fn_name).cloned() {
            return self.lower_generic_function_call(&template, args);
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
            return Err(IrError::LoweringSpanned {
                message: format!("no method `{method_name}` found for type `{type_name}`"),
                span: span.clone(),
            });
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
        Err(IrError::LoweringSpanned {
            message: format!("no method `{method_name}` found for type `{type_name}`"),
            span: span.clone(),
        })
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
            return Err(IrError::LoweringSpanned {
                message: format!("qualified trait call `{trait_name}::{method_name}` requires at least one argument"),
                span: span.clone(),
            });
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

        // If we can't determine the type, just inline as a regular call attempt
        Err(IrError::LoweringSpanned {
            message: format!("cannot resolve `{trait_name}::{method_name}` — type not determined"),
            span: span.clone(),
        })
    }

    /// Lower a generic function call by monomorphizing with inferred types.
    pub(crate) fn lower_generic_function_call(
        &mut self,
        template: &FreeFnInfo,
        args: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        // Lower args to get their types
        let arg_types: Vec<EvmType> = args.iter().map(|a| self.infer_expr_type(a)).collect();

        // Infer type params from argument types
        let inferred =
            self.infer_type_params_from_args(&template.type_params, &template.params, &arg_types)?;

        // Build mangled name
        let concrete_types: Vec<EvmType> = template
            .type_params
            .iter()
            .map(|tp| {
                let sig = inferred.get(&tp.name.name).unwrap();
                self.lower_type_sig(sig)
            })
            .collect();
        let mangled = Self::mangle_type_name(&template.name, &concrete_types);

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
            let (composite_type, composite_base) = arg_composite
                .get(i)
                .and_then(|c| c.as_ref())
                .map(|(ct, cb)| (Some(ct.clone()), *cb))
                .unwrap_or((None, None));
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
            self.lower_internal_function_body(name, params)?;
        }

        Ok(ast_helpers::call(name.to_string(), args_ir))
    }
}
