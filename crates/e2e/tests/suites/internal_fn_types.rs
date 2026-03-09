#![allow(missing_docs)]

//! Tests that internal (non-pub) functions get correct return type annotations
//! in the IR, regardless of their declared return type.

use crate::helpers::*;

// =============================================================================
// Test 1: Internal function returning bool
// =============================================================================

#[test]
fn internal_fn_returning_bool() {
    let bc = compile_contract("examples/tests/internal_bool.edge");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(
        selector("is_positive(uint256)"),
        &[encode_u256(42)],
    ));
    assert!(r.success, "call failed");
    assert_eq!(
        decode_u256(&r.output),
        1,
        "is_positive(42) should be true (1)"
    );

    let r = evm.call(calldata(
        selector("is_positive(uint256)"),
        &[encode_u256(0)],
    ));
    assert!(r.success, "call failed");
    assert_eq!(
        decode_u256(&r.output),
        0,
        "is_positive(0) should be false (0)"
    );
}

// =============================================================================
// Test 2: Internal function returning u256 (regression)
// =============================================================================

#[test]
fn internal_fn_returning_u256() {
    let bc = compile_contract("examples/tests/internal_math.edge");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(
        selector("get_double(uint256)"),
        &[encode_u256(21)],
    ));
    assert!(r.success, "call failed");
    assert_eq!(decode_u256(&r.output), 42, "get_double(21) should be 42");
}

// =============================================================================
// Test 3: Storage set/get (regression, no internal fn)
// =============================================================================

#[test]
fn internal_fn_storage_regression() {
    let bc = compile_contract("examples/tests/internal_void.edge");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(selector("set_value(uint256)"), &[encode_u256(99)]));
    assert!(r.success, "set_value call failed");

    let r = evm.call(calldata(selector("get_value()"), &[]));
    assert!(r.success, "get_value call failed");
    assert_eq!(decode_u256(&r.output), 99, "get_value() should be 99");
}
