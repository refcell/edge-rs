//! Function dispatcher: selector computation, BST/linear dispatch.

use std::rc::Rc;

use super::AstToEgglog;
use crate::{
    ast_helpers,
    schema::{EvmBaseType, EvmBinaryOp, EvmContext, EvmExpr, EvmType, RcExpr},
    IrError,
};

impl AstToEgglog {
    /// Build the function dispatcher for a contract.
    ///
    /// Inlines function bodies directly in the dispatcher. For contracts with
    /// fewer than 4 public functions, uses a linear if-else chain. For 4+
    /// functions, builds a balanced binary search tree sorted by selector value
    /// for O(log N) dispatch instead of O(N).
    ///
    /// Uses `LetBind` to compute the calldata selector once, then Var references
    /// in each condition to avoid redundant CALLDATALOAD+SHR per branch.
    pub(crate) fn build_dispatcher(
        &self,
        contract_name: &str,
        fn_bodies: &[(&edge_ast::ContractFnDecl, Option<RcExpr>)],
    ) -> Result<RcExpr, IrError> {
        let ctx = EvmContext::InFunction(format!("{contract_name}::dispatcher"));

        // Fallback: REVERT if no selector matches
        let fallback: RcExpr = ast_helpers::revert(
            ast_helpers::const_int(0, ctx.clone()),
            ast_helpers::const_int(0, ctx.clone()),
            Rc::new(EvmExpr::Arg(
                EvmType::Base(EvmBaseType::StateT),
                ctx.clone(),
            )),
        );

        // Collect dispatchable functions with their selector values
        let mut entries: Vec<(u32, String, RcExpr)> = Vec::new();
        for (fn_decl, body_ir) in fn_bodies {
            if !fn_decl.is_ext && !fn_decl.is_pub {
                continue;
            }
            let body = match body_ir {
                Some(b) => Rc::clone(b),
                None => continue,
            };
            let sig = self.compute_function_signature(&fn_decl.name.name, &fn_decl.params);
            let sel_val = Self::compute_selector_value(&sig);
            entries.push((sel_val, sig, body));
        }

        if entries.is_empty() {
            return Ok(fallback);
        }

        // Sort by selector value for binary search
        entries.sort_by_key(|(sel, _, _)| *sel);

        let selector_var = ast_helpers::var("__selector".to_string());

        let result = if entries.len() >= 4 {
            // Binary search dispatch for 4+ functions
            Self::build_bst_dispatch(&entries, &selector_var, &fallback, &ctx)
        } else {
            // Linear dispatch for few functions
            Self::build_linear_dispatch(&entries, &selector_var, &fallback, &ctx)
        };

        // Wrap in LetBind that computes the selector once
        // Load first 4 bytes of calldata as selector
        let calldataload = Rc::new(EvmExpr::Bop(
            EvmBinaryOp::CalldataLoad,
            ast_helpers::const_int(0, ctx.clone()),
            Rc::new(EvmExpr::Arg(
                EvmType::Base(EvmBaseType::StateT),
                ctx.clone(),
            )),
        ));
        // Shift right by 224 bits to get top 4 bytes
        // IR convention: Bop(Shr, shift_amount, value)
        let shifted = ast_helpers::bop(
            EvmBinaryOp::Shr,
            ast_helpers::const_int(224, ctx),
            calldataload,
        );

        Ok(ast_helpers::let_bind(
            "__selector".to_string(),
            shifted,
            result,
        ))
    }

    /// Compute the numeric 4-byte selector value for a function signature.
    fn compute_selector_value(sig: &str) -> u32 {
        let mut hash = [0u8; 32];
        edge_types::bytes::hash_bytes(&mut hash, sig);
        u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]])
    }

    /// Build a linear if-else dispatch chain (for < 4 functions).
    fn build_linear_dispatch(
        entries: &[(u32, String, RcExpr)],
        selector_var: &RcExpr,
        fallback: &RcExpr,
        ctx: &EvmContext,
    ) -> RcExpr {
        let mut result = Rc::clone(fallback);
        for (_sel_val, sig, body) in entries.iter().rev() {
            let selector_expr = ast_helpers::selector(sig.clone());
            let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
            let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
            result = ast_helpers::if_then_else(cond, inputs, Rc::clone(body), result);
        }
        result
    }

    /// Build a balanced binary search tree dispatch (for 4+ functions).
    ///
    /// At each node: check EQ with pivot selector. If no match, use GT
    /// to decide which subtree to recurse into.
    fn build_bst_dispatch(
        entries: &[(u32, String, RcExpr)],
        selector_var: &RcExpr,
        fallback: &RcExpr,
        ctx: &EvmContext,
    ) -> RcExpr {
        match entries.len() {
            0 => Rc::clone(fallback),
            1 => {
                // Leaf: single EQ check
                let (_, sig, body) = &entries[0];
                let selector_expr = ast_helpers::selector(sig.clone());
                let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
                let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
                ast_helpers::if_then_else(cond, inputs, Rc::clone(body), Rc::clone(fallback))
            }
            2 => {
                // Two entries: linear chain (no benefit from GT)
                let right = {
                    let (_, sig, body) = &entries[1];
                    let selector_expr = ast_helpers::selector(sig.clone());
                    let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
                    let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
                    ast_helpers::if_then_else(cond, inputs, Rc::clone(body), Rc::clone(fallback))
                };
                let (_, sig, body) = &entries[0];
                let selector_expr = ast_helpers::selector(sig.clone());
                let cond = ast_helpers::eq(Rc::clone(selector_var), selector_expr);
                let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
                ast_helpers::if_then_else(cond, inputs, Rc::clone(body), right)
            }
            _ => {
                // Split at midpoint
                let mid = entries.len() / 2;
                let (pivot_val, pivot_sig, pivot_body) = &entries[mid];

                // EQ check with pivot
                let pivot_selector = ast_helpers::selector(pivot_sig.clone());
                let eq_cond = ast_helpers::eq(Rc::clone(selector_var), pivot_selector);

                // GT comparison for branching
                let pivot_const = ast_helpers::const_int(*pivot_val as i64, ctx.clone());
                let gt_cond =
                    ast_helpers::bop(EvmBinaryOp::Gt, Rc::clone(selector_var), pivot_const);

                let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());

                // Recurse on left (selectors < pivot) and right (selectors > pivot)
                let left_tree =
                    Self::build_bst_dispatch(&entries[..mid], selector_var, fallback, ctx);
                let right_tree =
                    Self::build_bst_dispatch(&entries[mid + 1..], selector_var, fallback, ctx);

                // If GT(sel, pivot) then right_tree else left_tree
                let gt_branch =
                    ast_helpers::if_then_else(gt_cond, Rc::clone(&inputs), right_tree, left_tree);

                // If EQ(sel, pivot) then pivot_body else gt_branch
                ast_helpers::if_then_else(eq_cond, inputs, Rc::clone(pivot_body), gt_branch)
            }
        }
    }

    /// Compute the ABI function signature string.
    pub(crate) fn compute_function_signature(
        &self,
        name: &str,
        params: &[(edge_ast::Ident, edge_ast::ty::TypeSig)],
    ) -> String {
        let param_types: Vec<String> = params
            .iter()
            .map(|(_, ty)| self.type_sig_to_abi_string(ty))
            .collect();
        format!("{name}({})", param_types.join(","))
    }
}
