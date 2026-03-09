#![allow(missing_docs)]

//! Execution-level acceptance tests for pattern contracts.
//!
//! Tests compile `reentrancy_guard.edge` and `timelock.edge` to bytecode, deploy
//! on an in-memory revm EVM, and verify stateful behaviour.
//!
//! ## Compiler caveats
//! Internal helper functions (_lock, _unlock, _requireAdmin) are lowered to
//! stubs that push 0 and return.  This means locking/authorization guards are
//! bypassed in tests, but observable state mutations (timestamps, cancelled,
//! executed flags) are tested correctly.

use crate::helpers::*;

const ALICE_ADDR: [u8; 20] = {
    let mut a = [0u8; 20];
    a[19] = 0xA1;
    a
};

// =============================================================================
// ReentrancyGuard (persistent storage)
// =============================================================================

#[test]
fn test_reentrancy_guard_protected_withdraw_succeeds() {
    let bc = compile_named("std/patterns/reentrancy_guard.edge", "ReentrancyGuard");
    let mut evm = EvmHandle::new(bc);

    // protectedWithdraw calls _lock, _doWithdraw, _unlock — all internal stubs.
    // With stubs bypassed the function should succeed without reverting.
    let r = evm.call(calldata(
        selector("protectedWithdraw(address,uint256)"),
        &[encode_address(ALICE_ADDR), encode_u256(100)],
    ));
    assert!(r.success, "protectedWithdraw reverted unexpectedly");
}

#[test]
fn test_reentrancy_guard_unknown_selector_reverts() {
    let bc = compile_named("std/patterns/reentrancy_guard.edge", "ReentrancyGuard");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// TransientReentrancyGuard (transient storage)
// =============================================================================

#[test]
fn test_transient_reentrancy_guard_protected_withdraw_succeeds() {
    let bc = compile_named(
        "std/patterns/reentrancy_guard.edge",
        "TransientReentrancyGuard",
    );
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(
        selector("protectedWithdraw(address,uint256)"),
        &[encode_address(ALICE_ADDR), encode_u256(100)],
    ));
    assert!(r.success, "transient protectedWithdraw reverted unexpectedly");
}

#[test]
fn test_transient_reentrancy_guard_unknown_selector_reverts() {
    let bc = compile_named(
        "std/patterns/reentrancy_guard.edge",
        "TransientReentrancyGuard",
    );
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// Timelock
// =============================================================================

#[test]
fn test_timelock_is_ready_unscheduled_returns_false() {
    let bc = compile_contract("std/patterns/timelock.edge");
    let mut evm = EvmHandle::new(bc);

    // isReady(id=0, current_time=9999) — nothing scheduled, ts=0 → returns false
    let r = evm.call(calldata(
        selector("isReady(bytes32,uint256)"),
        &[[0u8; 32], encode_u256(9999)],
    ));
    assert!(r.success, "isReady reverted");
    assert!(
        !decode_bool(&r.output),
        "isReady should return false for unscheduled op"
    );
}

#[test]
fn test_timelock_schedule_and_is_ready() {
    let bc = compile_contract("std/patterns/timelock.edge");
    let mut evm = EvmHandle::new(bc);

    // min_delay is 0 initially (uninitialized storage), so any delay >= 0 passes.
    // _requireAdmin is a stub so the admin check is bypassed.
    let id = [0xabu8; 32];
    let delay: u64 = 100;

    // Schedule the operation
    let mut args = [0u8; 32 * 4];
    args[0..32].copy_from_slice(&id);
    args[32..64].copy_from_slice(&encode_address(ALICE_ADDR));
    args[64..96].copy_from_slice(&encode_u256(0)); // value
    args[96..128].copy_from_slice(&encode_u256(delay));

    let r = evm.call({
        let mut cd = selector("schedule(bytes32,address,uint256,uint256)").to_vec();
        cd.extend_from_slice(&args);
        cd
    });
    assert!(r.success, "schedule reverted");

    // isReady(id, current_time=delay) — should be true since ts=delay and current_time>=ts
    let r = evm.call(calldata(
        selector("isReady(bytes32,uint256)"),
        &[id, encode_u256(delay)],
    ));
    assert!(r.success, "isReady reverted after schedule");
    assert!(
        decode_bool(&r.output),
        "isReady should be true after scheduling with current_time >= delay"
    );
}

#[test]
fn test_timelock_unknown_selector_reverts() {
    let bc = compile_contract("std/patterns/timelock.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// Factory
// =============================================================================

#[test]
fn test_factory_is_deployed_initially_false() {
    let bc = compile_contract("std/patterns/factory.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("isDeployed(bytes32)"), &[[0u8; 32]]));
    assert!(r.success, "isDeployed() reverted");
    assert!(
        !decode_bool(&r.output),
        "isDeployed should be false initially"
    );
}

#[test]
fn test_factory_unknown_selector_reverts() {
    let bc = compile_contract("std/patterns/factory.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}
