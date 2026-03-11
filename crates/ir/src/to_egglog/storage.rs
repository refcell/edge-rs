//! Storage lowering: emit statements, storage field lookup.

use std::rc::Rc;

use edge_diagnostics;

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
            let log_region = self.alloc_region(data_exprs.len());
            for (i, data_expr) in data_exprs.iter().enumerate() {
                let offset = ast_helpers::add(
                    Rc::clone(&log_region),
                    ast_helpers::const_int((i * 32) as i64, ctx.clone()),
                );
                let mstore = ast_helpers::mstore(
                    offset,
                    Rc::clone(data_expr),
                    Rc::clone(&self.current_state),
                );
                self.current_state = Rc::clone(&mstore);
                side_effects.push(mstore);
            }
            (
                log_region,
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

    /// Find the storage slot index and data location for a named field.
    #[allow(dead_code)]
    pub(crate) fn find_storage_slot(&self, name: &str) -> Result<(usize, DataLocation), IrError> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                if let Some(slot) = binding.storage_slot {
                    return Ok((slot, binding.location));
                }
            }
        }
        Err(IrError::Diagnostic(edge_diagnostics::Diagnostic::error(
            format!("cannot find storage field `{name}` in the current contract",),
        )))
    }
}
