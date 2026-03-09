#![allow(missing_docs)]

//! Execution-level acceptance tests for utility library contracts.
//!
//! Tests compile math.edge and bits.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify pure-function output correctness.
//!
//! ## Compiler caveats
//! Top-level constants (`WAD`, `ADDR_MASK`, etc.) resolve to 0 when referenced
//! by name in function bodies, so WAD-based helpers (`wad_mul`, `wad_div`) are
//! not tested here. Functions that only use parameters and integer literals
//! work correctly.

use crate::helpers::*;

// =============================================================================
// math.edge — safe arithmetic
// =============================================================================

#[test]
fn test_math_safe_add() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("safe_add(uint256,uint256)"),
        &[encode_u256(10), encode_u256(20)],
    ));
    assert!(r.success, "safe_add reverted");
    assert_eq!(
        decode_u256(&r.output),
        30,
        "safe_add(10, 20) should return 30"
    );
}

#[test]
fn test_math_safe_sub() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("safe_sub(uint256,uint256)"),
        &[encode_u256(30), encode_u256(10)],
    ));
    assert!(r.success, "safe_sub reverted");
    assert_eq!(
        decode_u256(&r.output),
        20,
        "safe_sub(30, 10) should return 20"
    );
}

#[test]
fn test_math_saturating_sub_underflow() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 5 - 10 would underflow; saturating_sub returns 0
    let r = evm.call(calldata(
        selector("saturating_sub(uint256,uint256)"),
        &[encode_u256(5), encode_u256(10)],
    ));
    assert!(r.success, "saturating_sub reverted");
    assert_eq!(
        decode_u256(&r.output),
        0,
        "saturating_sub(5, 10) should return 0"
    );
}

#[test]
fn test_math_saturating_sub_no_underflow() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("saturating_sub(uint256,uint256)"),
        &[encode_u256(10), encode_u256(5)],
    ));
    assert!(r.success, "saturating_sub reverted");
    assert_eq!(
        decode_u256(&r.output),
        5,
        "saturating_sub(10, 5) should return 5"
    );
}

#[test]
fn test_math_max() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("max(uint256,uint256)"),
        &[encode_u256(5), encode_u256(10)],
    ));
    assert!(r.success, "max reverted");
    assert_eq!(decode_u256(&r.output), 10, "max(5, 10) should return 10");
}

#[test]
fn test_math_max_equal() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("max(uint256,uint256)"),
        &[encode_u256(7), encode_u256(7)],
    ));
    assert!(r.success, "max reverted");
    assert_eq!(decode_u256(&r.output), 7, "max(7, 7) should return 7");
}

#[test]
fn test_math_min() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("min(uint256,uint256)"),
        &[encode_u256(5), encode_u256(10)],
    ));
    assert!(r.success, "min reverted");
    assert_eq!(decode_u256(&r.output), 5, "min(5, 10) should return 5");
}

#[test]
fn test_math_clamp_within_range() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("clamp(uint256,uint256,uint256)"),
        &[encode_u256(15), encode_u256(10), encode_u256(20)],
    ));
    assert!(r.success, "clamp reverted");
    assert_eq!(
        decode_u256(&r.output),
        15,
        "clamp(15, 10, 20) should return 15"
    );
}

#[test]
fn test_math_clamp_below_lo() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("clamp(uint256,uint256,uint256)"),
        &[encode_u256(5), encode_u256(10), encode_u256(20)],
    ));
    assert!(r.success, "clamp reverted");
    assert_eq!(
        decode_u256(&r.output),
        10,
        "clamp(5, 10, 20) should clamp to lo=10"
    );
}

#[test]
fn test_math_clamp_above_hi() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("clamp(uint256,uint256,uint256)"),
        &[encode_u256(25), encode_u256(10), encode_u256(20)],
    ));
    assert!(r.success, "clamp reverted");
    assert_eq!(
        decode_u256(&r.output),
        20,
        "clamp(25, 10, 20) should clamp to hi=20"
    );
}

#[test]
fn test_math_mul_div_down_exact() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 6 * 4 / 3 = 8 (exact)
    let r = evm.call(calldata(
        selector("mul_div_down(uint256,uint256,uint256)"),
        &[encode_u256(6), encode_u256(4), encode_u256(3)],
    ));
    assert!(r.success, "mul_div_down reverted");
    assert_eq!(
        decode_u256(&r.output),
        8,
        "mul_div_down(6, 4, 3) should return 8"
    );
}

#[test]
fn test_math_mul_div_down_truncates() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 10 * 3 / 4 = 7.5 → truncates to 7
    let r = evm.call(calldata(
        selector("mul_div_down(uint256,uint256,uint256)"),
        &[encode_u256(10), encode_u256(3), encode_u256(4)],
    ));
    assert!(r.success, "mul_div_down reverted");
    assert_eq!(
        decode_u256(&r.output),
        7,
        "mul_div_down(10, 3, 4) should truncate to 7"
    );
}

#[test]
fn test_math_mul_div_up_exact() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 6 * 4 / 3 = 8 (exact, no rounding up)
    let r = evm.call(calldata(
        selector("mul_div_up(uint256,uint256,uint256)"),
        &[encode_u256(6), encode_u256(4), encode_u256(3)],
    ));
    assert!(r.success, "mul_div_up reverted");
    assert_eq!(
        decode_u256(&r.output),
        8,
        "mul_div_up(6, 4, 3) should return 8"
    );
}

#[test]
fn test_math_mul_div_up_rounds_up() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 10 * 3 / 4 = 7.5 → rounds up to 8
    let r = evm.call(calldata(
        selector("mul_div_up(uint256,uint256,uint256)"),
        &[encode_u256(10), encode_u256(3), encode_u256(4)],
    ));
    assert!(r.success, "mul_div_up reverted");
    assert_eq!(
        decode_u256(&r.output),
        8,
        "mul_div_up(10, 3, 4) should round up to 8"
    );
}

#[test]
fn test_math_unknown_selector_reverts() {
    let bc = compile_contract("std/math.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// bits.edge — bitwise operations
// =============================================================================

#[test]
fn test_bits_most_significant_bit_zero() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("most_significant_bit(uint256)"),
        &[encode_u256(0)],
    ));
    assert!(r.success, "most_significant_bit reverted");
    assert_eq!(decode_u256(&r.output), 0, "msb(0) should return 0");
}

#[test]
fn test_bits_most_significant_bit_one() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("most_significant_bit(uint256)"),
        &[encode_u256(1)],
    ));
    assert!(r.success, "most_significant_bit reverted");
    assert_eq!(decode_u256(&r.output), 0, "msb(1) should return 0 (bit 0)");
}

#[test]
fn test_bits_most_significant_bit_powers_of_two() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // msb(2) = 1, msb(4) = 2, msb(8) = 3, msb(256) = 8
    for (input, expected) in [(2u64, 1u64), (4, 2), (8, 3), (256, 8)] {
        let r = evm.call(calldata(
            selector("most_significant_bit(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(r.success, "most_significant_bit({input}) reverted");
        assert_eq!(
            decode_u256(&r.output),
            expected,
            "msb({input}) should return {expected}"
        );
    }
}

#[test]
fn test_bits_popcount_zero() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("popcount(uint256)"), &[encode_u256(0)]));
    assert!(r.success, "popcount reverted");
    assert_eq!(decode_u256(&r.output), 0, "popcount(0) should return 0");
}

#[test]
fn test_bits_popcount_values() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    for (input, expected) in [(1u64, 1u64), (3, 2), (7, 3), (255, 8)] {
        let r = evm.call(calldata(
            selector("popcount(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(r.success, "popcount({input}) reverted");
        assert_eq!(
            decode_u256(&r.output),
            expected,
            "popcount({input}) should return {expected}"
        );
    }
}

#[test]
fn test_bits_is_power_of_two_true() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    for input in [1u64, 2, 4, 8, 16, 256] {
        let r = evm.call(calldata(
            selector("is_power_of_two(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(r.success, "is_power_of_two({input}) reverted");
        assert!(
            decode_bool(&r.output),
            "is_power_of_two({input}) should be true"
        );
    }
}

#[test]
fn test_bits_is_power_of_two_false() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    for input in [0u64, 3, 5, 6, 7, 9, 10] {
        let r = evm.call(calldata(
            selector("is_power_of_two(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(r.success, "is_power_of_two({input}) reverted");
        assert!(
            !decode_bool(&r.output),
            "is_power_of_two({input}) should be false"
        );
    }
}

#[test]
fn test_bits_extract_bit() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // 5 = 0b101: bit0=1, bit1=0, bit2=1
    for (pos, expected) in [(0u64, 1u64), (1, 0), (2, 1), (3, 0)] {
        let r = evm.call(calldata(
            selector("extract_bit(uint256,uint256)"),
            &[encode_u256(5), encode_u256(pos)],
        ));
        assert!(r.success, "extract_bit(5, {pos}) reverted");
        assert_eq!(
            decode_u256(&r.output),
            expected,
            "extract_bit(5, {pos}) should return {expected}"
        );
    }
}

#[test]
fn test_bits_set_bit() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // set_bit(4, 0) = 5 (4=100b, set bit 0 → 101b=5)
    let r = evm.call(calldata(
        selector("set_bit(uint256,uint256)"),
        &[encode_u256(4), encode_u256(0)],
    ));
    assert!(r.success, "set_bit reverted");
    assert_eq!(decode_u256(&r.output), 5, "set_bit(4, 0) should return 5");
}

#[test]
fn test_bits_clear_bit() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // clear_bit(5, 0) = 4 (5=101b, clear bit 0 → 100b=4)
    let r = evm.call(calldata(
        selector("clear_bit(uint256,uint256)"),
        &[encode_u256(5), encode_u256(0)],
    ));
    assert!(r.success, "clear_bit reverted");
    assert_eq!(decode_u256(&r.output), 4, "clear_bit(5, 0) should return 4");
}

#[test]
fn test_bits_toggle_bit() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // toggle_bit(5, 2) = 1 (5=101b, toggle bit 2 → 001b=1)
    let r = evm.call(calldata(
        selector("toggle_bit(uint256,uint256)"),
        &[encode_u256(5), encode_u256(2)],
    ));
    assert!(r.success, "toggle_bit reverted");
    assert_eq!(
        decode_u256(&r.output),
        1,
        "toggle_bit(5, 2) should return 1"
    );
}

#[test]
fn test_bits_least_significant_bit_zero() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // lsb(0) = 256 (no bits set)
    let r = evm.call(calldata(
        selector("least_significant_bit(uint256)"),
        &[encode_u256(0)],
    ));
    assert!(r.success, "least_significant_bit(0) reverted");
    assert_eq!(
        decode_u256(&r.output),
        256,
        "lsb(0) should return 256 (sentinel)"
    );
}

#[test]
fn test_bits_unknown_selector_reverts() {
    let bc = compile_contract("std/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// bytes.edge — b32 utilities (functions using only parameters work correctly;
//              functions that reference top-level named constants like ADDR_MASK
//              resolve those constants to 0 at this compiler stage)
// =============================================================================

#[test]
fn test_bytes_is_zero_true() {
    let bc = compile_contract("std/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("is_zero(bytes32)"), &[[0u8; 32]]));
    assert!(r.success, "is_zero reverted");
    assert!(decode_bool(&r.output), "is_zero(0) should return true");
}

#[test]
fn test_bytes_is_zero_false() {
    let bc = compile_contract("std/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("is_zero(bytes32)"), &[encode_u256(1)]));
    assert!(r.success, "is_zero reverted");
    assert!(!decode_bool(&r.output), "is_zero(1) should return false");
}

#[test]
fn test_bytes_left_pad_zero_shift() {
    let bc = compile_contract("std/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    // left_pad(0xff, 0) = 0xff (shift by 0 bytes = no shift)
    let r = evm.call(calldata(
        selector("left_pad(uint256,uint256)"),
        &[encode_u256(0xff), encode_u256(0)],
    ));
    assert!(r.success, "left_pad reverted");
    assert_eq!(
        decode_u256(&r.output),
        0xff,
        "left_pad(0xff, 0) should return 0xff"
    );
}

#[test]
fn test_bytes_left_pad_one_byte() {
    let bc = compile_contract("std/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    // left_pad(0x01, 1) = 0x0100 (shift left by 8 bits)
    let r = evm.call(calldata(
        selector("left_pad(uint256,uint256)"),
        &[encode_u256(1), encode_u256(1)],
    ));
    assert!(r.success, "left_pad reverted");
    assert_eq!(
        decode_u256(&r.output),
        0x100,
        "left_pad(1, 1) should return 0x100"
    );
}

#[test]
fn test_bytes_unknown_selector_reverts() {
    let bc = compile_contract("std/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}
