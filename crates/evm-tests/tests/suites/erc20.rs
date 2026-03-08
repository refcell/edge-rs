//! Semantic tests for `test_erc20.edge` (ERC20 with public mint)
//!
//! Deploys the ERC20 contract on an in-memory EVM and tests core functionality.

use alloy_primitives::{Address, U256};
use edge_evm_tests::{
    abi_decode_bool, abi_decode_u256, abi_encode_address, abi_encode_u256, fn_selector, EvmTestHost,
};

const ERC20_PATH: &str = "../../examples/tests/test_erc20.edge";

fn sel_total_supply() -> [u8; 4] {
    fn_selector("totalSupply()")
}
fn sel_balance_of() -> [u8; 4] {
    fn_selector("balanceOf(address)")
}
fn sel_transfer() -> [u8; 4] {
    fn_selector("transfer(address,uint256)")
}
fn sel_allowance() -> [u8; 4] {
    fn_selector("allowance(address,address)")
}
fn sel_approve() -> [u8; 4] {
    fn_selector("approve(address,uint256)")
}
fn sel_transfer_from() -> [u8; 4] {
    fn_selector("transferFrom(address,address,uint256)")
}
fn sel_mint() -> [u8; 4] {
    fn_selector("mint(address,uint256)")
}

fn addr(n: u8) -> Address {
    Address::from([n; 20])
}

fn args_addr_u256(a: Address, v: U256) -> Vec<u8> {
    let mut args = abi_encode_address(a);
    args.extend_from_slice(&abi_encode_u256(v));
    args
}

fn args_addr_addr(a: Address, b: Address) -> Vec<u8> {
    let mut args = abi_encode_address(a);
    args.extend_from_slice(&abi_encode_address(b));
    args
}

fn args_addr_addr_u256(a: Address, b: Address, v: U256) -> Vec<u8> {
    let mut args = abi_encode_address(a);
    args.extend_from_slice(&abi_encode_address(b));
    args.extend_from_slice(&abi_encode_u256(v));
    args
}

// ── Deploy ──────────────────────────────────────────────────────────────────

#[test]
fn erc20_deploy() {
    let _host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
}

// ── totalSupply ─────────────────────────────────────────────────────────────

#[test]
fn erc20_initial_total_supply_is_zero() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let r = host.call(sel_total_supply(), &[]);
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::ZERO);
}

// ── balanceOf ───────────────────────────────────────────────────────────────

#[test]
fn erc20_initial_balance_is_zero() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let alice = addr(0x0A);
    let r = host.call(sel_balance_of(), &abi_encode_address(alice));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::ZERO);
}

// ── mint ────────────────────────────────────────────────────────────────────

#[test]
fn erc20_mint_increases_balance_and_supply() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let alice = addr(0x0A);

    let r = host.call(sel_mint(), &args_addr_u256(alice, U256::from(1000)));
    assert!(r.success, "mint() should succeed");

    let r = host.call(sel_balance_of(), &abi_encode_address(alice));
    assert_eq!(abi_decode_u256(&r.output), U256::from(1000));

    let r = host.call(sel_total_supply(), &[]);
    assert_eq!(abi_decode_u256(&r.output), U256::from(1000));
}

#[test]
fn erc20_mint_emits_transfer_event() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let alice = addr(0x0A);

    let r = host.call(sel_mint(), &args_addr_u256(alice, U256::from(500)));
    assert!(r.success);

    // Should emit Transfer(address(0), alice, 500)
    assert_eq!(r.logs.len(), 1, "mint should emit 1 log");
    let log = &r.logs[0];
    let expected_topic0 = alloy_primitives::keccak256("Transfer(address,address,uint256)");
    assert_eq!(log.data.topics()[0], expected_topic0);
}

#[test]
fn erc20_multiple_mints_accumulate() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let alice = addr(0x0A);

    host.call(sel_mint(), &args_addr_u256(alice, U256::from(100)));
    host.call(sel_mint(), &args_addr_u256(alice, U256::from(200)));
    host.call(sel_mint(), &args_addr_u256(alice, U256::from(300)));

    let r = host.call(sel_balance_of(), &abi_encode_address(alice));
    assert_eq!(abi_decode_u256(&r.output), U256::from(600));

    let r = host.call(sel_total_supply(), &[]);
    assert_eq!(abi_decode_u256(&r.output), U256::from(600));
}

#[test]
fn erc20_mint_to_multiple_accounts() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let alice = addr(0x0A);
    let bob = addr(0x0B);

    host.call(sel_mint(), &args_addr_u256(alice, U256::from(1000)));
    host.call(sel_mint(), &args_addr_u256(bob, U256::from(2000)));

    let r = host.call(sel_balance_of(), &abi_encode_address(alice));
    assert_eq!(abi_decode_u256(&r.output), U256::from(1000));

    let r = host.call(sel_balance_of(), &abi_encode_address(bob));
    assert_eq!(abi_decode_u256(&r.output), U256::from(2000));

    let r = host.call(sel_total_supply(), &[]);
    assert_eq!(abi_decode_u256(&r.output), U256::from(3000));
}

// ── allowance ───────────────────────────────────────────────────────────────

#[test]
fn erc20_initial_allowance_is_zero() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let alice = addr(0x0A);
    let bob = addr(0x0B);

    let r = host.call(sel_allowance(), &args_addr_addr(alice, bob));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::ZERO);
}

// ── transfer ────────────────────────────────────────────────────────────────

#[test]
fn erc20_transfer() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let deployer = host.caller();
    let bob = addr(0x0B);

    host.call(sel_mint(), &args_addr_u256(deployer, U256::from(1000)));

    let r = host.call(sel_transfer(), &args_addr_u256(bob, U256::from(300)));
    assert!(r.success);
    assert!(abi_decode_bool(&r.output), "transfer should return true");

    let r = host.call(sel_balance_of(), &abi_encode_address(deployer));
    assert_eq!(abi_decode_u256(&r.output), U256::from(700));

    let r = host.call(sel_balance_of(), &abi_encode_address(bob));
    assert_eq!(abi_decode_u256(&r.output), U256::from(300));
}

#[test]
fn erc20_transfer_emits_event() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let deployer = host.caller();
    let bob = addr(0x0B);

    host.call(sel_mint(), &args_addr_u256(deployer, U256::from(1000)));

    let r = host.call(sel_transfer(), &args_addr_u256(bob, U256::from(300)));
    assert!(r.success);
    assert!(!r.logs.is_empty(), "transfer should emit events");

    let expected_topic0 = alloy_primitives::keccak256("Transfer(address,address,uint256)");
    assert_eq!(r.logs[0].data.topics()[0], expected_topic0);
}

#[test]
fn erc20_transfer_preserves_total_supply() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let deployer = host.caller();
    let bob = addr(0x0B);

    host.call(sel_mint(), &args_addr_u256(deployer, U256::from(1000)));
    host.call(sel_transfer(), &args_addr_u256(bob, U256::from(300)));

    let r = host.call(sel_total_supply(), &[]);
    assert_eq!(abi_decode_u256(&r.output), U256::from(1000));
}

// ── approve / allowance ─────────────────────────────────────────────────────

#[test]
fn erc20_approve_and_allowance() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let deployer = host.caller();
    let spender = addr(0x0C);

    let r = host.call(sel_approve(), &args_addr_u256(spender, U256::from(500)));
    assert!(r.success);
    assert!(abi_decode_bool(&r.output), "approve should return true");

    let r = host.call(sel_allowance(), &args_addr_addr(deployer, spender));
    assert_eq!(abi_decode_u256(&r.output), U256::from(500));
}

#[test]
fn erc20_approve_emits_approval_event() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let spender = addr(0x0C);

    let r = host.call(sel_approve(), &args_addr_u256(spender, U256::from(500)));
    assert!(r.success);
    assert_eq!(r.logs.len(), 1, "approve should emit 1 log");

    let expected_topic0 = alloy_primitives::keccak256("Approval(address,address,uint256)");
    assert_eq!(r.logs[0].data.topics()[0], expected_topic0);
}

// ── transferFrom ────────────────────────────────────────────────────────────

#[test]
fn erc20_transfer_from() {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, 0);
    let alice = addr(0x0A);
    let bob = addr(0x0B);
    let deployer = host.caller();

    host.call(sel_mint(), &args_addr_u256(alice, U256::from(1000)));

    // Alice approves deployer for 500
    host.set_caller(alice);
    host.call(sel_approve(), &args_addr_u256(deployer, U256::from(500)));

    // Deployer calls transferFrom(alice, bob, 200)
    host.set_caller(deployer);
    let r = host.call(
        sel_transfer_from(),
        &args_addr_addr_u256(alice, bob, U256::from(200)),
    );
    assert!(r.success);
    assert!(abi_decode_bool(&r.output));

    let r = host.call(sel_balance_of(), &abi_encode_address(alice));
    assert_eq!(abi_decode_u256(&r.output), U256::from(800));

    let r = host.call(sel_balance_of(), &abi_encode_address(bob));
    assert_eq!(abi_decode_u256(&r.output), U256::from(200));

    let r = host.call(sel_allowance(), &args_addr_addr(alice, deployer));
    assert_eq!(abi_decode_u256(&r.output), U256::from(300));
}
