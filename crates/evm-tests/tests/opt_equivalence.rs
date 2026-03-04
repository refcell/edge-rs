//! Optimization equivalence tests.
//!
//! Runs the same call sequences at -O0, -O1, and -O2 and asserts identical
//! return values and storage state. This catches optimizer bugs that change
//! program semantics.

use alloy_primitives::{Address, U256};
use edge_evm_tests::{
    abi_decode_u256, abi_encode_address, abi_encode_u256, fn_selector, EvmTestHost,
};

const COUNTER_PATH: &str = "../../examples/counter.edge";
const ERC20_PATH: &str = "../../examples/test_erc20.edge";

fn addr(n: u8) -> Address {
    Address::from([n; 20])
}

// ── Counter optimization equivalence ────────────────────────────────────────

fn counter_sequence(opt_level: u8) -> (Vec<U256>, U256) {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, opt_level);
    let get = fn_selector("get()");
    let inc = fn_selector("increment()");
    let dec = fn_selector("decrement()");
    let reset = fn_selector("reset()");

    let mut results = Vec::new();

    // get → 0
    results.push(abi_decode_u256(&host.call(get, &[]).output));

    // increment 3 times
    let res = host.call(inc, &[]);
    println!("res: {res:?}");
    host.call(inc, &[]);
    host.call(inc, &[]);
    results.push(abi_decode_u256(&host.call(get, &[]).output));

    // decrement once
    host.call(dec, &[]);
    results.push(abi_decode_u256(&host.call(get, &[]).output));

    // reset
    host.call(reset, &[]);
    results.push(abi_decode_u256(&host.call(get, &[]).output));

    // increment again
    host.call(inc, &[]);
    results.push(abi_decode_u256(&host.call(get, &[]).output));

    let storage_slot_0 = host.sload(U256::ZERO);
    (results, storage_slot_0)
}

#[test]
fn counter_o0_o1_equivalent() {
    let (results_o0, storage_o0) = counter_sequence(0);
    let (results_o1, storage_o1) = counter_sequence(1);

    assert_eq!(
        results_o0, results_o1,
        "Counter O0 vs O1: return values differ"
    );
    assert_eq!(
        storage_o0, storage_o1,
        "Counter O0 vs O1: storage slot 0 differs"
    );
}

#[test]
fn counter_o0_o2_equivalent() {
    let (results_o0, storage_o0) = counter_sequence(0);
    let (results_o2, storage_o2) = counter_sequence(2);

    assert_eq!(
        results_o0, results_o2,
        "Counter O0 vs O2: return values differ"
    );
    assert_eq!(
        storage_o0, storage_o2,
        "Counter O0 vs O2: storage slot 0 differs"
    );
}

// ── ERC20 optimization equivalence ──────────────────────────────────────────

fn erc20_sequence(opt_level: u8) -> Vec<U256> {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, opt_level);
    let alice = addr(0x0A);
    let bob = addr(0x0B);

    let total_supply = fn_selector("totalSupply()");
    let balance_of = fn_selector("balanceOf(address)");
    let mint = fn_selector("mint(address,uint256)");

    let mut results = Vec::new();

    // Initial state
    results.push(abi_decode_u256(&host.call(total_supply, &[]).output));
    results.push(abi_decode_u256(
        &host.call(balance_of, &abi_encode_address(alice)).output,
    ));

    // Mint 1000 to alice
    let mut args = abi_encode_address(alice);
    args.extend_from_slice(&abi_encode_u256(U256::from(1000)));
    host.call(mint, &args);

    results.push(abi_decode_u256(&host.call(total_supply, &[]).output));
    results.push(abi_decode_u256(
        &host.call(balance_of, &abi_encode_address(alice)).output,
    ));

    // Mint 2000 to bob
    let mut args = abi_encode_address(bob);
    args.extend_from_slice(&abi_encode_u256(U256::from(2000)));
    host.call(mint, &args);

    results.push(abi_decode_u256(&host.call(total_supply, &[]).output));
    results.push(abi_decode_u256(
        &host.call(balance_of, &abi_encode_address(alice)).output,
    ));
    results.push(abi_decode_u256(
        &host.call(balance_of, &abi_encode_address(bob)).output,
    ));

    // Mint more to alice
    let mut args = abi_encode_address(alice);
    args.extend_from_slice(&abi_encode_u256(U256::from(500)));
    host.call(mint, &args);

    results.push(abi_decode_u256(&host.call(total_supply, &[]).output));
    results.push(abi_decode_u256(
        &host.call(balance_of, &abi_encode_address(alice)).output,
    ));

    results
}

#[test]
fn erc20_o0_o1_equivalent() {
    let results_o0 = erc20_sequence(0);
    let results_o1 = erc20_sequence(1);

    assert_eq!(
        results_o0, results_o1,
        "ERC20 O0 vs O1: return values differ"
    );
}

#[test]
fn erc20_o0_o2_equivalent() {
    let results_o0 = erc20_sequence(0);
    let results_o2 = erc20_sequence(2);

    assert_eq!(
        results_o0, results_o2,
        "ERC20 O0 vs O2: return values differ"
    );
}
