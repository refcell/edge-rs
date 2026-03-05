#![allow(missing_docs)]

//! Compile-level acceptance tests for all solmate-style example contracts.
//!
//! Each test verifies that the contract compiles to non-empty bytecode without
//! errors. EVM execution tests are left to per-contract test files once the
//! relevant IR features (mappings, calldata args, etc.) stabilise.

use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compile_contract(relative_path: &str) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
}

// =============================================================================
// access/
// =============================================================================

#[test]
fn test_ownable_compiles() {
    let bc = compile_contract("examples/access/ownable.edge");
    assert!(!bc.is_empty(), "ownable.edge produced empty bytecode");
}

#[test]
fn test_pausable_compiles() {
    let bc = compile_contract("examples/access/pausable.edge");
    assert!(!bc.is_empty(), "pausable.edge produced empty bytecode");
}

#[test]
fn test_roles_compiles() {
    let bc = compile_contract("examples/access/roles.edge");
    assert!(!bc.is_empty(), "roles.edge produced empty bytecode");
}

// =============================================================================
// finance/
// =============================================================================

#[test]
#[ignore = "requires tuple instantiation in egglog IR"]
fn test_amm_compiles() {
    let bc = compile_contract("examples/finance/amm.edge");
    assert!(!bc.is_empty(), "amm.edge produced empty bytecode");
}

#[test]
fn test_multisig_compiles() {
    let bc = compile_contract("examples/finance/multisig.edge");
    assert!(!bc.is_empty(), "multisig.edge produced empty bytecode");
}

#[test]
#[ignore = "requires top-level const scoping in egglog IR"]
fn test_staking_compiles() {
    let bc = compile_contract("examples/finance/staking.edge");
    assert!(!bc.is_empty(), "staking.edge produced empty bytecode");
}

// =============================================================================
// lib/
// =============================================================================

#[test]
fn test_auth_compiles() {
    let bc = compile_contract("examples/lib/auth.edge");
    assert!(!bc.is_empty(), "auth.edge produced empty bytecode");
}

#[test]
#[ignore = "requires top-level const scoping in egglog IR"]
fn test_math_compiles() {
    let bc = compile_contract("examples/lib/math.edge");
    assert!(!bc.is_empty(), "math.edge produced empty bytecode");
}

#[test]
fn test_safe_transfer_compiles() {
    let bc = compile_contract("examples/lib/safe_transfer.edge");
    assert!(!bc.is_empty(), "safe_transfer.edge produced empty bytecode");
}

// =============================================================================
// patterns/
// =============================================================================

#[test]
fn test_factory_compiles() {
    let bc = compile_contract("examples/patterns/factory.edge");
    assert!(!bc.is_empty(), "factory.edge produced empty bytecode");
}

#[test]
fn test_reentrancy_guard_compiles() {
    let bc = compile_contract("examples/patterns/reentrancy_guard.edge");
    assert!(
        !bc.is_empty(),
        "reentrancy_guard.edge produced empty bytecode"
    );
}

#[test]
fn test_timelock_compiles() {
    let bc = compile_contract("examples/patterns/timelock.edge");
    assert!(!bc.is_empty(), "timelock.edge produced empty bytecode");
}

// =============================================================================
// tokens/
// =============================================================================

#[test]
#[ignore = "requires top-level const scoping in egglog IR"]
fn test_tokens_erc20_compiles() {
    let bc = compile_contract("examples/tokens/erc20.edge");
    assert!(!bc.is_empty(), "tokens/erc20.edge produced empty bytecode");
}

#[test]
#[ignore = "requires external call support in egglog IR"]
fn test_tokens_erc4626_compiles() {
    let bc = compile_contract("examples/tokens/erc4626.edge");
    assert!(
        !bc.is_empty(),
        "tokens/erc4626.edge produced empty bytecode"
    );
}

#[test]
fn test_tokens_erc721_compiles() {
    let bc = compile_contract("examples/tokens/erc721.edge");
    assert!(!bc.is_empty(), "tokens/erc721.edge produced empty bytecode");
}

#[test]
fn test_weth_compiles() {
    let bc = compile_contract("examples/tokens/weth.edge");
    assert!(!bc.is_empty(), "weth.edge produced empty bytecode");
}

// =============================================================================
// types/
// =============================================================================

#[test]
#[ignore = "module-only file, no contract to compile"]
fn test_types_compiles() {
    let bc = compile_contract("examples/types.edge");
    assert!(!bc.is_empty(), "types.edge produced empty bytecode");
}

#[test]
#[ignore = "requires array instantiation in egglog IR"]
fn test_types_arrays_compiles() {
    let bc = compile_contract("examples/types/arrays.edge");
    assert!(!bc.is_empty(), "types/arrays.edge produced empty bytecode");
}

#[test]
fn test_types_comptime_compiles() {
    let bc = compile_contract("examples/types/comptime.edge");
    assert!(
        !bc.is_empty(),
        "types/comptime.edge produced empty bytecode"
    );
}

#[test]
#[ignore = "requires match statement in egglog IR"]
fn test_types_enums_compiles() {
    let bc = compile_contract("examples/types/enums.edge");
    assert!(!bc.is_empty(), "types/enums.edge produced empty bytecode");
}

#[test]
#[ignore = "requires tuple instantiation in egglog IR"]
fn test_types_generics_compiles() {
    let bc = compile_contract("examples/types/generics.edge");
    assert!(
        !bc.is_empty(),
        "types/generics.edge produced empty bytecode"
    );
}

#[test]
#[ignore = "requires struct instantiation in egglog IR"]
fn test_types_structs_compiles() {
    let bc = compile_contract("examples/types/structs.edge");
    assert!(!bc.is_empty(), "types/structs.edge produced empty bytecode");
}

// =============================================================================
// utils/
// =============================================================================

#[test]
#[ignore = "module-only file, no contract to compile"]
fn test_bits_compiles() {
    let bc = compile_contract("examples/utils/bits.edge");
    assert!(!bc.is_empty(), "bits.edge produced empty bytecode");
}

#[test]
#[ignore = "requires top-level const scoping in egglog IR"]
fn test_bytes_compiles() {
    let bc = compile_contract("examples/utils/bytes.edge");
    assert!(!bc.is_empty(), "bytes.edge produced empty bytecode");
}

#[test]
#[ignore = "requires for-loop variable scoping in egglog IR"]
fn test_merkle_compiles() {
    let bc = compile_contract("examples/utils/merkle.edge");
    assert!(!bc.is_empty(), "merkle.edge produced empty bytecode");
}
