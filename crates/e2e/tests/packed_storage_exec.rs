#![allow(missing_docs)]

//! Execution-level tests for packed structs in contract storage.
//!
//! Verifies that packed struct fields in storage:
//! - Occupy a single storage slot
//! - Sub-field reads work correctly (SLOAD + SHR + AND)
//! - Sub-field writes work correctly (read-modify-write)
//! - Whole-struct writes pack fields into the storage slot

use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use revm::{
    context::{Context, TxEnv},
    database::{CacheDB, EmptyDB},
    handler::MainnetContext,
    primitives::{Address, Bytes, TxKind, U256},
    state::AccountInfo,
    ExecuteCommitEvm, MainBuilder, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

// =============================================================================
// Shared helpers
// =============================================================================

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compile_contract_opt(relative_path: &str, opt_level: u8) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    config.optimization_level = opt_level;
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

type TestEvm = MainnetEvm<MainnetContext<CacheDB<EmptyDB>>>;

// =============================================================================
// EVM handle
// =============================================================================

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

fn for_all_opt_levels(contract_path: &str, test_fn: impl Fn(&mut EvmHandle, u8)) {
    for opt in 0..=3 {
        let bc = compile_contract_opt(contract_path, opt);
        let mut h = EvmHandle::new(bc);
        test_fn(&mut h, opt);
    }
}

// =============================================================================
// Packed storage tests
// =============================================================================

const PACKED_STORAGE: &str = "examples/tests/test_packed_storage.edge";

// ----- Whole-struct write + sub-field reads -----

#[test]
fn test_packed_storage_read_r() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("store_and_read_r()"), &[]));
        assert!(ok, "store_and_read_r() reverted at O{o}");
        assert_eq!(decode_u256(&out), 10, "store_and_read_r() wrong at O{o}");
    });
}

#[test]
fn test_packed_storage_read_g() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("store_and_read_g()"), &[]));
        assert!(ok, "store_and_read_g() reverted at O{o}");
        assert_eq!(decode_u256(&out), 20, "store_and_read_g() wrong at O{o}");
    });
}

#[test]
fn test_packed_storage_read_b() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("store_and_read_b()"), &[]));
        assert!(ok, "store_and_read_b() reverted at O{o}");
        assert_eq!(decode_u256(&out), 30, "store_and_read_b() wrong at O{o}");
    });
}

#[test]
fn test_packed_storage_read_sum() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("store_and_read_sum()"), &[]));
        assert!(ok, "store_and_read_sum() reverted at O{o}");
        assert_eq!(decode_u256(&out), 60, "store_and_read_sum() wrong at O{o}");
    });
}

#[test]
fn test_packed_storage_pair_a() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("store_pair_read_a()"), &[]));
        assert!(ok, "store_pair_read_a() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "store_pair_read_a() wrong at O{o}");
    });
}

#[test]
fn test_packed_storage_pair_b() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("store_pair_read_b()"), &[]));
        assert!(ok, "store_pair_read_b() reverted at O{o}");
        assert_eq!(decode_u256(&out), 99, "store_pair_read_b() wrong at O{o}");
    });
}

// ----- Sub-field writes -----

#[test]
fn test_packed_storage_write_subfield() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("write_subfield_r()"), &[]));
        assert!(ok, "write_subfield_r() reverted at O{o}");
        assert_eq!(decode_u256(&out), 99, "write_subfield_r() wrong at O{o}");
    });
}

#[test]
fn test_packed_storage_write_preserves_other_fields() {
    for_all_opt_levels(PACKED_STORAGE, |h, o| {
        let (ok, out) = h.call(calldata(selector("write_subfield_preserves()"), &[]));
        assert!(ok, "write_subfield_preserves() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            50, // g=20 + b=30
            "write_subfield_preserves() wrong at O{o}"
        );
    });
}
