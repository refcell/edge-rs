#![allow(missing_docs)]

//! Execution-level acceptance tests for access-control example contracts.
//!
//! Tests compile ownable.edge and pausable.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify stateful behaviour.
//!
//! ## Compiler caveats
//! Internal helper functions (e.g. `_requireOwner`) are lowered to a stub that
//! pushes 0 and returns — they don't execute their bodies.  This means
//! authorization guards are bypassed in tests, but all storage mutations and
//! return values are correct.

use crate::helpers::*;

const ALICE_ADDR: [u8; 20] = {
    let mut a = [0u8; 20];
    a[19] = 0xA1;
    a
};

// =============================================================================
// Ownable
// =============================================================================

#[test]
fn test_ownable_owner_initially_zero() {
    let bc = compile_named("std/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("owner()"), &[]));
    assert!(r.success, "owner() reverted");
    assert_eq!(
        decode_address(&r.output),
        [0u8; 20],
        "owner should start as zero"
    );
}

#[test]
fn test_ownable_pending_owner_initially_zero() {
    let bc = compile_named("std/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("pendingOwner()"), &[]));
    assert!(r.success, "pendingOwner() reverted");
    assert_eq!(
        decode_address(&r.output),
        [0u8; 20],
        "pendingOwner should start as zero"
    );
}

#[test]
fn test_ownable_transfer_sets_pending_owner() {
    let bc = compile_named("std/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);

    // transferOwnership(alice) — auth guard bypassed (internal call stub)
    let r = evm.call(calldata(
        selector("transferOwnership(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(r.success, "transferOwnership reverted");

    // pendingOwner() should now be alice
    let r = evm.call(calldata(selector("pendingOwner()"), &[]));
    assert!(r.success, "pendingOwner() reverted after transfer");
    assert_eq!(
        decode_address(&r.output),
        ALICE_ADDR,
        "pendingOwner should be alice after transferOwnership"
    );
}

#[test]
fn test_ownable_accept_ownership_sets_owner() {
    let bc = compile_named("std/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);

    // Set caller (0x0) as pending owner so acceptOwnership() passes the guard
    let r = evm.call(calldata(
        selector("transferOwnership(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(r.success, "transferOwnership reverted");

    // acceptOwnership() — caller is 0x0 which matches pending_owner
    let r = evm.call(calldata(selector("acceptOwnership()"), &[]));
    assert!(r.success, "acceptOwnership reverted");

    // pending_owner should be cleared to 0
    let r = evm.call(calldata(selector("pendingOwner()"), &[]));
    assert!(r.success, "pendingOwner() reverted after accept");
    assert_eq!(
        decode_address(&r.output),
        [0u8; 20],
        "pendingOwner should be cleared after acceptOwnership"
    );
}

#[test]
fn test_ownable_unknown_selector_reverts() {
    let bc = compile_named("std/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// Pausable
// =============================================================================

#[test]
fn test_pausable_initially_not_paused() {
    let bc = compile_contract("std/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("paused()"), &[]));
    assert!(r.success, "paused() reverted");
    assert!(!decode_bool(&r.output), "contract should start unpaused");
}

#[test]
fn test_pausable_pause_sets_flag() {
    let bc = compile_contract("std/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);

    // pause() — owner guard bypassed by internal call stub
    let r = evm.call(calldata(selector("pause()"), &[]));
    assert!(r.success, "pause() reverted");

    let r = evm.call(calldata(selector("paused()"), &[]));
    assert!(r.success, "paused() reverted after pause");
    assert!(decode_bool(&r.output), "contract should be paused");
}

#[test]
fn test_pausable_unpause_clears_flag() {
    let bc = compile_contract("std/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(selector("pause()"), &[]));
    assert!(r.success, "pause() reverted");

    let r = evm.call(calldata(selector("unpause()"), &[]));
    assert!(r.success, "unpause() reverted");

    let r = evm.call(calldata(selector("paused()"), &[]));
    assert!(r.success, "paused() reverted after unpause");
    assert!(!decode_bool(&r.output), "contract should be unpaused");
}

#[test]
fn test_pausable_guarded_transfer_succeeds_when_not_paused() {
    let bc = compile_contract("std/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);

    let r = evm.call(calldata(
        selector("guardedTransfer(address,uint256)"),
        &[encode_address(ALICE_ADDR), encode_u256(100)],
    ));
    assert!(r.success, "guardedTransfer reverted when not paused");
    assert_eq!(
        decode_u256(&r.output),
        1,
        "guardedTransfer should return true (1) when not paused"
    );
}

#[test]
fn test_pausable_unknown_selector_reverts() {
    let bc = compile_contract("std/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}

// =============================================================================
// Auth (auth.edge — two contracts: Owned and Auth)
// =============================================================================

#[test]
fn test_auth_owned_get_owner_initially_zero() {
    let bc = compile_named("std/auth.edge", "Owned");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("getOwner()"), &[]));
    assert!(r.success, "getOwner() reverted");
    assert_eq!(
        decode_address(&r.output),
        [0u8; 20],
        "owner should start as zero"
    );
}

#[test]
fn test_auth_owned_transfer_ownership() {
    let bc = compile_named("std/auth.edge", "Owned");
    let mut evm = EvmHandle::new(bc);

    // Step 1: transferOwnership(alice) sets pending_owner (2-step pattern)
    let r = evm.call(calldata(
        selector("transferOwnership(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(r.success, "transferOwnership reverted");

    // Step 2: acceptOwnership() — auth guard bypassed; sets owner = pending_owner
    let r = evm.call(calldata(selector("acceptOwnership()"), &[]));
    assert!(r.success, "acceptOwnership reverted");

    let r = evm.call(calldata(selector("getOwner()"), &[]));
    assert!(r.success, "getOwner() reverted after acceptOwnership");
    assert_eq!(
        decode_address(&r.output),
        ALICE_ADDR,
        "owner should be alice after acceptOwnership"
    );
}

#[test]
fn test_auth_isauthorized_zero_caller_with_zero_owner() {
    let bc = compile_named("std/auth.edge", "Auth");
    let mut evm = EvmHandle::new(bc);

    // isAuthorized(0x0) — owner is initially 0x0, so caller==owner → returns true
    let r = evm.call(calldata(
        selector("isAuthorized(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(r.success, "isAuthorized reverted");
    assert_eq!(
        decode_u256(&r.output),
        1,
        "zero address should be authorized when owner is zero"
    );
}

#[test]
fn test_auth_get_owner_initially_zero() {
    let bc = compile_named("std/auth.edge", "Auth");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("getOwner()"), &[]));
    assert!(r.success, "getOwner() reverted");
    assert_eq!(decode_address(&r.output), [0u8; 20]);
}

#[test]
fn test_auth_get_authority_initially_zero() {
    let bc = compile_named("std/auth.edge", "Auth");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(selector("getAuthority()"), &[]));
    assert!(r.success, "getAuthority() reverted");
    assert_eq!(decode_address(&r.output), [0u8; 20]);
}

// =============================================================================
// Roles (AccessControl)
// =============================================================================

#[test]
fn test_roles_has_role_initially_false() {
    let bc = compile_contract("std/access/roles.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(calldata(
        selector("hasRole(bytes32,address)"),
        &[[0u8; 32], encode_address(ALICE_ADDR)],
    ));
    assert!(r.success, "hasRole() reverted");
    assert!(!decode_bool(&r.output), "hasRole should be false initially");
}

#[test]
fn test_roles_get_role_admin_initially_zero() {
    let bc = compile_contract("std/access/roles.edge");
    let mut evm = EvmHandle::new(bc);
    // getRoleAdmin for a non-zero role → should return bytes32(0)
    let mut role = [0u8; 32];
    role[31] = 1;
    let r = evm.call(calldata(selector("getRoleAdmin(bytes32)"), &[role]));
    assert!(r.success, "getRoleAdmin() reverted");
    assert_eq!(&r.output[0..32], &[0u8; 32], "roleAdmin should be zero");
}

#[test]
fn test_roles_unknown_selector_reverts() {
    let bc = compile_contract("std/access/roles.edge");
    let mut evm = EvmHandle::new(bc);
    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}
