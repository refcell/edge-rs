#![allow(missing_docs)]

//! Execution-level correctness tests for inline assembly.
//!
//! Every test runs at O0, O1, O2, and O3 to catch optimizer bugs.

use crate::helpers::*;

const ASM: &str = "examples/tests/test_inline_asm.edge";

#[test]
fn test_asm_add() {
    for_all_opt_levels(ASM, |h, o| {
        let r = h.call(selector("asm_add()").to_vec());
        assert!(r.success, "asm_add reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 3, "1+2=3 at O{o}");
    });
}

#[test]
fn test_asm_mul_add() {
    for_all_opt_levels(ASM, |h, o| {
        let r = h.call(selector("asm_mul_add()").to_vec());
        assert!(r.success, "asm_mul_add reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 7, "2*3+1=7 at O{o}");
    });
}

#[test]
fn test_asm_identity() {
    for_all_opt_levels(ASM, |h, o| {
        let r = h.call(selector("asm_identity()").to_vec());
        assert!(r.success, "asm_identity reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 99, "identity(99)=99 at O{o}");
    });
}

#[test]
fn test_asm_hex_literal() {
    for_all_opt_levels(ASM, |h, o| {
        let r = h.call(selector("asm_hex_literal()").to_vec());
        assert!(r.success, "asm_hex_literal reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 255, "0xff=255 at O{o}");
    });
}

#[test]
fn test_asm_caller() {
    for_all_opt_levels(ASM, |h, o| {
        let r = h.call(selector("asm_caller()").to_vec());
        assert!(r.success, "asm_caller reverted at O{o}");
        // CALLER opcode returns msg.sender — which is Address::ZERO in our test setup
        assert_eq!(decode_u256(&r.output), 0, "caller should be 0 at O{o}");
    });
}

#[test]
fn test_asm_local_var() {
    for_all_opt_levels(ASM, |h, o| {
        let r = h.call(selector("asm_local_var()").to_vec());
        assert!(r.success, "asm_local_var reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 30, "10+20=30 at O{o}");
    });
}

#[test]
fn test_asm_computed_local() {
    for_all_opt_levels(ASM, |h, o| {
        let r = h.call(selector("asm_computed_local()").to_vec());
        assert!(r.success, "asm_computed_local reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 50, "(3+7)*5=50 at O{o}");
    });
}
