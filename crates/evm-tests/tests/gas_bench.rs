//! Gas benchmarks for Edge contracts vs estimated Solidity equivalents.
//!
//! All tests are `#[ignore]` so they don't run in CI.
//! Run with:
//!   cargo test -p edge-evm-tests gas_bench -- --ignored --nocapture

use alloy_primitives::{Address, U256};
use edge_evm_tests::{
    abi_decode_u256, abi_encode_address, abi_encode_u256, fn_selector, EvmTestHost,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const COUNTER_PATH: &str = "../../examples/counter.edge";
const ERC20_PATH: &str = "../../examples/test_erc20.edge";

fn sel(sig: &str) -> [u8; 4] {
    fn_selector(sig)
}

fn addr(n: u8) -> Address {
    Address::from([n; 20])
}

fn args_addr_u256(a: Address, v: U256) -> Vec<u8> {
    let mut buf = abi_encode_address(a);
    buf.extend_from_slice(&abi_encode_u256(v));
    buf
}

fn args_addr_addr(a: Address, b: Address) -> Vec<u8> {
    let mut buf = abi_encode_address(a);
    buf.extend_from_slice(&abi_encode_address(b));
    buf
}

fn args_addr_addr_u256(a: Address, b: Address, v: U256) -> Vec<u8> {
    let mut buf = abi_encode_address(a);
    buf.extend_from_slice(&abi_encode_address(b));
    buf.extend_from_slice(&abi_encode_u256(v));
    buf
}

// ---------------------------------------------------------------------------
// Counter gas benchmarks
// ---------------------------------------------------------------------------

fn bench_counter(opt_level: u8) {
    let deploy = EvmTestHost::deploy_edge_measured(COUNTER_PATH, opt_level);
    let mut host = deploy.host;
    let runtime_size = host.runtime_code_size();

    println!();
    println!("=== Counter O{opt_level} ===");
    println!("  Init code size:    {} bytes", deploy.init_code_size);
    println!("  Runtime code size: {} bytes", runtime_size);
    println!("  Deploy gas:        {}", deploy.deploy_gas);
    println!();

    // get() — initial (cold SLOAD)
    let r = host.call(sel("get()"), &[]);
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::ZERO);
    println!("  get() [initial]:        {} gas", r.gas_used);

    // get() — warm
    let r = host.call(sel("get()"), &[]);
    assert!(r.success);
    println!("  get() [warm]:           {} gas", r.gas_used);

    // increment() — first (cold SLOAD + cold SSTORE 0→1)
    let r = host.call(sel("increment()"), &[]);
    assert!(r.success);
    println!("  increment() [first]:    {} gas", r.gas_used);

    // increment() — second (warm SLOAD + warm SSTORE 1→2)
    let r = host.call(sel("increment()"), &[]);
    assert!(r.success);
    println!("  increment() [second]:   {} gas", r.gas_used);

    // increment() — third
    let r = host.call(sel("increment()"), &[]);
    assert!(r.success);
    println!("  increment() [third]:    {} gas", r.gas_used);

    // decrement() — warm
    let r = host.call(sel("decrement()"), &[]);
    assert!(r.success);
    println!("  decrement() [warm]:     {} gas", r.gas_used);

    // reset() — warm (SSTORE nonzero→0, gets refund)
    let r = host.call(sel("reset()"), &[]);
    assert!(r.success);
    println!("  reset() [warm]:         {} gas", r.gas_used);

    // reset() — from zero (SSTORE 0→0)
    let r = host.call(sel("reset()"), &[]);
    assert!(r.success);
    println!("  reset() [noop]:         {} gas", r.gas_used);
}

#[test]
#[ignore]
fn gas_bench_counter_o0() {
    bench_counter(0);
}

#[test]
#[ignore]
fn gas_bench_counter_o1() {
    bench_counter(1);
}

#[test]
#[ignore]
fn gas_bench_counter_o2() {
    bench_counter(2);
}

// ---------------------------------------------------------------------------
// ERC20 gas benchmarks
// ---------------------------------------------------------------------------

fn bench_erc20(opt_level: u8) {
    let deploy = EvmTestHost::deploy_edge_measured(ERC20_PATH, opt_level);
    let mut host = deploy.host;
    let runtime_size = host.runtime_code_size();
    let deployer = host.caller();
    let alice = addr(0x0A);
    let bob = addr(0x0B);
    let spender = addr(0x0C);

    println!();
    println!("=== ERC20 O{opt_level} ===");
    println!("  Init code size:    {} bytes", deploy.init_code_size);
    println!("  Runtime code size: {} bytes", runtime_size);
    println!("  Deploy gas:        {}", deploy.deploy_gas);
    println!();

    // -- totalSupply (cold then warm) --
    let r = host.call(sel("totalSupply()"), &[]);
    assert!(r.success);
    println!("  totalSupply() [cold]:       {} gas", r.gas_used);

    let r = host.call(sel("totalSupply()"), &[]);
    assert!(r.success);
    println!("  totalSupply() [warm]:       {} gas", r.gas_used);

    // -- mint: first mint (cold storage writes) --
    let r = host.call(
        sel("mint(address,uint256)"),
        &args_addr_u256(alice, U256::from(10_000)),
    );
    assert!(r.success);
    println!("  mint() [first]:             {} gas", r.gas_used);

    // -- mint: second mint (warm storage) --
    let r = host.call(
        sel("mint(address,uint256)"),
        &args_addr_u256(alice, U256::from(5_000)),
    );
    assert!(r.success);
    println!("  mint() [second/warm]:       {} gas", r.gas_used);

    // -- balanceOf (warm) --
    let r = host.call(
        sel("balanceOf(address)"),
        &abi_encode_address(alice),
    );
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(15_000));
    println!("  balanceOf() [warm]:         {} gas", r.gas_used);

    // -- balanceOf (cold account) --
    let r = host.call(
        sel("balanceOf(address)"),
        &abi_encode_address(bob),
    );
    assert!(r.success);
    println!("  balanceOf() [cold acct]:    {} gas", r.gas_used);

    // -- transfer: first (cold recipient storage) --
    // Caller is deployer, so mint to deployer first
    host.call(
        sel("mint(address,uint256)"),
        &args_addr_u256(deployer, U256::from(50_000)),
    );

    let r = host.call(
        sel("transfer(address,uint256)"),
        &args_addr_u256(bob, U256::from(1_000)),
    );
    assert!(r.success);
    println!("  transfer() [cold recip]:    {} gas", r.gas_used);

    // -- transfer: warm --
    let r = host.call(
        sel("transfer(address,uint256)"),
        &args_addr_u256(bob, U256::from(1_000)),
    );
    assert!(r.success);
    println!("  transfer() [warm]:          {} gas", r.gas_used);

    // -- approve --
    let r = host.call(
        sel("approve(address,uint256)"),
        &args_addr_u256(spender, U256::from(5_000)),
    );
    assert!(r.success);
    println!("  approve() [first]:          {} gas", r.gas_used);

    let r = host.call(
        sel("approve(address,uint256)"),
        &args_addr_u256(spender, U256::from(3_000)),
    );
    assert!(r.success);
    println!("  approve() [update]:         {} gas", r.gas_used);

    // -- allowance --
    let r = host.call(
        sel("allowance(address,address)"),
        &args_addr_addr(deployer, spender),
    );
    assert!(r.success);
    assert_eq!(abi_decode_u256(&r.output), U256::from(3_000));
    println!("  allowance() [warm]:         {} gas", r.gas_used);

    // -- transferFrom --
    // deployer approved spender for 3000, mint to deployer was already done
    host.set_caller(spender);
    let r = host.call(
        sel("transferFrom(address,address,uint256)"),
        &args_addr_addr_u256(deployer, alice, U256::from(500)),
    );
    assert!(r.success);
    println!("  transferFrom() [warm]:      {} gas", r.gas_used);
}

#[test]
#[ignore]
fn gas_bench_erc20_o0() {
    bench_erc20(0);
}

#[test]
#[ignore]
fn gas_bench_erc20_o1() {
    bench_erc20(1);
}

#[test]
#[ignore]
fn gas_bench_erc20_o2() {
    bench_erc20(2);
}

// ---------------------------------------------------------------------------
// Summary comparison table
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn gas_bench_summary() {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                    Edge Gas Benchmark Summary                           ║");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!();

    // Counter across all opt levels
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ COUNTER                                                                 │");
    println!("├─────────────────────┬──────────┬──────────┬──────────┬──────────────────┤");
    println!("│ Metric              │  Edge O0 │  Edge O1 │  Edge O2 │ Solidity (est.)  │");
    println!("├─────────────────────┼──────────┼──────────┼──────────┼──────────────────┤");

    let mut counter_data: Vec<(&str, [u64; 3])> = Vec::new();
    let mut counter_sizes: [(usize, usize, u64); 3] = [(0, 0, 0); 3];

    for (i, opt) in [0u8, 1, 2].iter().enumerate() {
        let deploy = EvmTestHost::deploy_edge_measured(COUNTER_PATH, *opt);
        let mut h = deploy.host;
        counter_sizes[i] = (deploy.init_code_size, h.runtime_code_size(), deploy.deploy_gas);

        // Warm up storage with an increment, then reset
        h.call(sel("increment()"), &[]);
        h.call(sel("reset()"), &[]);

        // Now measure "warm" calls (storage slot already accessed this tx context)
        let get_gas = h.call(sel("get()"), &[]).gas_used;
        let inc_gas = h.call(sel("increment()"), &[]).gas_used;
        let dec_gas = h.call(sel("decrement()"), &[]).gas_used;
        let reset_gas = h.call(sel("reset()"), &[]).gas_used;

        if i == 0 {
            counter_data.push(("get()", [0; 3]));
            counter_data.push(("increment()", [0; 3]));
            counter_data.push(("decrement()", [0; 3]));
            counter_data.push(("reset()", [0; 3]));
        }
        counter_data[0].1[i] = get_gas;
        counter_data[1].1[i] = inc_gas;
        counter_data[2].1[i] = dec_gas;
        counter_data[3].1[i] = reset_gas;
    }

    // Estimated Solidity gas costs (warm storage, typical solc 0.8.x output):
    //   get():       ~2,200 (SLOAD=100 warm + overhead)
    //   increment(): ~5,200 (SLOAD + SSTORE warm nonzero→nonzero)
    //   decrement(): ~5,200 (same pattern)
    //   reset():     ~5,200 (SSTORE warm + possible refund)
    let sol_counter = [2_200u64, 5_200, 5_200, 5_200];

    for (j, (name, gas)) in counter_data.iter().enumerate() {
        println!(
            "│ {:19} │ {:>8} │ {:>8} │ {:>8} │ {:>8} (est.)  │",
            name, gas[0], gas[1], gas[2], sol_counter[j]
        );
    }

    println!("├─────────────────────┼──────────┼──────────┼──────────┼──────────────────┤");
    println!(
        "│ Deploy gas          │ {:>8} │ {:>8} │ {:>8} │ {:>8} (est.)  │",
        counter_sizes[0].2, counter_sizes[1].2, counter_sizes[2].2, 46_000u64
    );
    println!(
        "│ Runtime size (B)    │ {:>8} │ {:>8} │ {:>8} │ {:>8} (est.)  │",
        counter_sizes[0].1, counter_sizes[1].1, counter_sizes[2].1, 200
    );
    println!("└─────────────────────┴──────────┴──────────┴──────────┴──────────────────┘");

    println!();

    // ERC20 across all opt levels
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ ERC20                                                                   │");
    println!("├─────────────────────┬──────────┬──────────┬──────────┬──────────────────┤");
    println!("│ Metric              │  Edge O0 │  Edge O1 │  Edge O2 │ Solidity (est.)  │");
    println!("├─────────────────────┼──────────┼──────────┼──────────┼──────────────────┤");

    let mut erc20_data: Vec<(&str, [u64; 3])> = Vec::new();
    let mut erc20_sizes: [(usize, usize, u64); 3] = [(0, 0, 0); 3];

    // Solidity ERC20 estimates (warm, typical OZ-style):
    //   totalSupply():  ~2,400
    //   balanceOf():    ~2,600 (mapping SLOAD)
    //   transfer():     ~7,500 (2x SLOAD + 2x SSTORE + LOG3)
    //   approve():      ~5,800 (SSTORE + LOG3)
    //   allowance():    ~2,800 (nested mapping SLOAD)
    //   transferFrom(): ~10,000 (3x SLOAD + 3x SSTORE + LOG3)
    //   mint():         ~8,500 (2x SLOAD + 2x SSTORE + LOG3)
    let sol_erc20 = [2_400u64, 2_600, 7_500, 5_800, 2_800, 10_000, 8_500];
    let erc20_labels = [
        "totalSupply()",
        "balanceOf()",
        "transfer()",
        "approve()",
        "allowance()",
        "transferFrom()",
        "mint()",
    ];

    for (i, opt) in [0u8, 1, 2].iter().enumerate() {
        let deploy = EvmTestHost::deploy_edge_measured(ERC20_PATH, *opt);
        let mut h = deploy.host;
        erc20_sizes[i] = (deploy.init_code_size, h.runtime_code_size(), deploy.deploy_gas);

        let deployer = h.caller();
        let alice = addr(0x0A);
        let bob = addr(0x0B);
        let spender = addr(0x0C);

        // Seed state: mint to deployer and alice
        h.call(
            sel("mint(address,uint256)"),
            &args_addr_u256(deployer, U256::from(100_000)),
        );
        h.call(
            sel("mint(address,uint256)"),
            &args_addr_u256(alice, U256::from(50_000)),
        );
        // Warm up bob's balance slot
        h.call(
            sel("transfer(address,uint256)"),
            &args_addr_u256(bob, U256::from(1)),
        );
        // Set up allowance
        h.call(
            sel("approve(address,uint256)"),
            &args_addr_u256(spender, U256::from(50_000)),
        );

        // Now measure warm calls
        let total_supply_gas = h.call(sel("totalSupply()"), &[]).gas_used;
        let balance_of_gas = h
            .call(sel("balanceOf(address)"), &abi_encode_address(alice))
            .gas_used;
        let transfer_gas = h
            .call(
                sel("transfer(address,uint256)"),
                &args_addr_u256(bob, U256::from(100)),
            )
            .gas_used;
        let approve_gas = h
            .call(
                sel("approve(address,uint256)"),
                &args_addr_u256(spender, U256::from(10_000)),
            )
            .gas_used;
        let allowance_gas = h
            .call(
                sel("allowance(address,address)"),
                &args_addr_addr(deployer, spender),
            )
            .gas_used;
        h.set_caller(spender);
        let transfer_from_gas = h
            .call(
                sel("transferFrom(address,address,uint256)"),
                &args_addr_addr_u256(deployer, alice, U256::from(100)),
            )
            .gas_used;
        h.set_caller(deployer);
        let mint_gas = h
            .call(
                sel("mint(address,uint256)"),
                &args_addr_u256(alice, U256::from(100)),
            )
            .gas_used;

        let gas_vals = [
            total_supply_gas,
            balance_of_gas,
            transfer_gas,
            approve_gas,
            allowance_gas,
            transfer_from_gas,
            mint_gas,
        ];

        if i == 0 {
            for label in &erc20_labels {
                erc20_data.push((label, [0; 3]));
            }
        }
        for (j, g) in gas_vals.iter().enumerate() {
            erc20_data[j].1[i] = *g;
        }
    }

    for (j, (name, gas)) in erc20_data.iter().enumerate() {
        println!(
            "│ {:19} │ {:>8} │ {:>8} │ {:>8} │ {:>8} (est.)  │",
            name, gas[0], gas[1], gas[2], sol_erc20[j]
        );
    }

    println!("├─────────────────────┼──────────┼──────────┼──────────┼──────────────────┤");
    println!(
        "│ Deploy gas          │ {:>8} │ {:>8} │ {:>8} │ {:>8} (est.)  │",
        erc20_sizes[0].2, erc20_sizes[1].2, erc20_sizes[2].2, 460_000u64
    );
    println!(
        "│ Runtime size (B)    │ {:>8} │ {:>8} │ {:>8} │ {:>8} (est.)  │",
        erc20_sizes[0].1, erc20_sizes[1].1, erc20_sizes[2].1, 1_200
    );
    println!("└─────────────────────┴──────────┴──────────┴──────────┴──────────────────┘");

    println!();
    println!("Note: Solidity estimates are for warm storage, solc 0.8.x with optimizer.");
    println!("      Cold SLOAD adds ~2000 gas, cold SSTORE 0→nonzero adds ~20000 gas.");
    println!("      Edge checked arithmetic adds ~6 opcodes per add/sub (overflow check).");
    println!();
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
}
