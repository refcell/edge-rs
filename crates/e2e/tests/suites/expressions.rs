#![allow(missing_docs)]

use edge_driver::compiler::Compiler;
use revm::{
    context::TxEnv,
    database::{CacheDB, EmptyDB},
    handler::{MainBuilder, MainnetContext},
    primitives::{Address, Bytes, TxKind},
    state::AccountInfo,
    ExecuteCommitEvm, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

// =============================================================================
// Helpers: Compile
// =============================================================================

fn compile_contract_source(source: &str) -> Vec<u8> {
    let mut compiler = Compiler::from_source(source);
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

/// Build calldata: 4-byte selector followed by encoded arguments (32 bytes each).
fn encode_call_with_args(sel: [u8; 4], args: &[[u8; 32]]) -> Vec<u8> {
    let mut calldata = sel.to_vec();
    for arg in args {
        calldata.extend_from_slice(arg);
    }
    calldata
}

/// Decode a 32-byte ABI-encoded uint256 return value into a u64.
fn decode_u256(output: &[u8]) -> u64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    assert_eq!(&output[0..24], &[0u8; 24], "u256 too large for u64");
    u64::from_be_bytes(output[24..32].try_into().unwrap())
}

// =============================================================================
// EVM test harness
// =============================================================================

const CALLER: Address = Address::new([0x01; 20]);

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

        let mut evm = revm::context::Context::mainnet()
            .with_db(db)
            .build_mainnet();

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

// =============================================================================
// Inline contract sources for testing
// =============================================================================

const EXPRESSIONS_CONTRACT: &str = r#"
abi IExpr {
    fn arithmetic(a: u256, b: u256) -> (u256);
    fn comparisons(x: u256, y: u256) -> (u256);
    fn bitwise(x: u256, y: u256) -> (u256);
    fn complex(a: u256, b: u256, c: u256) -> (u256);
}

contract Expressions {
    pub fn arithmetic(a: u256, b: u256) -> (u256) {
        return a + b;
    }

    pub fn comparisons(x: u256, y: u256) -> (u256) {
        let result: u256;
        if (x == y) {
            result = 1;
        } else {
            result = 0;
        }
        return result;
    }

    pub fn bitwise(x: u256, y: u256) -> (u256) {
        return x & y;
    }

    pub fn complex(a: u256, b: u256, c: u256) -> (u256) {
        return a + b * c;
    }
}
"#;

// =============================================================================
// Tests
// =============================================================================

#[test]
fn test_expressions_compiles() {
    let bytecode = compile_contract_source(EXPRESSIONS_CONTRACT);
    assert!(
        !bytecode.is_empty(),
        "expressions contract produced empty bytecode"
    );
    assert!(bytecode.len() > 10, "bytecode too short to be valid");
}

#[test]
fn test_arithmetic_add() {
    let bytecode = compile_contract_source(EXPRESSIONS_CONTRACT);
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
    let bytecode = compile_contract_source(EXPRESSIONS_CONTRACT);
    let mut evm = EvmHandle::new(bytecode);

    let sel = selector("comparisons(uint256,uint256)");
    let x = encode_u256(5);
    let y = encode_u256(5);
    let calldata = encode_call_with_args(sel, &[x, y]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "comparisons(5, 5) reverted");
    assert_eq!(
        decode_u256(&out),
        1,
        "comparisons(5, 5) should return true (1)"
    );
}

#[test]
fn test_comparisons_neq() {
    let bytecode = compile_contract_source(EXPRESSIONS_CONTRACT);
    let mut evm = EvmHandle::new(bytecode);

    let sel = selector("comparisons(uint256,uint256)");
    let x = encode_u256(5);
    let y = encode_u256(6);
    let calldata = encode_call_with_args(sel, &[x, y]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "comparisons(5, 6) reverted");
    assert_eq!(
        decode_u256(&out),
        0,
        "comparisons(5, 6) should return false (0)"
    );
}

#[test]
fn test_bitwise() {
    let bytecode = compile_contract_source(EXPRESSIONS_CONTRACT);
    let mut evm = EvmHandle::new(bytecode);

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
    let bytecode = compile_contract_source(EXPRESSIONS_CONTRACT);
    let mut evm = EvmHandle::new(bytecode);

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
