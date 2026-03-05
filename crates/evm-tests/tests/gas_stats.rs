//! Runtime gas statistics for counter and ERC20 interactions.
//!
//! Compares gas costs between OptimizeFor::Gas and OptimizeFor::Size at each
//! optimization level (O0, O1, O2).

use alloy_primitives::{Address, U256};
use edge_evm_tests::{
    abi_decode_u256, abi_encode_address, abi_encode_u256, fn_selector, DeployResult, EvmTestHost,
};

const COUNTER_PATH: &str = "../../examples/counter.edge";
const ERC20_PATH: &str = "../../examples/test_erc20.edge";

fn addr(n: u8) -> Address {
    Address::from([n; 20])
}
fn args_addr_u256(a: Address, v: U256) -> Vec<u8> {
    let mut args = abi_encode_address(a);
    args.extend_from_slice(&abi_encode_u256(v));
    args
}
fn args_addr_addr_u256(a: Address, b: Address, v: U256) -> Vec<u8> {
    let mut args = abi_encode_address(a);
    args.extend_from_slice(&abi_encode_address(b));
    args.extend_from_slice(&abi_encode_u256(v));
    args
}

struct Stats {
    rows: Vec<(&'static str, u64)>,
}

fn counter_stats(deploy: DeployResult) -> Stats {
    let mut host = deploy.host;
    let runtime_size = host.runtime_code_size();
    let mut rows: Vec<(&str, u64)> = vec![
        ("deploy", deploy.deploy_gas),
        ("init code (bytes)", deploy.init_code_size as u64),
        ("runtime code (bytes)", runtime_size as u64),
    ];

    let r = host.call(fn_selector("get()"), &[]);
    assert!(r.success);
    rows.push(("get (cold)", r.gas_used));

    let r = host.call(fn_selector("increment()"), &[]);
    assert!(r.success);
    rows.push(("increment (0->1)", r.gas_used));

    let r = host.call(fn_selector("get()"), &[]);
    assert!(r.success);
    rows.push(("get (warm)", r.gas_used));

    let r = host.call(fn_selector("increment()"), &[]);
    assert!(r.success);
    rows.push(("increment (1->2)", r.gas_used));

    let r = host.call(fn_selector("decrement()"), &[]);
    assert!(r.success);
    rows.push(("decrement (2->1)", r.gas_used));

    let r = host.call(fn_selector("reset()"), &[]);
    assert!(r.success);
    rows.push(("reset (1->0)", r.gas_used));

    Stats { rows }
}

fn erc20_stats(deploy: DeployResult) -> Stats {
    let mut host = deploy.host;
    let runtime_size = host.runtime_code_size();
    let deployer = host.caller();
    let alice = addr(0x0A);
    let bob = addr(0x0B);

    let mut rows: Vec<(&str, u64)> = vec![
        ("deploy", deploy.deploy_gas),
        ("init code (bytes)", deploy.init_code_size as u64),
        ("runtime code (bytes)", runtime_size as u64),
    ];

    let r = host.call(fn_selector("totalSupply()"), &[]);
    assert!(r.success);
    rows.push(("totalSupply (cold)", r.gas_used));

    let r = host.call(
        fn_selector("balanceOf(address)"),
        &abi_encode_address(alice),
    );
    assert!(r.success);
    rows.push(("balanceOf (zero)", r.gas_used));

    let r = host.call(
        fn_selector("mint(address,uint256)"),
        &args_addr_u256(alice, U256::from(1000)),
    );
    assert!(r.success);
    rows.push(("mint (1st acct)", r.gas_used));

    let r = host.call(
        fn_selector("mint(address,uint256)"),
        &args_addr_u256(bob, U256::from(2000)),
    );
    assert!(r.success);
    rows.push(("mint (2nd acct)", r.gas_used));

    let r = host.call(
        fn_selector("balanceOf(address)"),
        &abi_encode_address(alice),
    );
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(1000));
    rows.push(("balanceOf (warm)", r.gas_used));

    let r = host.call(fn_selector("totalSupply()"), &[]);
    assert!(r.success);
    rows.push(("totalSupply (warm)", r.gas_used));

    host.set_caller(alice);
    let r = host.call(
        fn_selector("transfer(address,uint256)"),
        &args_addr_u256(bob, U256::from(300)),
    );
    assert!(r.success);
    rows.push(("transfer", r.gas_used));

    let r = host.call(
        fn_selector("approve(address,uint256)"),
        &args_addr_u256(deployer, U256::from(500)),
    );
    assert!(r.success);
    rows.push(("approve", r.gas_used));

    host.set_caller(deployer);
    let r = host.call(
        fn_selector("transferFrom(address,address,uint256)"),
        &args_addr_addr_u256(alice, bob, U256::from(200)),
    );
    assert!(r.success);
    rows.push(("transferFrom", r.gas_used));

    Stats { rows }
}

fn delta_str(baseline: u64, val: u64) -> String {
    if baseline == 0 {
        return "n/a".to_string();
    }
    let diff = val as i64 - baseline as i64;
    let pct = (diff as f64 / baseline as f64) * 100.0;
    format!("{:+} ({:+.1}%)", diff, pct)
}

fn print_table(title: &str, col_labels: &[&str], datasets: &[&Stats]) {
    let w = 22; // operation column width
    let c = 12; // data column width
    let d = 16; // delta column width

    println!("\n  {title}");
    print!("  {:<w$}", "");
    for label in col_labels {
        print!("{:>c$}", label);
    }
    if col_labels.len() > 1 {
        print!("  {:>d$}", "delta");
    }
    println!();
    println!(
        "  {}",
        "-".repeat(w + c * col_labels.len() + if col_labels.len() > 1 { 2 + d } else { 0 })
    );

    let n = datasets[0].rows.len();
    for i in 0..n {
        let name = datasets[0].rows[i].0;
        print!("  {:<w$}", name);
        let vals: Vec<u64> = datasets.iter().map(|s| s.rows[i].1).collect();
        for &v in &vals {
            print!("{:>c$}", v);
        }
        if vals.len() > 1 {
            print!("  {:>d$}", delta_str(vals[0], vals[vals.len() - 1]));
        }
        println!();
    }
}

#[test]
fn gas_statistics() {
    // Collect all data first
    let c_g0 = counter_stats(EvmTestHost::deploy_edge_measured(COUNTER_PATH, 0));
    let c_g1 = counter_stats(EvmTestHost::deploy_edge_measured(COUNTER_PATH, 1));
    let c_g2 = counter_stats(EvmTestHost::deploy_edge_measured(COUNTER_PATH, 2));
    let c_s0 = counter_stats(EvmTestHost::deploy_edge_for_size_measured(COUNTER_PATH, 0));
    let c_s1 = counter_stats(EvmTestHost::deploy_edge_for_size_measured(COUNTER_PATH, 1));
    let c_s2 = counter_stats(EvmTestHost::deploy_edge_for_size_measured(COUNTER_PATH, 2));

    let e_g0 = erc20_stats(EvmTestHost::deploy_edge_measured(ERC20_PATH, 0));
    let e_g1 = erc20_stats(EvmTestHost::deploy_edge_measured(ERC20_PATH, 1));
    let e_g2 = erc20_stats(EvmTestHost::deploy_edge_measured(ERC20_PATH, 2));
    let e_s0 = erc20_stats(EvmTestHost::deploy_edge_for_size_measured(ERC20_PATH, 0));
    let e_s1 = erc20_stats(EvmTestHost::deploy_edge_for_size_measured(ERC20_PATH, 1));
    let e_s2 = erc20_stats(EvmTestHost::deploy_edge_for_size_measured(ERC20_PATH, 2));

    println!("\n{}", "=".repeat(70));
    println!("  COUNTER — optimize for gas (O0 -> O1 -> O2)");
    println!("{}", "=".repeat(70));
    print_table(
        "Gas mode across opt levels",
        &["O0", "O1", "O2"],
        &[&c_g0, &c_g1, &c_g2],
    );
    print_table(
        "Size mode across opt levels",
        &["O0", "O1", "O2"],
        &[&c_s0, &c_s1, &c_s2],
    );
    print_table("Gas vs Size at O2", &["gas", "size"], &[&c_g2, &c_s2]);

    println!("\n{}", "=".repeat(70));
    println!("  ERC20 — optimize for gas vs size");
    println!("{}", "=".repeat(70));
    print_table(
        "Gas mode across opt levels",
        &["O0", "O1", "O2"],
        &[&e_g0, &e_g1, &e_g2],
    );
    print_table(
        "Size mode across opt levels",
        &["O0", "O1", "O2"],
        &[&e_s0, &e_s1, &e_s2],
    );
    print_table("Gas vs Size at O2", &["gas", "size"], &[&e_g2, &e_s2]);

    println!();
}
