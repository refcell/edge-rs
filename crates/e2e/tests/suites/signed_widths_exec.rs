#![allow(missing_docs)]

//! Execution-level tests for signed sub-256-bit integer width semantics.
//!
//! Verifies that:
//! - i8 checked arithmetic reverts on overflow/underflow
//! - Signed division uses SDIV
//! - Signed comparisons use SLT/SGT
//! - Unary negation works
//! - UnsafeAdd/Sub/Mul wrap without reverting
//! - Casts between signed and unsigned preserve/reinterpret bits correctly

use edge_driver::compiler::Compiler;

use crate::helpers::*;

/// Encode a signed i64 value as a 32-byte two's complement ABI encoding.
/// Positive values are zero-padded, negative values are sign-extended (0xFF padded).
fn encode_i256(val: i64) -> [u8; 32] {
    let mut out = if val < 0 { [0xFFu8; 32] } else { [0u8; 32] };
    out[24..].copy_from_slice(&val.to_be_bytes());
    out
}

/// Decode a 32-byte two's complement return value as a signed i64.
fn decode_i256(output: &[u8]) -> i64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    // Check if negative: top byte has high bit set
    let negative = output[0] & 0x80 != 0;
    if negative {
        // All upper bytes should be 0xFF for values fitting in i64
        assert!(
            output[0..24].iter().all(|&b| b == 0xFF),
            "signed value too large for i64"
        );
    } else {
        assert_eq!(&output[0..24], &[0u8; 24], "signed value too large for i64");
    }
    i64::from_be_bytes(output[24..32].try_into().unwrap())
}

// =============================================================================
// i8 checked addition
// =============================================================================

#[test]
fn test_i8_add_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 50 + 30 = 80, fits in i8 (-128..127)
    let r = evm.call(calldata(
        selector("i8_add_ok(int8,int8)"),
        &[encode_i256(50), encode_i256(30)],
    ));
    assert!(r.success, "i8_add_ok(50, 30) reverted unexpectedly");
    assert_eq!(decode_i256(&r.output), 80);
}

#[test]
fn test_i8_add_negative_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -50 + 30 = -20, fits in i8
    let r = evm.call(calldata(
        selector("i8_add_ok(int8,int8)"),
        &[encode_i256(-50), encode_i256(30)],
    ));
    assert!(r.success, "i8_add_ok(-50, 30) reverted unexpectedly");
    assert_eq!(decode_i256(&r.output), -20);
}

#[test]
fn test_i8_add_both_negative_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -30 + (-40) = -70, fits in i8
    let r = evm.call(calldata(
        selector("i8_add_ok(int8,int8)"),
        &[encode_i256(-30), encode_i256(-40)],
    ));
    assert!(r.success, "i8_add_ok(-30, -40) reverted unexpectedly");
    assert_eq!(decode_i256(&r.output), -70);
}

#[test]
fn test_i8_add_overflow_reverts() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 100 + 100 = 200 > 127 → revert
    let r = evm.call(calldata(
        selector("i8_add_overflow(int8,int8)"),
        &[encode_i256(100), encode_i256(100)],
    ));
    assert!(!r.success, "i8_add_overflow(100, 100) should revert");
}

#[test]
fn test_i8_add_boundary_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 127 + 0 = 127, exact max
    let r = evm.call(calldata(
        selector("i8_add_ok(int8,int8)"),
        &[encode_i256(127), encode_i256(0)],
    ));
    assert!(r.success, "i8_add_ok(127, 0) reverted");
    assert_eq!(decode_i256(&r.output), 127);
}

#[test]
fn test_i8_add_boundary_overflow() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 127 + 1 = 128 > 127 → revert
    let r = evm.call(calldata(
        selector("i8_add_overflow(int8,int8)"),
        &[encode_i256(127), encode_i256(1)],
    ));
    assert!(!r.success, "i8_add_overflow(127, 1) should revert");
}

#[test]
fn test_i8_add_negative_boundary_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -128 + 0 = -128, exact min
    let r = evm.call(calldata(
        selector("i8_add_ok(int8,int8)"),
        &[encode_i256(-128), encode_i256(0)],
    ));
    assert!(r.success, "i8_add_ok(-128, 0) reverted");
    assert_eq!(decode_i256(&r.output), -128);
}

// =============================================================================
// i8 checked subtraction
// =============================================================================

#[test]
fn test_i8_sub_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 50 - 30 = 20
    let r = evm.call(calldata(
        selector("i8_sub_ok(int8,int8)"),
        &[encode_i256(50), encode_i256(30)],
    ));
    assert!(r.success, "i8_sub_ok(50, 30) reverted");
    assert_eq!(decode_i256(&r.output), 20);
}

#[test]
fn test_i8_sub_negative_result_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 30 - 50 = -20, fits in i8
    let r = evm.call(calldata(
        selector("i8_sub_ok(int8,int8)"),
        &[encode_i256(30), encode_i256(50)],
    ));
    assert!(r.success, "i8_sub_ok(30, 50) reverted");
    assert_eq!(decode_i256(&r.output), -20);
}

#[test]
fn test_i8_sub_underflow_reverts() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -100 - 100 = -200 < -128 → revert
    let r = evm.call(calldata(
        selector("i8_sub_underflow(int8,int8)"),
        &[encode_i256(-100), encode_i256(100)],
    ));
    assert!(!r.success, "i8_sub_underflow(-100, 100) should revert");
}

// =============================================================================
// i8 checked multiplication
// =============================================================================

#[test]
fn test_i8_mul_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 10 * 10 = 100, fits in i8
    let r = evm.call(calldata(
        selector("i8_mul_ok(int8,int8)"),
        &[encode_i256(10), encode_i256(10)],
    ));
    assert!(r.success, "i8_mul_ok(10, 10) reverted");
    assert_eq!(decode_i256(&r.output), 100);
}

#[test]
fn test_i8_mul_negative_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -10 * 10 = -100, fits in i8
    let r = evm.call(calldata(
        selector("i8_mul_ok(int8,int8)"),
        &[encode_i256(-10), encode_i256(10)],
    ));
    assert!(r.success, "i8_mul_ok(-10, 10) reverted");
    assert_eq!(decode_i256(&r.output), -100);
}

#[test]
fn test_i8_mul_overflow_reverts() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 20 * 20 = 400 > 127 → revert
    let r = evm.call(calldata(
        selector("i8_mul_overflow(int8,int8)"),
        &[encode_i256(20), encode_i256(20)],
    ));
    assert!(!r.success, "i8_mul_overflow(20, 20) should revert");
}

// =============================================================================
// i8 signed division (SDIV)
// =============================================================================

#[test]
fn test_i8_sdiv_positive() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 100 / 10 = 10
    let r = evm.call(calldata(
        selector("i8_sdiv(int8,int8)"),
        &[encode_i256(100), encode_i256(10)],
    ));
    assert!(r.success, "i8_sdiv(100, 10) reverted");
    assert_eq!(decode_i256(&r.output), 10);
}

#[test]
fn test_i8_sdiv_negative_dividend() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -100 / 10 = -10 (signed division)
    let r = evm.call(calldata(
        selector("i8_sdiv(int8,int8)"),
        &[encode_i256(-100), encode_i256(10)],
    ));
    assert!(r.success, "i8_sdiv(-100, 10) reverted");
    assert_eq!(decode_i256(&r.output), -10);
}

#[test]
fn test_i8_sdiv_negative_divisor() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 100 / -10 = -10 (signed division)
    let r = evm.call(calldata(
        selector("i8_sdiv(int8,int8)"),
        &[encode_i256(100), encode_i256(-10)],
    ));
    assert!(r.success, "i8_sdiv(100, -10) reverted");
    assert_eq!(decode_i256(&r.output), -10);
}

#[test]
fn test_i8_sdiv_both_negative() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -100 / -10 = 10 (negative / negative = positive)
    let r = evm.call(calldata(
        selector("i8_sdiv(int8,int8)"),
        &[encode_i256(-100), encode_i256(-10)],
    ));
    assert!(r.success, "i8_sdiv(-100, -10) reverted");
    assert_eq!(decode_i256(&r.output), 10);
}

// =============================================================================
// i8 negation
// =============================================================================

#[test]
fn test_i8_negate_positive() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -42 (negate 42)
    let r = evm.call(calldata(selector("i8_negate(int8)"), &[encode_i256(42)]));
    assert!(r.success, "i8_negate(42) reverted");
    assert_eq!(decode_i256(&r.output), -42);
}

#[test]
fn test_i8_negate_negative() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -(-42) = 42
    let r = evm.call(calldata(selector("i8_negate(int8)"), &[encode_i256(-42)]));
    assert!(r.success, "i8_negate(-42) reverted");
    assert_eq!(decode_i256(&r.output), 42);
}

#[test]
fn test_i8_negate_zero() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -0 = 0
    let r = evm.call(calldata(selector("i8_negate(int8)"), &[encode_i256(0)]));
    assert!(r.success, "i8_negate(0) reverted");
    assert_eq!(decode_i256(&r.output), 0);
}

// =============================================================================
// i8 signed comparisons (SLT / SGT)
// =============================================================================

#[test]
fn test_i8_slt_true() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -50 < 50 → 1
    let r = evm.call(calldata(
        selector("i8_slt(int8,int8)"),
        &[encode_i256(-50), encode_i256(50)],
    ));
    assert!(r.success, "i8_slt(-50, 50) reverted");
    assert_eq!(decode_u256(&r.output), 1);
}

#[test]
fn test_i8_slt_false() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 50 < -50 → 0
    let r = evm.call(calldata(
        selector("i8_slt(int8,int8)"),
        &[encode_i256(50), encode_i256(-50)],
    ));
    assert!(r.success, "i8_slt(50, -50) reverted");
    assert_eq!(decode_u256(&r.output), 0);
}

#[test]
fn test_i8_sgt_true() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 50 > -50 → 1
    let r = evm.call(calldata(
        selector("i8_sgt(int8,int8)"),
        &[encode_i256(50), encode_i256(-50)],
    ));
    assert!(r.success, "i8_sgt(50, -50) reverted");
    assert_eq!(decode_u256(&r.output), 1);
}

#[test]
fn test_i8_sgt_false() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -50 > 50 → 0
    let r = evm.call(calldata(
        selector("i8_sgt(int8,int8)"),
        &[encode_i256(-50), encode_i256(50)],
    ));
    assert!(r.success, "i8_sgt(-50, 50) reverted");
    assert_eq!(decode_u256(&r.output), 0);
}

// =============================================================================
// Unsafe (unchecked) signed arithmetic
// =============================================================================

#[test]
fn test_i8_unsafe_add_ok() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // unsafe_add(50, 30) = 80, no revert
    let r = evm.call(calldata(
        selector("i8_unsafe_add(int8,int8)"),
        &[encode_i256(50), encode_i256(30)],
    ));
    assert!(r.success, "i8_unsafe_add(50, 30) reverted");
    assert_eq!(decode_i256(&r.output), 80);
}

#[test]
fn test_i8_unsafe_add_wraps() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // unsafe_add(100, 100) = 200 — UnsafeAdd bypasses all truncation,
    // result is raw u256 add (200), returned without sign-extend/mask
    let r = evm.call(calldata(
        selector("i8_unsafe_add(int8,int8)"),
        &[encode_i256(100), encode_i256(100)],
    ));
    assert!(
        r.success,
        "i8_unsafe_add(100, 100) should NOT revert (unsafe)"
    );
    assert_eq!(decode_i256(&r.output), 200);
}

#[test]
fn test_i8_unsafe_sub_wraps() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // unsafe_sub(-100, 100) = -200 in u256 two's complement — no truncation
    let r = evm.call(calldata(
        selector("i8_unsafe_sub(int8,int8)"),
        &[encode_i256(-100), encode_i256(100)],
    ));
    assert!(
        r.success,
        "i8_unsafe_sub(-100, 100) should NOT revert (unsafe)"
    );
    assert_eq!(decode_i256(&r.output), -200);
}

#[test]
fn test_i8_unsafe_mul_wraps() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // unsafe_mul(20, 20) = 400 — raw u256 mul, no truncation
    let r = evm.call(calldata(
        selector("i8_unsafe_mul(int8,int8)"),
        &[encode_i256(20), encode_i256(20)],
    ));
    assert!(
        r.success,
        "i8_unsafe_mul(20, 20) should NOT revert (unsafe)"
    );
    assert_eq!(decode_i256(&r.output), 400);
}

// =============================================================================
// Casts between signed and unsigned
// =============================================================================

#[test]
fn test_cast_i8_to_u8_positive() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 42i8 as u8 = 42
    let r = evm.call(calldata(
        selector("cast_i8_to_u8(int8)"),
        &[encode_i256(42)],
    ));
    assert!(r.success, "cast_i8_to_u8(42) reverted");
    assert_eq!(decode_u256(&r.output), 42);
}

#[test]
fn test_cast_i8_to_u8_negative() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // -1i8 as u8 = 255 (0xFF)
    let r = evm.call(calldata(
        selector("cast_i8_to_u8(int8)"),
        &[encode_i256(-1)],
    ));
    assert!(r.success, "cast_i8_to_u8(-1) reverted");
    assert_eq!(decode_u256(&r.output), 255);
}

#[test]
fn test_cast_u8_to_i8_small() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 42u8 as i8 = 42 (fits in positive range)
    let r = evm.call(calldata(
        selector("cast_u8_to_i8(uint8)"),
        &[encode_u256(42)],
    ));
    assert!(r.success, "cast_u8_to_i8(42) reverted");
    assert_eq!(decode_i256(&r.output), 42);
}

#[test]
fn test_cast_u8_to_i8_high() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 200u8 as i8 = -56 (0xC8 sign-extended)
    let r = evm.call(calldata(
        selector("cast_u8_to_i8(uint8)"),
        &[encode_u256(200)],
    ));
    assert!(r.success, "cast_u8_to_i8(200) reverted");
    assert_eq!(decode_i256(&r.output), -56);
}

#[test]
fn test_cast_i8_to_u256_positive() {
    let bc = compile_contract("examples/tests/test_signed_widths.edge");
    let mut evm = EvmHandle::new(bc);
    // 42i8 as u256 = 42
    let r = evm.call(calldata(
        selector("cast_i8_to_u256(int8)"),
        &[encode_i256(42)],
    ));
    assert!(r.success, "cast_i8_to_u256(42) reverted");
    assert_eq!(decode_u256(&r.output), 42);
}

// =============================================================================
// Compile-time constant overflow rejection (signed)
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
fn test_mixed_width_i8_i256_rejected() {
    assert_compile_error(
        "contract T { pub fn f(a: i8, b: i256) -> (i256) { return a + b; } }",
        &["mismatched types", "i8", "i256"],
        &["use an explicit cast"],
    );
}

#[test]
fn test_mixed_width_i8_u8_rejected() {
    assert_compile_error(
        "contract T { pub fn f(a: i8, b: u8) -> (u256) { return a + b; } }",
        &["mismatched types", "i8", "u8"],
        &["use an explicit cast"],
    );
}

#[test]
fn test_unsuffixed_literal_adopts_i8() {
    // `x + 2` where x: i8 — the `2` should be treated as i8
    assert_compiles("contract T { pub fn f(x: i8) -> (i8) { return x + 2; } }");
}

#[test]
fn test_explicit_cast_fixes_signed_mismatch() {
    assert_compiles(
        "contract T { pub fn f(a: i8, b: i256) -> (i256) { return (a as i256) + b; } }",
    );
}
