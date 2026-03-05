//! Debug tests to isolate the O2 size-mode optimizer bug.
//!
//! Separates IR-level and bytecode-level optimization to identify which layer
//! produces incorrect results.

use alloy_primitives::{Address, U256};
use edge_evm_tests::{
    abi_decode_u256, abi_encode_address, abi_encode_u256, compile_edge_split, fn_selector,
    EvmTestHost,
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

fn test_erc20_transfer(bytecode: &[u8], label: &str) -> bool {
    let mut host = EvmTestHost::deploy_bytecode(bytecode);
    let mint = fn_selector("mint(address,uint256)");
    let transfer = fn_selector("transfer(address,uint256)");
    let balance_of = fn_selector("balanceOf(address)");

    let deployer = host.caller();
    let alice = addr(0x0A);

    // Mint 1000 to deployer
    let r = host.call(mint, &args_addr_u256(deployer, U256::from(1000)));
    if !r.success {
        eprintln!("[{label}] mint failed");
        return false;
    }

    // Transfer 300 to alice
    let r = host.call(transfer, &args_addr_u256(alice, U256::from(300)));
    if !r.success {
        eprintln!("[{label}] transfer failed");
        return false;
    }

    // Check deployer balance = 700
    let r = host.call(balance_of, &abi_encode_address(deployer));
    let deployer_bal = abi_decode_u256(&r.output);
    if deployer_bal != U256::from(700) {
        eprintln!("[{label}] deployer balance: expected 700, got {deployer_bal}");
        return false;
    }

    // Check alice balance = 300
    let r = host.call(balance_of, &abi_encode_address(alice));
    let alice_bal = abi_decode_u256(&r.output);
    if alice_bal != U256::from(300) {
        eprintln!("[{label}] alice balance: expected 300, got {alice_bal}");
        return false;
    }

    eprintln!("[{label}] PASS");
    true
}

#[test]
fn isolate_o2_size_bug() {
    let optimize_for = edge_ir::OptimizeFor::Size;

    // Baseline: O0 IR + O0 bytecode (should always work)
    let bc = compile_edge_split(ERC20_PATH, 0, 0, optimize_for);
    assert!(test_erc20_transfer(&bc, "IR=O0 BC=O0"));

    // O2 IR + O0 bytecode → tests IR optimizer alone
    let bc = compile_edge_split(ERC20_PATH, 2, 0, optimize_for);
    let ir_ok = test_erc20_transfer(&bc, "IR=O2 BC=O0");

    // O0 IR + O2 bytecode → tests bytecode optimizer alone
    let bc = compile_edge_split(ERC20_PATH, 0, 2, optimize_for);
    let bc_ok = test_erc20_transfer(&bc, "IR=O0 BC=O2");

    // O2 IR + O2 bytecode → full O2 size mode
    let bc = compile_edge_split(ERC20_PATH, 2, 2, optimize_for);
    let both_ok = test_erc20_transfer(&bc, "IR=O2 BC=O2");

    // Also test O1 IR + O2 bytecode
    let bc = compile_edge_split(ERC20_PATH, 1, 2, optimize_for);
    let o1ir_o2bc = test_erc20_transfer(&bc, "IR=O1 BC=O2");

    // And O2 IR + O1 bytecode
    let bc = compile_edge_split(ERC20_PATH, 2, 1, optimize_for);
    let o2ir_o1bc = test_erc20_transfer(&bc, "IR=O2 BC=O1");

    eprintln!("\n=== Summary ===");
    eprintln!("IR=O0 BC=O0: {}", if true { "PASS" } else { "FAIL" });
    eprintln!("IR=O2 BC=O0: {}", if ir_ok { "PASS" } else { "FAIL" });
    eprintln!("IR=O0 BC=O2: {}", if bc_ok { "PASS" } else { "FAIL" });
    eprintln!("IR=O2 BC=O2: {}", if both_ok { "PASS" } else { "FAIL" });
    eprintln!("IR=O1 BC=O2: {}", if o1ir_o2bc { "PASS" } else { "FAIL" });
    eprintln!("IR=O2 BC=O1: {}", if o2ir_o1bc { "PASS" } else { "FAIL" });

    // At least one of these should fail to identify the bug layer
    assert!(
        ir_ok && bc_ok && both_ok && o1ir_o2bc && o2ir_o1bc,
        "One or more configurations failed — see output above to identify which optimizer layer is broken"
    );
}
