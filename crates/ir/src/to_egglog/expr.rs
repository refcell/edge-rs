//! Expression and statement lowering.

use std::rc::Rc;

use super::{AstToEgglog, Scope, VarBinding};
use crate::{
    ast_helpers,
    schema::{
        DataLocation, EvmBaseType, EvmBinaryOp, EvmConstant, EvmEnvOp, EvmExpr, EvmTernaryOp,
        EvmType, EvmUnaryOp, RcExpr,
    },
    IrError,
};

/// Check if an expression is an untyped (unsuffixed) literal.
/// Unsuffixed int literals and bool literals are considered untyped and
/// will adopt the type of the other operand in a binary expression.
fn is_untyped_literal(expr: &edge_ast::Expr) -> bool {
    match expr {
        edge_ast::Expr::Literal(lit) => match lit.as_ref() {
            // no suffix
            edge_ast::Lit::Int(_, None, _) | edge_ast::Lit::Bool(_, _) => true,
            _ => false,
        },
        _ => false,
    }
}

/// Format an `EvmBaseType` as a user-facing type name.
fn fmt_base_type(bt: &EvmBaseType) -> String {
    match bt {
        EvmBaseType::UIntT(n) => format!("u{n}"),
        EvmBaseType::IntT(n) => format!("i{n}"),
        EvmBaseType::BytesT(n) => format!("bytes{n}"),
        EvmBaseType::AddrT => "addr".to_string(),
        EvmBaseType::BoolT => "bool".to_string(),
        EvmBaseType::UnitT => "()".to_string(),
        EvmBaseType::StateT => "state".to_string(),
    }
}

/// Extract the source text for an AST expression from the file source.
fn expr_src_text(expr: &edge_ast::Expr) -> String {
    let span = expr.span();
    // Walk up: try the expression's own file, which should have the source
    if let Some(ref file) = span.file {
        if let Some(ref src) = file.source {
            if let Some(text) = src.get(span.start..=span.end) {
                return text.trim().to_string();
            }
        }
    }
    // Fallback: extract name/value from common leaf nodes
    match expr {
        edge_ast::Expr::Ident(id) => id.name.clone(),
        edge_ast::Expr::Literal(lit) => match lit.as_ref() {
            edge_ast::Lit::Int(bytes, _, _) => {
                let val = ruint::aliases::U256::from_be_bytes::<32>(*bytes);
                format!("{val}")
            }
            edge_ast::Lit::Bool(b, _) => format!("{b}"),
            edge_ast::Lit::Str(s, _) => format!("\"{s}\""),
            edge_ast::Lit::Hex(bytes, _) => {
                format!(
                    "0x{}",
                    bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
                )
            }
            edge_ast::Lit::Bin(bytes, _) => {
                format!(
                    "0b{}",
                    bytes.iter().map(|b| format!("{b:08b}")).collect::<String>()
                )
            }
        },
        _ => "expr".to_string(),
    }
}

/// Extract a constant U256 value from an IR Const node, if it is one.
fn const_value(expr: &RcExpr) -> Option<ruint::aliases::U256> {
    match expr.as_ref() {
        EvmExpr::Const(EvmConstant::SmallInt(n), _, _) => {
            if *n >= 0 {
                Some(ruint::aliases::U256::from(*n as u64))
            } else {
                // Negative values wrap around in U256
                Some(ruint::aliases::U256::MAX - ruint::aliases::U256::from((-*n - 1) as u64))
            }
        }
        EvmExpr::Const(EvmConstant::LargeInt(hex), _, _) => {
            ruint::aliases::U256::from_str_radix(hex, 16).ok()
        }
        _ => None,
    }
}

impl AstToEgglog {
    /// Lower a statement.
    pub(crate) fn lower_stmt(&mut self, stmt: &edge_ast::Stmt) -> Result<RcExpr, IrError> {
        match stmt {
            edge_ast::Stmt::VarDecl(ident, type_sig, init_expr, _span) => {
                let ty = type_sig
                    .as_ref()
                    .map(|ts| self.lower_type_sig(ts))
                    .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));

                // If the type annotation is a generic type (e.g., Result<u256>),
                // trigger monomorphization so the concrete type is registered.
                let mut composite_type = None;
                if let Some(edge_ast::ty::TypeSig::Named(name_ident, type_args)) = type_sig {
                    if !type_args.is_empty()
                        && self.generic_type_templates.contains_key(&name_ident.name)
                    {
                        if let Some(mangled) = self.try_monomorphize_named_type(
                            &name_ident.name,
                            type_args,
                            Some(&name_ident.span),
                        )? {
                            composite_type = Some(mangled);
                        }
                    }
                }

                let zero = ast_helpers::const_int(0, self.current_ctx.clone());
                let var_name = format!("{}__local_{}", self.inline_prefix, ident.name);
                let binding = VarBinding {
                    value: zero,
                    location: DataLocation::Memory,
                    storage_slot: None,
                    _ty: ty,
                    let_bind_name: Some(var_name.clone()),
                    composite_type,
                    composite_base: None,
                    composite_type_args: Vec::new(),
                };
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(ident.name.clone(), binding);

                // If there's an initializer, emit VarStore for the assignment
                if let Some(init) = init_expr {
                    self.last_composite_alloc = None;
                    // Set type sig hint so struct instantiation can disambiguate generics
                    self.type_sig_hint = type_sig.as_ref().cloned();
                    let rhs_ir = self.lower_expr(init)?;
                    self.type_sig_hint = None;
                    // Track composite type from RHS if applicable
                    if let Some((comp_type, comp_base)) = self.last_composite_alloc.take() {
                        if let Some(scope) = self.scopes.last_mut() {
                            if let Some(binding) = scope.bindings.get_mut(&ident.name) {
                                binding.composite_type = Some(comp_type);
                                binding.composite_base = Some(comp_base);
                            }
                        }
                    }
                    Ok(ast_helpers::var_store(var_name, rhs_ir))
                } else {
                    // VarDecl without init produces no side effects; the LetBind
                    // wrapper is added by lower_code_block
                    Ok(ast_helpers::empty(
                        EvmType::Base(EvmBaseType::UnitT),
                        self.current_ctx.clone(),
                    ))
                }
            }

            edge_ast::Stmt::VarAssign(lhs, rhs, _span) => {
                // Clear composite tracking before evaluating RHS
                self.last_composite_alloc = None;
                // Set type hint from the LHS variable's declared type
                if let edge_ast::Expr::Ident(ident) = lhs {
                    for scope in self.scopes.iter().rev() {
                        if let Some(binding) = scope.bindings.get(&ident.name) {
                            self.type_hint = Some(binding._ty.clone());
                            break;
                        }
                    }
                }
                let rhs_ir = self.lower_expr(rhs)?;
                self.type_hint = None;
                // If RHS was a struct/array instantiation, wire composite info to LHS binding
                // (skip storage fields — they already have composite_type set from lower_contract)
                let rhs_composite = self.last_composite_alloc.clone();
                if let Some((ref type_name, ref base)) = rhs_composite {
                    if let edge_ast::Expr::Ident(ident) = lhs {
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(binding) = scope.bindings.get_mut(&ident.name) {
                                if binding.storage_slot.is_none() {
                                    binding.composite_type = Some(type_name.clone());
                                    binding.composite_base = Some(Rc::clone(base));
                                }
                                break;
                            }
                        }
                    }
                }
                self.last_composite_alloc = None;

                // Intercept ArrayIndex write for Index/Map dispatch: base[index] = val → base.set(index, val)
                if let edge_ast::Expr::ArrayIndex(arr_base, arr_index, _, arr_span) = lhs {
                    if let Some(result) =
                        self.try_lower_storage_array_write(arr_base, arr_index, &rhs_ir)?
                    {
                        return Ok(result);
                    }
                    if let Some(result) =
                        self.try_lower_array_element_write(arr_base, arr_index, &rhs_ir)?
                    {
                        return Ok(result);
                    }
                    if self.std_ops_traits.contains("Index") {
                        return self.lower_method_call(
                            arr_base,
                            "set",
                            &[arr_index.as_ref().clone(), rhs.clone()],
                            arr_span,
                        );
                    }
                }

                self.lower_assignment_with_composite(lhs, rhs_ir, rhs_composite.as_ref())
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
                    composite_type: None,
                    composite_base: None,
                    composite_type_args: Vec::new(),
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
                    // Check for tuple return: return (a, b, c)
                    if let edge_ast::Expr::TupleInstantiation(_, elements, _) = expr {
                        self.lower_tuple_return(elements)
                    } else {
                        let val = self.lower_expr(expr)?;
                        // ABI-encode the return value to memory and RETURN
                        let ret_buf = self.alloc_region(1);
                        let size = ast_helpers::const_int(32, self.current_ctx.clone());
                        let mstore_expr = ast_helpers::mstore(
                            Rc::clone(&ret_buf),
                            val,
                            Rc::clone(&self.current_state),
                        );
                        self.current_state = Rc::clone(&mstore_expr);
                        let ret =
                            ast_helpers::return_op(ret_buf, size, Rc::clone(&self.current_state));
                        Ok(ast_helpers::concat(mstore_expr, ret))
                    }
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

            edge_ast::Stmt::Break(span) => {
                self.warnings.push(
                    edge_diagnostics::Diagnostic::warning(
                        "`break` is not yet implemented and will be ignored",
                    )
                    .with_label(span.clone(), "has no effect"),
                );
                Ok(ast_helpers::empty(
                    EvmType::Base(EvmBaseType::UnitT),
                    self.current_ctx.clone(),
                ))
            }
            edge_ast::Stmt::Continue(span) => {
                self.warnings.push(
                    edge_diagnostics::Diagnostic::warning(
                        "`continue` is not yet implemented and will be ignored",
                    )
                    .with_label(span.clone(), "has no effect"),
                );
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

            edge_ast::Stmt::Match(discriminant, arms, _span) => {
                self.lower_match(discriminant, arms)
            }

            edge_ast::Stmt::IfMatch(expr, pattern, body) => {
                self.lower_if_match(expr, pattern, body)
            }

            // Type/trait/impl/abi/comptime-fn declarations are collected in lower_program; skip here
            edge_ast::Stmt::TypeAssign(_, _, _)
            | edge_ast::Stmt::TraitDecl(_, _)
            | edge_ast::Stmt::ImplBlock(_)
            | edge_ast::Stmt::AbiDecl(_)
            | edge_ast::Stmt::ComptimeFn(_, _) => Ok(ast_helpers::empty(
                EvmType::Base(EvmBaseType::UnitT),
                self.current_ctx.clone(),
            )),

            // TODO: implement remaining statement types
            other => Err(IrError::Unsupported(format!(
                "Statement type not yet supported: {other:?}"
            ))),
        }
    }

    /// Lower an expression.
    pub(crate) fn lower_expr(&mut self, expr: &edge_ast::Expr) -> Result<RcExpr, IrError> {
        match expr {
            edge_ast::Expr::Literal(lit) => self.lower_literal(lit),

            edge_ast::Expr::Ident(ident) => self.lower_ident(&ident.name, Some(&ident.span)),

            edge_ast::Expr::Binary(lhs, op, rhs, span) => {
                // Check for operator overloading on user-defined types
                if let Some(result) = self.try_operator_overload(lhs, op, rhs, span)? {
                    return Ok(result);
                }
                let lhs_ty = self.infer_expr_type(lhs);
                let rhs_ty = self.infer_expr_type(rhs);

                // Unsuffixed literals adopt the type of the other operand.
                // e.g. `x: u8 = 3; x - 2;` → the `2` becomes u8.
                let lhs_untyped = is_untyped_literal(lhs);
                let rhs_untyped = is_untyped_literal(rhs);
                let operand_ty = if lhs_untyped && !rhs_untyped {
                    rhs_ty
                } else if rhs_untyped && !lhs_untyped {
                    lhs_ty
                } else {
                    // Both typed (or both untyped) — check for mismatches
                    if let (EvmType::Base(ref lbt), EvmType::Base(ref rbt)) = (&lhs_ty, &rhs_ty) {
                        let lw = lbt.bit_width();
                        let rw = rbt.bit_width();
                        let l_signed = matches!(lbt, EvmBaseType::IntT(_));
                        let r_signed = matches!(rbt, EvmBaseType::IntT(_));
                        if (lw != rw || l_signed != r_signed) && lw > 0 && rw > 0 {
                            let lty_name = fmt_base_type(lbt);
                            let rty_name = fmt_base_type(rbt);
                            let lhs_src = expr_src_text(lhs);
                            let rhs_src = expr_src_text(rhs);
                            return Err(IrError::Diagnostic(
                                edge_diagnostics::Diagnostic::error(format!(
                                    "mismatched types: `{lty_name}` and `{rty_name}`"
                                ))
                                .with_label(span.clone(), format!("cannot apply `{op:?}` to `{lty_name}` and `{rty_name}`"))
                                .with_note(format!(
                                    "use an explicit cast: `{lhs_src} as {rty_name}` or `{rhs_src} as {lty_name}`"
                                )),
                            ));
                        }
                    }
                    lhs_ty
                };

                let lhs_ir = self.lower_expr(lhs)?;
                let rhs_ir = self.lower_expr(rhs)?;
                self.lower_binary_op(op, lhs_ir, rhs_ir, &operand_ty, span)
            }

            edge_ast::Expr::Unary(op, expr, _span) => {
                let operand_ty = self.infer_expr_type(expr);
                let expr_ir = self.lower_expr(expr)?;
                self.lower_unary_op(op, expr_ir, &operand_ty)
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

            edge_ast::Expr::FunctionCall(callee, args, type_args, span) => {
                self.lower_function_call(callee, args, type_args, span)
            }

            edge_ast::Expr::At(builtin_name, args, _span) => {
                self.lower_builtin(&builtin_name.name, args)
            }

            edge_ast::Expr::Assign(lhs, rhs, _span) => {
                // Clear composite tracking before evaluating RHS
                self.last_composite_alloc = None;
                let rhs_ir = self.lower_expr(rhs)?;
                // If RHS was a struct/array instantiation, wire composite info to LHS binding
                // (skip storage fields — they already have composite_type set from lower_contract)
                let rhs_composite = self.last_composite_alloc.clone();
                if let Some((ref type_name, ref base)) = rhs_composite {
                    if let edge_ast::Expr::Ident(ident) = lhs.as_ref() {
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(binding) = scope.bindings.get_mut(&ident.name) {
                                if binding.storage_slot.is_none() {
                                    binding.composite_type = Some(type_name.clone());
                                    binding.composite_base = Some(Rc::clone(base));
                                }
                                break;
                            }
                        }
                    }
                }
                self.last_composite_alloc = None;

                // Intercept ArrayIndex write for Index/Map dispatch: base[index] = val → base.set(index, val)
                if let edge_ast::Expr::ArrayIndex(arr_base, arr_index, _, arr_span) = lhs.as_ref() {
                    if let Some(result) =
                        self.try_lower_storage_array_write(arr_base, arr_index, &rhs_ir)?
                    {
                        return Ok(result);
                    }
                    if let Some(result) =
                        self.try_lower_array_element_write(arr_base, arr_index, &rhs_ir)?
                    {
                        return Ok(result);
                    }
                    if self.std_ops_traits.contains("Index") {
                        return self.lower_method_call(
                            arr_base,
                            "set",
                            &[arr_index.as_ref().clone(), rhs.as_ref().clone()],
                            arr_span,
                        );
                    }
                }

                self.lower_assignment_with_composite(lhs, rhs_ir, rhs_composite.as_ref())
            }

            edge_ast::Expr::ArrayIndex(base, index, end_index, _span) => {
                // Slice access: arr[start:end] → pointer to base + start * 32
                if end_index.is_some() {
                    if let edge_ast::Expr::Ident(ident) = base.as_ref() {
                        if let Some((_type_name, base_expr)) =
                            self.lookup_composite_info(&ident.name)
                        {
                            // Evaluate start index (must be a constant for now)
                            if let edge_ast::Expr::Literal(lit) = index.as_ref() {
                                if let edge_ast::Lit::Int(start_bytes, _, _) = lit.as_ref() {
                                    let start =
                                        u64::from_be_bytes(start_bytes[24..32].try_into().unwrap())
                                            as usize;
                                    let new_base = ast_helpers::add(
                                        base_expr,
                                        ast_helpers::const_int(
                                            (start * 32) as i64,
                                            self.current_ctx.clone(),
                                        ),
                                    );
                                    // Compute slice length from end index
                                    let slice_len = end_index.as_ref().map_or(0, |end_idx| {
                                        if let edge_ast::Expr::Literal(end_lit) = end_idx.as_ref() {
                                            if let edge_ast::Lit::Int(end_bytes, _, _) =
                                                end_lit.as_ref()
                                            {
                                                let end = u64::from_be_bytes(
                                                    end_bytes[24..32].try_into().unwrap(),
                                                )
                                                    as usize;
                                                end - start
                                            } else {
                                                0
                                            }
                                        } else {
                                            0
                                        }
                                    });
                                    self.last_composite_alloc = Some((
                                        format!("__array__{slice_len}"),
                                        Rc::clone(&new_base),
                                    ));
                                    return Ok(new_base);
                                }
                            }
                        }
                    }
                    return Err(IrError::Unsupported("dynamic slice access".to_owned()));
                }

                // Check if base is a storage array field
                if let Some(result) = self.try_lower_storage_array_read(base, index)? {
                    return Ok(result);
                }

                // Check if base is a memory-backed array/struct variable
                if let Some(result) = self.try_lower_array_element_read(base, index)? {
                    return Ok(result);
                }

                // Try Index trait dispatch: base[index] → base.index(index)
                if self.std_ops_traits.contains("Index") {
                    return self.lower_method_call(base, "index", &[index.as_ref().clone()], _span);
                }

                Err(IrError::Unsupported(
                    "array index on non-array type; use Map<K,V>.get(key) for mappings".to_owned(),
                ))
            }

            edge_ast::Expr::Paren(inner, _span) => self.lower_expr(inner),

            edge_ast::Expr::FieldAccess(obj, field, _span) => {
                self.lower_field_access(obj, &field.name)
            }

            edge_ast::Expr::Path(components, span) => {
                // Check if this is a union variant path like Direction::North
                if components.len() == 2 {
                    let type_name = &components[0].name;
                    let variant_name = &components[1].name;
                    if self.union_types.contains_key(type_name) {
                        return self.lower_union_instantiation_expr(
                            type_name,
                            variant_name,
                            &[],
                            Some(span),
                        );
                    }
                    // Check for generic union types (e.g., Option::None where Option<T> was monomorphized)
                    if self.generic_type_templates.contains_key(type_name) {
                        if let Some(mangled) = self.resolve_generic_type_name(type_name) {
                            return self.lower_union_instantiation_expr(
                                &mangled,
                                variant_name,
                                &[],
                                Some(span),
                            );
                        }
                    }
                }
                // Resolve module-prefixed paths or error on invalid partial paths.
                let name = self.resolve_path_to_name(components)?;
                self.lower_ident(&name, components.last().map(|c| &c.span))
            }

            edge_ast::Expr::TupleInstantiation(_, elements, _span) => {
                if elements.len() == 1 {
                    return self.lower_expr(&elements[0]);
                }
                // Allocate memory for tuple elements
                let base_ir = self.alloc_region(elements.len());
                let mut result =
                    ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
                for (i, elem) in elements.iter().enumerate() {
                    let val = self.lower_expr(elem)?;
                    let offset = ast_helpers::add(
                        Rc::clone(&base_ir),
                        ast_helpers::const_int((i * 32) as i64, self.current_ctx.clone()),
                    );
                    let mstore = ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
                    result = ast_helpers::concat(result, mstore);
                }
                // Track as composite for field access
                self.last_composite_alloc = Some(("__tuple".to_string(), Rc::clone(&base_ir)));
                // Return base address as the tuple "value"
                Ok(ast_helpers::concat(result, base_ir))
            }

            edge_ast::Expr::TupleFieldAccess(obj, index, _span) => {
                // Check if obj is a variable with composite info
                if let edge_ast::Expr::Ident(ident) = obj.as_ref() {
                    if let Some((_type_name, base_expr)) = self.lookup_composite_info(&ident.name) {
                        let field_offset = ast_helpers::add(
                            base_expr,
                            ast_helpers::const_int(
                                (*index as usize * 32) as i64,
                                self.current_ctx.clone(),
                            ),
                        );
                        return Ok(ast_helpers::mload(
                            field_offset,
                            Rc::clone(&self.current_state),
                        ));
                    }
                }
                // Fallback: lower object and use Get for IR-level tuple access
                let obj_ir = self.lower_expr(obj)?;
                Ok(ast_helpers::get(obj_ir, *index as usize))
            }

            edge_ast::Expr::StructInstantiation(_, type_name, fields, _span) => {
                self.lower_struct_instantiation(&type_name.name, fields)
            }

            edge_ast::Expr::ArrayInstantiation(_, elements, _span) => {
                self.lower_array_instantiation(elements)
            }

            edge_ast::Expr::UnionInstantiation(type_name, variant_name, args, span) => self
                .lower_union_instantiation_expr(
                    &type_name.name,
                    &variant_name.name,
                    args,
                    Some(span),
                ),

            edge_ast::Expr::PatternMatch(expr, pattern, _span) => {
                self.lower_pattern_match(expr, pattern)
            }

            edge_ast::Expr::InlineAsm(inputs, outputs, ops, span) => {
                self.lower_inline_asm(inputs, outputs, ops, span)
            }

            edge_ast::Expr::Cast(expr, target_type, _span) => self.lower_cast(expr, target_type),

            // TODO: implement remaining expression types
            other => Err(IrError::Unsupported(format!(
                "Expression type not yet supported: {other:?}"
            ))),
        }
    }

    /// Lower a literal value.
    pub(crate) fn lower_literal(&self, lit: &edge_ast::Lit) -> Result<RcExpr, IrError> {
        match lit {
            edge_ast::Lit::Int(bytes, maybe_ty, _span) => {
                let ty = maybe_ty
                    .as_ref()
                    .map(|pt| self.lower_primitive_type(pt))
                    .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));
                // Check if value fits in SmallInt (first 24 bytes are zero and high bit of remaining 8 is not set)
                let is_small = bytes[..24].iter().all(|&b| b == 0) && (bytes[24] & 0x80) == 0;
                if is_small {
                    let mut val: u64 = 0;
                    for &b in &bytes[24..] {
                        val = (val << 8) | (b as u64);
                    }
                    Ok(Rc::new(EvmExpr::Const(
                        EvmConstant::SmallInt(val as i64),
                        ty,
                        self.current_ctx.clone(),
                    )))
                } else {
                    let hex_str: String = bytes
                        .iter()
                        .skip_while(|&&b| b == 0)
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    let hex_str = if hex_str.is_empty() {
                        "00".to_string()
                    } else {
                        hex_str
                    };
                    Ok(Rc::new(EvmExpr::Const(
                        EvmConstant::LargeInt(hex_str),
                        ty,
                        self.current_ctx.clone(),
                    )))
                }
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
    pub(crate) fn lower_ident(
        &self,
        name: &str,
        span: Option<&edge_types::span::Span>,
    ) -> Result<RcExpr, IrError> {
        // Search scopes from innermost to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                // Unit-typed storage fields return the slot constant directly.
                // They have no data to SLOAD — their value IS the slot number.
                // This enables types like Map<K,V> = () to work as slot references.
                // Note: () can lower as either Base(UnitT) or TupleT([]).
                let is_unit = matches!(binding._ty, EvmType::Base(EvmBaseType::UnitT))
                    || matches!(&binding._ty, EvmType::TupleT(v) if v.is_empty());
                if binding.storage_slot.is_some() && is_unit {
                    return Ok(ast_helpers::const_int(
                        binding.storage_slot.unwrap_or(0) as i64,
                        self.current_ctx.clone(),
                    ));
                }
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
        // Always emit a Diagnostic error — use span when available
        let diag = edge_diagnostics::Diagnostic::error(format!(
            "cannot find value `{name}` in this scope",
        ));
        let diag = if let Some(span) = span {
            diag.with_label(span.clone(), "not found in this scope")
        } else {
            diag
        };
        Err(IrError::Diagnostic(diag))
    }

    /// Lower an assignment expression.
    /// `rhs_composite` is set when the RHS was a struct/array instantiation, giving `(type_name, base_expr)`.
    pub(crate) fn lower_assignment_with_composite(
        &mut self,
        lhs: &edge_ast::Expr,
        rhs_ir: RcExpr,
        rhs_composite: Option<&(String, RcExpr)>,
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
                                // For packed struct storage fields: if the RHS was a struct
                                // instantiation (stored in memory), MLOAD the packed word
                                // from the memory base instead of storing the base address.
                                // We must keep rhs_ir in the result (it contains MSTOREs).
                                if let Some((_type_name, base_expr)) = rhs_composite {
                                    if binding.composite_type.is_some() {
                                        let packed_word = ast_helpers::mload(
                                            Rc::clone(base_expr),
                                            Rc::clone(&self.current_state),
                                        );
                                        let new_state = ast_helpers::sstore(
                                            slot,
                                            packed_word,
                                            Rc::clone(&self.current_state),
                                        );
                                        self.current_state = Rc::clone(&new_state);
                                        // Include RHS side effects (MSTOREs) before the SSTORE
                                        return Ok(ast_helpers::concat(rhs_ir, new_state));
                                    }
                                }
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
                                if let Some((_type_name, base_expr)) = rhs_composite {
                                    if binding.composite_type.is_some() {
                                        let packed_word = ast_helpers::mload(
                                            Rc::clone(base_expr),
                                            Rc::clone(&self.current_state),
                                        );
                                        let new_state = ast_helpers::tstore(
                                            slot,
                                            packed_word,
                                            Rc::clone(&self.current_state),
                                        );
                                        self.current_state = Rc::clone(&new_state);
                                        return Ok(ast_helpers::concat(rhs_ir, new_state));
                                    }
                                }
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
                Err(IrError::Diagnostic(
                    edge_diagnostics::Diagnostic::error(format!(
                        "cannot find value `{name}` in this scope",
                    ))
                    .with_label(ident.span.clone(), "not found in this scope"),
                ))
            }
            edge_ast::Expr::ArrayIndex(base, index, _end_index, _span) => {
                // Check storage array write first
                if let Some(result) = self.try_lower_storage_array_write(base, index, &rhs_ir)? {
                    return Ok(result);
                }
                if let Some(result) = self.try_lower_array_element_write(base, index, &rhs_ir)? {
                    return Ok(result);
                }
                // Index write dispatch is handled in the Assign branch above
                Err(IrError::Unsupported(
                    "array index write on non-array type; use Map<K,V>.set(key, val) for mappings"
                        .to_owned(),
                ))
            }
            edge_ast::Expr::FieldAccess(obj, field, _span) => {
                // Storage-backed packed struct sub-field write: self.color.r = 5
                if let edge_ast::Expr::Ident(ident) = obj.as_ref() {
                    if let Some(result) =
                        self.try_lower_storage_packed_field_write(&ident.name, &field.name, rhs_ir)?
                    {
                        return Ok(result);
                    }
                }
                Err(IrError::Unsupported(
                    "field access assignment target not yet supported".to_owned(),
                ))
            }
            _ => Err(IrError::Unsupported(
                "complex assignment target not yet supported".to_owned(),
            )),
        }
    }

    /// Lower a binary operator with type-aware operator selection and width truncation.
    ///
    /// For signed integer types (`IntT`), uses signed EVM opcodes (SDIV, SMOD, SLT, SGT, SAR).
    /// For sub-256-bit types, truncates results to the correct width after arithmetic.
    pub(crate) fn lower_binary_op(
        &self,
        op: &edge_ast::BinOp,
        lhs: RcExpr,
        rhs: RcExpr,
        operand_ty: &EvmType,
        span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        let is_signed = matches!(operand_ty, EvmType::Base(EvmBaseType::IntT(_)));
        let bit_width = match operand_ty {
            EvmType::Base(bt) => bt.bit_width(),
            _ => 256,
        };

        let ir_op = match op {
            edge_ast::BinOp::Add | edge_ast::BinOp::AddAssign => {
                if bit_width < 256 {
                    return self.width_checked_op(
                        EvmBinaryOp::Add,
                        lhs,
                        rhs,
                        bit_width,
                        is_signed,
                        span,
                    );
                }
                EvmBinaryOp::CheckedAdd
            }
            edge_ast::BinOp::Sub | edge_ast::BinOp::SubAssign => {
                if bit_width < 256 {
                    return self.width_checked_op(
                        EvmBinaryOp::Sub,
                        lhs,
                        rhs,
                        bit_width,
                        is_signed,
                        span,
                    );
                }
                EvmBinaryOp::CheckedSub
            }
            edge_ast::BinOp::Mul | edge_ast::BinOp::MulAssign => {
                if bit_width < 256 {
                    return self.width_checked_op(
                        EvmBinaryOp::Mul,
                        lhs,
                        rhs,
                        bit_width,
                        is_signed,
                        span,
                    );
                }
                EvmBinaryOp::CheckedMul
            }
            edge_ast::BinOp::Div | edge_ast::BinOp::DivAssign => {
                if is_signed {
                    EvmBinaryOp::SDiv
                } else {
                    EvmBinaryOp::Div
                }
            }
            edge_ast::BinOp::Mod | edge_ast::BinOp::ModAssign => {
                if is_signed {
                    EvmBinaryOp::SMod
                } else {
                    EvmBinaryOp::Mod
                }
            }
            edge_ast::BinOp::Exp | edge_ast::BinOp::ExpAssign => EvmBinaryOp::Exp,
            edge_ast::BinOp::BitwiseAnd | edge_ast::BinOp::BitwiseAndAssign => EvmBinaryOp::And,
            edge_ast::BinOp::BitwiseOr | edge_ast::BinOp::BitwiseOrAssign => EvmBinaryOp::Or,
            edge_ast::BinOp::BitwiseXor | edge_ast::BinOp::BitwiseXorAssign => EvmBinaryOp::Xor,
            edge_ast::BinOp::Shl | edge_ast::BinOp::ShlAssign => {
                // IR convention: Bop(Shl, shift_amount, value)
                // AST: value << shift → swap to (shift, value)
                let result = ast_helpers::bop(EvmBinaryOp::Shl, rhs, lhs);
                return Ok(self.truncate_to_width(result, bit_width, is_signed));
            }
            edge_ast::BinOp::Shr | edge_ast::BinOp::ShrAssign => {
                // IR convention: Bop(Shr, shift_amount, value)
                // AST: value >> shift → swap to (shift, value)
                // Signed: use SAR (arithmetic shift right)
                let shr_op = if is_signed {
                    EvmBinaryOp::Sar
                } else {
                    EvmBinaryOp::Shr
                };
                return Ok(ast_helpers::bop(shr_op, rhs, lhs));
            }
            edge_ast::BinOp::LogicalAnd => EvmBinaryOp::LogAnd,
            edge_ast::BinOp::LogicalOr => EvmBinaryOp::LogOr,
            edge_ast::BinOp::Eq => EvmBinaryOp::Eq,
            edge_ast::BinOp::Neq => {
                // a != b -> IsZero(Eq(a, b))
                let eq_expr = ast_helpers::eq(lhs, rhs);
                return Ok(ast_helpers::iszero(eq_expr));
            }
            edge_ast::BinOp::Lt => {
                if is_signed {
                    EvmBinaryOp::SLt
                } else {
                    EvmBinaryOp::Lt
                }
            }
            edge_ast::BinOp::Lte => {
                // a <= b -> IsZero(Gt(a, b))
                let gt_op = if is_signed {
                    EvmBinaryOp::SGt
                } else {
                    EvmBinaryOp::Gt
                };
                let gt_expr = ast_helpers::bop(gt_op, lhs, rhs);
                return Ok(ast_helpers::iszero(gt_expr));
            }
            edge_ast::BinOp::Gt => {
                if is_signed {
                    EvmBinaryOp::SGt
                } else {
                    EvmBinaryOp::Gt
                }
            }
            edge_ast::BinOp::Gte => {
                // a >= b -> IsZero(Lt(a, b))
                let lt_op = if is_signed {
                    EvmBinaryOp::SLt
                } else {
                    EvmBinaryOp::Lt
                };
                let lt_expr = ast_helpers::bop(lt_op, lhs, rhs);
                return Ok(ast_helpers::iszero(lt_expr));
            }
        };

        let result = ast_helpers::bop(ir_op, lhs, rhs);

        // Truncate arithmetic results to the correct width for sub-256-bit types.
        // Comparisons and logical ops return booleans — no truncation needed.
        let needs_truncation = matches!(
            op,
            edge_ast::BinOp::Add
                | edge_ast::BinOp::AddAssign
                | edge_ast::BinOp::Sub
                | edge_ast::BinOp::SubAssign
                | edge_ast::BinOp::Mul
                | edge_ast::BinOp::MulAssign
                | edge_ast::BinOp::Div
                | edge_ast::BinOp::DivAssign
                | edge_ast::BinOp::Mod
                | edge_ast::BinOp::ModAssign
                | edge_ast::BinOp::Exp
                | edge_ast::BinOp::ExpAssign
                | edge_ast::BinOp::BitwiseOr
                | edge_ast::BinOp::BitwiseOrAssign
                | edge_ast::BinOp::BitwiseXor
                | edge_ast::BinOp::BitwiseXorAssign
        );

        if needs_truncation {
            Ok(self.truncate_to_width(result, bit_width, is_signed))
        } else {
            Ok(result)
        }
    }

    /// Truncate a value to the given integer width.
    /// For unsigned: AND with mask. For signed: sign-extend. No-op for 256-bit.
    fn truncate_to_width(&self, value: RcExpr, bit_width: u16, is_signed: bool) -> RcExpr {
        if bit_width >= 256 {
            return value;
        }
        if is_signed {
            ast_helpers::sign_extend(value, bit_width, self.current_ctx.clone())
        } else {
            ast_helpers::mask_to_width(value, bit_width, self.current_ctx.clone())
        }
    }

    /// Emit a width-specific checked arithmetic operation for sub-256-bit types.
    ///
    /// For sub-256-bit integers, the u256-level `CheckedAdd`/`CheckedSub`/`CheckedMul`
    /// never trigger (inputs are too small to overflow u256). Instead, we:
    /// 1. Compute the full u256 result with an unchecked op
    /// 2. Truncate to the target width (mask for unsigned, sign-extend for signed)
    /// 3. Compare truncated vs full — if different, the result overflowed the width
    /// 4. Revert on overflow
    fn width_checked_op(
        &self,
        op: EvmBinaryOp,
        lhs: RcExpr,
        rhs: RcExpr,
        bit_width: u16,
        is_signed: bool,
        span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        // Compile-time overflow detection: if both operands are constants,
        // compute the result and check if it fits in the target width.
        if let (Some(lv), Some(rv)) = (const_value(&lhs), const_value(&rhs)) {
            let result = match op {
                EvmBinaryOp::Add => lv.wrapping_add(rv),
                EvmBinaryOp::Sub => lv.wrapping_sub(rv),
                EvmBinaryOp::Mul => lv.wrapping_mul(rv),
                _ => lv, // won't happen for checked ops
            };
            let max_val = if bit_width == 0 {
                ruint::aliases::U256::ZERO
            } else {
                (ruint::aliases::U256::from(1u64) << bit_width) - ruint::aliases::U256::from(1u64)
            };
            if result > max_val {
                let op_name = match op {
                    EvmBinaryOp::Add => "+",
                    EvmBinaryOp::Sub => "-",
                    EvmBinaryOp::Mul => "*",
                    _ => "?",
                };
                let ty_name = if is_signed {
                    format!("i{bit_width}")
                } else {
                    format!("u{bit_width}")
                };
                return Err(IrError::Diagnostic(
                    edge_diagnostics::Diagnostic::error(format!(
                        "attempt to compute `{lv}_{ty_name} {op_name} {rv}_{ty_name}`, which would overflow"
                    ))
                    .with_label(span.clone(), format!("overflows `{ty_name}` (max {max_val})"))
                ));
            }
        }

        let ctx = self.current_ctx.clone();

        // Step 1: unchecked full-width result
        let full = ast_helpers::bop(op, lhs, rhs);

        // Step 2: truncate to target width
        let truncated = if is_signed {
            ast_helpers::sign_extend(Rc::clone(&full), bit_width, ctx.clone())
        } else {
            ast_helpers::mask_to_width(Rc::clone(&full), bit_width, ctx.clone())
        };

        // Step 3: overflow check — if truncated != full, width overflow occurred
        let is_overflow = ast_helpers::iszero(ast_helpers::eq(Rc::clone(&truncated), full));

        // Step 4: revert on overflow, otherwise return truncated value
        let empty = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
        let revert_expr = ast_helpers::revert(
            ast_helpers::const_int(0, ctx.clone()),
            ast_helpers::const_int(0, ctx),
            Rc::clone(&empty),
        );

        // If(overflow?, inputs, then=revert, else=truncated_value)
        Ok(ast_helpers::if_then_else(
            is_overflow,
            empty,
            revert_expr,
            truncated,
        ))
    }

    /// Lower a type cast: `expr as TargetType`.
    ///
    /// Casts between integer types:
    /// - Narrowing unsigned: mask to target width
    /// - Widening unsigned: no-op (already zero-extended in u256)
    /// - Narrowing signed: mask then sign-extend to target width
    /// - Widening signed → unsigned: mask to remove sign extension
    /// - Unsigned → signed: sign-extend to target width
    /// - Same width: no-op (or reinterpret signedness)
    fn lower_cast(
        &mut self,
        expr: &edge_ast::Expr,
        target_type: &edge_ast::ty::TypeSig,
    ) -> Result<RcExpr, IrError> {
        let val = self.lower_expr(expr)?;
        let target_evm_ty = self.lower_type_sig(target_type);
        let ctx = self.current_ctx.clone();

        let target_bits = match &target_evm_ty {
            EvmType::Base(bt) => bt.bit_width(),
            _ => return Ok(val), // Non-integer cast: no-op
        };
        let target_signed = matches!(target_evm_ty, EvmType::Base(EvmBaseType::IntT(_)));

        if target_signed {
            // Cast to signed: sign-extend from target width
            // First mask to target width, then sign-extend
            let masked = ast_helpers::mask_to_width(val, target_bits, ctx.clone());
            Ok(ast_helpers::sign_extend(masked, target_bits, ctx))
        } else {
            // Cast to unsigned: mask to target width
            Ok(ast_helpers::mask_to_width(val, target_bits, ctx))
        }
    }

    /// Map a binary operator to its corresponding trait name and method.
    /// Returns None for operators that don't have trait overloading.
    const fn op_to_trait(op: &edge_ast::BinOp) -> Option<(&'static str, &'static str)> {
        match op {
            edge_ast::BinOp::Add | edge_ast::BinOp::AddAssign => Some(("Add", "add")),
            edge_ast::BinOp::Sub | edge_ast::BinOp::SubAssign => Some(("Sub", "sub")),
            edge_ast::BinOp::Mul | edge_ast::BinOp::MulAssign => Some(("Mul", "mul")),
            edge_ast::BinOp::Div | edge_ast::BinOp::DivAssign => Some(("Div", "div")),
            edge_ast::BinOp::Mod | edge_ast::BinOp::ModAssign => Some(("Mod", "mod_")),
            edge_ast::BinOp::Eq => Some(("Eq", "eq")),
            edge_ast::BinOp::Neq => Some(("Eq", "ne")),
            edge_ast::BinOp::Lt => Some(("Ord", "lt")),
            edge_ast::BinOp::Lte => Some(("Ord", "le")),
            edge_ast::BinOp::Gt => Some(("Ord", "gt")),
            edge_ast::BinOp::Gte => Some(("Ord", "ge")),
            _ => None,
        }
    }

    /// Try to resolve a binary operator as a trait method call on user-defined types.
    /// Returns `Ok(Some(expr))` if overloaded, `Ok(None)` if primitive.
    ///
    /// Only dispatches to operator traits from `std::ops` (Add, Sub, Mul, etc.).
    /// The trait must be imported (`use std::ops::Add;`) and implemented for the type.
    fn try_operator_overload(
        &mut self,
        lhs: &edge_ast::Expr,
        op: &edge_ast::BinOp,
        rhs: &edge_ast::Expr,
        span: &edge_types::span::Span,
    ) -> Result<Option<RcExpr>, IrError> {
        let (trait_name, method_name) = match Self::op_to_trait(op) {
            Some(pair) => pair,
            None => return Ok(None),
        };

        // Check if the LHS is a user-defined type (skip primitives — they use built-in ops)
        let lhs_type = self.infer_receiver_type(lhs);
        if let Some(ref type_name) = lhs_type {
            if Self::is_primitive_type(type_name) {
                return Ok(None);
            }
            // Only dispatch to operator traits from std::ops.
            // User-defined traits named "Add" etc. do NOT get operator overloading.
            if !self.std_ops_traits.contains(trait_name) {
                let op_sym = Self::op_symbol(op);
                return Err(IrError::Diagnostic(
                    edge_diagnostics::Diagnostic::error(format!(
                        "cannot apply operator `{op_sym}` to type `{type_name}`",
                    ))
                    .with_label(
                        span.clone(),
                        format!("no implementation for `{type_name} {op_sym} {type_name}`"),
                    )
                    .with_note(format!(
                        "the trait `std::ops::{trait_name}` is not imported; add `use std::ops::{trait_name};` and implement it for `{type_name}`",
                    )),
                ));
            }

            // Look up trait impl for this type
            if let Some((fn_decl, body)) =
                self.find_trait_impl_method(type_name, trait_name, method_name)
            {
                let params: Vec<(String, edge_ast::ty::TypeSig)> = fn_decl
                    .params
                    .iter()
                    .map(|(id, ty)| (id.name.clone(), ty.clone()))
                    .collect();
                let result =
                    self.inline_function_call(&params, &body, &[lhs.clone(), rhs.clone()])?;
                return Ok(Some(result));
            }
            // std::ops trait is imported but type doesn't implement it
            let op_sym = Self::op_symbol(op);
            return Err(IrError::Diagnostic(
                edge_diagnostics::Diagnostic::error(format!(
                    "cannot apply operator `{op_sym}` to type `{type_name}`",
                ))
                .with_label(
                    span.clone(),
                    format!("no implementation for `{type_name} {op_sym} {type_name}`"),
                )
                .with_note(format!(
                    "an implementation of `std::ops::{trait_name}` might be missing for `{type_name}`",
                )),
            ));
        }

        // Primitive types — use built-in ops
        Ok(None)
    }

    /// Human-readable symbol for an operator (for error messages).
    const fn op_symbol(op: &edge_ast::BinOp) -> &'static str {
        match op {
            edge_ast::BinOp::Add | edge_ast::BinOp::AddAssign => "+",
            edge_ast::BinOp::Sub | edge_ast::BinOp::SubAssign => "-",
            edge_ast::BinOp::Mul | edge_ast::BinOp::MulAssign => "*",
            edge_ast::BinOp::Div | edge_ast::BinOp::DivAssign => "/",
            edge_ast::BinOp::Mod | edge_ast::BinOp::ModAssign => "%",
            edge_ast::BinOp::Eq => "==",
            edge_ast::BinOp::Neq => "!=",
            edge_ast::BinOp::Lt => "<",
            edge_ast::BinOp::Lte => "<=",
            edge_ast::BinOp::Gt => ">",
            edge_ast::BinOp::Gte => ">=",
            _ => "?",
        }
    }

    /// Lower a unary operator with type-aware width truncation.
    pub(crate) fn lower_unary_op(
        &self,
        op: &edge_ast::UnaryOp,
        expr: RcExpr,
        operand_ty: &EvmType,
    ) -> Result<RcExpr, IrError> {
        let is_signed = matches!(operand_ty, EvmType::Base(EvmBaseType::IntT(_)));
        let bit_width = match operand_ty {
            EvmType::Base(bt) => bt.bit_width(),
            _ => 256,
        };

        let ir_op = match op {
            edge_ast::UnaryOp::Neg => EvmUnaryOp::Neg,
            edge_ast::UnaryOp::BitwiseNot => EvmUnaryOp::Not,
            edge_ast::UnaryOp::LogicalNot => return Ok(ast_helpers::iszero(expr)),
        };
        let result = ast_helpers::uop(ir_op, expr);

        // Neg and Not can produce values outside the width — truncate
        Ok(self.truncate_to_width(result, bit_width, is_signed))
    }

    /// Resolve a multi-component path to a name.
    ///
    /// - If the first component is a known module prefix (from `use std::math`),
    ///   returns the last component (e.g., `math::mul_div_down` → `mul_div_down`).
    /// - Otherwise, errors on partially qualified paths.
    pub(crate) fn resolve_path_to_name(
        &self,
        components: &[edge_ast::Ident],
    ) -> Result<String, IrError> {
        if components.len() == 1 {
            return Ok(components[0].name.clone());
        }

        // Check if the first component is a known module prefix.
        if components.len() == 2 && self.module_prefixes.contains(&components[0].name) {
            return Ok(components[1].name.clone());
        }

        let path_str = components
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        let last = &components.last().unwrap().name;
        // Span the full path from first to last component
        let full_span = edge_types::span::Span {
            start: components.first().unwrap().span.start,
            end: components.last().unwrap().span.end,
            file: components.first().unwrap().span.file.clone(),
        };
        Err(IrError::Diagnostic(
            edge_diagnostics::Diagnostic::error(format!("unresolved path `{path_str}`",))
                .with_label(full_span, "not found in this scope")
                .with_note(format!(
                    "use the unqualified name `{last}` with a `use` import instead",
                )),
        ))
    }

    /// Lower a builtin call (@caller, @callvalue, etc.).
    pub(crate) fn lower_builtin(
        &self,
        name: &str,
        _args: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
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

    /// Lower `return (a, b, c)` — MSTORE each element at sequential 32-byte
    /// offsets, then RETURN the entire memory range.
    ///
    /// Array-typed elements (storage or memory-backed) are expanded inline:
    /// each array element gets its own 32-byte return slot.
    pub(crate) fn lower_tuple_return(
        &mut self,
        elements: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        // Pre-compute total return slots needed (arrays expand to multiple slots)
        let mut total_slots = 0usize;
        for elem in elements {
            if let edge_ast::Expr::Ident(ident) = elem {
                if let Some(&(_, len)) = self.storage_array_fields.get(&ident.name) {
                    total_slots += len;
                    continue;
                }
                if let Some((type_name, _)) = self.lookup_composite_info(&ident.name) {
                    if let Some(len_str) = type_name.strip_prefix("__array__") {
                        if let Ok(len) = len_str.parse::<usize>() {
                            total_slots += len;
                            continue;
                        }
                    }
                }
            }
            total_slots += 1;
        }

        // Allocate a return buffer region
        let ret_base = self.alloc_region(total_slots);

        let mut result =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        let mut slot = 0usize;

        for elem in elements {
            // Check if this element is a storage array identifier
            if let edge_ast::Expr::Ident(ident) = elem {
                if let Some(&(base_slot, len)) = self.storage_array_fields.get(&ident.name) {
                    // Expand storage array: SLOAD each element into a return slot
                    for j in 0..len {
                        let arr_slot = ast_helpers::const_int(
                            (base_slot + j) as i64,
                            self.current_ctx.clone(),
                        );
                        let val = ast_helpers::sload(arr_slot, Rc::clone(&self.current_state));
                        let offset = ast_helpers::add(
                            Rc::clone(&ret_base),
                            ast_helpers::const_int((slot * 32) as i64, self.current_ctx.clone()),
                        );
                        let mstore =
                            ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
                        self.current_state = Rc::clone(&mstore);
                        result = ast_helpers::concat(result, mstore);
                        slot += 1;
                    }
                    continue;
                }

                // Check if this element is a memory-backed array variable
                if let Some((type_name, base_expr)) = self.lookup_composite_info(&ident.name) {
                    if let Some(len_str) = type_name.strip_prefix("__array__") {
                        if let Ok(len) = len_str.parse::<usize>() {
                            // Use MCOPY to bulk-copy array data to return offset
                            let dest = ast_helpers::add(
                                Rc::clone(&ret_base),
                                ast_helpers::const_int(
                                    (slot * 32) as i64,
                                    self.current_ctx.clone(),
                                ),
                            );
                            let size =
                                ast_helpers::const_int((len * 32) as i64, self.current_ctx.clone());
                            let copy = ast_helpers::mcopy(dest, base_expr, size);
                            result = ast_helpers::concat(result, copy);
                            slot += len;
                            continue;
                        }
                    }
                }
            }

            // Scalar element: single MSTORE
            let val = self.lower_expr(elem)?;
            let offset = ast_helpers::add(
                Rc::clone(&ret_base),
                ast_helpers::const_int((slot * 32) as i64, self.current_ctx.clone()),
            );
            let mstore = ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&mstore);
            result = ast_helpers::concat(result, mstore);
            slot += 1;
        }

        let size = ast_helpers::const_int((slot * 32) as i64, self.current_ctx.clone());
        let ret = ast_helpers::return_op(ret_base, size, Rc::clone(&self.current_state));
        result = ast_helpers::concat(result, ret);
        Ok(result)
    }

    /// Lower inline assembly: `asm(inputs...) -> (outputs...) { opcodes... }`
    ///
    /// Encoding strategy:
    /// - Input expressions are lowered and stored as children of `InlineAsm`
    /// - Asm body ops are encoded to raw bytecode
    /// - For multiple outputs, MSTORE/POP ops are appended to bytecode to capture outputs to memory
    /// - Named outputs become `LetBind` variables (read via `Var`/MLOAD)
    /// - `_` outputs are discarded (`POP` in bytecode)
    pub(crate) fn lower_inline_asm(
        &mut self,
        inputs: &[edge_ast::Expr],
        outputs: &[Option<edge_ast::Ident>],
        ops: &[edge_ast::AsmOp],
        _span: &edge_types::span::Span,
    ) -> Result<RcExpr, IrError> {
        let num_outputs = outputs.len();

        // 1. Encode asm body to bytecode
        let mut bytecode = Vec::new();
        for op in ops {
            match op {
                edge_ast::AsmOp::Opcode(name, span) => {
                    let byte =
                        opcode_name_to_byte(name).ok_or_else(|| IrError::LoweringSpanned {
                            message: format!("unknown EVM opcode: {name}"),
                            span: span.clone(),
                        })?;
                    bytecode.push(byte);
                }
                edge_ast::AsmOp::Literal(val_str, span) => {
                    let bytes =
                        parse_asm_literal(val_str).ok_or_else(|| IrError::LoweringSpanned {
                            message: format!("invalid asm literal: {val_str}"),
                            span: span.clone(),
                        })?;
                    if bytes.is_empty() || bytes.len() > 32 {
                        return Err(IrError::LoweringSpanned {
                            message: format!("asm literal must be 1-32 bytes, got {}", bytes.len()),
                            span: span.clone(),
                        });
                    }
                    // Emit PUSHn + bytes (PUSH1=0x60, PUSH2=0x61, ...)
                    bytecode.push(0x5f + bytes.len() as u8);
                    bytecode.extend_from_slice(&bytes);
                }
                edge_ast::AsmOp::Ident(name, span) => {
                    // Check if it's a known constant — replace with PUSH
                    let mut found = false;
                    for scope in self.scopes.iter().rev() {
                        if let Some(binding) = scope.bindings.get(name.as_str()) {
                            // If it's a compile-time constant (no let_bind_name), inline its value
                            if binding.let_bind_name.is_none() {
                                if let EvmExpr::Const(EvmConstant::SmallInt(val), _, _) =
                                    binding.value.as_ref()
                                {
                                    let bytes = int_to_minimal_bytes(*val as u64);
                                    bytecode.push(0x5f + bytes.len() as u8);
                                    bytecode.extend_from_slice(&bytes);
                                    found = true;
                                    break;
                                }
                            }
                            // Memory-backed variable — can't directly reference in raw asm.
                            // The user would need to pass it as an input instead.
                            return Err(IrError::LoweringSpanned {
                                message: format!(
                                    "variable `{name}` cannot be used directly in asm body; \
                                     pass it as an input argument instead"
                                ),
                                span: span.clone(),
                            });
                        }
                    }
                    if !found {
                        // Treat as ad-hoc opcode name (case-insensitive)
                        let upper = name.to_uppercase();
                        let byte = opcode_name_to_byte(&upper).ok_or_else(|| {
                            IrError::LoweringSpanned {
                                message: format!(
                                    "unknown identifier `{name}` in asm block \
                                     (not a variable, constant, or EVM opcode)"
                                ),
                                span: span.clone(),
                            }
                        })?;
                        bytecode.push(byte);
                    }
                }
            }
        }

        // 3. Handle outputs
        // After asm body executes, stack has `num_outputs` values (TOS = first output).
        // For 0-1 outputs, we can use the simple encoding.
        // For N>1 outputs, append MSTORE/POP to bytecode to capture outputs to memory.

        // Collect lowered input expressions in reverse order so that codegen's
        // forward push places the first asm arg on TOS (EVM operand convention).
        let mut input_exprs: Vec<RcExpr> = Vec::new();
        for input_expr in inputs.iter().rev() {
            input_exprs.push(self.lower_expr(input_expr)?);
        }

        if num_outputs <= 1 {
            // Simple case: 0 or 1 output
            let hex = bytes_to_hex(&bytecode);
            let asm_node = Rc::new(EvmExpr::InlineAsm(input_exprs, hex, num_outputs as i32));
            Ok(asm_node)
        } else {
            // Multiple outputs: allocate memory slots and append MSTORE/POP to bytecode
            // InlineAsm multi-output: offsets are embedded directly in raw bytecode hex,
            // so we can't use symbolic MemRegion here. Allocate a region to reserve the
            // space, then use the region ID to derive deterministic concrete offsets.
            // The post-egglog mem_region pass won't touch raw bytecode, but reserving
            // the region prevents other allocations from overlapping.
            let num_named = outputs.iter().filter(|o| o.is_some()).count();
            let _asm_region = self.alloc_region(num_named);
            // Use high concrete offsets based on region ID to avoid conflicts.
            // These will be consistent because region IDs are monotonically assigned.
            let asm_base = if let crate::schema::EvmExpr::MemRegion(id, _) = _asm_region.as_ref() {
                // Place at 0xF000 + id * num_named * 32 to avoid overlapping
                0xF000 + (*id as usize) * num_named * 32
            } else {
                0xF000
            };
            let mut output_offsets: Vec<Option<usize>> = Vec::new();
            let mut asm_slot = 0;
            for output in outputs {
                if output.is_some() {
                    output_offsets.push(Some(asm_base + asm_slot * 32));
                    asm_slot += 1;
                } else {
                    output_offsets.push(None);
                }
            }

            // Append MSTORE/POP for each output (TOS = outputs[0])
            for offset_opt in &output_offsets {
                if let Some(offset) = offset_opt {
                    // PUSH2 <offset> MSTORE  (stores TOS to memory[offset])
                    let offset_bytes = (*offset as u16).to_be_bytes();
                    bytecode.push(0x61); // PUSH2
                    bytecode.extend_from_slice(&offset_bytes);
                    bytecode.push(0x52); // MSTORE
                } else {
                    // Discard with POP
                    bytecode.push(0x50); // POP
                }
            }

            // All outputs consumed from stack by MSTORE/POP, num_outputs=0
            let hex = bytes_to_hex(&bytecode);
            let asm_node = Rc::new(EvmExpr::InlineAsm(input_exprs, hex, 0));

            let mut result = asm_node;

            // Create LetBind variables for named outputs (wrap from outside in)
            // We need to wrap in LetBind/Drop for each named output.
            // The body of the outermost LetBind is the whole expression + drops.
            // But we need to return a value — use the first named output's Var as the "result" for now,
            // or use a tuple-like memory layout.

            // Collect named outputs with their memory offsets
            let mut named_outputs: Vec<(String, usize)> = Vec::new();
            for (output, offset_opt) in outputs.iter().zip(output_offsets.iter()) {
                if let (Some(ident), Some(offset)) = (output, offset_opt) {
                    let var_name = format!("{}__local_{}", self.inline_prefix, ident.name);
                    named_outputs.push((var_name, *offset));
                }
            }

            // Register bindings in current scope so the outputs are accessible
            for (var_name, _offset) in &named_outputs {
                let binding = VarBinding {
                    value: ast_helpers::const_int(0, self.current_ctx.clone()),
                    location: DataLocation::Memory,
                    storage_slot: None,
                    _ty: EvmType::Base(EvmBaseType::UIntT(256)),
                    let_bind_name: Some(var_name.clone()),
                    composite_type: None,
                    composite_base: None,
                    composite_type_args: Vec::new(),
                };
                // Get the original name (without prefix) for scope lookup
                let orig_name = outputs
                    .iter()
                    .zip(output_offsets.iter())
                    .find_map(|(o, off_opt)| {
                        if let (Some(ident), Some(_)) = (o, off_opt) {
                            let vn = format!("{}__local_{}", self.inline_prefix, ident.name);
                            if &vn == var_name {
                                Some(ident.name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(orig_name, binding);
            }

            // Wrap with LetBind for each named output.
            // The LetBind value is the MLOAD from the pre-allocated memory slot.
            // Wrap innermost-first: the innermost LetBind's body is `result`.
            // We do NOT add Drops here — the outputs are used by subsequent code
            // and will be dropped when the enclosing scope ends.
            for (var_name, offset) in named_outputs.iter().rev() {
                let mload_val = ast_helpers::mload(
                    ast_helpers::const_int(*offset as i64, self.current_ctx.clone()),
                    Rc::clone(&self.current_state),
                );
                result = ast_helpers::let_bind(var_name.clone(), mload_val, result);
            }

            Ok(result)
        }
    }
}

/// Map an EVM opcode name (uppercase) to its byte value.
fn opcode_name_to_byte(name: &str) -> Option<u8> {
    Some(match name {
        "STOP" => 0x00,
        "ADD" => 0x01,
        "MUL" => 0x02,
        "SUB" => 0x03,
        "DIV" => 0x04,
        "SDIV" => 0x05,
        "MOD" => 0x06,
        "SMOD" => 0x07,
        "ADDMOD" => 0x08,
        "MULMOD" => 0x09,
        "EXP" => 0x0a,
        "SIGNEXTEND" => 0x0b,
        "LT" => 0x10,
        "GT" => 0x11,
        "SLT" => 0x12,
        "SGT" => 0x13,
        "EQ" => 0x14,
        "ISZERO" => 0x15,
        "AND" => 0x16,
        "OR" => 0x17,
        "XOR" => 0x18,
        "NOT" => 0x19,
        "BYTE" => 0x1a,
        "SHL" => 0x1b,
        "SHR" => 0x1c,
        "SAR" => 0x1d,
        "KECCAK256" | "SHA3" => 0x20,
        "ADDRESS" => 0x30,
        "BALANCE" => 0x31,
        "ORIGIN" => 0x32,
        "CALLER" => 0x33,
        "CALLVALUE" => 0x34,
        "CALLDATALOAD" => 0x35,
        "CALLDATASIZE" => 0x36,
        "CALLDATACOPY" => 0x37,
        "CODESIZE" => 0x38,
        "CODECOPY" => 0x39,
        "GASPRICE" => 0x3a,
        "EXTCODESIZE" => 0x3b,
        "EXTCODECOPY" => 0x3c,
        "RETURNDATASIZE" => 0x3d,
        "RETURNDATACOPY" => 0x3e,
        "EXTCODEHASH" => 0x3f,
        "BLOCKHASH" => 0x40,
        "COINBASE" => 0x41,
        "TIMESTAMP" => 0x42,
        "NUMBER" => 0x43,
        "PREVRANDAO" | "DIFFICULTY" => 0x44,
        "GASLIMIT" => 0x45,
        "CHAINID" => 0x46,
        "SELFBALANCE" => 0x47,
        "BASEFEE" => 0x48,
        "BLOBHASH" => 0x49,
        "BLOBBASEFEE" => 0x4a,
        "POP" => 0x50,
        "MLOAD" => 0x51,
        "MSTORE" => 0x52,
        "MSTORE8" => 0x53,
        "SLOAD" => 0x54,
        "SSTORE" => 0x55,
        "JUMP" => 0x56,
        "JUMPI" => 0x57,
        "PC" => 0x58,
        "MSIZE" => 0x59,
        "GAS" => 0x5a,
        "JUMPDEST" => 0x5b,
        "TLOAD" => 0x5c,
        "TSTORE" => 0x5d,
        "MCOPY" => 0x5e,
        "PUSH0" => 0x5f,
        "PUSH1" => 0x60,
        "PUSH2" => 0x61,
        "PUSH3" => 0x62,
        "PUSH4" => 0x63,
        "PUSH5" => 0x64,
        "PUSH6" => 0x65,
        "PUSH7" => 0x66,
        "PUSH8" => 0x67,
        "PUSH9" => 0x68,
        "PUSH10" => 0x69,
        "PUSH11" => 0x6a,
        "PUSH12" => 0x6b,
        "PUSH13" => 0x6c,
        "PUSH14" => 0x6d,
        "PUSH15" => 0x6e,
        "PUSH16" => 0x6f,
        "PUSH17" => 0x70,
        "PUSH18" => 0x71,
        "PUSH19" => 0x72,
        "PUSH20" => 0x73,
        "PUSH21" => 0x74,
        "PUSH22" => 0x75,
        "PUSH23" => 0x76,
        "PUSH24" => 0x77,
        "PUSH25" => 0x78,
        "PUSH26" => 0x79,
        "PUSH27" => 0x7a,
        "PUSH28" => 0x7b,
        "PUSH29" => 0x7c,
        "PUSH30" => 0x7d,
        "PUSH31" => 0x7e,
        "PUSH32" => 0x7f,
        "DUP1" => 0x80,
        "DUP2" => 0x81,
        "DUP3" => 0x82,
        "DUP4" => 0x83,
        "DUP5" => 0x84,
        "DUP6" => 0x85,
        "DUP7" => 0x86,
        "DUP8" => 0x87,
        "DUP9" => 0x88,
        "DUP10" => 0x89,
        "DUP11" => 0x8a,
        "DUP12" => 0x8b,
        "DUP13" => 0x8c,
        "DUP14" => 0x8d,
        "DUP15" => 0x8e,
        "DUP16" => 0x8f,
        "SWAP1" => 0x90,
        "SWAP2" => 0x91,
        "SWAP3" => 0x92,
        "SWAP4" => 0x93,
        "SWAP5" => 0x94,
        "SWAP6" => 0x95,
        "SWAP7" => 0x96,
        "SWAP8" => 0x97,
        "SWAP9" => 0x98,
        "SWAP10" => 0x99,
        "SWAP11" => 0x9a,
        "SWAP12" => 0x9b,
        "SWAP13" => 0x9c,
        "SWAP14" => 0x9d,
        "SWAP15" => 0x9e,
        "SWAP16" => 0x9f,
        "LOG0" => 0xa0,
        "LOG1" => 0xa1,
        "LOG2" => 0xa2,
        "LOG3" => 0xa3,
        "LOG4" => 0xa4,
        "CREATE" => 0xf0,
        "CALL" => 0xf1,
        "CALLCODE" => 0xf2,
        "RETURN" => 0xf3,
        "DELEGATECALL" => 0xf4,
        "CREATE2" => 0xf5,
        "STATICCALL" => 0xfa,
        "REVERT" => 0xfd,
        "INVALID" => 0xfe,
        "SELFDESTRUCT" => 0xff,
        _ => return None,
    })
}

/// Convert bytes to hex string.
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Convert an integer to minimal big-endian bytes (at least 1 byte).
fn int_to_minimal_bytes(val: u64) -> Vec<u8> {
    if val == 0 {
        return vec![0];
    }
    let bytes = val.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[start..].to_vec()
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Parse an asm literal value string into bytes.
/// Supports hex (0x...) and decimal formats.
fn parse_asm_literal(s: &str) -> Option<Vec<u8>> {
    if let Some(hex_str) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        // Hex literal — decode to bytes
        let hex_str = if hex_str.len() % 2 != 0 {
            format!("0{hex_str}")
        } else {
            hex_str.to_string()
        };
        decode_hex(&hex_str)
    } else {
        // Decimal literal
        let val: u64 = s.parse().ok()?;
        if val == 0 {
            Some(vec![0])
        } else {
            // Encode as minimal big-endian bytes
            let bytes = val.to_be_bytes();
            let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
            Some(bytes[start..].to_vec())
        }
    }
}
