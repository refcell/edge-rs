//! Tests for subroutine extraction (size-optimized mode).
//!
//! Verifies that contracts compiled with --optimize-for=size use
//! subroutine extraction and still produce correct results.
//!
//! Note: We test at O1 rather than O2 because the O2 bytecode-level
//! optimizer has a pre-existing bug in size mode that's unrelated to
//! subroutine extraction.

use alloy_primitives::{Address, U256};
use edge_evm_tests::{
    abi_decode_u256, abi_encode_address, abi_encode_u256, compile_edge, compile_edge_for_size,
    fn_selector, EvmTestHost,
};

const ERC20_PATH: &str = "../../examples/test_erc20.edge";

fn addr(n: u8) -> Address {
    Address::from([n; 20])
}

fn args_addr_u256(a: Address, v: U256) -> Vec<u8> {
    let mut args = abi_encode_address(a);
    args.extend_from_slice(&abi_encode_u256(v));
    args
}

#[test]
fn size_opt_produces_smaller_code() {
    let gas_bytecode = compile_edge(ERC20_PATH, 1);
    let size_bytecode = compile_edge_for_size(ERC20_PATH, 1);
    println!(
        "Gas-optimized: {} bytes, Size-optimized: {} bytes",
        gas_bytecode.len(),
        size_bytecode.len()
    );
    assert!(
        size_bytecode.len() <= gas_bytecode.len(),
        "Size-optimized ({} bytes) should be <= gas-optimized ({} bytes)",
        size_bytecode.len(),
        gas_bytecode.len(),
    );
}

#[test]
fn size_opt_erc20_total_supply() {
    let mut host = EvmTestHost::deploy_edge_for_size(ERC20_PATH, 1);
    let mint = fn_selector("mint(address,uint256)");
    let total = fn_selector("totalSupply()");

    let r = host.call(mint, &args_addr_u256(addr(1), U256::from(1000)));
    assert!(r.success);

    let r = host.call(total, &[]);
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(1000));
}

#[test]
fn size_opt_erc20_transfer() {
    let mut host = EvmTestHost::deploy_edge_for_size(ERC20_PATH, 1);
    let mint = fn_selector("mint(address,uint256)");
    let transfer = fn_selector("transfer(address,uint256)");
    let balance_of = fn_selector("balanceOf(address)");

    let deployer = host.caller();
    let alice = addr(0x0A);

    // Mint to deployer
    let r = host.call(mint, &args_addr_u256(deployer, U256::from(1000)));
    assert!(r.success);

    // Transfer to alice
    let r = host.call(transfer, &args_addr_u256(alice, U256::from(300)));
    assert!(r.success);

    // Check balances
    let r = host.call(balance_of, &abi_encode_address(deployer));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(700));

    let r = host.call(balance_of, &abi_encode_address(alice));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(300));
}

#[test]
fn size_opt_erc20_approve_and_transfer_from() {
    let mut host = EvmTestHost::deploy_edge_for_size(ERC20_PATH, 1);
    let mint = fn_selector("mint(address,uint256)");
    let approve = fn_selector("approve(address,uint256)");
    let transfer_from = fn_selector("transferFrom(address,address,uint256)");
    let balance_of = fn_selector("balanceOf(address)");

    let deployer = host.caller();
    let alice = addr(0x0A);
    let bob = addr(0x0B);

    // Mint to alice
    let r = host.call(mint, &args_addr_u256(alice, U256::from(1000)));
    assert!(r.success);

    // Alice approves deployer
    host.set_caller(alice);
    let r = host.call(approve, &args_addr_u256(deployer, U256::from(500)));
    assert!(r.success);

    // Deployer transfers from alice to bob
    host.set_caller(deployer);
    let mut tf_args = abi_encode_address(alice);
    tf_args.extend_from_slice(&abi_encode_address(bob));
    tf_args.extend_from_slice(&abi_encode_u256(U256::from(200)));
    let r = host.call(transfer_from, &tf_args);
    assert!(r.success);

    // Check balances
    let r = host.call(balance_of, &abi_encode_address(alice));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(800));

    let r = host.call(balance_of, &abi_encode_address(bob));
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(200));
}
