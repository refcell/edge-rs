#![allow(missing_docs)]

//! Execution-level acceptance tests for type-demonstration contracts.
//!
//! Tests compile comptime.edge to bytecode, deploy on an in-memory revm EVM,
//! and verify basic store/load round-trip behaviour.

use crate::helpers::*;

// =============================================================================
// ComptimeExample (comptime.edge)
// =============================================================================

#[test]
fn test_comptime_load_initially_zero() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("load()"), &[]));
    assert!(r.success, "load() reverted");
    assert_eq!(
        decode_u256(&r.output),
        0,
        "load() should return 0 before any store"
    );
}

#[test]
fn test_comptime_store_and_load_roundtrip() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(selector("store(uint256)"), &[encode_u256(42)]));
    assert!(r.success, "store(42) reverted");

    let r = evm.call(calldata(selector("load()"), &[]));
    assert!(r.success, "load() reverted after store");
    assert_eq!(
        decode_u256(&r.output),
        42,
        "load() should return 42 after store(42)"
    );
}

#[test]
fn test_comptime_store_overwrites() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(selector("store(uint256)"), &[encode_u256(100)]));
    assert!(r.success, "store(100) reverted");

    let r = evm.call(calldata(selector("store(uint256)"), &[encode_u256(999)]));
    assert!(r.success, "store(999) reverted");

    let r = evm.call(calldata(selector("load()"), &[]));
    assert!(r.success, "load() reverted");
    assert_eq!(
        decode_u256(&r.output),
        999,
        "load() should return most-recently stored value"
    );
}

#[test]
fn test_comptime_unknown_selector_reverts() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}
