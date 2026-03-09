#![allow(missing_docs)]

//! Execution-level acceptance tests for token contracts.
//!
//! Tests compile weth.edge and amm.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify stateful behaviour.
//!
//! ## Compiler caveats
//! Internal helper functions (_transfer) are lowered to stubs.
//! Multi-value returns (getReserves) return 0 at this compiler stage
//! since tuple instantiation is not yet fully supported.

use crate::helpers::*;

const ALICE_ADDR: [u8; 20] = {
    let mut a = [0u8; 20];
    a[19] = 0xA1;
    a
};

// =============================================================================
// WETH
// =============================================================================

#[test]
fn test_weth_total_supply_initially_zero() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(res.success, "totalSupply() reverted");
    assert_eq!(decode_u256(&res.output), 0, "totalSupply should start at 0");
}

#[test]
fn test_weth_balance_of_zero_initially() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(res.success, "balanceOf() reverted");
    assert_eq!(decode_u256(&res.output), 0, "balanceOf should start at 0");
}

#[test]
fn test_weth_approve_returns_true() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(
        selector("approve(address,uint256)"),
        &[encode_address(ALICE_ADDR), encode_u256(1000)],
    ));
    assert!(res.success, "approve() reverted");
    assert!(decode_bool(&res.output), "approve should return true");
}

#[test]
fn test_weth_allowance_initially_zero() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(
        selector("allowance(address,address)"),
        &[encode_address([0u8; 20]), encode_address(ALICE_ADDR)],
    ));
    assert!(res.success, "allowance() reverted");
    assert_eq!(decode_u256(&res.output), 0, "allowance should start at 0");
}

#[test]
fn test_weth_deposit_increases_total_supply() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);

    // deposit() — sends ETH value, mints WETH
    let deposit_amount: u64 = 1_000_000;
    let res = evm.call_with_value(calldata(selector("deposit()"), &[]), deposit_amount);
    assert!(res.success, "deposit() reverted");

    let res = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(res.success, "totalSupply() reverted after deposit");
    assert_eq!(
        decode_u256(&res.output),
        deposit_amount,
        "totalSupply should equal deposit amount"
    );
}

#[test]
fn test_weth_deposit_increases_balance() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);

    let deposit_amount: u64 = 500_000;
    // caller is Address::ZERO
    let res = evm.call_with_value(calldata(selector("deposit()"), &[]), deposit_amount);
    assert!(res.success, "deposit() reverted");

    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(res.success, "balanceOf() reverted after deposit");
    assert_eq!(
        decode_u256(&res.output),
        deposit_amount,
        "balanceOf(caller) should equal deposit amount"
    );
}

#[test]
fn test_weth_unknown_selector_reverts() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!res.success, "unknown selector should revert");
}

// =============================================================================
// AMM
// =============================================================================

#[test]
fn test_amm_total_supply_initially_zero() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(res.success, "totalSupply() reverted");
    assert_eq!(
        decode_u256(&res.output),
        0,
        "AMM totalSupply should start at 0"
    );
}

#[test]
fn test_amm_balance_of_initially_zero() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(res.success, "balanceOf() reverted");
    assert_eq!(
        decode_u256(&res.output),
        0,
        "AMM balanceOf should start at 0"
    );
}

#[test]
fn test_amm_get_reserves_initially_zero() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(selector("getReserves()"), &[]));
    assert!(res.success, "getReserves() reverted");
    // Tuple return: 64 bytes (two u256 values)
    assert!(
        res.output.len() >= 64,
        "getReserves should return 64 bytes, got {}",
        res.output.len()
    );
    assert_eq!(decode_u256(&res.output[0..32]), 0, "reserve0 should be 0");
    assert_eq!(decode_u256(&res.output[32..64]), 0, "reserve1 should be 0");
}

#[test]
fn test_amm_unknown_selector_reverts() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!res.success, "unknown selector should revert");
}

// =============================================================================
// ERC721
// =============================================================================

#[test]
fn test_erc721_total_supply_initially_zero() {
    let bc = compile_contract("std/tokens/erc721.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(res.success, "totalSupply() reverted");
    assert_eq!(
        decode_u256(&res.output),
        0,
        "ERC721 totalSupply should start at 0"
    );
}

#[test]
fn test_erc721_unknown_selector_reverts() {
    let bc = compile_contract("std/tokens/erc721.edge");
    let mut evm = EvmHandle::new(bc);
    let res = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!res.success, "unknown selector should revert");
}
