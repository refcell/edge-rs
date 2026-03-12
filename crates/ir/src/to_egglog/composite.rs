//! Composite type lowering: structs, arrays, field access, union instantiation.

use std::rc::Rc;

use edge_diagnostics;

use super::AstToEgglog;
use crate::{
    ast_helpers,
    schema::{EvmBaseType, EvmBinaryOp, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// Look up the variant index for a union type.
    /// Handles both concrete union types and generic unions (resolves to monomorphized name).
    pub(crate) fn variant_index(
        &self,
        type_name: &str,
        variant_name: &str,
        span: Option<&edge_types::span::Span>,
    ) -> Result<usize, IrError> {
        tracing::trace!("variant_index: type_name={type_name}, variant_name={variant_name}");
        // Try direct lookup first
        let variants = if let Some(v) = self.union_types.get(type_name) {
            v
        } else if let Some(mangled) = self.resolve_generic_type_name(type_name) {
            self.union_types.get(&mangled).ok_or_else(|| {
                let diag = edge_diagnostics::Diagnostic::error(format!(
                    "unknown union type: `{type_name}`"
                ));
                IrError::Diagnostic(if let Some(s) = span {
                    diag.with_label(s.clone(), "not found")
                } else {
                    diag
                })
            })?
        } else {
            // Check if resolution failed due to ambiguity (multiple monomorphizations)
            let candidate_count = self
                .monomorphized_types
                .iter()
                .filter(|((base, _), _)| base == type_name)
                .count();
            let diag = if candidate_count > 1 {
                edge_diagnostics::Diagnostic::error(format!(
                    "ambiguous generic type `{type_name}`: {candidate_count} monomorphizations exist",
                )).with_note("provide explicit type arguments to disambiguate")
            } else {
                edge_diagnostics::Diagnostic::error(format!("unknown union type: `{type_name}`"))
            };
            return Err(IrError::Diagnostic(if let Some(s) = span {
                diag.with_label(s.clone(), "not found")
            } else {
                diag
            }));
        };
        variants
            .iter()
            .position(|(name, _)| name == variant_name)
            .ok_or_else(|| {
                let available: Vec<&str> = variants.iter().map(|(n, _)| n.as_str()).collect();
                let diag = edge_diagnostics::Diagnostic::error(format!(
                    "no variant named `{variant_name}` in union `{type_name}`",
                ))
                .with_note(format!("available variants: {}", available.join(", ")));
                IrError::Diagnostic(if let Some(s) = span {
                    diag.with_label(s.clone(), "variant not found")
                } else {
                    diag
                })
            })
    }

    /// Lower a union instantiation expression, handling both simple enums and data-carrying unions.
    /// Simple: `Direction::North` → integer discriminant
    /// Data: `Result::Ok(42)` → MSTORE discriminant at base, MSTORE data at base+32, return base
    pub(crate) fn lower_union_instantiation_expr(
        &mut self,
        type_name: &str,
        variant_name: &str,
        args: &[edge_ast::Expr],
        span: Option<&edge_types::span::Span>,
    ) -> Result<RcExpr, IrError> {
        let idx = self.variant_index(type_name, variant_name, span)?;
        // Resolve generic type names to monomorphized versions
        let resolved_name = if self.union_types.contains_key(type_name) {
            type_name.to_string()
        } else {
            self.resolve_generic_type_name(type_name).ok_or_else(|| {
                let candidate_count = self.monomorphized_types.iter()
                    .filter(|((base, _), _)| base == type_name)
                    .count();
                let diag = if candidate_count > 1 {
                    edge_diagnostics::Diagnostic::error(format!(
                        "ambiguous generic type `{type_name}`: {candidate_count} monomorphizations exist",
                    )).with_note("provide explicit type arguments to disambiguate")
                } else {
                    edge_diagnostics::Diagnostic::error(format!(
                        "unknown union type: `{type_name}`",
                    ))
                };
                IrError::Diagnostic(if let Some(s) = span {
                    diag.with_label(s.clone(), "not found")
                } else {
                    diag
                })
            })?
        };
        let variants = self.union_types.get(&resolved_name).ok_or_else(|| {
            let diag =
                edge_diagnostics::Diagnostic::error(format!("unknown union type: `{type_name}`",));
            IrError::Diagnostic(if let Some(s) = span {
                diag.with_label(s.clone(), "not found")
            } else {
                diag
            })
        })?;
        let has_data = variants.get(idx).map(|(_, d)| *d).unwrap_or(false);

        if !has_data || args.is_empty() {
            // Simple enum: just the discriminant integer
            Ok(ast_helpers::const_int(idx as i64, self.current_ctx.clone()))
        } else {
            // Data-carrying union: allocate 2 words (discriminant + data)
            let base_ir = self.alloc_region(2);

            let disc_ir = ast_helpers::const_int(idx as i64, self.current_ctx.clone());
            let data_offset_ir = ast_helpers::add(
                Rc::clone(&base_ir),
                ast_helpers::const_int(32, self.current_ctx.clone()),
            );

            // MSTORE(base, discriminant, state)
            let store_disc =
                ast_helpers::mstore(Rc::clone(&base_ir), disc_ir, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&store_disc);

            // MSTORE(base+32, data, state)
            let data_val = self.lower_expr(&args[0])?;
            let store_data =
                ast_helpers::mstore(data_offset_ir, data_val, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&store_data);

            let result = ast_helpers::concat(store_disc, store_data);
            // The "value" of this union is the base address
            let result = ast_helpers::concat(result, base_ir);
            Ok(result)
        }
    }

    /// Lower a struct instantiation: `Point { x: 10, y: 20 }`
    /// For unpacked structs: stores fields at sequential 32-byte memory offsets.
    /// For packed structs: packs fields into minimal words via SHL+OR.
    /// Returns the base memory address as the struct "value".
    pub(crate) fn lower_struct_instantiation(
        &mut self,
        type_name: &str,
        fields: &[(edge_ast::Ident, edge_ast::Expr)],
    ) -> Result<RcExpr, IrError> {
        // Resolve generic struct names to monomorphized versions.
        // Use type_sig_hint from VarDecl annotation when available for precise resolution.
        let resolved_name = if self.struct_types.contains_key(type_name) {
            type_name.to_string()
        } else {
            // Try precise resolution via type_sig_hint first
            let from_hint =
                if let Some(edge_ast::ty::TypeSig::Named(ref hint_name, ref hint_args)) =
                    self.type_sig_hint
                {
                    if (hint_name.name == type_name || hint_name.name.starts_with(type_name))
                        && !hint_args.is_empty()
                    {
                        self.resolve_generic_type_name_with_args(type_name, hint_args)
                    } else {
                        None
                    }
                } else {
                    None
                };
            from_hint.unwrap_or_else(|| {
                self.resolve_generic_type_name(type_name)
                    .unwrap_or_else(|| type_name.to_string())
            })
        };
        let struct_info = self.struct_types.get(&resolved_name).cloned();

        // Check for packed struct path
        if let Some(ref info) = struct_info {
            if info.is_packed {
                if let Some(ref layout) = info.packed_layout {
                    return self.lower_packed_struct_instantiation(
                        &resolved_name,
                        fields,
                        &info.fields,
                        layout,
                    );
                }
            }
        }

        // Unpacked path
        let field_info = struct_info.map(|i| i.fields).unwrap_or_else(|| {
            // Unknown struct type — treat each field as a 32-byte word in order
            fields
                .iter()
                .map(|(name, _)| (name.name.clone(), EvmType::Base(EvmBaseType::UIntT(256))))
                .collect()
        });

        let base_ir = self.alloc_region(field_info.len());

        let mut result =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());

        // Store each field at base + field_index * 32
        // Match fields by name to handle out-of-order initialization
        for (name, expr) in fields {
            let field_idx = field_info
                .iter()
                .position(|(n, _)| n == &name.name)
                .unwrap_or_else(|| {
                    // Field not found in type — use position in instantiation
                    fields
                        .iter()
                        .position(|(n, _)| n.name == name.name)
                        .unwrap_or(0)
                });
            let val = self.lower_expr(expr)?;
            let offset = ast_helpers::add(
                Rc::clone(&base_ir),
                ast_helpers::const_int((field_idx * 32) as i64, self.current_ctx.clone()),
            );
            let mstore = ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&mstore);
            result = ast_helpers::concat(result, mstore);
        }

        // Track this allocation for VarAssign wiring
        self.last_composite_alloc = Some((resolved_name, Rc::clone(&base_ir)));

        // Return the base address as the struct value
        Ok(ast_helpers::concat(result, base_ir))
    }

    /// Lower a packed struct instantiation.
    /// Packs all fields into minimal 256-bit words using SHL+OR, then MSTOREs.
    fn lower_packed_struct_instantiation(
        &mut self,
        resolved_name: &str,
        fields: &[(edge_ast::Ident, edge_ast::Expr)],
        field_defs: &[(String, EvmType)],
        layout: &super::PackedLayout,
    ) -> Result<RcExpr, IrError> {
        let base_ir = self.alloc_region(layout.word_count);

        let mut result =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());

        // Build packed word: OR together (val & mask) << bit_offset for each field
        // For now, single-word only (word_index == 0 for all fields)
        let mut packed_word: Option<RcExpr> = None;

        for (name, expr) in fields {
            let field_idx = field_defs
                .iter()
                .position(|(n, _)| n == &name.name)
                .unwrap_or_else(|| {
                    fields
                        .iter()
                        .position(|(n, _)| n.name == name.name)
                        .unwrap_or(0)
                });

            let fl = &layout.field_layouts[field_idx];
            let val = self.lower_expr(expr)?;

            // Mask the value to its bit width: val & ((1 << bit_width) - 1)
            let mask = Self::make_mask(fl.bit_width, &self.current_ctx);
            let masked = ast_helpers::bitand(val, mask);

            // Shift to position: masked << bit_offset
            let shifted = if fl.bit_offset > 0 {
                let shift = ast_helpers::const_int(fl.bit_offset as i64, self.current_ctx.clone());
                ast_helpers::shl(shift, masked)
            } else {
                masked
            };

            // OR into accumulated word
            packed_word = Some(match packed_word {
                Some(acc) => ast_helpers::bitor(acc, shifted),
                None => shifted,
            });
        }

        // MSTORE the packed word
        if let Some(word) = packed_word {
            let mstore =
                ast_helpers::mstore(Rc::clone(&base_ir), word, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&mstore);
            result = ast_helpers::concat(result, mstore);
        }

        // Track this allocation for VarAssign wiring
        self.last_composite_alloc = Some((resolved_name.to_string(), Rc::clone(&base_ir)));

        // Return the base address as the struct value
        Ok(ast_helpers::concat(result, base_ir))
    }

    /// Create a mask for the given bit width: `(1 << bit_width) - 1`
    fn make_mask(bit_width: u16, ctx: &crate::schema::EvmContext) -> RcExpr {
        if bit_width >= 256 {
            // Full word — all ones
            ast_helpers::const_bigint(
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
                ctx.clone(),
            )
        } else if bit_width <= 63 {
            let mask_val = (1i64 << bit_width) - 1;
            ast_helpers::const_int(mask_val, ctx.clone())
        } else {
            // Build hex mask string for wider masks (64..255 bits)
            // (1 << N) - 1 = N/4 'f' hex digits, possibly with a partial leading digit
            let full_nibbles = (bit_width / 4) as usize;
            let remainder = bit_width % 4;
            let mut hex = String::with_capacity(full_nibbles + 1);
            if remainder > 0 {
                hex.push(char::from_digit((1u32 << remainder) - 1, 16).unwrap());
            }
            for _ in 0..full_nibbles {
                hex.push('f');
            }
            ast_helpers::const_bigint(hex, ctx.clone())
        }
    }

    /// Lower an array instantiation: `[10, 20, 30]`
    /// Stores elements at sequential 32-byte memory offsets.
    /// Returns the base memory address.
    pub(crate) fn lower_array_instantiation(
        &mut self,
        elements: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
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
            self.current_state = Rc::clone(&mstore);
            result = ast_helpers::concat(result, mstore);
        }

        // Track this allocation for VarAssign wiring (encode length for tuple return expansion)
        self.last_composite_alloc =
            Some((format!("__array__{}", elements.len()), Rc::clone(&base_ir)));

        Ok(ast_helpers::concat(result, base_ir))
    }

    /// Lower field access: `obj.field`
    /// For struct-typed variables: compute memory offset and MLOAD.
    /// For packed structs: MLOAD + SHR + AND to extract the field.
    /// Falls back to storage field access for contract storage fields.
    pub(crate) fn lower_field_access(
        &mut self,
        obj: &edge_ast::Expr,
        field_name: &str,
    ) -> Result<RcExpr, IrError> {
        // Check if obj is an identifier bound to a struct-typed variable
        if let edge_ast::Expr::Ident(ident) = obj {
            // Check for storage-backed packed struct field read (e.g., self.color.r)
            if let Some(result) =
                self.try_lower_storage_packed_field_read(&ident.name, field_name)?
            {
                return Ok(result);
            }

            let lookup = self.lookup_composite_info(&ident.name);
            if let Some((type_name, base_expr)) = lookup {
                if let Some(struct_info) = self.struct_types.get(&type_name).cloned() {
                    if let Some(field_idx) =
                        struct_info.fields.iter().position(|(n, _)| n == field_name)
                    {
                        // Packed struct field read
                        if struct_info.is_packed {
                            if let Some(ref layout) = struct_info.packed_layout {
                                let fl = &layout.field_layouts[field_idx];
                                let word_offset = ast_helpers::add(
                                    base_expr,
                                    ast_helpers::const_int(
                                        (fl.word_index * 32) as i64,
                                        self.current_ctx.clone(),
                                    ),
                                );
                                let word =
                                    ast_helpers::mload(word_offset, Rc::clone(&self.current_state));
                                return Ok(Self::extract_packed_field(word, fl, &self.current_ctx));
                            }
                        }
                        // Unpacked struct field read
                        let offset = ast_helpers::add(
                            base_expr,
                            ast_helpers::const_int(
                                (field_idx * 32) as i64,
                                self.current_ctx.clone(),
                            ),
                        );
                        return Ok(ast_helpers::mload(offset, Rc::clone(&self.current_state)));
                    }
                }
            }
            // Also check if obj is itself a field access (nested: rect.origin.x)
        } else if let edge_ast::Expr::FieldAccess(inner_obj, inner_field, _) = obj {
            // Nested field access: inner_obj.inner_field.field_name
            // First resolve the inner struct to get its base offset
            if let edge_ast::Expr::Ident(ident) = inner_obj.as_ref() {
                let lookup = self.lookup_composite_info(&ident.name);
                if let Some((type_name, base_expr)) = lookup {
                    if let Some(struct_info) = self.struct_types.get(&type_name).cloned() {
                        if let Some(inner_idx) = struct_info
                            .fields
                            .iter()
                            .position(|(n, _)| n == &inner_field.name)
                        {
                            // The inner field's type should be a struct too
                            let inner_type = &struct_info.fields[inner_idx].0;
                            let inner_base_ir = ast_helpers::mload(
                                ast_helpers::add(
                                    base_expr,
                                    ast_helpers::const_int(
                                        (inner_idx * 32) as i64,
                                        self.current_ctx.clone(),
                                    ),
                                ),
                                Rc::clone(&self.current_state),
                            );
                            let _ = inner_type;
                            for (_sname, sinfo) in &self.struct_types {
                                if let Some(fidx) =
                                    sinfo.fields.iter().position(|(n, _)| n == field_name)
                                {
                                    let field_offset = ast_helpers::const_int(
                                        (fidx * 32) as i64,
                                        self.current_ctx.clone(),
                                    );
                                    let addr = ast_helpers::add(inner_base_ir, field_offset);
                                    return Ok(ast_helpers::mload(
                                        addr,
                                        Rc::clone(&self.current_state),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: treat as contract storage field access
        let _obj_ir = self.lower_expr(obj)?;
        self.lower_ident(field_name, None)
    }

    /// Extract a packed field from a loaded word: `SHR` by `bit_offset`, AND with mask.
    fn extract_packed_field(
        word: RcExpr,
        fl: &super::PackedFieldLayout,
        ctx: &crate::schema::EvmContext,
    ) -> RcExpr {
        let shifted = if fl.bit_offset > 0 {
            let shift = ast_helpers::const_int(fl.bit_offset as i64, ctx.clone());
            ast_helpers::shr(shift, word)
        } else {
            word
        };
        if fl.bit_width >= 256 {
            shifted
        } else {
            let mask = Self::make_mask(fl.bit_width, ctx);
            ast_helpers::bitand(shifted, mask)
        }
    }

    /// Look up composite (struct/array) type info for a variable.
    /// Returns `(type_name, base_address_expr)` where the base is a symbolic `MemRegion` expression.
    pub(crate) fn lookup_composite_info(&self, var_name: &str) -> Option<(String, RcExpr)> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(var_name) {
                if let (Some(ref ct), Some(ref cb)) =
                    (&binding.composite_type, &binding.composite_base)
                {
                    return Some((ct.clone(), Rc::clone(cb)));
                }
            }
        }
        None
    }

    /// Check if a variable is an array parameter with dynamic base address.
    pub(crate) fn lookup_array_param_binding(&self, var_name: &str) -> Option<RcExpr> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(var_name) {
                if binding.composite_type.as_deref() == Some("__array_param__") {
                    return Some(Rc::clone(&binding.value));
                }
            }
        }
        None
    }

    /// Extract the array length from a `__array__N` composite type string.
    fn extract_array_len_from_composite(type_name: &str) -> Option<usize> {
        type_name
            .strip_prefix("__array__")
            .and_then(|s| s.parse::<usize>().ok())
    }

    /// Try to extract a compile-time constant integer from an AST expression.
    fn try_const_index(expr: &edge_ast::Expr) -> Option<u64> {
        match expr {
            edge_ast::Expr::Literal(lit) => match lit.as_ref() {
                edge_ast::lit::Lit::Int(bytes, _, _) => {
                    Some(u64::from_be_bytes(bytes[24..32].try_into().unwrap()))
                }
                _ => None,
            },
            edge_ast::Expr::Paren(inner, _) => Self::try_const_index(inner),
            _ => None,
        }
    }

    /// Validate array bounds. For const indices, emits a compile error if out of bounds.
    /// For non-const indices, emits a runtime bounds check that reverts on OOB.
    fn check_array_bounds(
        &mut self,
        index: &edge_ast::Expr,
        array_len: usize,
        idx_ir: &RcExpr,
    ) -> Result<Option<RcExpr>, IrError> {
        if let Some(const_idx) = Self::try_const_index(index) {
            if const_idx >= array_len as u64 {
                return Err(IrError::LoweringSpanned {
                    message: format!(
                        "array index {const_idx} is out of bounds for array of length {array_len}"
                    ),
                    span: index.span(),
                });
            }
            // Const index is in bounds — no runtime check needed
            return Ok(None);
        }

        // Non-const index: emit `if (index >= len) { revert(0, 0) }`
        let len_ir = ast_helpers::const_int(array_len as i64, self.current_ctx.clone());
        let in_bounds = ast_helpers::bop(EvmBinaryOp::Lt, Rc::clone(idx_ir), len_ir);
        let zero = ast_helpers::const_int(0, self.current_ctx.clone());
        let revert = ast_helpers::revert(Rc::clone(&zero), zero, Rc::clone(&self.current_state));
        let empty = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        let inputs =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());
        let bounds_check = ast_helpers::if_then_else(in_bounds, inputs, empty, revert);
        self.current_state = Rc::clone(&bounds_check);
        Ok(Some(bounds_check))
    }

    /// Try to lower an array element read for memory-backed arrays.
    /// Returns None if the base is not a memory-backed array.
    pub(crate) fn try_lower_array_element_read(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
    ) -> Result<Option<RcExpr>, IrError> {
        if let edge_ast::Expr::Ident(ident) = base {
            // Check for fixed-offset composite (array/struct with known base)
            if let Some((type_name, base_expr)) = self.lookup_composite_info(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                if let Some(array_len) = Self::extract_array_len_from_composite(&type_name) {
                    let check = self.check_array_bounds(index, array_len, &idx_ir)?;
                    if let Some(bounds_ir) = check {
                        let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                        let offset =
                            ast_helpers::add(base_expr, ast_helpers::mul(idx_ir, word_size));
                        let load = ast_helpers::mload(offset, Rc::clone(&self.current_state));
                        return Ok(Some(ast_helpers::concat(bounds_ir, load)));
                    }
                }
                // If the type has an Index trait impl, defer to trait dispatch
                // instead of raw MLOAD (e.g., Vec<T> should use Index::index, not raw field access).
                if self
                    .trait_impls
                    .contains_key(&(type_name, "Index".to_string()))
                {
                    return Ok(None);
                }
                let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                let offset = ast_helpers::add(base_expr, ast_helpers::mul(idx_ir, word_size));
                return Ok(Some(ast_helpers::mload(
                    offset,
                    Rc::clone(&self.current_state),
                )));
            }
            // Check for dynamic-base array parameter
            if let Some(base_ir) = self.lookup_array_param_binding(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                let offset = ast_helpers::add(base_ir, ast_helpers::mul(idx_ir, word_size));
                return Ok(Some(ast_helpers::mload(
                    offset,
                    Rc::clone(&self.current_state),
                )));
            }
        }
        Ok(None)
    }

    /// Try to lower an array element write for memory-backed arrays.
    /// Returns None if the base is not a memory-backed array.
    pub(crate) fn try_lower_array_element_write(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
        value: &RcExpr,
    ) -> Result<Option<RcExpr>, IrError> {
        if let edge_ast::Expr::Ident(ident) = base {
            if let Some((type_name, base_expr)) = self.lookup_composite_info(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                if let Some(array_len) = Self::extract_array_len_from_composite(&type_name) {
                    let check = self.check_array_bounds(index, array_len, &idx_ir)?;
                    if let Some(bounds_ir) = check {
                        let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                        let offset =
                            ast_helpers::add(base_expr, ast_helpers::mul(idx_ir, word_size));
                        let mstore = ast_helpers::mstore(
                            offset,
                            Rc::clone(value),
                            Rc::clone(&self.current_state),
                        );
                        self.current_state = Rc::clone(&mstore);
                        return Ok(Some(ast_helpers::concat(bounds_ir, mstore)));
                    }
                }
                let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                let offset = ast_helpers::add(base_expr, ast_helpers::mul(idx_ir, word_size));
                let mstore =
                    ast_helpers::mstore(offset, Rc::clone(value), Rc::clone(&self.current_state));
                self.current_state = Rc::clone(&mstore);
                return Ok(Some(mstore));
            }
            // Dynamic-base array parameter write
            if let Some(base_ir) = self.lookup_array_param_binding(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                let offset = ast_helpers::add(base_ir, ast_helpers::mul(idx_ir, word_size));
                let mstore =
                    ast_helpers::mstore(offset, Rc::clone(value), Rc::clone(&self.current_state));
                self.current_state = Rc::clone(&mstore);
                return Ok(Some(mstore));
            }
        }
        Ok(None)
    }

    /// Try to lower a storage array element read: `values[index]` where `values: &s [u256; N]`.
    /// Returns None if the base is not a storage array field.
    pub(crate) fn try_lower_storage_array_read(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
    ) -> Result<Option<RcExpr>, IrError> {
        if let edge_ast::Expr::Ident(ident) = base {
            if let Some(&(base_slot, len)) = self.storage_array_fields.get(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let check = self.check_array_bounds(index, len, &idx_ir)?;
                let base_slot_ir =
                    ast_helpers::const_int(base_slot as i64, self.current_ctx.clone());
                let slot = ast_helpers::add(base_slot_ir, idx_ir);
                let load = ast_helpers::sload(slot, Rc::clone(&self.current_state));
                if let Some(bounds_ir) = check {
                    return Ok(Some(ast_helpers::concat(bounds_ir, load)));
                }
                return Ok(Some(load));
            }
        }
        Ok(None)
    }

    /// Try to lower a storage array element write: `values[index] = val`.
    /// Returns None if the base is not a storage array field.
    pub(crate) fn try_lower_storage_array_write(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
        value: &RcExpr,
    ) -> Result<Option<RcExpr>, IrError> {
        if let edge_ast::Expr::Ident(ident) = base {
            if let Some(&(base_slot, len)) = self.storage_array_fields.get(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let check = self.check_array_bounds(index, len, &idx_ir)?;
                let base_slot_ir =
                    ast_helpers::const_int(base_slot as i64, self.current_ctx.clone());
                let slot = ast_helpers::add(base_slot_ir, idx_ir);
                let store =
                    ast_helpers::sstore(slot, Rc::clone(value), Rc::clone(&self.current_state));
                self.current_state = Rc::clone(&store);
                if let Some(bounds_ir) = check {
                    return Ok(Some(ast_helpers::concat(bounds_ir, store)));
                }
                return Ok(Some(store));
            }
        }
        Ok(None)
    }

    // =========================================================================
    // Storage-backed packed struct field access
    // =========================================================================

    /// Look up a storage binding that has a packed struct composite type.
    /// Returns `(slot, location, type_name)` if the variable is a storage-backed packed struct.
    fn lookup_storage_packed_binding(
        &self,
        var_name: &str,
    ) -> Option<(usize, crate::schema::DataLocation, String)> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(var_name) {
                if let (Some(slot), Some(ref type_name)) =
                    (binding.storage_slot, &binding.composite_type)
                {
                    if let Some(info) = self.struct_types.get(type_name) {
                        if info.is_packed {
                            return Some((slot, binding.location, type_name.clone()));
                        }
                    }
                }
            }
        }
        None
    }

    /// Try to lower a storage-backed packed struct field read.
    /// e.g., `self.color.r` where `color: &s Rgb` and `Rgb = packed { r: u8, g: u8, b: u8 }`
    /// Generates: `SLOAD(slot)` → `SHR(bit_offset)` → `AND(mask)`
    pub(crate) fn try_lower_storage_packed_field_read(
        &self,
        var_name: &str,
        field_name: &str,
    ) -> Result<Option<RcExpr>, IrError> {
        let (slot, location, type_name) = match self.lookup_storage_packed_binding(var_name) {
            Some(v) => v,
            None => return Ok(None),
        };
        let struct_info = self.struct_types.get(&type_name).unwrap();
        let field_idx = match struct_info.fields.iter().position(|(n, _)| n == field_name) {
            Some(i) => i,
            None => return Ok(None),
        };
        let layout = struct_info.packed_layout.as_ref().unwrap();
        let fl = &layout.field_layouts[field_idx];

        let slot_ir = ast_helpers::const_int(slot as i64, self.current_ctx.clone());
        let word = match location {
            crate::schema::DataLocation::Transient => {
                ast_helpers::tload(slot_ir, Rc::clone(&self.current_state))
            }
            _ => ast_helpers::sload(slot_ir, Rc::clone(&self.current_state)),
        };
        Ok(Some(Self::extract_packed_field(
            word,
            fl,
            &self.current_ctx,
        )))
    }

    /// Try to lower a storage-backed packed struct sub-field write.
    /// e.g., `self.color.r = 5` where `color: &s Rgb` and `Rgb = packed { ... }`
    /// Generates read-modify-write: SLOAD → clear field bits → OR new value → SSTORE
    pub(crate) fn try_lower_storage_packed_field_write(
        &mut self,
        var_name: &str,
        field_name: &str,
        new_value: RcExpr,
    ) -> Result<Option<RcExpr>, IrError> {
        let (slot, location, type_name) = match self.lookup_storage_packed_binding(var_name) {
            Some(v) => v,
            None => return Ok(None),
        };
        let struct_info = self.struct_types.get(&type_name).unwrap();
        let field_idx = match struct_info.fields.iter().position(|(n, _)| n == field_name) {
            Some(i) => i,
            None => return Ok(None),
        };
        let layout = struct_info.packed_layout.as_ref().unwrap();
        let fl = &layout.field_layouts[field_idx];

        let slot_ir = ast_helpers::const_int(slot as i64, self.current_ctx.clone());

        // Load current word from storage
        let current_word = match location {
            crate::schema::DataLocation::Transient => {
                ast_helpers::tload(Rc::clone(&slot_ir), Rc::clone(&self.current_state))
            }
            _ => ast_helpers::sload(Rc::clone(&slot_ir), Rc::clone(&self.current_state)),
        };

        // Clear the field bits: current_word & ~(mask << bit_offset)
        let mask = Self::make_mask(fl.bit_width, &self.current_ctx);
        let shifted_mask = if fl.bit_offset > 0 {
            let shift = ast_helpers::const_int(fl.bit_offset as i64, self.current_ctx.clone());
            ast_helpers::shl(shift, mask)
        } else {
            mask
        };
        let inverted_mask = Rc::new(crate::schema::EvmExpr::Uop(
            crate::schema::EvmUnaryOp::Not,
            shifted_mask,
        ));
        let cleared = ast_helpers::bitand(current_word, inverted_mask);

        // Shift new value into position: (new_value & mask) << bit_offset
        let new_mask = Self::make_mask(fl.bit_width, &self.current_ctx);
        let masked_new = ast_helpers::bitand(new_value, new_mask);
        let shifted_new = if fl.bit_offset > 0 {
            let shift = ast_helpers::const_int(fl.bit_offset as i64, self.current_ctx.clone());
            ast_helpers::shl(shift, masked_new)
        } else {
            masked_new
        };

        // OR together: cleared | shifted_new
        let new_word = ast_helpers::bitor(cleared, shifted_new);

        // Store back
        let store = match location {
            crate::schema::DataLocation::Transient => {
                ast_helpers::tstore(slot_ir, new_word, Rc::clone(&self.current_state))
            }
            _ => ast_helpers::sstore(slot_ir, new_word, Rc::clone(&self.current_state)),
        };
        self.current_state = Rc::clone(&store);
        Ok(Some(store))
    }
}
