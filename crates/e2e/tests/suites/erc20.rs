#![allow(missing_docs)]

use crate::helpers::*;

/// Create a test address from a single byte suffix.
const fn test_address(suffix: u8) -> [u8; 20] {
    let mut addr = [0u8; 20];
    addr[19] = suffix;
    addr
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn test_erc20_compiles() {
    let bytecode = compile_contract("examples/erc20.edge");
    assert!(!bytecode.is_empty(), "erc20.edge produced empty bytecode");
    assert!(bytecode.len() > 10, "bytecode too short to be valid");
}

#[test]
fn test_erc20_initial_supply() {
    let bytecode = compile_contract("examples/erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let res = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(res.success, "totalSupply() reverted on fresh contract");
    assert_eq!(
        decode_u256(&res.output),
        0,
        "initial totalSupply should be 0"
    );
}

#[test]
fn test_erc20_mint_and_balance() {
    let bytecode = compile_contract("examples/tests/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let alice = test_address(0x02);

    // Mint 1000 tokens to alice via public mint().
    let mint_cd = calldata(
        selector("mint(address,uint256)"),
        &[encode_address(alice), encode_u256(1000)],
    );

    let res = evm.call(mint_cd);
    assert!(res.success, "mint(alice, 1000) reverted");

    // Query balance of alice
    let bal_cd = calldata(selector("balanceOf(address)"), &[encode_address(alice)]);

    let res = evm.call(bal_cd);
    assert!(res.success, "balanceOf(alice) reverted");
    assert_eq!(
        decode_u256(&res.output),
        1000,
        "alice balance should be 1000 after mint"
    );
}

#[test]
fn test_erc20_transfer() {
    let bytecode = compile_contract("examples/tests/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    // CALLER is Address::ZERO. transfer() uses @caller() as `from`.
    let bob = test_address(0x02);
    let caller_addr: [u8; 20] = CALLER.0 .0;

    // Mint 1000 tokens to CALLER.
    let res = evm.call(calldata(
        selector("mint(address,uint256)"),
        &[encode_address(caller_addr), encode_u256(1000)],
    ));
    assert!(res.success, "mint(caller, 1000) reverted");

    // Transfer 300 from CALLER to bob via public transfer(to, amount).
    let res = evm.call(calldata(
        selector("transfer(address,uint256)"),
        &[encode_address(bob), encode_u256(300)],
    ));
    assert!(res.success, "transfer(bob, 300) reverted");
    assert_eq!(
        decode_u256(&res.output),
        1,
        "transfer should return true (1)"
    );

    // Check CALLER balance (should be 700)
    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(caller_addr)],
    ));
    assert!(res.success, "balanceOf(caller) reverted");
    assert_eq!(
        decode_u256(&res.output),
        700,
        "caller balance should be 700 after transfer"
    );

    // Check bob balance (should be 300)
    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(bob)],
    ));
    assert!(res.success, "balanceOf(bob) reverted");
    assert_eq!(
        decode_u256(&res.output),
        300,
        "bob balance should be 300 after transfer"
    );
}

#[test]
fn test_erc20_approve_and_transferfrom() {
    let bytecode = compile_contract("examples/tests/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    // CALLER = Address::ZERO. approve() uses @caller() as owner,
    // transferFrom() uses @caller() for allowance lookup.
    // Single-caller EVM: self-approve then transferFrom from self.
    let caller_addr: [u8; 20] = CALLER.0 .0;
    let charlie = test_address(0x03);

    // Mint 1000 tokens to CALLER.
    let res = evm.call(calldata(
        selector("mint(address,uint256)"),
        &[encode_address(caller_addr), encode_u256(1000)],
    ));
    assert!(res.success, "mint(caller, 1000) reverted");

    // CALLER approves itself to spend 500 tokens (self-approval).
    // approve(spender=CALLER, 500) → allowances[CALLER][CALLER] = 500
    let res = evm.call(calldata(
        selector("approve(address,uint256)"),
        &[encode_address(caller_addr), encode_u256(500)],
    ));
    assert!(res.success, "approve(caller, 500) reverted");
    assert_eq!(
        decode_u256(&res.output),
        1,
        "approve should return true (1)"
    );

    // Check allowance: CALLER→CALLER should be 500
    let res = evm.call(calldata(
        selector("allowance(address,address)"),
        &[encode_address(caller_addr), encode_address(caller_addr)],
    ));
    assert!(res.success, "allowance(caller, caller) reverted");
    assert_eq!(
        decode_u256(&res.output),
        500,
        "caller should have 500 self-allowance"
    );

    // transferFrom(CALLER, charlie, 300) — spender=@caller()=CALLER
    let res = evm.call(calldata(
        selector("transferFrom(address,address,uint256)"),
        &[
            encode_address(caller_addr),
            encode_address(charlie),
            encode_u256(300),
        ],
    ));
    assert!(res.success, "transferFrom(caller, charlie, 300) reverted");
    assert_eq!(
        decode_u256(&res.output),
        1,
        "transferFrom should return true (1)"
    );

    // Check updated allowance: CALLER→CALLER should be 200 (500 - 300)
    let res = evm.call(calldata(
        selector("allowance(address,address)"),
        &[encode_address(caller_addr), encode_address(caller_addr)],
    ));
    assert!(res.success, "allowance after transferFrom reverted");
    assert_eq!(
        decode_u256(&res.output),
        200,
        "self-allowance should be 200 after transferFrom"
    );

    // Verify charlie received the tokens
    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(charlie)],
    ));
    assert!(res.success, "balanceOf(charlie) reverted");
    assert_eq!(
        decode_u256(&res.output),
        300,
        "charlie balance should be 300 after transferFrom"
    );
}

// =============================================================================
// Additional lifecycle tests (using test_erc20.edge)
// =============================================================================

#[test]
fn test_erc20_total_supply_after_mint() {
    let bytecode = compile_contract("examples/tests/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let alice = test_address(0x02);

    // Mint 1000 tokens
    let res = evm.call(calldata(
        selector("mint(address,uint256)"),
        &[encode_address(alice), encode_u256(1000)],
    ));
    assert!(res.success, "mint reverted");

    // totalSupply should be 1000
    let res = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(res.success, "totalSupply() reverted");
    assert_eq!(
        decode_u256(&res.output),
        1000,
        "totalSupply should be 1000 after mint"
    );
}

#[test]
fn test_erc20_transfer_updates_both_balances() {
    let bytecode = compile_contract("examples/tests/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let caller_addr: [u8; 20] = CALLER.0 .0;
    let alice = test_address(0x02);

    // Mint 1000 to CALLER
    let res = evm.call(calldata(
        selector("mint(address,uint256)"),
        &[encode_address(caller_addr), encode_u256(1000)],
    ));
    assert!(res.success, "mint reverted");

    // Transfer 400 to alice
    let res = evm.call(calldata(
        selector("transfer(address,uint256)"),
        &[encode_address(alice), encode_u256(400)],
    ));
    assert!(res.success, "transfer reverted");

    // Caller: 600, Alice: 400
    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(caller_addr)],
    ));
    assert!(res.success, "balanceOf(caller) reverted");
    assert_eq!(decode_u256(&res.output), 600);

    let res = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(alice)],
    ));
    assert!(res.success, "balanceOf(alice) reverted");
    assert_eq!(decode_u256(&res.output), 400);
}

#[test]
fn test_erc20_approve_sets_allowance() {
    let bytecode = compile_contract("examples/tests/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let caller_addr: [u8; 20] = CALLER.0 .0;
    let alice = test_address(0x02);

    // approve(alice, 500) → allowances[CALLER][alice] = 500
    let res = evm.call(calldata(
        selector("approve(address,uint256)"),
        &[encode_address(alice), encode_u256(500)],
    ));
    assert!(res.success, "approve reverted");
    assert_eq!(decode_u256(&res.output), 1, "approve should return true");

    // Check allowance
    let res = evm.call(calldata(
        selector("allowance(address,address)"),
        &[encode_address(caller_addr), encode_address(alice)],
    ));
    assert!(res.success, "allowance reverted");
    assert_eq!(decode_u256(&res.output), 500, "allowance should be 500");
}

#[test]
fn test_erc20_unknown_selector_reverts() {
    let bytecode = compile_contract("examples/tests/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);
    let res = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!res.success, "unknown selector should revert");
}
