#![allow(missing_docs)]

//! Execution-level tests for sub-256-bit integer width semantics.
//!
//! Verifies that:
//! - u8/u128 arithmetic truncates correctly
//! - Overflow/underflow at sub-256-bit widths is caught and reverts
//! - Bitwise ops and division work correctly at narrow widths
//! - Compile-time constant overflow is rejected
//! - Mixed-width arithmetic is rejected (requires explicit casts)
//! - Unsuffixed literals adopt the type of the other operand

use edge_driver::compiler::Compiler;

use crate::helpers::*;

// =============================================================================
// u8 checked addition
// =============================================================================

#[test]
fn test_u8_add_ok() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_add_ok(uint8,uint8)"),
        &[encode_u256(100), encode_u256(50)],
    ));
    assert!(r.success, "u8_add_ok(100, 50) reverted unexpectedly");
    assert_eq!(decode_u256(&r.output), 150);
}

#[test]
fn test_u8_add_overflow_reverts() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_add_overflow(uint8,uint8)"),
        &[encode_u256(200), encode_u256(100)],
    ));
    assert!(!r.success, "u8_add_overflow(200, 100) should revert");
}

#[test]
fn test_u8_add_boundary_ok() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 255 + 0 = 255, should succeed
    let r = evm.call(calldata(
        selector("u8_add_ok(uint8,uint8)"),
        &[encode_u256(255), encode_u256(0)],
    ));
    assert!(r.success, "u8_add_ok(255, 0) reverted");
    assert_eq!(decode_u256(&r.output), 255);
}

#[test]
fn test_u8_add_boundary_overflow() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 255 + 1 = 256 > 255 → revert
    let r = evm.call(calldata(
        selector("u8_add_overflow(uint8,uint8)"),
        &[encode_u256(255), encode_u256(1)],
    ));
    assert!(!r.success, "u8_add_overflow(255, 1) should revert");
}

// =============================================================================
// u8 checked subtraction
// =============================================================================

#[test]
fn test_u8_sub_ok() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_sub_ok(uint8,uint8)"),
        &[encode_u256(100), encode_u256(50)],
    ));
    assert!(r.success, "u8_sub_ok(100, 50) reverted");
    assert_eq!(decode_u256(&r.output), 50);
}

#[test]
fn test_u8_sub_underflow_reverts() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_sub_underflow(uint8,uint8)"),
        &[encode_u256(50), encode_u256(100)],
    ));
    assert!(!r.success, "u8_sub_underflow(50, 100) should revert");
}

// =============================================================================
// u8 checked multiplication
// =============================================================================

#[test]
fn test_u8_mul_ok() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_mul_ok(uint8,uint8)"),
        &[encode_u256(10), encode_u256(20)],
    ));
    assert!(r.success, "u8_mul_ok(10, 20) reverted");
    assert_eq!(decode_u256(&r.output), 200);
}

#[test]
fn test_u8_mul_overflow_reverts() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_mul_overflow(uint8,uint8)"),
        &[encode_u256(20), encode_u256(20)],
    ));
    assert!(
        !r.success,
        "u8_mul_overflow(20, 20) should revert (400 > 255)"
    );
}

// =============================================================================
// u8 bitwise and division (always safe)
// =============================================================================

#[test]
fn test_u8_and() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_and(uint8,uint8)"),
        &[encode_u256(0xAB), encode_u256(0x0F)],
    ));
    assert!(r.success, "u8_and reverted");
    assert_eq!(decode_u256(&r.output), 0x0B);
}

#[test]
fn test_u8_div() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_div(uint8,uint8)"),
        &[encode_u256(200), encode_u256(10)],
    ));
    assert!(r.success, "u8_div reverted");
    assert_eq!(decode_u256(&r.output), 20);
}

// =============================================================================
// u128 overflow
// =============================================================================

#[test]
fn test_u128_add_ok() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u128_add_ok(uint128,uint128)"),
        &[encode_u256(1000), encode_u256(2000)],
    ));
    assert!(r.success, "u128_add_ok(1000, 2000) reverted");
    assert_eq!(decode_u256(&r.output), 3000);
}

// =============================================================================
// u8 truncation on OR and SHL
// =============================================================================

#[test]
fn test_u8_or_truncate() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("u8_or_truncate(uint8,uint8)"),
        &[encode_u256(0xF0), encode_u256(0x0F)],
    ));
    assert!(r.success, "u8_or_truncate reverted");
    assert_eq!(decode_u256(&r.output), 0xFF);
}

#[test]
fn test_u8_shl() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 3 << 2 = 12, fits in u8
    let r = evm.call(calldata(
        selector("u8_shl(uint8,uint8)"),
        &[encode_u256(3), encode_u256(2)],
    ));
    assert!(r.success, "u8_shl(3, 2) reverted");
    assert_eq!(decode_u256(&r.output), 12);
}

#[test]
fn test_u8_shl_truncates() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 0x80 << 1 = 0x100 → truncated to 0x00 for u8
    let r = evm.call(calldata(
        selector("u8_shl(uint8,uint8)"),
        &[encode_u256(0x80), encode_u256(1)],
    ));
    assert!(r.success, "u8_shl(0x80, 1) reverted");
    assert_eq!(decode_u256(&r.output), 0);
}

// =============================================================================
// Chained u8 operations
// =============================================================================

#[test]
fn test_u8_chain() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // (10 + 20) * 2 = 60, fits in u8
    let r = evm.call(calldata(
        selector("u8_chain(uint8,uint8)"),
        &[encode_u256(10), encode_u256(20)],
    ));
    assert!(r.success, "u8_chain(10, 20) reverted");
    assert_eq!(decode_u256(&r.output), 60);
}

#[test]
fn test_u8_chain_overflow_reverts() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // (100 + 100) * 2 = 400 > 255 → revert (overflow on mul)
    let r = evm.call(calldata(
        selector("u8_chain(uint8,uint8)"),
        &[encode_u256(100), encode_u256(100)],
    ));
    assert!(!r.success, "u8_chain(100, 100) should revert");
}

// =============================================================================
// Literal type suffix
// =============================================================================

#[test]
fn test_literal_u8_suffix() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("literal_u8_add()"), &[]));
    assert!(r.success, "literal_u8_add() reverted");
    assert_eq!(decode_u256(&r.output), 52); // 42 + 10 = 52
}

#[test]
fn test_literal_u256_suffix() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("literal_u256_add()"), &[]));
    assert!(r.success, "literal_u256_add() reverted");
    assert_eq!(decode_u256(&r.output), 1001);
}

// =============================================================================
// Type casting
// =============================================================================

#[test]
fn test_cast_u256_to_u8() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 300 as u8 = 300 & 0xFF = 44
    let r = evm.call(calldata(
        selector("cast_u256_to_u8(uint256)"),
        &[encode_u256(300)],
    ));
    assert!(r.success, "cast_u256_to_u8(300) reverted");
    assert_eq!(decode_u256(&r.output), 44); // 300 % 256 = 44
}

#[test]
fn test_cast_u8_to_u256() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("cast_u8_to_u256(uint8)"),
        &[encode_u256(42)],
    ));
    assert!(r.success, "cast_u8_to_u256(42) reverted");
    assert_eq!(decode_u256(&r.output), 42);
}

#[test]
fn test_cast_and_add() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 300 as u8 = 44, 44 + 1 = 45
    let r = evm.call(calldata(
        selector("cast_and_add(uint256)"),
        &[encode_u256(300)],
    ));
    assert!(r.success, "cast_and_add(300) reverted");
    assert_eq!(decode_u256(&r.output), 45);
}

#[test]
fn test_cast_and_add_overflow() {
    let bc = compile_contract("examples/tests/test_int_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 255 as u8 = 255, 255 + 1 = 256 > u8 → revert
    let r = evm.call(calldata(
        selector("cast_and_add(uint256)"),
        &[encode_u256(255)],
    ));
    assert!(!r.success, "cast_and_add(255) should revert on u8 overflow");
}

// =============================================================================
// Compile-time constant overflow rejection
// =============================================================================

fn assert_compile_error(source: &str, expected_messages: &[&str], expected_rendered: &[&str]) {
    let mut compiler = Compiler::from_source(source);
    let result = compiler.compile();
    assert!(
        result.is_err(),
        "Expected compilation to fail, but it succeeded.\nSource:\n{source}"
    );

    let messages = compiler.diagnostic_messages();
    let all_messages = messages.join("\n");
    for exp in expected_messages {
        assert!(
            all_messages.contains(exp),
            "Expected message containing '{exp}', got:\n{all_messages}\nSource:\n{source}"
        );
    }

    let rendered = compiler.render_diagnostics();
    for exp in expected_rendered {
        assert!(
            rendered.contains(exp),
            "Expected rendered output containing '{exp}', got:\n{rendered}\nSource:\n{source}"
        );
    }
}

fn assert_compiles(source: &str) {
    let mut compiler = Compiler::from_source(source);
    let result = compiler.compile();
    assert!(
        result.is_ok(),
        "Expected compilation to succeed, but it failed.\nSource:\n{source}\nError: {:?}",
        result.unwrap_err()
    );
}

#[test]
fn test_const_overflow_u8_add() {
    assert_compile_error(
        "contract T { pub fn f() -> (u256) { return 250u8 + 250u8; } }",
        &["attempt to compute", "250_u8 + 250_u8", "would overflow"],
        &["overflows `u8`"],
    );
}

#[test]
fn test_const_overflow_u8_mul() {
    assert_compile_error(
        "contract T { pub fn f() -> (u256) { return 20u8 * 20u8; } }",
        &["attempt to compute", "20_u8 * 20_u8", "would overflow"],
        &["overflows `u8`"],
    );
}

#[test]
fn test_const_overflow_u8_sub() {
    assert_compile_error(
        "contract T { pub fn f() -> (u256) { return 0u8 - 1u8; } }",
        &["attempt to compute", "would overflow"],
        &["overflows `u8`"],
    );
}

#[test]
fn test_const_no_overflow_u8_add() {
    assert_compiles("contract T { pub fn f() -> (u256) { return 100u8 + 50u8; } }");
}

#[test]
fn test_const_no_overflow_u8_boundary() {
    assert_compiles("contract T { pub fn f() -> (u256) { return 200u8 + 55u8; } }");
}

#[test]
fn test_const_overflow_u8_boundary() {
    assert_compile_error(
        "contract T { pub fn f() -> (u256) { return 200u8 + 56u8; } }",
        &["attempt to compute", "200_u8 + 56_u8", "would overflow"],
        &["overflows `u8`"],
    );
}

#[test]
fn test_const_overflow_u128_mul() {
    assert_compile_error(
        "contract T { pub fn f() -> (u256) { let x: u128 = 340282366920938463463374607431768211455u128 * 2u128; return x; } }",
        &["attempt to compute", "would overflow"],
        &["overflows `u128`"],
    );
}

// =============================================================================
// Mixed-width rejection
// =============================================================================

#[test]
fn test_mixed_width_u8_u256_rejected() {
    assert_compile_error(
        "contract T { pub fn f(a: u8, b: u256) -> (u256) { return a + b; } }",
        &["mismatched types", "u8", "u256"],
        &["use an explicit cast"],
    );
}

#[test]
fn test_mixed_width_u8_u128_rejected() {
    assert_compile_error(
        "contract T { pub fn f(a: u8, b: u128) -> (u256) { return a + b; } }",
        &["mismatched types", "u8", "u128"],
        &["use an explicit cast"],
    );
}

// =============================================================================
// Unsuffixed literal type adoption
// =============================================================================

#[test]
fn test_unsuffixed_literal_adopts_u8() {
    // `x * 2` where x: u8 — the `2` should be treated as u8
    assert_compiles("contract T { pub fn f(x: u8) -> (u256) { return x * 2; } }");
}

#[test]
fn test_unsuffixed_literal_adopts_addr() {
    // `a == 0` where a: addr — the `0` should be treated as addr
    assert_compiles(
        "contract T { pub fn f(a: addr) -> (u256) { if (a == 0) { return 1; } return 0; } }",
    );
}

#[test]
fn test_unsuffixed_literal_adopts_bool() {
    // `b == true` where b: bool — both are bool, should work
    assert_compiles(
        "contract T { pub fn f(b: bool) -> (u256) { if (b == true) { return 1; } return 0; } }",
    );
}

#[test]
fn test_explicit_cast_fixes_mismatch() {
    // Cast makes mixed-width work
    assert_compiles(
        "contract T { pub fn f(a: u8, b: u256) -> (u256) { return (a as u256) + b; } }",
    );
}
