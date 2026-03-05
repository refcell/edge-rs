//! Semantic tests for loop_extraction.edge
//!
//! Tests storage-to-local hoisting (LICM) for loops with storage operations.

use alloy_primitives::U256;
use edge_evm_tests::{abi_decode_u256, abi_encode_u256, fn_selector, EvmTestHost};

const CONTRACT_PATH: &str = "../../examples/loop_extraction.edge";

fn sel_storage_while_sum() -> [u8; 4] {
    fn_selector("storage_while_sum(uint256)")
}

#[test]
fn loop_extraction_deploy_o0() {
    let _host = EvmTestHost::deploy_edge(CONTRACT_PATH, 0);
}

#[test]
fn loop_extraction_sum_10_o0() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 0);
    let r = host.call(sel_storage_while_sum(), &abi_encode_u256(U256::from(10)));
    assert!(r.success, "storage_while_sum(10) should succeed at O0");
    assert_eq!(
        abi_decode_u256(&r.output),
        U256::from(55),
        "sum(1..=10) should be 55"
    );
}

#[test]
fn loop_extraction_sum_10_o1() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 1);
    let r = host.call(sel_storage_while_sum(), &abi_encode_u256(U256::from(10)));
    assert!(r.success, "storage_while_sum(10) should succeed at O1");
    assert_eq!(
        abi_decode_u256(&r.output),
        U256::from(55),
        "sum(1..=10) should be 55 at O1"
    );
}

#[test]
fn loop_extraction_sum_10_o2() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 2);
    let r = host.call(sel_storage_while_sum(), &abi_encode_u256(U256::from(10)));
    assert!(r.success, "storage_while_sum(10) should succeed at O2");
    assert_eq!(
        abi_decode_u256(&r.output),
        U256::from(55),
        "sum(1..=10) should be 55 at O2"
    );
}

#[test]
fn loop_extraction_sum_0() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 0);
    let r = host.call(sel_storage_while_sum(), &abi_encode_u256(U256::ZERO));
    assert!(r.success, "storage_while_sum(0) should succeed");
    assert_eq!(
        abi_decode_u256(&r.output),
        U256::ZERO,
        "sum with n=0 should be 0 (loop doesn't execute)"
    );
}

#[test]
fn loop_extraction_sum_1() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 0);
    let r = host.call(sel_storage_while_sum(), &abi_encode_u256(U256::from(1)));
    assert!(r.success, "storage_while_sum(1) should succeed");
    assert_eq!(
        abi_decode_u256(&r.output),
        U256::from(1),
        "sum(1..=1) should be 1"
    );
}

#[test]
fn loop_extraction_storage_persists() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 0);

    // Call with n=5: accumulator should be 15, counter should be 6
    let r = host.call(sel_storage_while_sum(), &abi_encode_u256(U256::from(5)));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(15));

    // Check storage slots: accumulator=slot0=15, counter=slot1=6
    assert_eq!(host.sload(U256::ZERO), U256::from(15));
    assert_eq!(host.sload(U256::from(1)), U256::from(6));
}

// ============================================================
// storage_no_loop: straight-line SStore→SLoad forwarding tests
// ============================================================

fn sel_storage_no_loop() -> [u8; 4] {
    fn_selector("storage_no_loop(uint256)")
}

/// storage_no_loop(n):
///   accumulator = 0; counter = 1;
///   accumulator += counter;  // = 1
///   counter += 1;            // = 2
///   accumulator += n;        // = 1 + n
///   counter += n;            // = 2 + n
///   return accumulator;      // = 1 + n
#[test]
fn no_loop_o0_n10() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 0);
    let r = host.call(sel_storage_no_loop(), &abi_encode_u256(U256::from(10)));
    assert!(r.success, "storage_no_loop(10) should succeed at O0");
    assert_eq!(abi_decode_u256(&r.output), U256::from(11), "1 + 10 = 11");
}

#[test]
fn no_loop_o1_n10() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 1);
    let r = host.call(sel_storage_no_loop(), &abi_encode_u256(U256::from(10)));
    assert!(r.success, "storage_no_loop(10) should succeed at O1");
    assert_eq!(abi_decode_u256(&r.output), U256::from(11));
}

#[test]
fn no_loop_o2_n10() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 2);
    let r = host.call(sel_storage_no_loop(), &abi_encode_u256(U256::from(10)));
    assert!(r.success, "storage_no_loop(10) should succeed at O2");
    assert_eq!(abi_decode_u256(&r.output), U256::from(11));
}

#[test]
fn no_loop_storage_persists() {
    let mut host = EvmTestHost::deploy_edge(CONTRACT_PATH, 0);
    let r = host.call(sel_storage_no_loop(), &abi_encode_u256(U256::from(5)));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(6)); // 1 + 5

    // accumulator = 1 + n = 6, counter = 2 + n = 7
    assert_eq!(host.sload(U256::ZERO), U256::from(6));
    assert_eq!(host.sload(U256::from(1)), U256::from(7));
}
