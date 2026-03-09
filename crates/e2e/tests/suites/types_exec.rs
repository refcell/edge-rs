#![allow(missing_docs)]

//! Execution-level tests for composite types, inlined functions, and array features.
//!
//! Every test runs at O0, O1, O2, and O3 to catch optimizer bugs that change
//! program semantics.

use crate::helpers::*;

// =============================================================================
// Struct execution tests (examples/test_structs.edge)
// =============================================================================

const STRUCTS: &str = "examples/tests/test_structs.edge";

#[test]
fn test_struct_field_x() {
    for_all_opt_levels(STRUCTS, |h, o| {
        let (ok, out) = h.call(calldata(selector("point_x()"), &[]));
        assert!(ok, "point_x() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "point_x() wrong at O{o}");
    });
}

#[test]
fn test_struct_field_y() {
    for_all_opt_levels(STRUCTS, |h, o| {
        let (ok, out) = h.call(calldata(selector("point_y()"), &[]));
        assert!(ok, "point_y() reverted at O{o}");
        assert_eq!(decode_u256(&out), 99, "point_y() wrong at O{o}");
    });
}

#[test]
fn test_struct_field_sum() {
    for_all_opt_levels(STRUCTS, |h, o| {
        let (ok, out) = h.call(calldata(selector("point_sum()"), &[]));
        assert!(ok, "point_sum() reverted at O{o}");
        assert_eq!(decode_u256(&out), 30, "point_sum() wrong at O{o}");
    });
}

#[test]
fn test_struct_two_structs() {
    for_all_opt_levels(STRUCTS, |h, o| {
        let (ok, out) = h.call(calldata(selector("two_structs()"), &[]));
        assert!(ok, "two_structs() reverted at O{o}");
        assert_eq!(decode_u256(&out), 5, "two_structs() wrong at O{o}");
    });
}

// =============================================================================
// Enum execution tests (examples/test_enums2.edge)
// =============================================================================

const ENUMS: &str = "examples/tests/test_enums2.edge";

#[test]
fn test_enum_direction_north() {
    for_all_opt_levels(ENUMS, |h, o| {
        let (ok, out) = h.call(calldata(selector("direction_north()"), &[]));
        assert!(ok, "direction_north() reverted at O{o}");
        assert_eq!(decode_u256(&out), 1, "direction_north() wrong at O{o}");
    });
}

#[test]
fn test_enum_direction_west() {
    for_all_opt_levels(ENUMS, |h, o| {
        let (ok, out) = h.call(calldata(selector("direction_west()"), &[]));
        assert!(ok, "direction_west() reverted at O{o}");
        assert_eq!(decode_u256(&out), 4, "direction_west() wrong at O{o}");
    });
}

#[test]
fn test_enum_result_ok_value() {
    for_all_opt_levels(ENUMS, |h, o| {
        let (ok, out) = h.call(calldata(selector("result_ok_value()"), &[]));
        assert!(ok, "result_ok_value() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "result_ok_value() wrong at O{o}");
    });
}

#[test]
fn test_enum_result_err_value() {
    for_all_opt_levels(ENUMS, |h, o| {
        let (ok, out) = h.call(calldata(selector("result_err_value()"), &[]));
        assert!(ok, "result_err_value() reverted at O{o}");
        assert_eq!(decode_u256(&out), 99, "result_err_value() wrong at O{o}");
    });
}

#[test]
fn test_enum_is_north_true() {
    for_all_opt_levels(ENUMS, |h, o| {
        let (ok, out) = h.call(calldata(
            selector("is_north_check(uint256)"),
            &[encode_u256(0)],
        ));
        assert!(ok, "is_north_check(0) reverted at O{o}");
        assert_eq!(decode_u256(&out), 1, "is_north_check(0) wrong at O{o}");
    });
}

#[test]
fn test_enum_is_north_false() {
    for_all_opt_levels(ENUMS, |h, o| {
        let (ok, out) = h.call(calldata(
            selector("is_north_check(uint256)"),
            &[encode_u256(1)],
        ));
        assert!(ok, "is_north_check(1) reverted at O{o}");
        assert_eq!(decode_u256(&out), 0, "is_north_check(1) wrong at O{o}");
    });
}

// =============================================================================
// Array execution tests (examples/test_arrays.edge)
// =============================================================================

const ARRAYS: &str = "examples/tests/test_arrays.edge";

#[test]
fn test_array_element_access() {
    for_all_opt_levels(ARRAYS, |h, o| {
        let (ok, out) = h.call(calldata(selector("element_access()"), &[]));
        assert!(ok, "element_access() reverted at O{o}");
        assert_eq!(decode_u256(&out), 20, "element_access() wrong at O{o}");
    });
}

#[test]
fn test_array_sum() {
    for_all_opt_levels(ARRAYS, |h, o| {
        let (ok, out) = h.call(calldata(selector("sum_array()"), &[]));
        assert!(ok, "sum_array() reverted at O{o}");
        assert_eq!(decode_u256(&out), 100, "sum_array() wrong at O{o}");
    });
}

#[test]
fn test_storage_array_set_get() {
    for_all_opt_levels(ARRAYS, |h, o| {
        let (ok, _) = h.call(calldata(
            selector("set(uint256,uint256)"),
            &[encode_u256(2), encode_u256(777)],
        ));
        assert!(ok, "set(2, 777) reverted at O{o}");
        let (ok, out) = h.call(calldata(selector("get(uint256)"), &[encode_u256(2)]));
        assert!(ok, "get(2) reverted at O{o}");
        assert_eq!(decode_u256(&out), 777, "get(2) wrong at O{o}");
    });
}

#[test]
fn test_array_slice_sum() {
    for_all_opt_levels(ARRAYS, |h, o| {
        let (ok, out) = h.call(calldata(selector("slice_sum()"), &[]));
        assert!(ok, "slice_sum() reverted at O{o}");
        // arr = [10, 20, 30, 40, 50], slice = arr[1:3] = [20, 30], sum = 50
        assert_eq!(decode_u256(&out), 50, "slice_sum() wrong at O{o}");
    });
}

// =============================================================================
// Inlined function execution tests (examples/test_inline.edge)
// =============================================================================

const INLINE: &str = "examples/tests/test_inline.edge";

#[test]
fn test_inline_double() {
    for_all_opt_levels(INLINE, |h, o| {
        let (ok, out) = h.call(calldata(selector("double_val()"), &[]));
        assert!(ok, "double_val() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "double_val() wrong at O{o}");
    });
}

#[test]
fn test_inline_add_offset() {
    for_all_opt_levels(INLINE, |h, o| {
        let (ok, out) = h.call(calldata(selector("add_offset_val()"), &[]));
        assert!(ok, "add_offset_val() reverted at O{o}");
        // 5 + 7 + 10 = 22
        assert_eq!(decode_u256(&out), 22, "add_offset_val() wrong at O{o}");
    });
}

#[test]
fn test_inline_weighted_sum() {
    for_all_opt_levels(INLINE, |h, o| {
        let (ok, out) = h.call(calldata(selector("weighted_sum_val()"), &[]));
        assert!(ok, "weighted_sum_val() reverted at O{o}");
        // 4*3 + 6*5 = 12 + 30 = 42
        assert_eq!(decode_u256(&out), 42, "weighted_sum_val() wrong at O{o}");
    });
}

#[test]
fn test_inline_triple() {
    for_all_opt_levels(INLINE, |h, o| {
        let (ok, out) = h.call(calldata(selector("triple_val()"), &[]));
        assert!(ok, "triple_val() reverted at O{o}");
        // triple(10) = double(10) + 10 = 20 + 10 = 30
        assert_eq!(decode_u256(&out), 30, "triple_val() wrong at O{o}");
    });
}

#[test]
fn test_inline_in_branch() {
    for_all_opt_levels(INLINE, |h, o| {
        // x=20 > 10: double(20) = 40
        let (ok, out) = h.call(calldata(
            selector("inline_in_branch(uint256)"),
            &[encode_u256(20)],
        ));
        assert!(ok, "inline_in_branch(20) reverted at O{o}");
        assert_eq!(decode_u256(&out), 40, "inline_in_branch(20) wrong at O{o}");

        // x=5 <= 10: add_offset(5, 1) = 5 + 1 + 10 = 16
        let (ok, out) = h.call(calldata(
            selector("inline_in_branch(uint256)"),
            &[encode_u256(5)],
        ));
        assert!(ok, "inline_in_branch(5) reverted at O{o}");
        assert_eq!(decode_u256(&out), 16, "inline_in_branch(5) wrong at O{o}");
    });
}

#[test]
fn test_inline_in_loop() {
    for_all_opt_levels(INLINE, |h, o| {
        let (ok, out) = h.call(calldata(selector("inline_in_loop()"), &[]));
        assert!(ok, "inline_in_loop() reverted at O{o}");
        // double(1) + double(2) + double(3) + double(4) = 2 + 4 + 6 + 8 = 20
        assert_eq!(decode_u256(&out), 20, "inline_in_loop() wrong at O{o}");
    });
}

// =============================================================================
// Merkle / array params execution tests (examples/test_merkle.edge)
// =============================================================================

const MERKLE: &str = "examples/tests/test_merkle.edge";

#[test]
fn test_merkle_hash_two() {
    for_all_opt_levels(MERKLE, |h, o| {
        let mut a = [0u8; 32];
        a[31] = 0xAA;
        let mut b = [0u8; 32];
        b[31] = 0x55;
        let (ok, out) = h.call(calldata(
            selector("hash_two(bytes32,bytes32)"),
            &[encode_b32(a), encode_b32(b)],
        ));
        assert!(ok, "hash_two() reverted at O{o}");
        // XOR: 0xAA ^ 0x55 = 0xFF
        assert_eq!(
            out[31], 0xFF,
            "hash_two() wrong at O{o}: got {:02x}",
            out[31]
        );
    });
}

#[test]
fn test_merkle_verify_single_proof() {
    for_all_opt_levels(MERKLE, |h, o| {
        // leaf = 0x01, proof = [0x02, 0, 0, 0], proof_len = 1
        // hash_pair(0x01, 0x02) = 0x01 ^ 0x02 = 0x03 (since 0x01 < 0x02)
        // root = 0x03
        let mut leaf = [0u8; 32];
        leaf[31] = 0x01;
        let mut proof0 = [0u8; 32];
        proof0[31] = 0x02;
        let zero = [0u8; 32];
        let mut root = [0u8; 32];
        root[31] = 0x03;

        let (ok, out) = h.call(calldata(
            selector("verify(bytes32,bytes32,bytes32[4],uint256)"),
            &[
                encode_b32(root),
                encode_b32(leaf),
                encode_b32(proof0),
                encode_b32(zero),
                encode_b32(zero),
                encode_b32(zero),
                encode_u256(1),
            ],
        ));
        assert!(ok, "verify() reverted at O{o}");
        assert_eq!(decode_u256(&out), 1, "verify() should return true at O{o}");
    });
}

#[test]
fn test_merkle_verify_wrong_root() {
    for_all_opt_levels(MERKLE, |h, o| {
        let mut leaf = [0u8; 32];
        leaf[31] = 0x01;
        let mut proof0 = [0u8; 32];
        proof0[31] = 0x02;
        let zero = [0u8; 32];
        let mut wrong_root = [0u8; 32];
        wrong_root[31] = 0xFF;

        let (ok, out) = h.call(calldata(
            selector("verify(bytes32,bytes32,bytes32[4],uint256)"),
            &[
                encode_b32(wrong_root),
                encode_b32(leaf),
                encode_b32(proof0),
                encode_b32(zero),
                encode_b32(zero),
                encode_b32(zero),
                encode_u256(1),
            ],
        ));
        assert!(ok, "verify() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            0,
            "verify(wrong_root) should return false at O{o}"
        );
    });
}

#[test]
fn test_merkle_verify_two_proofs() {
    for_all_opt_levels(MERKLE, |h, o| {
        // leaf = 0x01, proof = [0x02, 0x04, 0, 0], proof_len = 2
        // Step 1: hash_pair(0x01, 0x02) = 0x03 (0x01 < 0x02 → XOR)
        // Step 2: hash_pair(0x03, 0x04) = 0x07 (0x03 < 0x04 → XOR)
        // root = 0x07
        let mut leaf = [0u8; 32];
        leaf[31] = 0x01;
        let mut proof0 = [0u8; 32];
        proof0[31] = 0x02;
        let mut proof1 = [0u8; 32];
        proof1[31] = 0x04;
        let zero = [0u8; 32];
        let mut root = [0u8; 32];
        root[31] = 0x07;

        let (ok, out) = h.call(calldata(
            selector("verify(bytes32,bytes32,bytes32[4],uint256)"),
            &[
                encode_b32(root),
                encode_b32(leaf),
                encode_b32(proof0),
                encode_b32(proof1),
                encode_b32(zero),
                encode_b32(zero),
                encode_u256(2),
            ],
        ));
        assert!(ok, "verify() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            1,
            "verify(2 proofs) should return true at O{o}"
        );
    });
}
