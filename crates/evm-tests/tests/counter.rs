//! Semantic tests for counter.edge
//!
//! Deploys the counter contract on an in-memory EVM and tests all functions.

use alloy_primitives::U256;
use edge_evm_tests::{abi_decode_u256, fn_selector, EvmTestHost};

const COUNTER_PATH: &str = "../../examples/counter.edge";

fn sel_get() -> [u8; 4] { fn_selector("get()") }
fn sel_increment() -> [u8; 4] { fn_selector("increment()") }
fn sel_decrement() -> [u8; 4] { fn_selector("decrement()") }
fn sel_reset() -> [u8; 4] { fn_selector("reset()") }

#[test]
fn counter_deploy() {
    let _host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);
}

#[test]
fn counter_get_initial_zero() {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);
    let result = host.call(sel_get(), &[]);
    assert!(result.success, "get() should succeed");
    assert_eq!(abi_decode_u256(&result.output), U256::ZERO);
}

#[test]
fn counter_increment_then_get() {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);

    let r = host.call(sel_increment(), &[]);
    assert!(r.success, "increment() should succeed");

    let r = host.call(sel_get(), &[]);
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(1));
}

#[test]
fn counter_multiple_increments() {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);

    for _ in 0..5 {
        let r = host.call(sel_increment(), &[]);
        assert!(r.success);
    }

    let r = host.call(sel_get(), &[]);
    assert_eq!(abi_decode_u256(&r.output), U256::from(5));
}

#[test]
fn counter_increment_decrement() {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);

    host.call(sel_increment(), &[]);
    host.call(sel_increment(), &[]);
    let r = host.call(sel_decrement(), &[]);
    assert!(r.success, "decrement() should succeed");

    let r = host.call(sel_get(), &[]);
    assert_eq!(abi_decode_u256(&r.output), U256::from(1));
}

#[test]
fn counter_reset() {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);

    host.call(sel_increment(), &[]);
    host.call(sel_increment(), &[]);
    host.call(sel_increment(), &[]);

    let r = host.call(sel_reset(), &[]);
    assert!(r.success, "reset() should succeed");

    let r = host.call(sel_get(), &[]);
    assert_eq!(abi_decode_u256(&r.output), U256::ZERO);
}

#[test]
fn counter_full_sequence() {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);

    // Start at 0
    assert_eq!(abi_decode_u256(&host.call(sel_get(), &[]).output), U256::ZERO);

    // Increment to 1
    host.call(sel_increment(), &[]);
    assert_eq!(abi_decode_u256(&host.call(sel_get(), &[]).output), U256::from(1));

    // Increment to 2
    host.call(sel_increment(), &[]);
    assert_eq!(abi_decode_u256(&host.call(sel_get(), &[]).output), U256::from(2));

    // Decrement to 1
    host.call(sel_decrement(), &[]);
    assert_eq!(abi_decode_u256(&host.call(sel_get(), &[]).output), U256::from(1));

    // Reset to 0
    host.call(sel_reset(), &[]);
    assert_eq!(abi_decode_u256(&host.call(sel_get(), &[]).output), U256::ZERO);
}

#[test]
fn counter_storage_slot_0() {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, 0);

    // Initially slot 0 should be 0
    assert_eq!(host.sload(U256::ZERO), U256::ZERO);

    host.call(sel_increment(), &[]);
    host.call(sel_increment(), &[]);
    host.call(sel_increment(), &[]);

    // Slot 0 should now be 3
    assert_eq!(host.sload(U256::ZERO), U256::from(3));
}
