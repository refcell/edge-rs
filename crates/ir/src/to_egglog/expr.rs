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

impl AstToEgglog {
    /// Lower a statement.
    pub(crate) fn lower_stmt(&mut self, stmt: &edge_ast::Stmt) -> Result<RcExpr, IrError> {
        match stmt {
            edge_ast::Stmt::VarDecl(ident, type_sig, _span) => {
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
                        if let Ok(Some(mangled)) =
                            self.try_monomorphize_named_type(&name_ident.name, type_args)
                        {
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
                    let_bind_name: Some(var_name),
                    composite_type,
                    composite_base: None,
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
                if let Some((ref type_name, base)) = self.last_composite_alloc.clone() {
                    if let edge_ast::Expr::Ident(ident) = lhs {
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(binding) = scope.bindings.get_mut(&ident.name) {
                                binding.composite_type = Some(type_name.clone());
                                binding.composite_base = Some(base);
                                break;
                            }
                        }
                    }
                }
                self.last_composite_alloc = None;
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
                    composite_type: None,
                    composite_base: None,
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
                        let offset = ast_helpers::const_int(0, self.current_ctx.clone());
                        let size = ast_helpers::const_int(32, self.current_ctx.clone());
                        let mstore_expr = ast_helpers::mstore(
                            Rc::clone(&offset),
                            val,
                            Rc::clone(&self.current_state),
                        );
                        self.current_state = Rc::clone(&mstore_expr);
                        let ret =
                            ast_helpers::return_op(offset, size, Rc::clone(&self.current_state));
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

            edge_ast::Expr::FunctionCall(callee, args, type_args, _span) => {
                self.lower_function_call(callee, args, type_args)
            }

            edge_ast::Expr::At(builtin_name, args, _span) => {
                self.lower_builtin(&builtin_name.name, args)
            }

            edge_ast::Expr::Assign(lhs, rhs, _span) => {
                // Clear composite tracking before evaluating RHS
                self.last_composite_alloc = None;
                let rhs_ir = self.lower_expr(rhs)?;
                // If RHS was a struct/array instantiation, wire composite info to LHS binding
                if let Some((ref type_name, base)) = self.last_composite_alloc.clone() {
                    if let edge_ast::Expr::Ident(ident) = lhs.as_ref() {
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(binding) = scope.bindings.get_mut(&ident.name) {
                                binding.composite_type = Some(type_name.clone());
                                binding.composite_base = Some(base);
                                break;
                            }
                        }
                    }
                }
                self.last_composite_alloc = None;
                self.lower_assignment(lhs, rhs_ir)
            }

            edge_ast::Expr::ArrayIndex(base, index, end_index, _span) => {
                // Slice access: arr[start:end] → pointer to base + start * 32
                if end_index.is_some() {
                    if let edge_ast::Expr::Ident(ident) = base.as_ref() {
                        if let Some((_type_name, base_offset)) =
                            self.lookup_composite_info(&ident.name)
                        {
                            // Evaluate start index (must be a constant for now)
                            if let edge_ast::Expr::Literal(lit) = index.as_ref() {
                                if let edge_ast::Lit::Int(start, _, _) = lit.as_ref() {
                                    let new_base = base_offset + (*start as usize) * 32;
                                    self.last_composite_alloc =
                                        Some(("__array__".to_string(), new_base));
                                    return Ok(ast_helpers::const_int(
                                        new_base as i64,
                                        self.current_ctx.clone(),
                                    ));
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
                self.try_lower_array_element_read(base, index)?
                    .map_or_else(|| self.lower_mapping_read(base, index), Ok)
            }

            edge_ast::Expr::Paren(inner, _span) => self.lower_expr(inner),

            edge_ast::Expr::FieldAccess(obj, field, _span) => {
                self.lower_field_access(obj, &field.name)
            }

            edge_ast::Expr::Path(components, _span) => {
                // Check if this is a union variant path like Direction::North
                if components.len() == 2 {
                    let type_name = &components[0].name;
                    let variant_name = &components[1].name;
                    if self.union_types.contains_key(type_name) {
                        return self.lower_union_instantiation_expr(type_name, variant_name, &[]);
                    }
                    // Check for generic union types (e.g., Option::None where Option<T> was monomorphized)
                    if self.generic_type_templates.contains_key(type_name) {
                        if let Some(mangled) = self.resolve_generic_type_name(type_name) {
                            return self.lower_union_instantiation_expr(
                                &mangled,
                                variant_name,
                                &[],
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
                let base = self.next_memory_offset;
                self.next_memory_offset += elements.len() * 32;
                let mut result =
                    ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
                for (i, elem) in elements.iter().enumerate() {
                    let val = self.lower_expr(elem)?;
                    let offset =
                        ast_helpers::const_int((base + i * 32) as i64, self.current_ctx.clone());
                    let mstore = ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
                    result = ast_helpers::concat(result, mstore);
                }
                // Track as composite for field access
                self.last_composite_alloc = Some(("__tuple".to_string(), base));
                // Return base address as the tuple "value"
                let base_val = ast_helpers::const_int(base as i64, self.current_ctx.clone());
                Ok(ast_helpers::concat(result, base_val))
            }

            edge_ast::Expr::TupleFieldAccess(obj, index, _span) => {
                // Check if obj is a variable with composite info
                if let edge_ast::Expr::Ident(ident) = obj.as_ref() {
                    if let Some((_type_name, base_offset)) = self.lookup_composite_info(&ident.name)
                    {
                        let field_offset = ast_helpers::const_int(
                            (base_offset + (*index as usize) * 32) as i64,
                            self.current_ctx.clone(),
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

            edge_ast::Expr::UnionInstantiation(type_name, variant_name, args, _span) => {
                self.lower_union_instantiation_expr(&type_name.name, &variant_name.name, args)
            }

            edge_ast::Expr::PatternMatch(expr, pattern, _span) => {
                self.lower_pattern_match(expr, pattern)
            }

            // TODO: implement remaining expression types
            other => Err(IrError::Unsupported(format!(
                "Expression type not yet supported: {other:?}"
            ))),
        }
    }

    /// Lower a literal value.
    pub(crate) fn lower_literal(&self, lit: &edge_ast::Lit) -> Result<RcExpr, IrError> {
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
    pub(crate) fn lower_ident(
        &self,
        name: &str,
        span: Option<&edge_types::span::Span>,
    ) -> Result<RcExpr, IrError> {
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
        span.map_or_else(
            || Err(IrError::Lowering(format!("undefined variable: {name}"))),
            |span| {
                Err(IrError::Diagnostic(
                    edge_diagnostics::Diagnostic::error(format!(
                        "cannot find value `{name}` in this scope",
                    ))
                    .with_label(span.clone(), "not found in this scope"),
                ))
            },
        )
    }

    /// Lower an assignment expression.
    pub(crate) fn lower_assignment(
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
                self.try_lower_array_element_write(base, index, &rhs_ir)?
                    .map_or_else(|| self.lower_mapping_write(base, index, rhs_ir), Ok)
            }
            _ => Err(IrError::Unsupported(
                "complex assignment target not yet supported".to_owned(),
            )),
        }
    }

    /// Lower a binary operator.
    pub(crate) fn lower_binary_op(
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

        // Check if the LHS is a user-defined type
        let lhs_type = self.infer_receiver_type(lhs);
        if let Some(ref type_name) = lhs_type {
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

    /// Lower a unary operator.
    pub(crate) fn lower_unary_op(
        &self,
        op: &edge_ast::UnaryOp,
        expr: RcExpr,
    ) -> Result<RcExpr, IrError> {
        let ir_op = match op {
            edge_ast::UnaryOp::Neg => EvmUnaryOp::Neg,
            edge_ast::UnaryOp::BitwiseNot => EvmUnaryOp::Not,
            edge_ast::UnaryOp::LogicalNot => EvmUnaryOp::IsZero,
        };
        Ok(ast_helpers::uop(ir_op, expr))
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
    pub(crate) fn lower_tuple_return(
        &mut self,
        elements: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        let mut result =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        for (i, elem) in elements.iter().enumerate() {
            let val = self.lower_expr(elem)?;
            let offset = ast_helpers::const_int((i * 32) as i64, self.current_ctx.clone());
            let mstore = ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&mstore);
            result = ast_helpers::concat(result, mstore);
        }
        let offset = ast_helpers::const_int(0, self.current_ctx.clone());
        let size = ast_helpers::const_int((elements.len() * 32) as i64, self.current_ctx.clone());
        let ret = ast_helpers::return_op(offset, size, Rc::clone(&self.current_state));
        result = ast_helpers::concat(result, ret);
        Ok(result)
    }
}
