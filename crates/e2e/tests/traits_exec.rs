#![allow(missing_docs)]

//! Execution-level tests for trait improvements:
//! - Trait bound validation (positive — bounds satisfied)
//! - Default trait methods
//! - Supertrait enforcement (positive — supertraits satisfied)
//! - UnsafeAdd/UnsafeSub/UnsafeMul

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

fn decode_u256_full(output: &[u8]) -> U256 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    U256::from_be_slice(&output[0..32])
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
// Trait bound validation — positive tests
// =============================================================================

const TRAIT_BOUNDS: &str = "examples/tests/test_trait_bounds.edge";

#[test]
fn test_trait_bound_satisfied() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_bound_satisfied()"), &[]));
        assert!(ok, "test_bound_satisfied() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            42,
            "test_bound_satisfied() wrong at O{o}"
        );
    });
}

#[test]
fn test_multiple_bounds() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_multiple_bounds()"), &[]));
        assert!(ok, "test_multiple_bounds() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            99,
            "test_multiple_bounds() wrong at O{o}"
        );
    });
}

#[test]
fn test_type_bound() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_type_bound()"), &[]));
        assert!(ok, "test_type_bound() reverted at O{o}");
        assert_eq!(decode_u256(&out), 77, "test_type_bound() wrong at O{o}");
    });
}

// Monomorphization with different types

#[test]
fn test_bound_other() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_bound_other()"), &[]));
        assert!(ok, "test_bound_other() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_bound_other() wrong at O{o}");
    });
}

#[test]
fn test_extract_wrapper() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_extract_wrapper()"), &[]));
        assert!(ok, "test_extract_wrapper() reverted at O{o}");
        // Wrapper.get_value() = val = 25
        assert_eq!(
            decode_u256(&out),
            25,
            "test_extract_wrapper() wrong at O{o}"
        );
    });
}

#[test]
fn test_extract_other() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_extract_other()"), &[]));
        assert!(ok, "test_extract_other() reverted at O{o}");
        // Other.get_value() = val * 10 = 3 * 10 = 30
        assert_eq!(decode_u256(&out), 30, "test_extract_other() wrong at O{o}");
    });
}

#[test]
fn test_extract_wrapper_method() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_extract_wrapper_method()"), &[]));
        assert!(ok, "test_extract_wrapper_method() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            25,
            "test_extract_wrapper_method() wrong at O{o}"
        );
    });
}

#[test]
fn test_extract_other_method() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_extract_other_method()"), &[]));
        assert!(ok, "test_extract_other_method() reverted at O{o}");
        // Other.get_value() = val * 10 = 3 * 10 = 30
        assert_eq!(
            decode_u256(&out),
            30,
            "test_extract_other_method() wrong at O{o}"
        );
    });
}

#[test]
fn test_scale_wrapper() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_scale_wrapper()"), &[]));
        assert!(ok, "test_scale_wrapper() reverted at O{o}");
        // Wrapper.scale(7, 6) = 7 * 6 = 42
        assert_eq!(decode_u256(&out), 42, "test_scale_wrapper() wrong at O{o}");
    });
}

#[test]
fn test_scale_other() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_scale_other()"), &[]));
        assert!(ok, "test_scale_other() reverted at O{o}");
        // Other.scale(5, 3) = 5 * 3 * 2 = 30
        assert_eq!(decode_u256(&out), 30, "test_scale_other() wrong at O{o}");
    });
}

#[test]
fn test_multiple_bounds_other() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_multiple_bounds_other()"), &[]));
        assert!(ok, "test_multiple_bounds_other() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            99,
            "test_multiple_bounds_other() wrong at O{o}"
        );
    });
}

#[test]
fn test_type_bound_other() {
    for_all_opt_levels(TRAIT_BOUNDS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_type_bound_other()"), &[]));
        assert!(ok, "test_type_bound_other() reverted at O{o}");
        // Container<Other> { item: Other { val: 8 } }.item.val = 8
        assert_eq!(
            decode_u256(&out),
            8,
            "test_type_bound_other() wrong at O{o}"
        );
    });
}

// =============================================================================
// Default trait methods
// =============================================================================

const DEFAULT_METHODS: &str = "examples/tests/test_default_methods.edge";

#[test]
fn test_default_method() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_default_method()"), &[]));
        assert!(ok, "test_default_method() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_default_method() wrong at O{o}");
    });
}

#[test]
fn test_override_method() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_override_method()"), &[]));
        assert!(ok, "test_override_method() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            50,
            "test_override_method() wrong at O{o}"
        );
    });
}

#[test]
fn test_required_method() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_required_method()"), &[]));
        assert!(ok, "test_required_method() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            99,
            "test_required_method() wrong at O{o}"
        );
    });
}

#[test]
fn test_chained_default() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_chained_default()"), &[]));
        assert!(ok, "test_chained_default() reverted at O{o}");
        // Counter.quadrupled() = value * 2 * 2 = 10 * 4 = 40
        assert_eq!(
            decode_u256(&out),
            40,
            "test_chained_default() wrong at O{o}"
        );
    });
}

#[test]
fn test_partial_override_chain() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_partial_override_chain()"), &[]));
        assert!(ok, "test_partial_override_chain() reverted at O{o}");
        // Special.quadrupled() = doubled(self) * 2 = (val*10)*2 = 3*10*2 = 60
        assert_eq!(
            decode_u256(&out),
            60,
            "test_partial_override_chain() wrong at O{o}"
        );
    });
}

#[test]
fn test_full_override() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_full_override()"), &[]));
        assert!(ok, "test_full_override() reverted at O{o}");
        // FullOverride.quadrupled() = val + 2 = 102
        assert_eq!(decode_u256(&out), 102, "test_full_override() wrong at O{o}");
    });
}

// Method-call syntax for default methods

#[test]
fn test_default_method_call() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_default_method_call()"), &[]));
        assert!(ok, "test_default_method_call() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            42,
            "test_default_method_call() wrong at O{o}"
        );
    });
}

#[test]
fn test_override_method_call() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_override_method_call()"), &[]));
        assert!(ok, "test_override_method_call() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            50,
            "test_override_method_call() wrong at O{o}"
        );
    });
}

#[test]
fn test_required_method_call() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_required_method_call()"), &[]));
        assert!(ok, "test_required_method_call() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            99,
            "test_required_method_call() wrong at O{o}"
        );
    });
}

#[test]
fn test_chained_default_call() {
    for_all_opt_levels(DEFAULT_METHODS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_chained_default_call()"), &[]));
        assert!(ok, "test_chained_default_call() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            40,
            "test_chained_default_call() wrong at O{o}"
        );
    });
}

// =============================================================================
// Supertrait enforcement — positive tests
// =============================================================================

const SUPERTRAITS: &str = "examples/tests/test_supertraits.edge";

#[test]
fn test_supertrait_base() {
    for_all_opt_levels(SUPERTRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_base_method()"), &[]));
        assert!(ok, "test_base_method() reverted at O{o}");
        assert_eq!(decode_u256(&out), 25, "test_base_method() wrong at O{o}");
    });
}

#[test]
fn test_supertrait_extended() {
    for_all_opt_levels(SUPERTRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_extended_method()"), &[]));
        assert!(ok, "test_extended_method() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            50,
            "test_extended_method() wrong at O{o}"
        );
    });
}

// Method-call syntax for supertraits

#[test]
fn test_supertrait_base_call() {
    for_all_opt_levels(SUPERTRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_base_method_call()"), &[]));
        assert!(ok, "test_base_method_call() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            25,
            "test_base_method_call() wrong at O{o}"
        );
    });
}

#[test]
fn test_supertrait_extended_call() {
    for_all_opt_levels(SUPERTRAITS, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_extended_method_call()"), &[]));
        assert!(ok, "test_extended_method_call() reverted at O{o}");
        assert_eq!(
            decode_u256(&out),
            50,
            "test_extended_method_call() wrong at O{o}"
        );
    });
}

// =============================================================================
// UnsafeAdd / UnsafeSub / UnsafeMul
// =============================================================================

const UNSAFE_ARITH: &str = "examples/tests/test_unsafe_arith.edge";

#[test]
fn test_unsafe_add() {
    for_all_opt_levels(UNSAFE_ARITH, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_unsafe_add()"), &[]));
        assert!(ok, "test_unsafe_add() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_unsafe_add() wrong at O{o}");
    });
}

#[test]
fn test_unsafe_sub() {
    for_all_opt_levels(UNSAFE_ARITH, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_unsafe_sub()"), &[]));
        assert!(ok, "test_unsafe_sub() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_unsafe_sub() wrong at O{o}");
    });
}

#[test]
fn test_unsafe_mul() {
    for_all_opt_levels(UNSAFE_ARITH, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_unsafe_mul()"), &[]));
        assert!(ok, "test_unsafe_mul() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "test_unsafe_mul() wrong at O{o}");
    });
}

#[test]
fn test_add_overflow_wraps() {
    for_all_opt_levels(UNSAFE_ARITH, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_add_overflow()"), &[]));
        assert!(ok, "test_add_overflow() should NOT revert at O{o}");
        assert_eq!(decode_u256(&out), 0, "MAX+1 should wrap to 0 at O{o}");
    });
}

#[test]
fn test_sub_underflow_wraps() {
    for_all_opt_levels(UNSAFE_ARITH, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_sub_underflow()"), &[]));
        assert!(ok, "test_sub_underflow() should NOT revert at O{o}");
        let val = decode_u256_full(&out);
        assert_eq!(val, U256::MAX, "0-1 should wrap to MAX at O{o}");
    });
}

#[test]
fn test_mul_overflow_wraps() {
    for_all_opt_levels(UNSAFE_ARITH, |h, o| {
        let (ok, out) = h.call(calldata(selector("test_mul_overflow()"), &[]));
        assert!(ok, "test_mul_overflow() should NOT revert at O{o}");
        let val = decode_u256_full(&out);
        // MAX * 2 = MAX << 1 = MAX - 1 (wrapping)
        let expected = U256::MAX - U256::from(1);
        assert_eq!(val, expected, "MAX*2 should wrap at O{o}");
    });
}
