#![allow(missing_docs)]

//! Execution-level tests for generics, impl blocks, and trait impls.
//!
//! Every test runs at O0, O1, O2, and O3 to catch optimizer bugs.

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
// Generic function tests (examples/test_generics.edge)
// =============================================================================

const GENERICS: &str = "examples/test_generics.edge";

#[test]
fn test_generic_identity() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_identity()"), &[]));
        assert!(ok, "test_identity() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_identity() wrong at O{o}");
    });
}

#[test]
fn test_generic_max() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_max()"), &[]));
        assert!(ok, "test_max() reverted at O{o}");
        assert_eq!(decode_u256(&out), 20, "test_max() wrong at O{o}");
    });
}

#[test]
fn test_generic_min() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_min()"), &[]));
        assert!(ok, "test_min() reverted at O{o}");
        assert_eq!(decode_u256(&out), 10, "test_min() wrong at O{o}");
    });
}

#[test]
fn test_generic_entry_value() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_entry_value()"), &[]));
        assert!(ok, "test_entry_value() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_entry_value() wrong at O{o}");
    });
}

#[test]
fn test_generic_entry_key() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_entry_key()"), &[]));
        assert!(ok, "test_entry_key() reverted at O{o}");
        assert_eq!(decode_u256(&out), 100, "test_entry_key() wrong at O{o}");
    });
}

#[test]
fn test_generic_result_ok() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_result_ok()"), &[]));
        assert!(ok, "test_result_ok() reverted at O{o}");
        assert_eq!(decode_u256(&out), 77, "test_result_ok() wrong at O{o}");
    });
}

#[test]
fn test_generic_result_err() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_result_err()"), &[]));
        assert!(ok, "test_result_err() reverted at O{o}");
        assert_eq!(decode_u256(&out), 99, "test_result_err() wrong at O{o}");
    });
}

#[test]
fn test_generic_option_some() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_option_some()"), &[]));
        assert!(ok, "test_option_some() reverted at O{o}");
        assert_eq!(decode_u256(&out), 55, "test_option_some() wrong at O{o}");
    });
}

#[test]
fn test_generic_option_none() {
    for_all_opt_levels(GENERICS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_option_none()"), &[]));
        assert!(ok, "test_option_none() reverted at O{o}");
        assert_eq!(decode_u256(&out), 0, "test_option_none() wrong at O{o}");
    });
}

// =============================================================================
// Impl block tests (examples/test_impl.edge)
// =============================================================================

const IMPL: &str = "examples/test_impl.edge";

#[test]
fn test_impl_point_sum() {
    for_all_opt_levels(IMPL, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_point_sum()"), &[]));
        assert!(ok, "test_point_sum() reverted at O{o}");
        assert_eq!(decode_u256(&out), 30, "test_point_sum() wrong at O{o}");
    });
}

#[test]
fn test_impl_point_scale() {
    for_all_opt_levels(IMPL, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_point_scale()"), &[]));
        assert!(ok, "test_point_scale() reverted at O{o}");
        // (5 * 3) + (7 * 3) = 15 + 21 = 36
        assert_eq!(decode_u256(&out), 36, "test_point_scale() wrong at O{o}");
    });
}

#[test]
fn test_impl_point_x() {
    for_all_opt_levels(IMPL, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_point_x()"), &[]));
        assert!(ok, "test_point_x() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_point_x() wrong at O{o}");
    });
}

#[test]
fn test_impl_counter_get() {
    for_all_opt_levels(IMPL, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_counter_get()"), &[]));
        assert!(ok, "test_counter_get() reverted at O{o}");
        assert_eq!(decode_u256(&out), 100, "test_counter_get() wrong at O{o}");
    });
}

#[test]
fn test_impl_counter_add() {
    for_all_opt_levels(IMPL, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_counter_add()"), &[]));
        assert!(ok, "test_counter_add() reverted at O{o}");
        assert_eq!(decode_u256(&out), 150, "test_counter_add() wrong at O{o}");
    });
}

// =============================================================================
// Trait impl tests (examples/test_traits.edge)
// =============================================================================

const TRAITS: &str = "examples/test_traits.edge";

#[test]
fn test_trait_double() {
    for_all_opt_levels(TRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_double()"), &[]));
        assert!(ok, "test_double() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_double() wrong at O{o}");
    });
}

#[test]
fn test_trait_triple() {
    for_all_opt_levels(TRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_triple()"), &[]));
        assert!(ok, "test_triple() reverted at O{o}");
        assert_eq!(decode_u256(&out), 30, "test_triple() wrong at O{o}");
    });
}

#[test]
fn test_trait_double_then_triple() {
    for_all_opt_levels(TRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_double_then_triple()"), &[]));
        assert!(ok, "test_double_then_triple() reverted at O{o}");
        // Doubler::double(5) = 10, then * 3 = 30
        assert_eq!(
            decode_u256(&out),
            30,
            "test_double_then_triple() wrong at O{o}"
        );
    });
}

#[test]
fn test_operator_overload_add() {
    for_all_opt_levels(TRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_add_overload()"), &[]));
        assert!(ok, "test_add_overload() reverted at O{o}");
        // Wrapper{10} + Wrapper{32} = Wrapper{42} → .val = 42
        assert_eq!(decode_u256(&out), 42, "test_add_overload() wrong at O{o}");
    });
}

#[test]
fn test_operator_overload_eq() {
    for_all_opt_levels(TRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_eq_overload()"), &[]));
        assert!(ok, "test_eq_overload() reverted at O{o}");
        // Wrapper{42} == Wrapper{42} → true → returns 1
        assert_eq!(decode_u256(&out), 1, "test_eq_overload() wrong at O{o}");
    });
}
