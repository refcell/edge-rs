#![allow(missing_docs)]

//! Execution-level acceptance tests for access-control example contracts.
//!
//! Tests compile ownable.edge and pausable.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify stateful behaviour.
//!
//! ## Compiler caveats
//! Internal helper functions (e.g. `_requireOwner`) are lowered to a stub that
//! pushes 0 and returns — they don't execute their bodies.  This means
//! authorization guards are bypassed in tests, but all storage mutations and
//! return values are correct.

use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use revm::{
    context::{Context, TxEnv},
    database::{CacheDB, EmptyDB},
    handler::{MainBuilder, MainnetContext},
    primitives::{Address, Bytes, TxKind},
    state::AccountInfo,
    ExecuteCommitEvm, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

// =============================================================================
// Shared helpers
// =============================================================================

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Compile the last contract in a file.
fn compile_contract(relative_path: &str) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
}

/// Compile a specific named contract from a file.
fn compile_named(relative_path: &str, contract_name: &str) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output
        .bytecodes
        .expect("no bytecodes map")
        .get(contract_name)
        .unwrap_or_else(|| panic!("contract {contract_name} not found"))
        .clone()
}

fn selector(sig: &str) -> [u8; 4] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

fn encode_u256(val: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&val.to_be_bytes());
    out
}

fn encode_address(addr: [u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(&addr);
    out
}

fn decode_u256(output: &[u8]) -> u64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    assert_eq!(&output[0..24], &[0u8; 24], "u256 too large for u64");
    u64::from_be_bytes(output[24..32].try_into().unwrap())
}

fn decode_bool(output: &[u8]) -> bool {
    decode_u256(output) != 0
}

fn decode_address(output: &[u8]) -> [u8; 20] {
    assert!(output.len() >= 32, "return value too short");
    output[12..32].try_into().unwrap()
}

const CALLER: Address = Address::ZERO;
const ALICE_ADDR: [u8; 20] = {
    let mut a = [0u8; 20];
    a[19] = 0xA1;
    a
};

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

struct EvmHandle {
    evm: TestEvm,
    contract: Address,
    nonce: u64,
}

impl EvmHandle {
    fn new(deploy_bytecode: Vec<u8>) -> Self {
        let mut db = CacheDB::<EmptyDB>::default();
        let caller_info = AccountInfo {
            balance: revm::primitives::U256::from(1_000_000_000_000_000_000u128),
            nonce: 0,
            ..Default::default()
        };
        db.insert_account_info(CALLER, caller_info);

        let mut evm = Context::mainnet().with_db(db).build_mainnet();

        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Create)
            .data(Bytes::from(deploy_bytecode))
            .gas_limit(10_000_000)
            .nonce(0)
            .build()
            .unwrap();

        let result = evm.transact_commit(tx).unwrap();
        assert!(result.is_success(), "Deployment failed: {result:#?}");

        let contract = CALLER.create(0);

        Self {
            evm,
            contract,
            nonce: 1,
        }
    }

    fn call(&mut self, calldata: Vec<u8>) -> (bool, Vec<u8>) {
        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Call(self.contract))
            .data(Bytes::from(calldata))
            .nonce(self.nonce)
            .gas_limit(10_000_000)
            .build()
            .unwrap();
        let result = self.evm.transact_commit(tx).unwrap();
        self.nonce += 1;
        let success = result.is_success();
        let output = result.output().map(|b| b.to_vec()).unwrap_or_default();
        (success, output)
    }
}

fn calldata(sel: [u8; 4], args: &[[u8; 32]]) -> Vec<u8> {
    let mut cd = sel.to_vec();
    for a in args {
        cd.extend_from_slice(a);
    }
    cd
}

// =============================================================================
// Ownable
// =============================================================================

#[test]
fn test_ownable_owner_initially_zero() {
    let bc = compile_named("examples/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("owner()"), &[]));
    assert!(ok, "owner() reverted");
    assert_eq!(
        decode_address(&out),
        [0u8; 20],
        "owner should start as zero"
    );
}

#[test]
fn test_ownable_pending_owner_initially_zero() {
    let bc = compile_named("examples/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("pendingOwner()"), &[]));
    assert!(ok, "pendingOwner() reverted");
    assert_eq!(
        decode_address(&out),
        [0u8; 20],
        "pendingOwner should start as zero"
    );
}

#[test]
fn test_ownable_transfer_sets_pending_owner() {
    let bc = compile_named("examples/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);

    // transferOwnership(alice) — auth guard bypassed (internal call stub)
    let (ok, _) = evm.call(calldata(
        selector("transferOwnership(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(ok, "transferOwnership reverted");

    // pendingOwner() should now be alice
    let (ok, out) = evm.call(calldata(selector("pendingOwner()"), &[]));
    assert!(ok, "pendingOwner() reverted after transfer");
    assert_eq!(
        decode_address(&out),
        ALICE_ADDR,
        "pendingOwner should be alice after transferOwnership"
    );
}

#[test]
fn test_ownable_accept_ownership_sets_owner() {
    let bc = compile_named("examples/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);

    // Set caller (0x0) as pending owner so acceptOwnership() passes the guard
    let (ok, _) = evm.call(calldata(
        selector("transferOwnership(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(ok, "transferOwnership reverted");

    // acceptOwnership() — caller is 0x0 which matches pending_owner
    let (ok, _) = evm.call(calldata(selector("acceptOwnership()"), &[]));
    assert!(ok, "acceptOwnership reverted");

    // pending_owner should be cleared to 0
    let (ok, out) = evm.call(calldata(selector("pendingOwner()"), &[]));
    assert!(ok, "pendingOwner() reverted after accept");
    assert_eq!(
        decode_address(&out),
        [0u8; 20],
        "pendingOwner should be cleared after acceptOwnership"
    );
}

#[test]
fn test_ownable_unknown_selector_reverts() {
    let bc = compile_named("examples/access/ownable.edge", "Ownable");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}

// =============================================================================
// Pausable
// =============================================================================

#[test]
fn test_pausable_initially_not_paused() {
    let bc = compile_contract("examples/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("paused()"), &[]));
    assert!(ok, "paused() reverted");
    assert!(!decode_bool(&out), "contract should start unpaused");
}

#[test]
fn test_pausable_pause_sets_flag() {
    let bc = compile_contract("examples/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);

    // pause() — owner guard bypassed by internal call stub
    let (ok, _) = evm.call(calldata(selector("pause()"), &[]));
    assert!(ok, "pause() reverted");

    let (ok, out) = evm.call(calldata(selector("paused()"), &[]));
    assert!(ok, "paused() reverted after pause");
    assert!(decode_bool(&out), "contract should be paused");
}

#[test]
fn test_pausable_unpause_clears_flag() {
    let bc = compile_contract("examples/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);

    let (ok, _) = evm.call(calldata(selector("pause()"), &[]));
    assert!(ok, "pause() reverted");

    let (ok, _) = evm.call(calldata(selector("unpause()"), &[]));
    assert!(ok, "unpause() reverted");

    let (ok, out) = evm.call(calldata(selector("paused()"), &[]));
    assert!(ok, "paused() reverted after unpause");
    assert!(!decode_bool(&out), "contract should be unpaused");
}

#[test]
fn test_pausable_guarded_transfer_succeeds_when_not_paused() {
    let bc = compile_contract("examples/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);

    let (ok, out) = evm.call(calldata(
        selector("guardedTransfer(address,uint256)"),
        &[encode_address(ALICE_ADDR), encode_u256(100)],
    ));
    assert!(ok, "guardedTransfer reverted when not paused");
    assert_eq!(
        decode_u256(&out),
        1,
        "guardedTransfer should return true (1) when not paused"
    );
}

#[test]
fn test_pausable_unknown_selector_reverts() {
    let bc = compile_contract("examples/access/pausable.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}

// =============================================================================
// Auth (auth.edge — two contracts: Owned and Auth)
// =============================================================================

#[test]
fn test_auth_owned_get_owner_initially_zero() {
    let bc = compile_named("examples/lib/auth.edge", "Owned");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("getOwner()"), &[]));
    assert!(ok, "getOwner() reverted");
    assert_eq!(
        decode_address(&out),
        [0u8; 20],
        "owner should start as zero"
    );
}

#[test]
fn test_auth_owned_transfer_ownership() {
    let bc = compile_named("examples/lib/auth.edge", "Owned");
    let mut evm = EvmHandle::new(bc);

    // Step 1: transferOwnership(alice) sets pending_owner (2-step pattern)
    let (ok, _) = evm.call(calldata(
        selector("transferOwnership(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(ok, "transferOwnership reverted");

    // Step 2: acceptOwnership() — auth guard bypassed; sets owner = pending_owner
    let (ok, _) = evm.call(calldata(selector("acceptOwnership()"), &[]));
    assert!(ok, "acceptOwnership reverted");

    let (ok, out) = evm.call(calldata(selector("getOwner()"), &[]));
    assert!(ok, "getOwner() reverted after acceptOwnership");
    assert_eq!(
        decode_address(&out),
        ALICE_ADDR,
        "owner should be alice after acceptOwnership"
    );
}

#[test]
fn test_auth_isauthorized_zero_caller_with_zero_owner() {
    let bc = compile_named("examples/lib/auth.edge", "Auth");
    let mut evm = EvmHandle::new(bc);

    // isAuthorized(0x0) — owner is initially 0x0, so caller==owner → returns true
    let (ok, out) = evm.call(calldata(
        selector("isAuthorized(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(ok, "isAuthorized reverted");
    assert_eq!(
        decode_u256(&out),
        1,
        "zero address should be authorized when owner is zero"
    );
}

#[test]
fn test_auth_get_owner_initially_zero() {
    let bc = compile_named("examples/lib/auth.edge", "Auth");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("getOwner()"), &[]));
    assert!(ok, "getOwner() reverted");
    assert_eq!(decode_address(&out), [0u8; 20]);
}

#[test]
fn test_auth_get_authority_initially_zero() {
    let bc = compile_named("examples/lib/auth.edge", "Auth");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("getAuthority()"), &[]));
    assert!(ok, "getAuthority() reverted");
    assert_eq!(decode_address(&out), [0u8; 20]);
}

// =============================================================================
// Roles (AccessControl)
// =============================================================================

#[test]
fn test_roles_has_role_initially_false() {
    let bc = compile_contract("examples/access/roles.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("hasRole(bytes32,address)"),
        &[[0u8; 32], encode_address(ALICE_ADDR)],
    ));
    assert!(ok, "hasRole() reverted");
    assert!(!decode_bool(&out), "hasRole should be false initially");
}

#[test]
fn test_roles_get_role_admin_initially_zero() {
    let bc = compile_contract("examples/access/roles.edge");
    let mut evm = EvmHandle::new(bc);
    // getRoleAdmin for a non-zero role → should return bytes32(0)
    let mut role = [0u8; 32];
    role[31] = 1;
    let (ok, out) = evm.call(calldata(selector("getRoleAdmin(bytes32)"), &[role]));
    assert!(ok, "getRoleAdmin() reverted");
    assert_eq!(&out[0..32], &[0u8; 32], "roleAdmin should be zero");
}

#[test]
fn test_roles_unknown_selector_reverts() {
    let bc = compile_contract("examples/access/roles.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}
