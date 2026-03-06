//! Composite type lowering: structs, arrays, field access, union instantiation.

use std::rc::Rc;

use super::AstToEgglog;
use crate::{
    ast_helpers,
    schema::{EvmBaseType, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// Look up the variant index for a union type.
    /// Handles both concrete union types and generic unions (resolves to monomorphized name).
    pub(crate) fn variant_index(
        &self,
        type_name: &str,
        variant_name: &str,
    ) -> Result<usize, IrError> {
        // Try direct lookup first
        let variants = if let Some(v) = self.union_types.get(type_name) {
            v
        } else if let Some(mangled) = self.resolve_generic_type_name(type_name) {
            self.union_types
                .get(&mangled)
                .ok_or_else(|| IrError::Lowering(format!("unknown union type: {type_name}")))?
        } else {
            return Err(IrError::Lowering(format!(
                "unknown union type: {type_name}"
            )));
        };
        variants
            .iter()
            .position(|(name, _)| name == variant_name)
            .ok_or_else(|| {
                let available: Vec<&str> = variants.iter().map(|(n, _)| n.as_str()).collect();
                IrError::Lowering(format!(
                    "no variant named `{variant_name}` in union `{type_name}`; available variants: {}",
                    available.join(", "),
                ))
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
    ) -> Result<RcExpr, IrError> {
        let idx = self.variant_index(type_name, variant_name)?;
        // Resolve generic type names to monomorphized versions
        let resolved_name = if self.union_types.contains_key(type_name) {
            type_name.to_string()
        } else {
            self.resolve_generic_type_name(type_name)
                .ok_or_else(|| IrError::Lowering(format!("unknown union type: {type_name}")))?
        };
        let variants = self
            .union_types
            .get(&resolved_name)
            .ok_or_else(|| IrError::Lowering(format!("unknown union type: {type_name}")))?;
        let has_data = variants.get(idx).map(|(_, d)| *d).unwrap_or(false);

        if !has_data || args.is_empty() {
            // Simple enum: just the discriminant integer
            Ok(ast_helpers::const_int(idx as i64, self.current_ctx.clone()))
        } else {
            // Data-carrying union: allocate 2 words (discriminant + data)
            let base = self.next_memory_offset;
            self.next_memory_offset += 64;

            let disc_ir = ast_helpers::const_int(idx as i64, self.current_ctx.clone());
            let base_ir = ast_helpers::const_int(base as i64, self.current_ctx.clone());
            let data_offset_ir =
                ast_helpers::const_int((base + 32) as i64, self.current_ctx.clone());

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
    /// Stores fields at sequential 32-byte memory offsets.
    /// Returns the base memory address as the struct "value".
    pub(crate) fn lower_struct_instantiation(
        &mut self,
        type_name: &str,
        fields: &[(edge_ast::Ident, edge_ast::Expr)],
    ) -> Result<RcExpr, IrError> {
        // Resolve generic struct names to monomorphized versions
        let resolved_name = if self.struct_types.contains_key(type_name) {
            type_name.to_string()
        } else {
            self.resolve_generic_type_name(type_name)
                .unwrap_or_else(|| type_name.to_string())
        };
        let field_info = self.struct_types.get(&resolved_name).cloned();
        let field_info = field_info.unwrap_or_else(|| {
            // Unknown struct type — treat each field as a 32-byte word in order
            fields
                .iter()
                .map(|(name, _)| (name.name.clone(), EvmType::Base(EvmBaseType::UIntT(256))))
                .collect()
        });

        let base = self.next_memory_offset;
        self.next_memory_offset += field_info.len() * 32;

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
            let offset =
                ast_helpers::const_int((base + field_idx * 32) as i64, self.current_ctx.clone());
            let mstore = ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&mstore);
            result = ast_helpers::concat(result, mstore);
        }

        // Track this allocation for VarAssign wiring
        self.last_composite_alloc = Some((resolved_name, base));

        // Return the base address as the struct value
        let base_ir = ast_helpers::const_int(base as i64, self.current_ctx.clone());
        Ok(ast_helpers::concat(result, base_ir))
    }

    /// Lower an array instantiation: `[10, 20, 30]`
    /// Stores elements at sequential 32-byte memory offsets.
    /// Returns the base memory address.
    pub(crate) fn lower_array_instantiation(
        &mut self,
        elements: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        let base = self.next_memory_offset;
        self.next_memory_offset += elements.len() * 32;

        let mut result =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), self.current_ctx.clone());

        for (i, elem) in elements.iter().enumerate() {
            let val = self.lower_expr(elem)?;
            let offset = ast_helpers::const_int((base + i * 32) as i64, self.current_ctx.clone());
            let mstore = ast_helpers::mstore(offset, val, Rc::clone(&self.current_state));
            self.current_state = Rc::clone(&mstore);
            result = ast_helpers::concat(result, mstore);
        }

        // Track this allocation for VarAssign wiring
        self.last_composite_alloc = Some(("__array__".to_string(), base));

        let base_ir = ast_helpers::const_int(base as i64, self.current_ctx.clone());
        Ok(ast_helpers::concat(result, base_ir))
    }

    /// Lower field access: `obj.field`
    /// For struct-typed variables: compute memory offset and MLOAD.
    /// Falls back to storage field access for contract storage fields.
    pub(crate) fn lower_field_access(
        &mut self,
        obj: &edge_ast::Expr,
        field_name: &str,
    ) -> Result<RcExpr, IrError> {
        // Check if obj is an identifier bound to a struct-typed variable
        if let edge_ast::Expr::Ident(ident) = obj {
            let lookup = self.lookup_composite_info(&ident.name);
            if let Some((type_name, base_offset)) = lookup {
                if let Some(field_info) = self.struct_types.get(&type_name).cloned() {
                    if let Some(field_idx) = field_info.iter().position(|(n, _)| n == field_name) {
                        let offset = ast_helpers::const_int(
                            (base_offset + field_idx * 32) as i64,
                            self.current_ctx.clone(),
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
                if let Some((type_name, base_offset)) = lookup {
                    if let Some(field_info) = self.struct_types.get(&type_name).cloned() {
                        if let Some(inner_idx) =
                            field_info.iter().position(|(n, _)| n == &inner_field.name)
                        {
                            // The inner field's type should be a struct too
                            let inner_type = &field_info[inner_idx].0;
                            // Look up field in inner struct type via the field's type
                            // For now, read the base address from memory and compute offset
                            let inner_base_ir = ast_helpers::mload(
                                ast_helpers::const_int(
                                    (base_offset + inner_idx * 32) as i64,
                                    self.current_ctx.clone(),
                                ),
                                Rc::clone(&self.current_state),
                            );
                            // Try to find the inner struct's type name
                            let _ = inner_type; // suppress unused warning
                                                // For now, try looking up field_name in all struct types
                            for (_sname, sfields) in &self.struct_types {
                                if let Some(fidx) =
                                    sfields.iter().position(|(n, _)| n == field_name)
                                {
                                    // inner_base + fidx * 32
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

    /// Look up composite (struct/array) type info for a variable.
    pub(crate) fn lookup_composite_info(&self, var_name: &str) -> Option<(String, usize)> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(var_name) {
                if let (Some(ref ct), Some(cb)) = (&binding.composite_type, binding.composite_base)
                {
                    return Some((ct.clone(), cb));
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

    /// Try to lower an array element read for memory-backed arrays.
    /// Returns None if the base is not a memory-backed array.
    pub(crate) fn try_lower_array_element_read(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
    ) -> Result<Option<RcExpr>, IrError> {
        if let edge_ast::Expr::Ident(ident) = base {
            // Check for fixed-offset composite (array/struct with known base)
            if let Some((_type_name, base_offset)) = self.lookup_composite_info(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let base_ir = ast_helpers::const_int(base_offset as i64, self.current_ctx.clone());
                let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                let offset = ast_helpers::add(base_ir, ast_helpers::mul(idx_ir, word_size));
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
            if let Some((_type_name, base_offset)) = self.lookup_composite_info(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let base_ir = ast_helpers::const_int(base_offset as i64, self.current_ctx.clone());
                let word_size = ast_helpers::const_int(32, self.current_ctx.clone());
                let offset = ast_helpers::add(base_ir, ast_helpers::mul(idx_ir, word_size));
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
            if let Some(&(base_slot, _len)) = self.storage_array_fields.get(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let base_slot_ir =
                    ast_helpers::const_int(base_slot as i64, self.current_ctx.clone());
                let slot = ast_helpers::add(base_slot_ir, idx_ir);
                let load = ast_helpers::sload(slot, Rc::clone(&self.current_state));
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
            if let Some(&(base_slot, _len)) = self.storage_array_fields.get(&ident.name) {
                let idx_ir = self.lower_expr(index)?;
                let base_slot_ir =
                    ast_helpers::const_int(base_slot as i64, self.current_ctx.clone());
                let slot = ast_helpers::add(base_slot_ir, idx_ir);
                let store =
                    ast_helpers::sstore(slot, Rc::clone(value), Rc::clone(&self.current_state));
                self.current_state = Rc::clone(&store);
                return Ok(Some(store));
            }
        }
        Ok(None)
    }
}
