//! Runtime gas efficiency benchmarks.
//!
//! Measures per-function gas usage across -O0, -O1, -O2 for each contract,
//! printing a comparison table and asserting that higher optimization levels
//! never regress runtime gas (i.e. are never worse than -O0).

use alloy_primitives::{Address, U256};
use edge_evm_tests::{abi_encode_address, abi_encode_u256, fn_selector, EvmTestHost};

const COUNTER_PATH: &str = "../../examples/counter.edge";
const ERC20_PATH: &str = "../../examples/test_erc20.edge";
const OPTIMIZABLE_PATH: &str = "../../examples/optimizable.edge";

fn addr(n: u8) -> Address {
    Address::from([n; 20])
}

/// Per-function gas measurement.
struct FnGas {
    name: &'static str,
    gas: [u64; 3], // O0, O1, O2
}

/// Per-contract runtime metrics across optimization levels.
struct ContractMetrics {
    name: &'static str,
    runtime_size: [usize; 3],
    fn_gas: Vec<FnGas>,
}

fn print_metrics(m: &ContractMetrics) {
    println!();
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  {:<64}║", format!("{} — Runtime Gas Efficiency", m.name));
    println!("╠══════════════════════╦═══════════╦═══════════╦══════════════════╣");
    println!("║ Function             ║    -O0    ║    -O1    ║    -O2           ║");
    println!("╠══════════════════════╬═══════════╬═══════════╬══════════════════╣");
    println!(
        "║ runtime size (bytes) ║ {:>7}   ║ {:>7}   ║ {:>7} {:>8} ║",
        m.runtime_size[0],
        m.runtime_size[1],
        m.runtime_size[2],
        delta_pct(m.runtime_size[0], m.runtime_size[2]),
    );
    println!("╠══════════════════════╬═══════════╬═══════════╬══════════════════╣");
    for f in &m.fn_gas {
        let d01 = delta_pct_u64(f.gas[0], f.gas[1]);
        let d02 = delta_pct_u64(f.gas[0], f.gas[2]);
        println!(
            "║ {:>20} ║ {:>7}   ║ {:>7}{:>2} ║ {:>7} {:>8} ║",
            f.name, f.gas[0], f.gas[1], d01, f.gas[2], d02,
        );
    }
    // Total row
    let totals: [u64; 3] = [
        m.fn_gas.iter().map(|f| f.gas[0]).sum(),
        m.fn_gas.iter().map(|f| f.gas[1]).sum(),
        m.fn_gas.iter().map(|f| f.gas[2]).sum(),
    ];
    println!("╠══════════════════════╬═══════════╬═══════════╬══════════════════╣");
    println!(
        "║ {:>20} ║ {:>7}   ║ {:>7}{:>2} ║ {:>7} {:>8} ║",
        "TOTAL",
        totals[0],
        totals[1],
        delta_pct_u64(totals[0], totals[1]),
        totals[2],
        delta_pct_u64(totals[0], totals[2]),
    );
    println!("╚══════════════════════╩═══════════╩═══════════╩══════════════════╝");
}

fn delta_pct(base: usize, opt: usize) -> String {
    if base == 0 {
        return String::new();
    }
    let pct = ((opt as f64 - base as f64) / base as f64 * 100.0).round() as i64;
    if pct == 0 {
        "—".to_string()
    } else if pct < 0 {
        format!("{pct}%")
    } else {
        format!("+{pct}%")
    }
}

fn delta_pct_u64(base: u64, opt: u64) -> String {
    if base == 0 {
        return String::new();
    }
    let pct = ((opt as f64 - base as f64) / base as f64 * 100.0).round() as i64;
    if pct == 0 {
        "—".to_string()
    } else if pct < 0 {
        format!("{pct}%")
    } else {
        format!("+{pct}%")
    }
}

// ── Counter ─────────────────────────────────────────────────────────────────

fn measure_counter(opt_level: u8) -> (usize, Vec<(&'static str, u64)>) {
    let mut host = EvmTestHost::deploy_edge(COUNTER_PATH, opt_level);
    let runtime_size = host.runtime_code_size();
    let mut fn_gas = Vec::new();

    let get = fn_selector("get()");
    let inc = fn_selector("increment()");
    let dec = fn_selector("decrement()");
    let reset = fn_selector("reset()");

    // Warm up storage with an increment so subsequent ops are on warm slots
    host.call(inc, &[]);

    fn_gas.push(("get()", host.call(get, &[]).gas_used));
    fn_gas.push(("increment()", host.call(inc, &[]).gas_used));
    fn_gas.push(("decrement()", host.call(dec, &[]).gas_used));
    fn_gas.push(("reset()", host.call(reset, &[]).gas_used));

    (runtime_size, fn_gas)
}

#[test]
fn counter_optimization_efficiency() {
    let levels: Vec<(usize, Vec<(&str, u64)>)> = (0..=2).map(|o| measure_counter(o)).collect();

    let fn_names: Vec<&str> = levels[0].1.iter().map(|(n, _)| *n).collect();
    let metrics = ContractMetrics {
        name: "Counter",
        runtime_size: [levels[0].0, levels[1].0, levels[2].0],
        fn_gas: fn_names
            .iter()
            .enumerate()
            .map(|(i, &name)| FnGas {
                name,
                gas: [levels[0].1[i].1, levels[1].1[i].1, levels[2].1[i].1],
            })
            .collect(),
    };
    print_metrics(&metrics);

    // Assert no function got significantly more expensive at O2 vs O0.
    // Allow up to 0.1% tolerance for minor bytecode shape differences.
    for f in &metrics.fn_gas {
        let tolerance = f.gas[0] / 1000; // 0.1%
        assert!(
            f.gas[2] <= f.gas[0] + tolerance,
            "{}: O2 gas ({}) should be <= O0 ({}) + 0.1% tolerance ({})",
            f.name,
            f.gas[2],
            f.gas[0],
            f.gas[0] + tolerance,
        );
    }
}

// ── ERC20 ───────────────────────────────────────────────────────────────────

fn measure_erc20(opt_level: u8) -> (usize, Vec<(&'static str, u64)>) {
    let mut host = EvmTestHost::deploy_edge(ERC20_PATH, opt_level);
    let runtime_size = host.runtime_code_size();
    let mut fn_gas = Vec::new();

    let deployer = host.caller();
    let alice = addr(0x0A);
    let bob = addr(0x0B);

    let total_supply = fn_selector("totalSupply()");
    let balance_of = fn_selector("balanceOf(address)");
    let mint = fn_selector("mint(address,uint256)");
    let transfer = fn_selector("transfer(address,uint256)");
    let approve = fn_selector("approve(address,uint256)");
    let allowance = fn_selector("allowance(address,address)");
    let transfer_from = fn_selector("transferFrom(address,address,uint256)");

    // Seed state so we measure warm-slot operations
    host.call(mint, &args_addr_u256(deployer, U256::from(10000)));
    host.call(mint, &args_addr_u256(alice, U256::from(10000)));

    fn_gas.push(("totalSupply()", host.call(total_supply, &[]).gas_used));
    fn_gas.push((
        "balanceOf()",
        host.call(balance_of, &abi_encode_address(deployer))
            .gas_used,
    ));
    fn_gas.push((
        "mint()",
        host.call(mint, &args_addr_u256(bob, U256::from(100)))
            .gas_used,
    ));
    fn_gas.push((
        "transfer()",
        host.call(transfer, &args_addr_u256(bob, U256::from(100)))
            .gas_used,
    ));
    fn_gas.push((
        "approve()",
        host.call(approve, &args_addr_u256(alice, U256::from(500)))
            .gas_used,
    ));
    fn_gas.push((
        "allowance()",
        host.call(allowance, &args_addr_addr(deployer, alice))
            .gas_used,
    ));

    // Set up allowance for transferFrom
    host.set_caller(alice);
    host.call(approve, &args_addr_u256(deployer, U256::from(5000)));
    host.set_caller(deployer);
    fn_gas.push((
        "transferFrom()",
        host.call(
            transfer_from,
            &args_addr_addr_u256(alice, bob, U256::from(50)),
        )
        .gas_used,
    ));

    (runtime_size, fn_gas)
}

// ── Optimizable ─────────────────────────────────────────────────────────────

fn measure_optimizable(opt_level: u8) -> (usize, Vec<(&'static str, u64)>) {
    let mut host = EvmTestHost::deploy_edge(OPTIMIZABLE_PATH, opt_level);
    let runtime_size = host.runtime_code_size();
    let mut fn_gas = Vec::new();

    let get = fn_selector("get()");
    let compute = fn_selector("compute(uint256)");
    let zero = fn_selector("zero(uint256)");

    let x = U256::from(42);

    fn_gas.push(("get()", host.call(get, &[]).gas_used));
    println!(
        "{:?}{:?}",
        alloy_primitives::hex::encode(compute),
        alloy_primitives::hex::encode(abi_encode_u256(x))
    );
    fn_gas.push((
        "compute(42)",
        host.call(compute, &abi_encode_u256(x)).gas_used,
    ));
    fn_gas.push(("zero(42)", host.call(zero, &abi_encode_u256(x)).gas_used));

    (runtime_size, fn_gas)
}

#[test]
fn optimizable_optimization_efficiency() {
    let levels: Vec<(usize, Vec<(&str, u64)>)> = (0..=2).map(|o| measure_optimizable(o)).collect();

    let fn_names: Vec<&str> = levels[0].1.iter().map(|(n, _)| *n).collect();
    let metrics = ContractMetrics {
        name: "Optimizable",
        runtime_size: [levels[0].0, levels[1].0, levels[2].0],
        fn_gas: fn_names
            .iter()
            .enumerate()
            .map(|(i, &name)| FnGas {
                name,
                gas: [levels[0].1[i].1, levels[1].1[i].1, levels[2].1[i].1],
            })
            .collect(),
    };
    print_metrics(&metrics);

    // Assert no function got significantly more expensive at O2 vs O0.
    // Allow up to 0.1% tolerance for minor bytecode shape differences.
    for f in &metrics.fn_gas {
        let tolerance = f.gas[0] / 1000; // 0.1%
        assert!(
            f.gas[2] <= f.gas[0] + tolerance,
            "{}: O2 gas ({}) should be <= O0 ({}) + 0.1% tolerance ({})",
            f.name,
            f.gas[2],
            f.gas[0],
            f.gas[0] + tolerance,
        );
    }
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

#[test]
fn erc20_optimization_efficiency() {
    let levels: Vec<(usize, Vec<(&str, u64)>)> = (0..=2).map(|o| measure_erc20(o)).collect();

    let fn_names: Vec<&str> = levels[0].1.iter().map(|(n, _)| *n).collect();
    let metrics = ContractMetrics {
        name: "ERC20",
        runtime_size: [levels[0].0, levels[1].0, levels[2].0],
        fn_gas: fn_names
            .iter()
            .enumerate()
            .map(|(i, &name)| FnGas {
                name,
                gas: [levels[0].1[i].1, levels[1].1[i].1, levels[2].1[i].1],
            })
            .collect(),
    };
    print_metrics(&metrics);

    // Assert no function got significantly more expensive at O2 vs O0.
    // Allow up to 0.1% tolerance for minor bytecode shape differences.
    for f in &metrics.fn_gas {
        let tolerance = f.gas[0] / 1000; // 0.1%
        assert!(
            f.gas[2] <= f.gas[0] + tolerance,
            "{}: O2 gas ({}) should be <= O0 ({}) + 0.1% tolerance ({})",
            f.name,
            f.gas[2],
            f.gas[0],
            f.gas[0] + tolerance,
        );
    }
}
