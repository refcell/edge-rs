//! Type lowering and resolution.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use super::{AstToEgglog, StructTypeInfo};
use crate::{
    schema::{DataLocation, EvmBaseType, EvmType},
    IrError,
};

impl AstToEgglog {
    /// Resolve a type alias to its underlying `TypeSig`.
    /// Returns the resolved type if it's a `Named` type with an alias, otherwise returns the input.
    pub(crate) fn resolve_type_alias<'a>(
        &'a self,
        ty: &'a edge_ast::ty::TypeSig,
    ) -> &'a edge_ast::ty::TypeSig {
        match ty {
            edge_ast::ty::TypeSig::Named(ident, _) => {
                self.type_aliases.get(&ident.name).map_or(ty, |r| r)
            }
            edge_ast::ty::TypeSig::Pointer(_, inner) => self.resolve_type_alias(inner),
            _ => ty,
        }
    }

    /// Try to resolve a generic type and monomorphize it.
    /// Returns the mangled name if the type is generic and was monomorphized.
    pub(crate) fn try_monomorphize_named_type(
        &mut self,
        name: &str,
        type_args: &[edge_ast::ty::TypeSig],
        span: Option<&edge_types::span::Span>,
    ) -> Result<Option<String>, IrError> {
        if type_args.is_empty() || !self.generic_type_templates.contains_key(name) {
            return Ok(None);
        }
        let mangled = self.monomorphize_type(name, type_args, span)?;
        Ok(Some(mangled))
    }

    /// Resolve a generic type name (e.g., "Result") to its monomorphized name (e.g., "`Result__u256`").
    /// Returns `Some` only when there's exactly one monomorphization (unambiguous).
    /// When there are multiple, returns `None` — caller should use
    /// `resolve_generic_type_name_with_args` for precise resolution.
    pub(crate) fn resolve_generic_type_name(&self, name: &str) -> Option<String> {
        // Check monomorphized_types cache for entries with this base name
        let candidates: Vec<&String> = self.monomorphized_types.iter()
            .filter(|((base, _), _)| base == name)
            .map(|(_, mangled)| mangled)
            .collect();
        if candidates.len() == 1 {
            return Some(candidates[0].clone());
        }
        if candidates.len() > 1 {
            // Multiple monomorphizations — ambiguous, return None
            return None;
        }

        // Fallback: scan union_types and struct_types for "{name}__" prefix,
        // but only return if unambiguous.
        let mut fallback_candidates = Vec::new();
        let prefix = format!("{name}__");
        for key in self.union_types.keys() {
            if key.starts_with(&prefix) {
                fallback_candidates.push(key.clone());
            }
        }
        for key in self.struct_types.keys() {
            if key.starts_with(&prefix) {
                fallback_candidates.push(key.clone());
            }
        }
        if fallback_candidates.len() == 1 {
            return Some(fallback_candidates.into_iter().next().unwrap());
        }
        None
    }

    /// Resolve a generic type name with specific type args to its monomorphized name.
    /// More precise than `resolve_generic_type_name` when multiple monomorphizations exist.
    pub(crate) fn resolve_generic_type_name_with_args(
        &self,
        name: &str,
        type_args: &[edge_ast::ty::TypeSig],
    ) -> Option<String> {
        let mangled_args: Vec<String> = type_args.iter()
            .map(|a| Self::type_sig_mangle(a))
            .collect();
        let cache_key = (name.to_string(), mangled_args);
        if let Some(mangled) = self.monomorphized_types.get(&cache_key) {
            return Some(mangled.clone());
        }
        // Fallback: construct the expected mangled name and check if it exists
        let expected = format!("{}_{}", name, cache_key.1.join("_"));
        if self.union_types.contains_key(&expected) || self.struct_types.contains_key(&expected) {
            return Some(expected);
        }
        None
    }

    /// Extract fixed array length from a type expression (literal integer).
    pub(crate) fn extract_array_length(len_expr: &edge_ast::Expr) -> Option<usize> {
        if let edge_ast::Expr::Literal(lit) = len_expr {
            if let edge_ast::Lit::Int(bytes, _, _) = lit.as_ref() {
                let n = u64::from_be_bytes(bytes[24..32].try_into().unwrap());
                return Some(n as usize);
            }
        }
        None
    }

    /// Convert a type signature to its ABI string representation.
    pub(crate) fn type_sig_to_abi_string(&self, ty: &edge_ast::ty::TypeSig) -> String {
        // Resolve aliases first
        let resolved = self.resolve_type_alias(ty);
        match resolved {
            edge_ast::ty::TypeSig::Primitive(prim) => match prim {
                edge_ast::ty::PrimitiveType::UInt(bits) => format!("uint{bits}"),
                edge_ast::ty::PrimitiveType::Int(bits) => format!("int{bits}"),
                edge_ast::ty::PrimitiveType::FixedBytes(bytes) => format!("bytes{bytes}"),
                edge_ast::ty::PrimitiveType::Address => "address".to_owned(),
                edge_ast::ty::PrimitiveType::Bool | edge_ast::ty::PrimitiveType::Bit => {
                    "bool".to_owned()
                }
            },
            edge_ast::ty::TypeSig::Pointer(_, inner) => self.type_sig_to_abi_string(inner),
            edge_ast::ty::TypeSig::Array(elem, len_expr)
            | edge_ast::ty::TypeSig::PackedArray(elem, len_expr) => {
                let elem_str = self.type_sig_to_abi_string(elem);
                Self::extract_array_length(len_expr)
                    .map_or_else(|| format!("{elem_str}[]"), |n| format!("{elem_str}[{n}]"))
            }
            _ => "uint256".to_owned(), // fallback
        }
    }

    /// Resolve whether a storage field's type is a packed struct.
    /// Returns `Some(type_name)` if the field type is a packed struct, `None` otherwise.
    /// Handles `Named("Rgb")`, `Pointer(_, Named("Rgb"))`, etc.
    pub(crate) fn resolve_storage_packed_struct_type(
        &self,
        ty: &edge_ast::ty::TypeSig,
    ) -> Option<String> {
        let inner = match ty {
            edge_ast::ty::TypeSig::Pointer(_, inner) => inner.as_ref(),
            _ => ty,
        };
        if let edge_ast::ty::TypeSig::Named(ident, _) = inner {
            // Check if this name resolves to a packed struct in struct_types
            if let Some(info) = self.struct_types.get(&ident.name) {
                if info.is_packed {
                    return Some(ident.name.clone());
                }
            }
            // Also check resolved generic names
            if let Some(mangled) = self.resolve_generic_type_name(&ident.name) {
                if let Some(info) = self.struct_types.get(&mangled) {
                    if info.is_packed {
                        return Some(mangled);
                    }
                }
            }
        }
        None
    }

    // ---- Type lowering helpers ----

    /// Extract the data location from a contract field's type signature.
    /// `&s T` → Storage (persistent), `&t T` → Transient, bare `T` → Storage (default).
    pub(crate) const fn extract_data_location(ty: &edge_ast::ty::TypeSig) -> DataLocation {
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
    pub(crate) fn lower_type_sig(&self, ty: &edge_ast::ty::TypeSig) -> EvmType {
        let resolved = self.resolve_type_alias(ty).clone();
        match &resolved {
            edge_ast::ty::TypeSig::Primitive(prim) => {
                EvmType::Base(self.lower_primitive_base_type(prim))
            }
            edge_ast::ty::TypeSig::Pointer(_, inner) => self.lower_type_sig(inner),
            edge_ast::ty::TypeSig::Tuple(types) => {
                let base_types: Vec<EvmBaseType> = types
                    .iter()
                    .map(|t| match self.lower_type_sig(t) {
                        EvmType::Base(b) => b,
                        EvmType::TupleT(_) | EvmType::ArrayT(..) => EvmBaseType::UIntT(256),
                    })
                    .collect();
                EvmType::TupleT(base_types)
            }
            edge_ast::ty::TypeSig::Array(elem, len_expr)
            | edge_ast::ty::TypeSig::PackedArray(elem, len_expr) => {
                let elem_base = match self.lower_type_sig(elem) {
                    EvmType::Base(b) => b,
                    _ => EvmBaseType::UIntT(256),
                };
                let len = Self::extract_array_length(len_expr).unwrap_or(0);
                EvmType::ArrayT(elem_base, len)
            }
            _ => EvmType::Base(EvmBaseType::UIntT(256)),
        }
    }

    /// Lower a primitive type to an EVM base type.
    pub(crate) const fn lower_primitive_type(&self, prim: &edge_ast::ty::PrimitiveType) -> EvmType {
        EvmType::Base(self.lower_primitive_base_type(prim))
    }

    /// Lower a primitive type to an EVM base type.
    pub(crate) const fn lower_primitive_base_type(
        &self,
        prim: &edge_ast::ty::PrimitiveType,
    ) -> EvmBaseType {
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
    pub(crate) fn params_to_type(
        &self,
        params: &[(edge_ast::Ident, edge_ast::ty::TypeSig)],
    ) -> EvmType {
        match params.len() {
            0 => EvmType::Base(EvmBaseType::UnitT),
            1 => self.lower_type_sig(&params[0].1),
            _ => {
                let base_types: Vec<EvmBaseType> = params
                    .iter()
                    .map(|(_, ty)| match self.lower_type_sig(ty) {
                        EvmType::Base(b) => b,
                        EvmType::TupleT(_) | EvmType::ArrayT(..) => EvmBaseType::UIntT(256),
                    })
                    .collect();
                EvmType::TupleT(base_types)
            }
        }
    }

    /// Build output type from return types.
    pub(crate) fn returns_to_type(&self, returns: &[edge_ast::ty::TypeSig]) -> EvmType {
        match returns.len() {
            0 => EvmType::Base(EvmBaseType::UnitT),
            1 => self.lower_type_sig(&returns[0]),
            _ => {
                let base_types: Vec<EvmBaseType> = returns
                    .iter()
                    .map(|ty| match self.lower_type_sig(ty) {
                        EvmType::Base(b) => b,
                        EvmType::TupleT(_) | EvmType::ArrayT(..) => EvmBaseType::UIntT(256),
                    })
                    .collect();
                EvmType::TupleT(base_types)
            }
        }
    }

    // ---- Generics support ----

    /// Substitute type parameters in a `TypeSig` with concrete types.
    pub(crate) fn substitute_type_params(
        type_sig: &edge_ast::ty::TypeSig,
        subst: &HashMap<String, edge_ast::ty::TypeSig>,
    ) -> edge_ast::ty::TypeSig {
        match type_sig {
            edge_ast::ty::TypeSig::Named(ident, args) => {
                // If this is a type parameter, substitute it
                if args.is_empty() {
                    if let Some(concrete) = subst.get(&ident.name) {
                        return concrete.clone();
                    }
                }
                // Otherwise, recursively substitute in type args
                let new_args: Vec<_> = args
                    .iter()
                    .map(|a| Self::substitute_type_params(a, subst))
                    .collect();
                edge_ast::ty::TypeSig::Named(ident.clone(), new_args)
            }
            edge_ast::ty::TypeSig::Array(elem, len) => edge_ast::ty::TypeSig::Array(
                Box::new(Self::substitute_type_params(elem, subst)),
                len.clone(),
            ),
            edge_ast::ty::TypeSig::PackedArray(elem, len) => edge_ast::ty::TypeSig::PackedArray(
                Box::new(Self::substitute_type_params(elem, subst)),
                len.clone(),
            ),
            edge_ast::ty::TypeSig::Pointer(loc, inner) => edge_ast::ty::TypeSig::Pointer(
                *loc,
                Box::new(Self::substitute_type_params(inner, subst)),
            ),
            edge_ast::ty::TypeSig::Tuple(types) => {
                let new_types: Vec<_> = types
                    .iter()
                    .map(|t| Self::substitute_type_params(t, subst))
                    .collect();
                edge_ast::ty::TypeSig::Tuple(new_types)
            }
            edge_ast::ty::TypeSig::Struct(fields) => {
                let new_fields: Vec<_> = fields
                    .iter()
                    .map(|f| edge_ast::ty::StructField {
                        name: f.name.clone(),
                        ty: Self::substitute_type_params(&f.ty, subst),
                    })
                    .collect();
                edge_ast::ty::TypeSig::Struct(new_fields)
            }
            // Primitives and other types don't contain type params
            other => other.clone(),
        }
    }

    /// Substitute type parameters in a code block (AST-level).
    /// Replaces type param names in Path expressions (e.g., V::sload → u256::sload).
    /// For generic types like Map<addr, u256>, uses the mangled name (Map__address_u256)
    /// so that qualified calls resolve to monomorphized trait impls.
    fn substitute_code_block(
        block: &edge_ast::CodeBlock,
        subst: &HashMap<String, edge_ast::ty::TypeSig>,
    ) -> edge_ast::CodeBlock {
        // Build a string→string map for path substitution using mangled names
        let name_subst: HashMap<&str, String> = subst.iter().map(|(k, v)| {
            (k.as_str(), Self::type_sig_mangle(v))
        }).collect();

        edge_ast::CodeBlock {
            stmts: block.stmts.iter().map(|item| {
                Self::substitute_block_item(item, &name_subst)
            }).collect(),
            span: block.span.clone(),
        }
    }

    fn substitute_block_item(
        item: &edge_ast::stmt::BlockItem,
        subst: &HashMap<&str, String>,
    ) -> edge_ast::stmt::BlockItem {
        match item {
            edge_ast::stmt::BlockItem::Stmt(stmt) => {
                edge_ast::stmt::BlockItem::Stmt(Box::new(Self::substitute_stmt(stmt, subst)))
            }
            edge_ast::stmt::BlockItem::Expr(expr) => {
                edge_ast::stmt::BlockItem::Expr(Self::substitute_expr(expr, subst))
            }
        }
    }

    fn substitute_stmt(
        stmt: &edge_ast::Stmt,
        subst: &HashMap<&str, String>,
    ) -> edge_ast::Stmt {
        match stmt {
            edge_ast::Stmt::VarDecl(ident, ty, init, span) => {
                edge_ast::Stmt::VarDecl(
                    ident.clone(),
                    ty.clone(),
                    init.as_ref().map(|e| Box::new(Self::substitute_expr(e, subst))),
                    span.clone(),
                )
            }
            edge_ast::Stmt::VarAssign(lhs, rhs, span) => {
                edge_ast::Stmt::VarAssign(
                    Self::substitute_expr(lhs, subst),
                    Self::substitute_expr(rhs, subst),
                    span.clone(),
                )
            }
            edge_ast::Stmt::Return(Some(expr), span) => {
                edge_ast::Stmt::Return(Some(Self::substitute_expr(expr, subst)), span.clone())
            }
            edge_ast::Stmt::Expr(expr) => {
                edge_ast::Stmt::Expr(Self::substitute_expr(expr, subst))
            }
            other => other.clone(),
        }
    }

    fn substitute_expr(
        expr: &edge_ast::Expr,
        subst: &HashMap<&str, String>,
    ) -> edge_ast::Expr {
        match expr {
            edge_ast::Expr::Path(components, span) => {
                let new_components: Vec<edge_ast::Ident> = components.iter().map(|c| {
                    if let Some(replacement) = subst.get(c.name.as_str()) {
                        edge_ast::Ident { name: replacement.clone(), span: c.span.clone() }
                    } else {
                        c.clone()
                    }
                }).collect();
                edge_ast::Expr::Path(new_components, span.clone())
            }
            edge_ast::Expr::FunctionCall(callee, args, turbofish, span) => {
                edge_ast::Expr::FunctionCall(
                    Box::new(Self::substitute_expr(callee, subst)),
                    args.iter().map(|a| Self::substitute_expr(a, subst)).collect(),
                    turbofish.clone(),
                    span.clone(),
                )
            }
            edge_ast::Expr::FieldAccess(obj, field, span) => {
                edge_ast::Expr::FieldAccess(
                    Box::new(Self::substitute_expr(obj, subst)),
                    field.clone(),
                    span.clone(),
                )
            }
            edge_ast::Expr::Binary(lhs, op, rhs, span) => {
                edge_ast::Expr::Binary(
                    Box::new(Self::substitute_expr(lhs, subst)),
                    op.clone(),
                    Box::new(Self::substitute_expr(rhs, subst)),
                    span.clone(),
                )
            }
            edge_ast::Expr::Paren(inner, span) => {
                edge_ast::Expr::Paren(
                    Box::new(Self::substitute_expr(inner, subst)),
                    span.clone(),
                )
            }
            _ => expr.clone(),
        }
    }

    /// Try to monomorphize a generic union from a variant constructor call.
    ///
    /// Given `Result::Ok(42)` where `Result<T> = Ok(T) | Err(u256)`:
    /// - Finds variant `Ok` in the template → data type is `T`
    /// - Infers `T = u256` from the argument (default assumption: u256 for integer literals)
    /// - Monomorphizes `Result<u256>` → `Result__u256`
    pub(crate) fn try_monomorphize_union_from_constructor(
        &mut self,
        generic_name: &str,
        variant_name: &str,
        args: &[edge_ast::Expr],
    ) -> Result<Option<String>, IrError> {
        let template = match self.generic_type_templates.get(generic_name) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };

        // Only works for union templates
        let members = match &template.type_sig {
            edge_ast::ty::TypeSig::Union(members) => members,
            _ => return Ok(None),
        };

        // Find the variant
        let variant = members.iter().find(|m| m.name.name == variant_name);
        let variant = match variant {
            Some(v) => v,
            None => return Ok(None),
        };

        // Build a substitution map by matching the variant's inner type against arg types
        let mut subst: HashMap<String, edge_ast::ty::TypeSig> = HashMap::new();
        let tp_names: HashSet<&str> = template
            .type_params
            .iter()
            .map(|tp| tp.name.name.as_str())
            .collect();

        if let Some(ref inner_ty) = variant.inner {
            // Variant carries data — infer from the constructor arg
            if args.len() == 1 {
                let arg_evm_ty = self.infer_expr_type(&args[0]);
                Self::unify_type(inner_ty, &arg_evm_ty, &tp_names, &mut subst)?;
            }
        }
        // For variants with no data (e.g., `Err(u256)` where the data type is concrete),
        // also try to infer from other variants' data types if possible.

        // Check that all type params were inferred
        let mut type_args = Vec::new();
        for param in &template.type_params {
            match subst.get(&param.name.name) {
                Some(ts) => type_args.push(ts.clone()),
                None => return Ok(None), // Can't infer all params
            }
        }

        let mangled = self.monomorphize_type(generic_name, &type_args, None)?;
        Ok(Some(mangled))
    }

    /// Monomorphize a generic type with concrete type arguments.
    /// Registers the result in `struct_types/union_types` and returns the mangled name.
    pub(crate) fn monomorphize_type(
        &mut self,
        generic_name: &str,
        type_args: &[edge_ast::ty::TypeSig],
        span: Option<&edge_types::span::Span>,
    ) -> Result<String, IrError> {
        // Use mangled type names for caching — EvmType loses source-level
        // distinctions (e.g., CustomHash and u256 both lower to UIntT(256)).
        let cache_key_types: Vec<String> =
            type_args.iter().map(Self::type_sig_mangle).collect();

        // Check cache
        let cache_key = (generic_name.to_string(), cache_key_types);
        if let Some(mangled) = self.monomorphized_types.get(&cache_key) {
            return Ok(mangled.clone());
        }

        let template = self
            .generic_type_templates
            .get(generic_name)
            .ok_or_else(|| {
                IrError::Diagnostic(edge_diagnostics::Diagnostic::error(format!(
                    "unknown generic type: `{generic_name}`",
                )))
            })?
            .clone();

        if template.type_params.len() != type_args.len() {
            let msg = format!(
                "type `{generic_name}` expects {} type argument{}, but {} {} supplied",
                template.type_params.len(),
                if template.type_params.len() == 1 {
                    ""
                } else {
                    "s"
                },
                type_args.len(),
                if type_args.len() == 1 { "was" } else { "were" },
            );
            let mut diag = edge_diagnostics::Diagnostic::error(msg);
            if let Some(s) = span {
                diag = diag.with_label(
                    s.clone(),
                    format!(
                        "expected {} type argument{}",
                        template.type_params.len(),
                        if template.type_params.len() == 1 {
                            ""
                        } else {
                            "s"
                        },
                    ),
                );
            }
            return Err(IrError::Diagnostic(diag));
        }

        // Validate trait bounds on type parameters
        for (tp, arg) in template.type_params.iter().zip(type_args.iter()) {
            if !tp.constraints.is_empty() {
                let concrete_name = Self::type_sig_display(arg);
                // For generic type args (e.g., Map<addr, u256>), also try the mangled name
                // since monomorphized impls are registered under the mangled name.
                let mangled_name = if let edge_ast::ty::TypeSig::Named(name, inner_args) = arg {
                    if !inner_args.is_empty() {
                        // Ensure the inner type is monomorphized first
                        match self.try_monomorphize_named_type(&name.name, inner_args, span) {
                            Ok(Some(m)) => Some(m),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                for constraint in &tp.constraints {
                    let key = (concrete_name.clone(), constraint.name.clone());
                    let mangled_key = mangled_name.as_ref().map(|m| (m.clone(), constraint.name.clone()));
                    let satisfied = self.trait_impls.contains_key(&key)
                        || mangled_key.as_ref().map_or(false, |k| self.trait_impls.contains_key(k));
                    if !satisfied {
                        let mut diag = edge_diagnostics::Diagnostic::error(format!(
                            "the trait bound `{}: {}` is not satisfied",
                            concrete_name, constraint.name,
                        ));
                        if let Some(s) = span {
                            diag = diag.with_label(
                                s.clone(),
                                format!(
                                    "`{}` does not implement `{}`",
                                    concrete_name, constraint.name,
                                ),
                            );
                        }
                        diag = diag.with_note(format!(
                            "required by a bound on type parameter `{}` in `{}`",
                            tp.name.name, generic_name,
                        ));
                        return Err(IrError::Diagnostic(diag));
                    }
                }
            }
        }

        // Build substitution map
        let subst: HashMap<String, edge_ast::ty::TypeSig> = template
            .type_params
            .iter()
            .zip(type_args.iter())
            .map(|(param, arg)| (param.name.name.clone(), arg.clone()))
            .collect();

        // Use mangled type names for identifier-safe names (no angle brackets).
        let type_name_strs: Vec<String> = type_args.iter().map(Self::type_sig_mangle).collect();
        let mangled = format!("{generic_name}__{}", type_name_strs.join("_"));

        // Substitute and register
        let concrete_sig = Self::substitute_type_params(&template.type_sig, &subst);

        match &concrete_sig {
            edge_ast::ty::TypeSig::Struct(fields) => {
                let field_info: Vec<(String, EvmType)> = fields
                    .iter()
                    .map(|f| (f.name.name.clone(), self.lower_type_sig(&f.ty)))
                    .collect();
                self.struct_types
                    .insert(mangled.clone(), StructTypeInfo::unpacked(field_info));
            }
            edge_ast::ty::TypeSig::PackedStruct(fields) => {
                let field_info: Vec<(String, EvmType)> = fields
                    .iter()
                    .map(|f| (f.name.name.clone(), self.lower_type_sig(&f.ty)))
                    .collect();
                self.struct_types
                    .insert(mangled.clone(), StructTypeInfo::packed(field_info));
            }
            edge_ast::ty::TypeSig::Union(members) => {
                let variants: Vec<(String, bool)> = members
                    .iter()
                    .map(|m| (m.name.name.clone(), m.inner.is_some()))
                    .collect();
                self.union_types.insert(mangled.clone(), variants);
            }
            _ => {
                // Type alias to a concrete type
                self.type_aliases.insert(mangled.clone(), concrete_sig);
            }
        }

        // Monomorphize impl blocks for this generic type
        if let Some(impl_blocks) = self.generic_impl_blocks.get(generic_name).cloned() {
            for gib in &impl_blocks {
                // Build substitution from the generic impl's type params to concrete args
                let impl_subst: HashMap<String, edge_ast::ty::TypeSig> = if gib.type_params.is_empty() {
                    // Use the type template's params (e.g., `impl Map<K, V>` where K,V from the type)
                    subst.clone()
                } else {
                    gib.type_params.iter()
                        .zip(type_args.iter())
                        .map(|(param, arg)| (param.name.name.clone(), arg.clone()))
                        .collect()
                };

                // Substitute type params in method bodies and register under mangled name
                let concrete_methods: Vec<edge_ast::item::ImplItem> = gib.items.iter().map(|item| {
                    match item {
                        edge_ast::item::ImplItem::FnAssign(fn_decl, body) => {
                            let new_params: Vec<(edge_ast::Ident, edge_ast::ty::TypeSig)> = fn_decl.params.iter().map(|(id, ty)| {
                                (id.clone(), Self::substitute_type_params(ty, &impl_subst))
                            }).collect();
                            let new_returns: Vec<edge_ast::ty::TypeSig> = fn_decl.returns.iter().map(|ty| {
                                Self::substitute_type_params(ty, &impl_subst)
                            }).collect();
                            let new_fn_decl = edge_ast::item::FnDecl {
                                name: fn_decl.name.clone(),
                                params: new_params,
                                returns: new_returns,
                                type_params: Vec::new(), // concrete, no type params
                                is_pub: fn_decl.is_pub,
                                is_ext: fn_decl.is_ext,
                                is_mut: fn_decl.is_mut,
                                span: fn_decl.span.clone(),
                            };
                            // Substitute type params in body expressions
                            let new_body = Self::substitute_code_block(body, &impl_subst);
                            edge_ast::item::ImplItem::FnAssign(new_fn_decl, new_body)
                        }
                        other => other.clone(),
                    }
                }).collect();

                if let Some(ref trait_name) = gib.trait_impl {
                    // Trait impl: register under mangled type name
                    let mut methods = IndexMap::new();
                    for item in &concrete_methods {
                        if let edge_ast::item::ImplItem::FnAssign(fn_decl, body) = item {
                            methods.insert(fn_decl.name.name.clone(), (fn_decl.clone(), body.clone()));
                        }
                    }
                    // Substitute type params in trait type args to get concrete types
                    let trait_type_args: Vec<edge_ast::ty::TypeSig> = gib.trait_type_params.iter()
                        .map(|p| {
                            let sig = edge_ast::ty::TypeSig::Named(p.name.clone(), Vec::new());
                            Self::substitute_type_params(&sig, &impl_subst)
                        })
                        .collect();
                    self.trait_impls.insert(
                        (mangled.clone(), trait_name.clone()),
                        super::TraitImplInfo {
                            methods,
                            trait_type_args,
                            span: edge_types::span::Span::EOF,
                        },
                    );
                } else {
                    // Inherent impl: register methods under mangled type name
                    let methods: Vec<super::InherentMethod> = concrete_methods.iter().filter_map(|item| {
                        if let edge_ast::item::ImplItem::FnAssign(fn_decl, body) = item {
                            Some(super::InherentMethod {
                                fn_decl: fn_decl.clone(),
                                body: body.clone(),
                            })
                        } else {
                            None
                        }
                    }).collect();
                    self.inherent_methods.entry(mangled.clone()).or_default().extend(methods);
                }
            }
        }

        self.monomorphized_types.insert(cache_key, mangled.clone());
        Ok(mangled)
    }

    /// Infer type parameters from argument types and optionally from the return type
    /// via the assignment target type hint.
    pub(crate) fn infer_type_params_from_args_and_return(
        &self,
        type_params: &[edge_ast::ty::TypeParam],
        param_types: &[(String, edge_ast::ty::TypeSig)],
        arg_types: &[EvmType],
        return_types: &[edge_ast::ty::TypeSig],
    ) -> Result<HashMap<String, edge_ast::ty::TypeSig>, IrError> {
        let tp_names: HashSet<&str> = type_params.iter().map(|tp| tp.name.name.as_str()).collect();
        let mut inferred: HashMap<String, edge_ast::ty::TypeSig> = HashMap::new();

        // Infer from argument types
        for ((_name, param_ty), arg_ty) in param_types.iter().zip(arg_types.iter()) {
            Self::unify_type(param_ty, arg_ty, &tp_names, &mut inferred)?;
        }

        // If some params are still unresolved, try to infer from the return type
        // using the assignment target type hint.
        if let Some(ref hint_ty) = self.type_hint {
            let unresolved: Vec<&str> = type_params
                .iter()
                .filter(|tp| !inferred.contains_key(&tp.name.name))
                .map(|tp| tp.name.name.as_str())
                .collect();
            if !unresolved.is_empty() && !return_types.is_empty() {
                // Unify each return type with the hint
                for ret_ty in return_types {
                    Self::unify_type(ret_ty, hint_ty, &tp_names, &mut inferred)?;
                }
            }
        }

        // Check all type params were inferred
        for tp in type_params {
            if !inferred.contains_key(&tp.name.name) {
                return Err(IrError::Diagnostic(
                    edge_diagnostics::Diagnostic::error(format!(
                        "type annotations needed: cannot infer type for parameter `{}`",
                        tp.name.name,
                    ))
                    .with_label(tp.name.span.clone(), "cannot infer type for this parameter")
                    .with_note("consider providing explicit type arguments"),
                ));
            }
        }

        Ok(inferred)
    }

    /// Try to unify a `TypeSig` parameter with a concrete `EvmType` to infer type params.
    /// Returns `Err` if a type parameter would be unified to two different concrete types.
    fn unify_type(
        param_ty: &edge_ast::ty::TypeSig,
        arg_ty: &EvmType,
        type_params: &HashSet<&str>,
        inferred: &mut HashMap<String, edge_ast::ty::TypeSig>,
    ) -> Result<(), IrError> {
        match param_ty {
            edge_ast::ty::TypeSig::Named(ident, args)
                if args.is_empty() && type_params.contains(ident.name.as_str()) =>
            {
                // This is a type parameter — infer from arg type
                let concrete = Self::evm_type_to_type_sig(arg_ty);
                if let Some(existing) = inferred.get(&ident.name) {
                    if *existing != concrete {
                        return Err(IrError::Diagnostic(
                            edge_diagnostics::Diagnostic::error(format!(
                                "conflicting types for parameter `{}`: expected `{}`, found `{}`",
                                ident.name,
                                Self::type_sig_display(existing),
                                Self::type_sig_display(&concrete),
                            ))
                            .with_label(
                                ident.span.clone(),
                                format!("conflicting inference for `{}`", ident.name),
                            ),
                        ));
                    }
                } else {
                    inferred.insert(ident.name.clone(), concrete);
                }
            }
            _ => {
                // Not a type parameter — no inference needed
            }
        }
        Ok(())
    }

    /// Mangle a `TypeSig` into an identifier-safe name for use as mangled type names.
    /// E.g., `Map<addr, u256>` → `Map__address_u256`, nested types recursively mangled.
    pub(crate) fn type_sig_mangle(ty: &edge_ast::ty::TypeSig) -> String {
        match ty {
            edge_ast::ty::TypeSig::Primitive(p) => {
                use edge_ast::ty::PrimitiveType;
                match p {
                    PrimitiveType::UInt(n) => format!("u{n}"),
                    PrimitiveType::Int(n) => format!("i{n}"),
                    PrimitiveType::FixedBytes(n) => format!("b{n}"),
                    PrimitiveType::Address => "address".to_string(),
                    PrimitiveType::Bool => "bool".to_string(),
                    PrimitiveType::Bit => "bit".to_string(),
                }
            }
            edge_ast::ty::TypeSig::Named(ident, args) => {
                if args.is_empty() {
                    ident.name.clone()
                } else {
                    let arg_strs: Vec<String> = args.iter()
                        .map(Self::type_sig_mangle)
                        .collect();
                    format!("{}__{}", ident.name, arg_strs.join("_"))
                }
            }
            _ => "unknown".to_string(),
        }
    }

    /// Simple display for a `TypeSig` (for error messages).
    pub(crate) fn type_sig_display(ty: &edge_ast::ty::TypeSig) -> String {
        match ty {
            edge_ast::ty::TypeSig::Primitive(p) => {
                use edge_ast::ty::PrimitiveType;
                match p {
                    PrimitiveType::UInt(n) => format!("u{n}"),
                    PrimitiveType::Int(n) => format!("i{n}"),
                    PrimitiveType::FixedBytes(n) => format!("b{n}"),
                    PrimitiveType::Address => "address".to_string(),
                    PrimitiveType::Bool => "bool".to_string(),
                    PrimitiveType::Bit => "bit".to_string(),
                }
            }
            edge_ast::ty::TypeSig::Named(ident, args) => {
                if args.is_empty() {
                    ident.name.clone()
                } else {
                    format!(
                        "{}<{}>",
                        ident.name,
                        args.iter()
                            .map(Self::type_sig_display)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            _ => "?".to_string(),
        }
    }

    /// Convert an `EvmType` back to a `TypeSig` for substitution.
    const fn evm_type_to_type_sig(ty: &EvmType) -> edge_ast::ty::TypeSig {
        match ty {
            EvmType::Base(EvmBaseType::UIntT(bits)) => {
                edge_ast::ty::TypeSig::Primitive(edge_ast::ty::PrimitiveType::UInt(*bits))
            }
            EvmType::Base(EvmBaseType::IntT(bits)) => {
                edge_ast::ty::TypeSig::Primitive(edge_ast::ty::PrimitiveType::Int(*bits))
            }
            EvmType::Base(EvmBaseType::AddrT) => {
                edge_ast::ty::TypeSig::Primitive(edge_ast::ty::PrimitiveType::Address)
            }
            EvmType::Base(EvmBaseType::BoolT) => {
                edge_ast::ty::TypeSig::Primitive(edge_ast::ty::PrimitiveType::Bool)
            }
            EvmType::Base(EvmBaseType::BytesT(n)) => {
                edge_ast::ty::TypeSig::Primitive(edge_ast::ty::PrimitiveType::FixedBytes(*n))
            }
            _ => edge_ast::ty::TypeSig::Primitive(edge_ast::ty::PrimitiveType::UInt(256)),
        }
    }

    /// Eagerly scan all type signatures in the program and monomorphize any generic types
    /// used with concrete type arguments.
    pub(crate) fn monomorphize_all_type_usages(
        &mut self,
        program: &edge_ast::Program,
    ) -> Result<(), IrError> {
        // Collect all type sigs used in the program
        let mut type_sigs = Vec::new();
        for stmt in &program.stmts {
            Self::collect_type_sigs_from_stmt(stmt, &mut type_sigs);
        }
        // Monomorphize any Named types with type args that are generic templates
        for ts in &type_sigs {
            self.try_monomorphize_type_sig(ts)?;
        }
        Ok(())
    }

    /// Recursively monomorphize a type signature if it references generic types.
    fn try_monomorphize_type_sig(&mut self, ts: &edge_ast::ty::TypeSig) -> Result<(), IrError> {
        match ts {
            edge_ast::ty::TypeSig::Named(ident, type_args) if !type_args.is_empty() => {
                if self.generic_type_templates.contains_key(&ident.name) {
                    self.try_monomorphize_named_type(&ident.name, type_args, Some(&ident.span))?;
                }
                // Recurse into type args
                for arg in type_args {
                    self.try_monomorphize_type_sig(arg)?;
                }
            }
            edge_ast::ty::TypeSig::Array(elem, _) | edge_ast::ty::TypeSig::PackedArray(elem, _) => {
                self.try_monomorphize_type_sig(elem)?;
            }
            edge_ast::ty::TypeSig::Pointer(_, inner) => {
                self.try_monomorphize_type_sig(inner)?;
            }
            edge_ast::ty::TypeSig::Tuple(types) => {
                for t in types {
                    self.try_monomorphize_type_sig(t)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Collect all type signatures used in a statement (function params, returns, var decls, etc.).
    fn collect_type_sigs_from_stmt(stmt: &edge_ast::Stmt, out: &mut Vec<edge_ast::ty::TypeSig>) {
        match stmt {
            edge_ast::Stmt::FnAssign(fn_decl, body) | edge_ast::Stmt::ComptimeFn(fn_decl, body) => {
                for (_, ts) in &fn_decl.params {
                    out.push(ts.clone());
                }
                for ts in &fn_decl.returns {
                    out.push(ts.clone());
                }
                Self::collect_type_sigs_from_block(body, out);
            }
            edge_ast::Stmt::VarDecl(_, Some(ts), _, _) => {
                out.push(ts.clone());
            }
            edge_ast::Stmt::ContractDecl(contract) => {
                for fn_decl in &contract.functions {
                    for (_, ts) in &fn_decl.params {
                        out.push(ts.clone());
                    }
                    for ts in &fn_decl.returns {
                        out.push(ts.clone());
                    }
                    if let Some(body) = &fn_decl.body {
                        Self::collect_type_sigs_from_block(body, out);
                    }
                }
            }
            edge_ast::Stmt::ImplBlock(impl_block) => {
                for item in &impl_block.items {
                    if let edge_ast::item::ImplItem::FnAssign(fn_decl, body) = item {
                        for (_, ts) in &fn_decl.params {
                            out.push(ts.clone());
                        }
                        for ts in &fn_decl.returns {
                            out.push(ts.clone());
                        }
                        Self::collect_type_sigs_from_block(body, out);
                    }
                }
            }
            _ => {}
        }
    }

    /// Collect type signatures from a code block (recursing into nested statements).
    fn collect_type_sigs_from_block(
        block: &edge_ast::CodeBlock,
        out: &mut Vec<edge_ast::ty::TypeSig>,
    ) {
        for item in &block.stmts {
            if let edge_ast::stmt::BlockItem::Stmt(stmt) = item {
                Self::collect_type_sigs_from_stmt(stmt, out);
            }
        }
    }
}
