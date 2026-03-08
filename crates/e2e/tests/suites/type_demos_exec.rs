#![allow(missing_docs)]

//! Execution-level acceptance tests for type-demonstration contracts.
//!
//! Tests compile comptime.edge to bytecode, deploy on an in-memory revm EVM,
//! and verify basic store/load round-trip behaviour.

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
// ComptimeExample (comptime.edge)
// =============================================================================

#[test]
fn test_comptime_load_initially_zero() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("load()"), &[]));
    assert!(ok, "load() reverted");
    assert_eq!(
        decode_u256(&out),
        0,
        "load() should return 0 before any store"
    );
}

#[test]
fn test_comptime_store_and_load_roundtrip() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);

    let (ok, _) = evm.call(calldata(selector("store(uint256)"), &[encode_u256(42)]));
    assert!(ok, "store(42) reverted");

    let (ok, out) = evm.call(calldata(selector("load()"), &[]));
    assert!(ok, "load() reverted after store");
    assert_eq!(
        decode_u256(&out),
        42,
        "load() should return 42 after store(42)"
    );
}

#[test]
fn test_comptime_store_overwrites() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);

    let (ok, _) = evm.call(calldata(selector("store(uint256)"), &[encode_u256(100)]));
    assert!(ok, "store(100) reverted");

    let (ok, _) = evm.call(calldata(selector("store(uint256)"), &[encode_u256(999)]));
    assert!(ok, "store(999) reverted");

    let (ok, out) = evm.call(calldata(selector("load()"), &[]));
    assert!(ok, "load() reverted");
    assert_eq!(
        decode_u256(&out),
        999,
        "load() should return most-recently stored value"
    );
}

#[test]
fn test_comptime_unknown_selector_reverts() {
    let bc = compile_named("examples/types/comptime.edge", "ComptimeExample");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}
