#![allow(missing_docs)]

//! Execution-level acceptance tests for finance contracts.
//!
//! Tests compile staking.edge and multisig.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify stateful behaviour.

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

fn compile_contract(relative_path: &str) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
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

const CALLER: Address = Address::ZERO;

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
        db.insert_account_info(
            CALLER,
            AccountInfo {
                balance: revm::primitives::U256::from(1_000_000_000_000_000_000u128),
                nonce: 0,
                ..Default::default()
            },
        );

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
// Staking
// =============================================================================

#[test]
fn test_staking_total_staked_initially_zero() {
    let bc = compile_contract("examples/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("totalStaked()"), &[]));
    assert!(ok, "totalStaked() reverted");
    assert_eq!(decode_u256(&out), 0, "totalStaked should start at 0");
}

#[test]
fn test_staking_staked_balance_initially_zero() {
    let bc = compile_contract("examples/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);
    let addr = [0u8; 20];
    let (ok, out) = evm.call(calldata(
        selector("stakedBalance(address)"),
        &[encode_address(addr)],
    ));
    assert!(ok, "stakedBalance() reverted");
    assert_eq!(decode_u256(&out), 0, "stakedBalance should start at 0");
}

#[test]
fn test_staking_stake_increases_total() {
    let bc = compile_contract("examples/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);

    let (ok, _) = evm.call(calldata(selector("stake(uint256)"), &[encode_u256(100)]));
    assert!(ok, "stake(100) reverted");

    let (ok, out) = evm.call(calldata(selector("totalStaked()"), &[]));
    assert!(ok, "totalStaked() reverted after stake");
    assert_eq!(decode_u256(&out), 100, "totalStaked should be 100");
}

#[test]
fn test_staking_stake_increases_balance() {
    let bc = compile_contract("examples/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);

    let (ok, _) = evm.call(calldata(selector("stake(uint256)"), &[encode_u256(100)]));
    assert!(ok, "stake(100) reverted");

    // CALLER is Address::ZERO
    let (ok, out) = evm.call(calldata(
        selector("stakedBalance(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(ok, "stakedBalance(caller) reverted");
    assert_eq!(decode_u256(&out), 100, "stakedBalance should be 100");
}

#[test]
fn test_staking_withdraw_decreases() {
    let bc = compile_contract("examples/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);

    // Stake 100
    let (ok, _) = evm.call(calldata(selector("stake(uint256)"), &[encode_u256(100)]));
    assert!(ok, "stake(100) reverted");

    // Withdraw 50
    let (ok, _) = evm.call(calldata(selector("withdraw(uint256)"), &[encode_u256(50)]));
    assert!(ok, "withdraw(50) reverted");

    let (ok, out) = evm.call(calldata(selector("totalStaked()"), &[]));
    assert!(ok, "totalStaked() reverted");
    assert_eq!(
        decode_u256(&out),
        50,
        "totalStaked should be 50 after withdraw"
    );

    let (ok, out) = evm.call(calldata(
        selector("stakedBalance(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(ok, "stakedBalance(caller) reverted");
    assert_eq!(decode_u256(&out), 50, "stakedBalance should be 50");
}

#[test]
fn test_staking_unknown_selector_reverts() {
    let bc = compile_contract("examples/finance/staking.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}

// =============================================================================
// Multisig
// =============================================================================

#[test]
fn test_multisig_threshold_initially_zero() {
    let bc = compile_contract("examples/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("getThreshold()"), &[]));
    assert!(ok, "getThreshold() reverted");
    assert_eq!(decode_u256(&out), 0, "threshold should start at 0");
}

#[test]
fn test_multisig_confirmations_initially_zero() {
    let bc = compile_contract("examples/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("getConfirmations(uint256)"),
        &[encode_u256(0)],
    ));
    assert!(ok, "getConfirmations(0) reverted");
    assert_eq!(decode_u256(&out), 0, "confirmations should start at 0");
}

#[test]
fn test_multisig_propose_reverts_for_non_owner() {
    let bc = compile_contract("examples/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);

    // propose() calls _requireOwner which checks is_owner[caller].
    // CALLER is not an owner, so this should revert.
    let target = [0u8; 20];
    let (ok, _) = evm.call(calldata(
        selector("propose(address,uint256,bytes32)"),
        &[encode_address(target), encode_u256(0), [0u8; 32]],
    ));
    assert!(!ok, "propose should revert for non-owner");
}

#[test]
fn test_multisig_unknown_selector_reverts() {
    let bc = compile_contract("examples/finance/multisig.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}
