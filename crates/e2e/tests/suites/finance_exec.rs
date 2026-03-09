#![allow(missing_docs)]

//! Execution-level acceptance tests for finance contracts.
//!
//! Tests compile staking.edge and multisig.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify stateful behaviour.

use crate::helpers::*;

// =============================================================================
// Staking
// =============================================================================

#[test]
fn test_staking_total_staked_initially_zero() {
    let bc = compile_contract("std/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("totalStaked()"), &[]));
    assert!(r.success, "totalStaked() reverted");
    assert_eq!(decode_u256(&r.output), 0, "totalStaked should start at 0");
}

#[test]
fn test_staking_staked_balance_initially_zero() {
    let bc = compile_contract("std/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);
    let addr = [0u8; 20];
    let r = evm.call(calldata(
        selector("stakedBalance(address)"),
        &[encode_address(addr)],
    ));
    assert!(r.success, "stakedBalance() reverted");
    assert_eq!(decode_u256(&r.output), 0, "stakedBalance should start at 0");
}

#[test]
fn test_staking_stake_increases_total() {
    let bc = compile_contract("std/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(selector("stake(uint256)"), &[encode_u256(100)]));
    assert!(r.success, "stake(100) reverted");

    let r = evm.call(calldata(selector("totalStaked()"), &[]));
    assert!(r.success, "totalStaked() reverted after stake");
    assert_eq!(decode_u256(&r.output), 100, "totalStaked should be 100");
}

#[test]
fn test_staking_stake_increases_balance() {
    let bc = compile_contract("std/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(selector("stake(uint256)"), &[encode_u256(100)]));
    assert!(r.success, "stake(100) reverted");

    // CALLER is Address::ZERO
    let r = evm.call(calldata(
        selector("stakedBalance(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(r.success, "stakedBalance(caller) reverted");
    assert_eq!(decode_u256(&r.output), 100, "stakedBalance should be 100");
}

#[test]
fn test_staking_withdraw_decreases() {
    let bc = compile_contract("std/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);

    // Stake 100
    let r = evm.call(calldata(selector("stake(uint256)"), &[encode_u256(100)]));
    assert!(r.success, "stake(100) reverted");

    // Withdraw 50
    let r = evm.call(calldata(selector("withdraw(uint256)"), &[encode_u256(50)]));
    assert!(r.success, "withdraw(50) reverted");

    let r = evm.call(calldata(selector("totalStaked()"), &[]));
    assert!(r.success, "totalStaked() reverted");
    assert_eq!(
        decode_u256(&r.output),
        50,
        "totalStaked should be 50 after withdraw"
    );

    let r = evm.call(calldata(
        selector("stakedBalance(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(r.success, "stakedBalance(caller) reverted");
    assert_eq!(decode_u256(&r.output), 50, "stakedBalance should be 50");
}

#[test]
fn test_staking_unknown_selector_reverts() {
    let bc = compile_contract("std/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// Multisig
// =============================================================================

#[test]
fn test_multisig_threshold_initially_zero() {
    let bc = compile_contract("std/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("getThreshold()"), &[]));
    assert!(r.success, "getThreshold() reverted");
    assert_eq!(decode_u256(&r.output), 0, "threshold should start at 0");
}

#[test]
fn test_multisig_confirmations_initially_zero() {
    let bc = compile_contract("std/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("getConfirmations(uint256)"),
        &[encode_u256(0)],
    ));
    assert!(r.success, "getConfirmations(0) reverted");
    assert_eq!(decode_u256(&r.output), 0, "confirmations should start at 0");
}

#[test]
fn test_multisig_propose_reverts_for_non_owner() {
    let bc = compile_contract("std/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);

    // propose() calls _requireOwner which checks is_owner[caller].
    // CALLER is not an owner, so this should revert.
    let target = [0u8; 20];
    let r = evm.call(calldata(
        selector("propose(address,uint256,bytes32)"),
        &[encode_address(target), encode_u256(0), [0u8; 32]],
    ));
    assert!(!r.success, "propose should revert for non-owner");
}

#[test]
fn test_multisig_unknown_selector_reverts() {
    let bc = compile_contract("std/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}
