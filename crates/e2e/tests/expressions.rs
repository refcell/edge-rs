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

/// Encode a u256 as a 32-byte big-endian word.
fn encode_u256(val: u64) -> [u8; 32] {
    let mut encoded = [0u8; 32];
    encoded[24..32].copy_from_slice(&val.to_be_bytes());
    encoded
}

/// Encode an address as a 32-byte left-padded word.
fn encode_address(addr: [u8; 20]) -> [u8; 32] {
    let mut encoded = [0u8; 32];
    encoded[12..32].copy_from_slice(&addr);
    encoded
}

/// Build calldata: 4-byte selector followed by encoded arguments (32 bytes each).
fn encode_call_with_args(sel: [u8; 4], args: &[[u8; 32]]) -> Vec<u8> {
    let mut calldata = sel.to_vec();
    for arg in args {
        calldata.extend_from_slice(arg);
    }
    calldata
}

/// Decode a 32-byte ABI-encoded uint256 return value into a u64.
///
/// Panics if `output` is shorter than 32 bytes or if the value exceeds `u64::MAX`.
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
/// calls are stateful.
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
fn test_expressions_compiles() {
    let bytecode = compile_contract("examples/expressions.edge");
    assert!(
        !bytecode.is_empty(),
        "expressions.edge produced empty bytecode"
    );
    assert!(bytecode.len() > 10, "bytecode too short to be valid");
}

#[test]
fn test_arithmetic_add() {
    let bytecode = compile_contract("examples/expressions.edge");
    let mut evm = EvmHandle::new(bytecode);

    let sel = selector("arithmetic(uint256,uint256)");
    let a = encode_u256(10);
    let b = encode_u256(3);
    let calldata = encode_call_with_args(sel, &[a, b]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "arithmetic(10, 3) reverted");
    assert_eq!(decode_u256(&out), 13, "10 + 3 should equal 13");
}

#[test]
fn test_comparisons_eq() {
    let bytecode = compile_contract("examples/expressions.edge");
    let mut evm = EvmHandle::new(bytecode);

    let sel = selector("comparisons(uint256,uint256)");
    let x = encode_u256(5);
    let y = encode_u256(5);
    let calldata = encode_call_with_args(sel, &[x, y]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "comparisons(5, 5) reverted");
    // In Edge/Solidity, boolean true is represented as 1
    assert_eq!(
        decode_u256(&out),
        1,
        "comparisons(5, 5) should return true (1)"
    );
}

#[test]
fn test_comparisons_neq() {
    let bytecode = compile_contract("examples/expressions.edge");
    let mut evm = EvmHandle::new(bytecode);

    let sel = selector("comparisons(uint256,uint256)");
    let x = encode_u256(5);
    let y = encode_u256(6);
    let calldata = encode_call_with_args(sel, &[x, y]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "comparisons(5, 6) reverted");
    // In Edge/Solidity, boolean false is represented as 0
    assert_eq!(
        decode_u256(&out),
        0,
        "comparisons(5, 6) should return false (0)"
    );
}

#[test]
fn test_bitwise() {
    let bytecode = compile_contract("examples/expressions.edge");
    let mut evm = EvmHandle::new(bytecode);

    // bitwise(0b1100, 0b1010) should return 0b1000 (AND result)
    let sel = selector("bitwise(uint256,uint256)");
    let x = encode_u256(0b1100);
    let y = encode_u256(0b1010);
    let calldata = encode_call_with_args(sel, &[x, y]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "bitwise(0b1100, 0b1010) reverted");
    assert_eq!(
        decode_u256(&out),
        0b1000,
        "0b1100 & 0b1010 should equal 0b1000"
    );
}

#[test]
fn test_complex() {
    let bytecode = compile_contract("examples/expressions.edge");
    let mut evm = EvmHandle::new(bytecode);

    // complex(2, 3, 4) should return 2 + 3*4 = 14
    let sel = selector("complex(uint256,uint256,uint256)");
    let a = encode_u256(2);
    let b = encode_u256(3);
    let c = encode_u256(4);
    let calldata = encode_call_with_args(sel, &[a, b, c]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "complex(2, 3, 4) reverted");
    assert_eq!(
        decode_u256(&out),
        14,
        "2 + 3 * 4 should equal 14 (order of operations)"
    );
}
