// Stress tests for stack/memory optimization.
// These test correctness and measure gas for contracts that
// exercise LetBind, Concat chains, conditionals, loops, and mappings.

use alloy_primitives::{Address, U256};
use edge_evm_tests::{abi_encode_u256, fn_selector, EvmTestHost};

fn decode_u256(data: &[u8]) -> U256 {
    if data.len() >= 32 {
        U256::from_be_slice(&data[..32])
    } else {
        U256::ZERO
    }
}

fn abi_encode_address(addr: Address) -> Vec<u8> {
    let mut buf = [0u8; 32];
    buf[12..32].copy_from_slice(addr.as_slice());
    buf.to_vec()
}

// ═══════════════════════════════════════════════════════════════════
// stress_variables
// ═══════════════════════════════════════════════════════════════════

fn deploy_stress_vars(opt: u8) -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/stress_variables.edge", opt)
}

#[test]
fn stress_vars_dot_product() {
    let mut host = deploy_stress_vars(0);
    let r = host.call_fn("dot_product()", &[]);
    assert!(r.success, "dot_product failed");
    // 3*2 + 5*4 + 7*6 = 6 + 20 + 42 = 68
    assert_eq!(decode_u256(&r.output), U256::from(68));
    println!("dot_product gas: {}", r.gas_used);
}

#[test]
fn stress_vars_polynomial() {
    let mut host = deploy_stress_vars(0);
    let args = abi_encode_u256(U256::from(2));
    let r = host.call_fn("polynomial(uint256)", &args);
    assert!(r.success, "polynomial failed");
    // Horner's: ((1*2 + 3)*2 + 5)*2 + 7 = (5*2 + 5)*2 + 7 = 15*2 + 7 = 37
    assert_eq!(decode_u256(&r.output), U256::from(37));
    println!("polynomial(2) gas: {}", r.gas_used);
}

#[test]
fn stress_vars_multi_swap() {
    let mut host = deploy_stress_vars(0);
    let mut args = abi_encode_u256(U256::from(10));
    args.extend_from_slice(&abi_encode_u256(U256::from(20)));
    args.extend_from_slice(&abi_encode_u256(U256::from(30)));
    let r = host.call_fn("multi_swap(uint256,uint256,uint256)", &args);
    assert!(r.success, "multi_swap failed");
    // (c + a) * b = (30 + 10) * 20 = 800
    assert_eq!(decode_u256(&r.output), U256::from(800));
    println!("multi_swap gas: {}", r.gas_used);
}

#[test]
fn stress_vars_fibonacci() {
    let mut host = deploy_stress_vars(0);
    let r = host.call_fn("fibonacci_unrolled()", &[]);
    assert!(r.success, "fibonacci failed");
    // fib: 0, 1, 1, 2, 3, 5, 8, 13
    assert_eq!(decode_u256(&r.output), U256::from(13));
    println!("fibonacci_unrolled gas: {}", r.gas_used);
}

#[test]
fn stress_vars_weighted_average() {
    let mut host = deploy_stress_vars(0);
    let mut args = abi_encode_u256(U256::from(100));
    args.extend_from_slice(&abi_encode_u256(U256::from(200)));
    args.extend_from_slice(&abi_encode_u256(U256::from(300)));
    args.extend_from_slice(&abi_encode_u256(U256::from(400)));
    let r = host.call_fn("weighted_average(uint256,uint256,uint256,uint256)", &args);
    assert!(r.success, "weighted_average failed");
    // (100*4 + 200*3 + 300*2 + 400*1) / (4+3+2+1) = (400+600+600+400)/10 = 2000/10 = 200
    assert_eq!(decode_u256(&r.output), U256::from(200));
    println!("weighted_average gas: {}", r.gas_used);
}

// ═══════════════════════════════════════════════════════════════════
// stress_conditionals
// ═══════════════════════════════════════════════════════════════════

fn deploy_stress_cond(opt: u8) -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/stress_conditionals.edge", opt)
}

#[test]
fn stress_cond_classify() {
    let mut host = deploy_stress_cond(0);

    // x=5 → category 0
    let r = host.call_fn("classify(uint256)", &abi_encode_u256(U256::from(5)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(0));

    // x=50 → category 1
    let r = host.call_fn("classify(uint256)", &abi_encode_u256(U256::from(50)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(1));

    // x=500 → category 2
    let r = host.call_fn("classify(uint256)", &abi_encode_u256(U256::from(500)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(2));

    // x=5000 → category 3
    let r = host.call_fn("classify(uint256)", &abi_encode_u256(U256::from(5000)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(3));

    println!("classify gas (worst case): {}", r.gas_used);
}

#[test]
fn stress_cond_clamp() {
    let mut host = deploy_stress_cond(0);

    // x=5, lo=10, hi=100 → clamped to 10
    let mut args = abi_encode_u256(U256::from(5));
    args.extend_from_slice(&abi_encode_u256(U256::from(10)));
    args.extend_from_slice(&abi_encode_u256(U256::from(100)));
    let r = host.call_fn("clamp(uint256,uint256,uint256)", &args);
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(10));

    // x=200, lo=10, hi=100 → clamped to 100
    let mut args = abi_encode_u256(U256::from(200));
    args.extend_from_slice(&abi_encode_u256(U256::from(10)));
    args.extend_from_slice(&abi_encode_u256(U256::from(100)));
    let r = host.call_fn("clamp(uint256,uint256,uint256)", &args);
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(100));

    // x=50, lo=10, hi=100 → stays 50
    let mut args = abi_encode_u256(U256::from(50));
    args.extend_from_slice(&abi_encode_u256(U256::from(10)));
    args.extend_from_slice(&abi_encode_u256(U256::from(100)));
    let r = host.call_fn("clamp(uint256,uint256,uint256)", &args);
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(50));
}

#[test]
fn stress_cond_abs_diff() {
    let mut host = deploy_stress_cond(0);

    // |10 - 3| = 7
    let mut args = abi_encode_u256(U256::from(10));
    args.extend_from_slice(&abi_encode_u256(U256::from(3)));
    let r = host.call_fn("abs_diff(uint256,uint256)", &args);
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(7));

    // |3 - 10| = 7
    let mut args = abi_encode_u256(U256::from(3));
    args.extend_from_slice(&abi_encode_u256(U256::from(10)));
    let r = host.call_fn("abs_diff(uint256,uint256)", &args);
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(7));
}

#[test]
fn stress_cond_tier_price() {
    let mut host = deploy_stress_cond(0);

    // 5 units at tier 1 = 500
    let r = host.call_fn("tier_price(uint256)", &abi_encode_u256(U256::from(5)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(500));

    // 20 units at tier 2 = 1600
    let r = host.call_fn("tier_price(uint256)", &abi_encode_u256(U256::from(20)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(1600));

    // 75 units at tier 3 = 4500
    let r = host.call_fn("tier_price(uint256)", &abi_encode_u256(U256::from(75)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(4500));

    // 200 units at tier 4 = 8000
    let r = host.call_fn("tier_price(uint256)", &abi_encode_u256(U256::from(200)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(8000));
}

// ═══════════════════════════════════════════════════════════════════
// stress_storage
// ═══════════════════════════════════════════════════════════════════

fn deploy_stress_storage(opt: u8) -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/stress_storage.edge", opt)
}

#[test]
fn stress_storage_balance_flow() {
    let mut host = deploy_stress_storage(0);
    let alice = Address::from([0x0A; 20]);
    let bob = Address::from([0x0B; 20]);

    // Check initial balance is 0
    let r = host.call_fn("balance_of(address)", &abi_encode_address(alice));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::ZERO);

    // TODO: deposit needs callvalue support in test host, skip for now

    // Check total is 0
    let r = host.call_fn("total_deposited()", &[]);
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::ZERO);
    println!("balance_of gas: {}", r.gas_used);
}

#[test]
fn stress_storage_swap_balances() {
    let mut host = deploy_stress_storage(0);
    let other = Address::from([0x0B; 20]);

    // swap_balances with both at 0 — should still succeed
    let r = host.call_fn("swap_balances(address)", &abi_encode_address(other));
    assert!(r.success);
    println!("swap_balances gas: {}", r.gas_used);
}

// ═══════════════════════════════════════════════════════════════════
// stress_loops
// ═══════════════════════════════════════════════════════════════════

fn deploy_stress_loops(opt: u8) -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/stress_loops.edge", opt)
}

#[test]
fn stress_loops_sum_to() {
    let mut host = deploy_stress_loops(0);

    // sum(1..10) = 55
    let r = host.call_fn("sum_to(uint256)", &abi_encode_u256(U256::from(10)));
    assert!(r.success, "sum_to failed: {:?}", r.output);
    assert_eq!(decode_u256(&r.output), U256::from(55));
    println!("sum_to(10) gas: {}", r.gas_used);

    // sum(1..100) = 5050
    let r = host.call_fn("sum_to(uint256)", &abi_encode_u256(U256::from(100)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(5050));
    println!("sum_to(100) gas: {}", r.gas_used);
}

#[test]
fn stress_loops_factorial() {
    let mut host = deploy_stress_loops(0);

    // 5! = 120
    let r = host.call_fn("factorial(uint256)", &abi_encode_u256(U256::from(5)));
    assert!(r.success, "factorial failed: {:?}", r.output);
    assert_eq!(decode_u256(&r.output), U256::from(120));
    println!("factorial(5) gas: {}", r.gas_used);

    // 10! = 3628800
    let r = host.call_fn("factorial(uint256)", &abi_encode_u256(U256::from(10)));
    assert!(r.success);
    assert_eq!(decode_u256(&r.output), U256::from(3628800));
    println!("factorial(10) gas: {}", r.gas_used);
}

#[test]
fn stress_loops_collatz() {
    let mut host = deploy_stress_loops(0);

    // collatz(1) = 0 steps (already at 1)
    let r = host.call_fn("collatz_steps(uint256)", &abi_encode_u256(U256::from(1)));
    assert!(r.success, "collatz failed: {:?}", r.output);
    assert_eq!(decode_u256(&r.output), U256::from(0));

    // collatz(6) = 8 steps: 6→3→10→5→16→8→4→2→1
    let r = host.call_fn("collatz_steps(uint256)", &abi_encode_u256(U256::from(6)));
    assert!(r.success, "collatz failed: {:?}", r.output);
    assert_eq!(decode_u256(&r.output), U256::from(8));
    println!("collatz(6) gas: {}", r.gas_used);

    // collatz(27) = 111 steps (famous hard case)
    let r = host.call_fn("collatz_steps(uint256)", &abi_encode_u256(U256::from(27)));
    assert!(r.success, "collatz failed: {:?}", r.output);
    assert_eq!(decode_u256(&r.output), U256::from(111));
    println!("collatz(27) gas: {}", r.gas_used);
}

// ═══════════════════════════════════════════════════════════════════
// Gas comparison: O0 vs O1 vs O2
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_gas_comparison() {
    println!("\n{}", "=".repeat(70));
    println!("  Stress Test Gas Comparison: O0 vs O1 vs O2");
    println!("{}", "=".repeat(70));

    let tests: Vec<(&str, &str, Vec<u8>)> = vec![
        ("stress_variables.edge", "dot_product()", vec![]),
        ("stress_variables.edge", "polynomial(uint256)", abi_encode_u256(U256::from(2))),
        ("stress_variables.edge", "fibonacci_unrolled()", vec![]),
        ("stress_conditionals.edge", "classify(uint256)", abi_encode_u256(U256::from(500))),
        ("stress_conditionals.edge", "clamp(uint256,uint256,uint256)", {
            let mut a = abi_encode_u256(U256::from(50));
            a.extend_from_slice(&abi_encode_u256(U256::from(10)));
            a.extend_from_slice(&abi_encode_u256(U256::from(100)));
            a
        }),
        ("stress_conditionals.edge", "tier_price(uint256)", abi_encode_u256(U256::from(75))),
        ("stress_loops.edge", "sum_to(uint256)", abi_encode_u256(U256::from(10))),
        ("stress_loops.edge", "factorial(uint256)", abi_encode_u256(U256::from(5))),
        ("stress_loops.edge", "collatz_steps(uint256)", abi_encode_u256(U256::from(27))),
    ];

    println!("{:<40} {:>8} {:>8} {:>8} {:>8}", "Function", "O0", "O1", "O2", "Savings");
    println!("{}", "-".repeat(70));

    for (file, sig, args) in &tests {
        let path = format!("../../examples/{file}");
        let selector = fn_selector(sig);
        let mut results = Vec::new();

        for opt in 0..=2u8 {
            let mut host = EvmTestHost::deploy_edge(&path, opt);
            let r = host.call(selector, args);
            assert!(r.success, "{sig} failed at O{opt}");
            results.push(r.gas_used);
        }

        let savings = if results[0] > results[2] {
            let pct = ((results[0] - results[2]) as f64 / results[0] as f64) * 100.0;
            format!("-{pct:.1}%")
        } else {
            "—".to_string()
        };

        // Truncate sig for display
        let short_sig = if sig.len() > 38 { &sig[..38] } else { sig };
        println!("{:<40} {:>8} {:>8} {:>8} {:>8}",
            short_sig, results[0], results[1], results[2], savings);
    }
    println!("{}", "=".repeat(70));
}
