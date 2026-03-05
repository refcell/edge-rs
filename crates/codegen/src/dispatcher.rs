//! Function selector dispatch table generation.
//!
//! Generates the EVM bytecode that reads the first 4 bytes of calldata
//! (the function selector) and jumps to the matching function body.

use edge_ir::schema::EvmContract;

use crate::{assembler::Assembler, expr_compiler::ExprCompiler};

/// Generate the function dispatcher for a contract.
///
/// The dispatcher compiles the runtime IR which contains the full
/// selector-checking if-else chain with inlined function bodies.
/// Each branch loads the selector from calldata, compares it, and
/// executes the matching function body (which terminates with RETURN/STOP).
pub fn generate_dispatcher(asm: &mut Assembler, contract: &EvmContract) {
    // The runtime IR handles selector loading in each if-condition
    let mut compiler = ExprCompiler::new(asm);
    compiler.compile_expr(&contract.runtime);
}

/// Compute the 4-byte function selector from a function signature.
///
/// `sig` should be in the form "functionName(type1,type2,...)"
pub fn compute_selector(sig: &str) -> [u8; 4] {
    let mut hash = [0u8; 32];
    edge_types::bytes::hash_bytes(&mut hash, &sig.to_owned());
    [hash[0], hash[1], hash[2], hash[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_selectors() {
        // Well-known ERC-20 selectors
        assert_eq!(
            compute_selector("transfer(address,uint256)"),
            [0xa9, 0x05, 0x9c, 0xbb]
        );
        assert_eq!(
            compute_selector("balanceOf(address)"),
            [0x70, 0xa0, 0x82, 0x31]
        );
        assert_eq!(
            compute_selector("approve(address,uint256)"),
            [0x09, 0x5e, 0xa7, 0xb3]
        );
        assert_eq!(compute_selector("totalSupply()"), [0x18, 0x16, 0x0d, 0xdd]);
    }

    #[test]
    fn test_simple_selectors() {
        // Counter contract selectors
        let inc = compute_selector("increment()");
        let dec = compute_selector("decrement()");
        let get = compute_selector("get()");
        let reset = compute_selector("reset()");

        // Just verify they're all different
        assert_ne!(inc, dec);
        assert_ne!(inc, get);
        assert_ne!(inc, reset);
        assert_ne!(dec, get);
        assert_ne!(dec, reset);
        assert_ne!(get, reset);
    }
}
