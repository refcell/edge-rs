#![allow(missing_docs)]

//! Tests that internal (non-pub) functions get correct return type annotations
//! in the IR, regardless of their declared return type.

use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use revm::{
    context::{Context, TxEnv},
    database::{CacheDB, EmptyDB},
    handler::{MainBuilder, MainnetContext},
    primitives::{Address, Bytes, TxKind, U256},
    state::AccountInfo,
    ExecuteCommitEvm, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

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

fn decode_u256(output: &[u8]) -> u64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    assert_eq!(&output[0..24], &[0u8; 24], "u256 too large for u64");
    u64::from_be_bytes(output[24..32].try_into().unwrap())
}

fn calldata(sig: &str, args: &[[u8; 32]]) -> Vec<u8> {
    let mut cd = selector(sig).to_vec();
    for arg in args {
        cd.extend_from_slice(arg);
    }
    cd
}

const CALLER: Address = Address::ZERO;

struct EvmHandle {
    evm: MainnetEvm<MainnetContext<CacheDB<EmptyDB>>>,
    contract: Address,
    nonce: u64,
}

impl EvmHandle {
    fn new(deploy_bytecode: Vec<u8>) -> Self {
        let mut db = CacheDB::<EmptyDB>::default();
        db.insert_account_info(
            CALLER,
            AccountInfo {
                balance: U256::from(u64::MAX),
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

    fn call(&mut self, cd: Vec<u8>) -> (bool, Vec<u8>) {
        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Call(self.contract))
            .data(Bytes::from(cd))
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

// =============================================================================
// Test 1: Internal function returning bool
// =============================================================================

#[test]
fn internal_fn_returning_bool() {
    let bc = compile_contract("examples/tests/internal_bool.edge");
    let mut evm = EvmHandle::new(bc);

    let (ok, out) = evm.call(calldata("is_positive(uint256)", &[encode_u256(42)]));
    assert!(ok, "call failed");
    assert_eq!(decode_u256(&out), 1, "is_positive(42) should be true (1)");

    let (ok, out) = evm.call(calldata("is_positive(uint256)", &[encode_u256(0)]));
    assert!(ok, "call failed");
    assert_eq!(decode_u256(&out), 0, "is_positive(0) should be false (0)");
}

// =============================================================================
// Test 2: Internal function returning u256 (regression)
// =============================================================================

#[test]
fn internal_fn_returning_u256() {
    let bc = compile_contract("examples/tests/internal_math.edge");
    let mut evm = EvmHandle::new(bc);

    let (ok, out) = evm.call(calldata("get_double(uint256)", &[encode_u256(21)]));
    assert!(ok, "call failed");
    assert_eq!(decode_u256(&out), 42, "get_double(21) should be 42");
}

// =============================================================================
// Test 3: Storage set/get (regression, no internal fn)
// =============================================================================

#[test]
fn internal_fn_storage_regression() {
    let bc = compile_contract("examples/tests/internal_void.edge");
    let mut evm = EvmHandle::new(bc);

    let (ok, _) = evm.call(calldata("set_value(uint256)", &[encode_u256(99)]));
    assert!(ok, "set_value call failed");

    let (ok, out) = evm.call(calldata("get_value()", &[]));
    assert!(ok, "get_value call failed");
    assert_eq!(decode_u256(&out), 99, "get_value() should be 99");
}
