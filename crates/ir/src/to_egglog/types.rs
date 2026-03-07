//! Type lowering and resolution.

use std::collections::{HashMap, HashSet};

use super::AstToEgglog;
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
    ) -> Result<Option<String>, IrError> {
        if type_args.is_empty() || !self.generic_type_templates.contains_key(name) {
            return Ok(None);
        }
        let mangled = self.monomorphize_type(name, type_args)?;
        Ok(Some(mangled))
    }

    /// Resolve a generic type name (e.g., "Result") to its monomorphized name (e.g., "`Result__u256`").
    /// Searches `union_types` and `struct_types` for any key starting with `"{name}__"`.
    /// Returns the first match found.
    pub(crate) fn resolve_generic_type_name(&self, name: &str) -> Option<String> {
        let prefix = format!("{name}__");
        for key in self.union_types.keys() {
            if key.starts_with(&prefix) {
                return Some(key.clone());
            }
        }
        for key in self.struct_types.keys() {
            if key.starts_with(&prefix) {
                return Some(key.clone());
            }
        }
        None
    }

    /// Extract fixed array length from a type expression (literal integer).
    pub(crate) fn extract_array_length(len_expr: &edge_ast::Expr) -> Option<usize> {
        if let edge_ast::Expr::Literal(lit) = len_expr {
            if let edge_ast::Lit::Int(n, _, _) = lit.as_ref() {
                return Some(*n as usize);
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
                        EvmType::TupleT(_) => EvmBaseType::UIntT(256),
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
                        EvmType::TupleT(_) => EvmBaseType::UIntT(256),
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

    /// Generate a mangled name for a monomorphized type.
    /// e.g., ("Stack", [UIntT(256)]) → "`Stack__u256`"
    pub(crate) fn mangle_type_name(base: &str, concrete_types: &[EvmType]) -> String {
        let type_strs: Vec<String> = concrete_types
            .iter()
            .map(|t| match t {
                EvmType::Base(EvmBaseType::UIntT(bits)) => format!("u{bits}"),
                EvmType::Base(EvmBaseType::IntT(bits)) => format!("i{bits}"),
                EvmType::Base(EvmBaseType::AddrT) => "addr".to_owned(),
                EvmType::Base(EvmBaseType::BoolT) => "bool".to_owned(),
                EvmType::Base(EvmBaseType::BytesT(n)) => format!("b{n}"),
                _ => "u256".to_owned(),
            })
            .collect();
        format!("{base}__{}", type_strs.join("_"))
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
        let tp_names: HashSet<&str> = template.type_params.iter().map(|s| s.as_str()).collect();

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
            match subst.get(param) {
                Some(ts) => type_args.push(ts.clone()),
                None => return Ok(None), // Can't infer all params
            }
        }

        let mangled = self.monomorphize_type(generic_name, &type_args)?;
        Ok(Some(mangled))
    }

    /// Monomorphize a generic type with concrete type arguments.
    /// Registers the result in `struct_types/union_types` and returns the mangled name.
    pub(crate) fn monomorphize_type(
        &mut self,
        generic_name: &str,
        type_args: &[edge_ast::ty::TypeSig],
    ) -> Result<String, IrError> {
        // Lower type args to EvmType for caching
        let concrete_types: Vec<EvmType> =
            type_args.iter().map(|t| self.lower_type_sig(t)).collect();

        // Check cache
        let cache_key = (generic_name.to_string(), concrete_types.clone());
        if let Some(mangled) = self.monomorphized_types.get(&cache_key) {
            return Ok(mangled.clone());
        }

        let template = self
            .generic_type_templates
            .get(generic_name)
            .ok_or_else(|| IrError::Lowering(format!("unknown generic type: {generic_name}")))?
            .clone();

        if template.type_params.len() != type_args.len() {
            return Err(IrError::Lowering(format!(
                "type `{generic_name}` expects {} type argument{}, but {} {} supplied",
                template.type_params.len(),
                if template.type_params.len() == 1 {
                    ""
                } else {
                    "s"
                },
                type_args.len(),
                if type_args.len() == 1 { "was" } else { "were" },
            )));
        }

        // Build substitution map
        let subst: HashMap<String, edge_ast::ty::TypeSig> = template
            .type_params
            .iter()
            .zip(type_args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();

        let mangled = Self::mangle_type_name(generic_name, &concrete_types);

        // Substitute and register
        let concrete_sig = Self::substitute_type_params(&template.type_sig, &subst);

        match &concrete_sig {
            edge_ast::ty::TypeSig::Struct(fields) | edge_ast::ty::TypeSig::PackedStruct(fields) => {
                let field_info: Vec<(String, EvmType)> = fields
                    .iter()
                    .map(|f| (f.name.name.clone(), self.lower_type_sig(&f.ty)))
                    .collect();
                self.struct_types.insert(mangled.clone(), field_info);
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

    /// Simple display for a `TypeSig` (for error messages).
    fn type_sig_display(ty: &edge_ast::ty::TypeSig) -> String {
        match ty {
            edge_ast::ty::TypeSig::Primitive(p) => format!("{p:?}").to_lowercase(),
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
                    self.try_monomorphize_named_type(&ident.name, type_args)?;
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
            edge_ast::Stmt::VarDecl(_, Some(ts), _) => {
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
