#![allow(missing_docs)]

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
    state::{AccountInfo, Bytecode},
    ExecuteCommitEvm, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

// =============================================================================
// Helpers: Compile
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

// =============================================================================
// Helpers: ABI (selectors and encoding/decoding)
// =============================================================================

fn selector(sig: &str) -> [u8; 4] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

fn encode_call(sel: [u8; 4], _args: &[[u8; 32]]) -> Vec<u8> {
    sel.to_vec()
}

/// Decode a 32-byte ABI-encoded uint256 return value into a u64.
///
/// Panics if `output` is shorter than 32 bytes or if the value exceeds u64::MAX.
fn decode_u256(output: &[u8]) -> u64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    // Value is big-endian; the high 24 bytes must be zero for a u64 to fit.
    assert_eq!(&output[0..24], &[0u8; 24], "u256 too large for u64");
    u64::from_be_bytes(output[24..32].try_into().unwrap())
}

// =============================================================================
// EVM test harness
// =============================================================================

/// Fixed address where the contract is deployed for tests.
///
/// Must be > 0x09 to avoid Ethereum precompile addresses (0x01–0x09).
const CONTRACT_ADDR: Address = Address::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x00,
]);

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

/// In-memory EVM with a single contract deployed at [`CONTRACT_ADDR`].
///
/// Storage changes are committed after each [`call`][EvmHandle::call], so
/// calls are stateful: `increment()` followed by `get()` returns 1.
struct EvmHandle {
    evm: TestEvm,
    /// Tracks caller nonce; `transact_commit` increments it in the DB each call.
    nonce: u64,
}

impl EvmHandle {
    /// Deploy `bytecode` at [`CONTRACT_ADDR`] and return a ready handle.
    fn new(bytecode: Vec<u8>) -> Self {
        let code = Bytecode::new_legacy(Bytes::from(bytecode));
        let account = AccountInfo::default().with_code(code);
        let mut db = CacheDB::<EmptyDB>::default();
        db.insert_account_info(CONTRACT_ADDR, account);
        let evm = Context::mainnet().with_db(db).build_mainnet();
        Self { evm, nonce: 0 }
    }

    /// Call the contract with `calldata`. Returns `(success, return_data)`.
    fn call(&mut self, calldata: Vec<u8>) -> (bool, Vec<u8>) {
        let tx = TxEnv::builder()
            .caller(Address::ZERO)
            .kind(TxKind::Call(CONTRACT_ADDR))
            .data(Bytes::from(calldata))
            .nonce(self.nonce)
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
// Tests
// =============================================================================

#[test]
fn test_counter_selectors() {
    assert_eq!(selector("increment()"), [0xd0, 0x9d, 0xe0, 0x8a]);
    assert_eq!(selector("decrement()"), [0x2b, 0xae, 0xce, 0xb7]);
    assert_eq!(selector("get()"), [0x6d, 0x4c, 0xe6, 0x3c]);
    assert_eq!(selector("reset()"), [0xd8, 0x26, 0xf8, 0x8f]);
}

#[test]
fn test_counter_compiles() {
    let bytecode = compile_contract("examples/counter.edge");
    assert!(!bytecode.is_empty(), "counter.edge produced empty bytecode");
    assert!(bytecode.len() > 10, "bytecode too short to be valid");
}

#[test]
fn test_counter_encode_call() {
    let sel = selector("get()");
    let calldata = encode_call(sel, &[]);
    assert_eq!(calldata.len(), 4, "calldata should be 4 bytes for get()");
    assert_eq!(calldata, vec![0x6d, 0x4c, 0xe6, 0x3c]);
}

/// Minimal sanity check: a contract that just does MSTORE + RETURN works in revm.
#[test]
fn test_revm_return_works() {
    // Bytecode: PUSH1 42, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    // Returns a 32-byte big-endian uint with value 42 in the last byte.
    let bytecode_bytes = vec![
        0x60, 0x2a, // PUSH1 42
        0x60, 0x00, // PUSH1 0
        0x52, // MSTORE (mem[0] = 42)
        0x60, 0x20, // PUSH1 32
        0x60, 0x00, // PUSH1 0
        0xf3, // RETURN(0, 32)
    ];
    // Use a safe non-precompile address (precompiles are 0x01–0x09)
    let addr = Address::new([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x01,
    ]);
    let code = Bytecode::new_legacy(Bytes::from(bytecode_bytes));
    let account = AccountInfo::default().with_code(code);
    let mut db = CacheDB::<EmptyDB>::default();
    db.insert_account_info(addr, account);
    let mut evm = Context::mainnet().with_db(db).build_mainnet();

    let tx = TxEnv::builder()
        .caller(Address::ZERO)
        .kind(TxKind::Call(addr))
        .data(Bytes::new())
        .build()
        .unwrap();
    let result = evm.transact_commit(tx).unwrap();
    assert!(result.is_success(), "minimal RETURN should succeed");
    let out = result.output().unwrap();
    assert_eq!(out.len(), 32, "should return 32 bytes");
    assert_eq!(out[31], 42, "last byte should be 42");
}

/// A freshly deployed counter starts at zero.
#[test]
fn test_counter_get_initial_zero() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    let (ok, out) = evm.call(encode_call(selector("get()"), &[]));
    assert!(ok, "get() reverted on fresh contract");
    assert_eq!(decode_u256(&out), 0, "initial count should be 0");
}

/// Calling `increment()` once should make `get()` return 1.
#[test]
fn test_counter_increment_and_get() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    let (ok, _) = evm.call(encode_call(selector("increment()"), &[]));
    assert!(ok, "increment() reverted");

    let (ok, out) = evm.call(encode_call(selector("get()"), &[]));
    assert!(ok, "get() reverted after increment");
    assert_eq!(
        decode_u256(&out),
        1,
        "count should be 1 after one increment"
    );
}

/// Full stateful sequence: increment twice, check 2, reset, check 0.
#[test]
fn test_counter_stateful_sequence() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    // Increment twice
    let (ok, _) = evm.call(encode_call(selector("increment()"), &[]));
    assert!(ok, "first increment() reverted");
    let (ok, _) = evm.call(encode_call(selector("increment()"), &[]));
    assert!(ok, "second increment() reverted");

    let (ok, out) = evm.call(encode_call(selector("get()"), &[]));
    assert!(ok, "get() reverted after 2 increments");
    assert_eq!(
        decode_u256(&out),
        2,
        "count should be 2 after two increments"
    );

    // Reset to zero
    let (ok, _) = evm.call(encode_call(selector("reset()"), &[]));
    assert!(ok, "reset() reverted");

    let (ok, out) = evm.call(encode_call(selector("get()"), &[]));
    assert!(ok, "get() reverted after reset");
    assert_eq!(decode_u256(&out), 0, "count should be 0 after reset");
}

/// An unknown selector should hit the revert fallback in the dispatcher.
#[test]
fn test_counter_unknown_selector_reverts() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}
