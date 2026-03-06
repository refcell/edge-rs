//! Storage and mapping lowering: emit, mapping slots, storage reads/writes.

use std::rc::Rc;

use super::AstToEgglog;
use crate::{
    ast_helpers,
    schema::{DataLocation, EvmExpr, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// Lower an emit statement.
    ///
    /// Generates LOG opcode with:
    /// - topic[0] = keccak256 of event signature
    /// - topic[1..] = indexed parameters
    /// - data = ABI-encoded non-indexed parameters (each MSTORE'd to memory)
    pub(crate) fn lower_emit(
        &mut self,
        event_name: &edge_ast::Ident,
        args: &[edge_ast::Expr],
    ) -> Result<RcExpr, IrError> {
        let ctx = self.current_ctx.clone();

        // Compute event signature for topic[0]
        // Build the event signature string: "EventName(type1,type2,...)"
        let event_info = self.events.get(&event_name.name).cloned();
        let sig = event_info.as_ref().map_or_else(
            || {
                // Fallback: build signature from arg count
                let types: Vec<&str> = args.iter().map(|_| "uint256").collect();
                format!("{}({})", event_name.name, types.join(","))
            },
            |fields| {
                let types: Vec<String> = fields
                    .iter()
                    .map(|(_, _, ty)| self.type_sig_to_abi_string(ty))
                    .collect();
                format!("{}({})", event_name.name, types.join(","))
            },
        );
        // Event topic0 must be the full 32-byte keccak256 hash (not a 4-byte selector)
        let mut hash = [0u8; 32];
        edge_types::bytes::hash_bytes(&mut hash, &sig);
        let hash_hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let sig_topic = ast_helpers::const_bigint(hash_hex, ctx.clone());

        // Separate indexed and non-indexed args
        let mut topics = vec![sig_topic];
        let mut data_exprs = Vec::new();
        let mut side_effects: Vec<RcExpr> = Vec::new();

        for (i, arg) in args.iter().enumerate() {
            let arg_ir = self.lower_expr(arg)?;
            let is_indexed = event_info
                .as_ref()
                .and_then(|fields| fields.get(i))
                .map(|(_, indexed, _)| *indexed)
                .unwrap_or(false);

            if is_indexed {
                topics.push(arg_ir);
            } else {
                data_exprs.push(arg_ir);
            }
        }

        // MSTORE non-indexed data to memory
        let (data_offset, data_size) = if data_exprs.is_empty() {
            (
                ast_helpers::const_int(0, ctx.clone()),
                ast_helpers::const_int(0, ctx),
            )
        } else {
            for (i, data_expr) in data_exprs.iter().enumerate() {
                let offset = (i * 32) as i64;
                let mstore = ast_helpers::mstore(
                    ast_helpers::const_int(offset, ctx.clone()),
                    Rc::clone(data_expr),
                    Rc::clone(&self.current_state),
                );
                self.current_state = Rc::clone(&mstore);
                side_effects.push(mstore);
            }
            (
                ast_helpers::const_int(0, ctx.clone()),
                ast_helpers::const_int((data_exprs.len() * 32) as i64, ctx),
            )
        };

        let topic_count = topics.len();
        let log = Rc::new(EvmExpr::Log(
            topic_count,
            topics,
            data_offset,
            data_size,
            Rc::clone(&self.current_state),
        ));
        self.current_state = Rc::clone(&log);

        // Build concat of side effects + log
        if side_effects.is_empty() {
            Ok(log)
        } else {
            let mut result = Rc::clone(&side_effects[0]);
            for effect in &side_effects[1..] {
                result = ast_helpers::concat(result, Rc::clone(effect));
            }
            Ok(ast_helpers::concat(result, log))
        }
    }

    /// Compute the storage slot for a mapping access.
    ///
    /// For `mapping[key]` at base slot `s`, Solidity uses:
    ///   `keccak256(abi.encode(key, s))` where key is left-padded to 32 bytes
    ///   at memory[0..32] and s is at memory[32..64].
    ///
    /// Returns `(side_effects_expr, computed_slot_expr)` where `side_effects_expr`
    /// is a Concat of MSTOREs that must be emitted before the slot is used.
    pub(crate) fn compute_mapping_slot(&mut self, key: RcExpr, base_slot: i64) -> (RcExpr, RcExpr) {
        let ctx = self.current_ctx.clone();
        // MSTORE(0, key)
        let mstore_key = ast_helpers::mstore(
            ast_helpers::const_int(0, ctx.clone()),
            key,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_key);
        // MSTORE(32, base_slot)
        let mstore_slot = ast_helpers::mstore(
            ast_helpers::const_int(32, ctx.clone()),
            ast_helpers::const_int(base_slot, ctx.clone()),
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_slot);
        // KECCAK256(0, 64, state) — state captures the memory contents
        let computed_slot = ast_helpers::keccak256(
            ast_helpers::const_int(0, ctx.clone()),
            ast_helpers::const_int(64, ctx),
            Rc::clone(&self.current_state),
        );
        let side_effects = ast_helpers::concat(mstore_key, mstore_slot);
        (side_effects, computed_slot)
    }

    /// Compute the storage slot for a nested mapping access.
    ///
    /// For `mapping[key1][key2]`, uses `keccak256(key2 . keccak256(key1 . base_slot))`.
    ///
    /// Uses memory[0..64] for the first level and memory[64..128] for the second
    /// to avoid the second level's MSTORE overwriting the first level's data before
    /// KECCAK256 reads it.
    pub(crate) fn compute_nested_mapping_slot(
        &mut self,
        outer_key: RcExpr,
        inner_key: RcExpr,
        base_slot: i64,
    ) -> (RcExpr, RcExpr) {
        let ctx = self.current_ctx.clone();
        // First level: keccak256(key1 . base_slot) at memory[0..64]
        let mstore_key1 = ast_helpers::mstore(
            ast_helpers::const_int(0, ctx.clone()),
            outer_key,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_key1);
        let mstore_slot1 = ast_helpers::mstore(
            ast_helpers::const_int(32, ctx.clone()),
            ast_helpers::const_int(base_slot, ctx.clone()),
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_slot1);
        // inner_slot — KECCAK256(0, 64, state) reads memory[0..64]
        let inner_slot = ast_helpers::keccak256(
            ast_helpers::const_int(0, ctx.clone()),
            ast_helpers::const_int(64, ctx.clone()),
            Rc::clone(&self.current_state),
        );
        // Second level: keccak256(key2 . inner_slot) at memory[64..128]
        // Using offset 64 avoids overwriting memory[0..64] before KECCAK256 reads it
        let mstore_key2 = ast_helpers::mstore(
            ast_helpers::const_int(64, ctx.clone()),
            inner_key,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_key2);
        let mstore_slot2 = ast_helpers::mstore(
            ast_helpers::const_int(96, ctx.clone()),
            inner_slot,
            Rc::clone(&self.current_state),
        );
        self.current_state = Rc::clone(&mstore_slot2);
        let computed_slot = ast_helpers::keccak256(
            ast_helpers::const_int(64, ctx.clone()),
            ast_helpers::const_int(64, ctx),
            Rc::clone(&self.current_state),
        );
        let side_effects = ast_helpers::concat(
            ast_helpers::concat(mstore_key1, mstore_slot1),
            ast_helpers::concat(mstore_key2, mstore_slot2),
        );
        (side_effects, computed_slot)
    }

    /// Lower a mapping read: `field[key]` or `field[key1][key2]`.
    pub(crate) fn lower_mapping_read(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
    ) -> Result<RcExpr, IrError> {
        // Check for nested mapping: base is itself an ArrayIndex
        if let edge_ast::Expr::ArrayIndex(outer_base, outer_index, _, _) = base {
            // nested: outer_base[outer_index][index]
            let field_name = match &**outer_base {
                edge_ast::Expr::Ident(id) => &id.name,
                _ => {
                    return Err(IrError::Unsupported(
                        "nested mapping on non-identifier".to_owned(),
                    ));
                }
            };
            let (base_slot, location) = self.find_storage_slot(field_name)?;
            let outer_key = self.lower_expr(outer_index)?;
            let inner_key = self.lower_expr(index)?;
            let (side_effects, computed_slot) =
                self.compute_nested_mapping_slot(outer_key, inner_key, base_slot as i64);
            let load = match location {
                DataLocation::Transient => {
                    ast_helpers::tload(computed_slot, Rc::clone(&self.current_state))
                }
                _ => ast_helpers::sload(computed_slot, Rc::clone(&self.current_state)),
            };
            return Ok(ast_helpers::concat(side_effects, load));
        }

        // Simple mapping: field[key]
        let field_name = match base {
            edge_ast::Expr::Ident(id) => &id.name,
            _ => {
                return Err(IrError::Unsupported(
                    "mapping on non-identifier base".to_owned(),
                ));
            }
        };
        let (base_slot, location) = self.find_storage_slot(field_name)?;
        let key = self.lower_expr(index)?;
        let (side_effects, computed_slot) = self.compute_mapping_slot(key, base_slot as i64);
        let load = match location {
            DataLocation::Transient => {
                ast_helpers::tload(computed_slot, Rc::clone(&self.current_state))
            }
            _ => ast_helpers::sload(computed_slot, Rc::clone(&self.current_state)),
        };
        Ok(ast_helpers::concat(side_effects, load))
    }

    /// Lower a mapping write: `field[key] = value` or `field[key1][key2] = value`.
    pub(crate) fn lower_mapping_write(
        &mut self,
        base: &edge_ast::Expr,
        index: &edge_ast::Expr,
        value: RcExpr,
    ) -> Result<RcExpr, IrError> {
        // Check for nested mapping
        if let edge_ast::Expr::ArrayIndex(outer_base, outer_index, _, _) = base {
            let field_name = match &**outer_base {
                edge_ast::Expr::Ident(id) => &id.name,
                _ => {
                    return Err(IrError::Unsupported(
                        "nested mapping on non-identifier".to_owned(),
                    ));
                }
            };
            let (base_slot, location) = self.find_storage_slot(field_name)?;
            let outer_key = self.lower_expr(outer_index)?;
            let inner_key = self.lower_expr(index)?;
            let (side_effects, computed_slot) =
                self.compute_nested_mapping_slot(outer_key, inner_key, base_slot as i64);
            let store = match location {
                DataLocation::Transient => {
                    ast_helpers::tstore(computed_slot, value, Rc::clone(&self.current_state))
                }
                _ => ast_helpers::sstore(computed_slot, value, Rc::clone(&self.current_state)),
            };
            self.current_state = Rc::clone(&store);
            return Ok(ast_helpers::concat(side_effects, store));
        }

        // Simple mapping write
        let field_name = match base {
            edge_ast::Expr::Ident(id) => &id.name,
            _ => {
                return Err(IrError::Unsupported(
                    "mapping on non-identifier base".to_owned(),
                ));
            }
        };
        let (base_slot, location) = self.find_storage_slot(field_name)?;
        let key = self.lower_expr(index)?;
        let (side_effects, computed_slot) = self.compute_mapping_slot(key, base_slot as i64);
        let store = match location {
            DataLocation::Transient => {
                ast_helpers::tstore(computed_slot, value, Rc::clone(&self.current_state))
            }
            _ => ast_helpers::sstore(computed_slot, value, Rc::clone(&self.current_state)),
        };
        self.current_state = Rc::clone(&store);
        Ok(ast_helpers::concat(side_effects, store))
    }

    /// Find the storage slot index and data location for a named field.
    pub(crate) fn find_storage_slot(&self, name: &str) -> Result<(usize, DataLocation), IrError> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                if let Some(slot) = binding.storage_slot {
                    return Ok((slot, binding.location));
                }
            }
        }
        Err(IrError::Lowering(format!(
            "cannot find storage field `{name}` in the current contract"
        )))
    }
}
