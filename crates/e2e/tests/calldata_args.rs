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

fn selector(sig: &str) -> [u8; 4] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

fn encode_u256(v: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = v.to_be_bytes();
    out[24..32].copy_from_slice(&bytes);
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compile_contract(code: &str) -> Vec<u8> {
    let tmp_path = "/tmp/edge_test.edge";
    std::fs::write(tmp_path, code).expect("failed to write test file");
    let path = PathBuf::from(tmp_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
}

const CONTRACT_ADDR: Address = Address::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x00,
]);

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<revm::handler::MainnetContext<TestDb>>;

struct EvmHandle {
    evm: TestEvm,
    nonce: u64,
}

impl EvmHandle {
    fn new(bytecode: Vec<u8>) -> Self {
        let code = Bytecode::new_legacy(Bytes::from(bytecode));
        let account = AccountInfo::default().with_code(code);
        let mut db = CacheDB::<EmptyDB>::default();
        db.insert_account_info(CONTRACT_ADDR, account);
        let evm = Context::mainnet().with_db(db).build_mainnet();
        Self { evm, nonce: 0 }
    }

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

#[test]
fn test_calldata_single_arg() {
    // Create test contract
    std::fs::write("/tmp/test_double.edge", "abi IDouble { fn double(x: u256) -> (u256); }\ncontract Double {\n    pub fn double(x: u256) -> (u256) {\n        return x + x;\n    }\n}\n").unwrap();

    let path = std::path::PathBuf::from("/tmp/test_double.edge");
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    let bytecode = output.bytecode.expect("no bytecode produced");

    let mut evm = EvmHandle::new(bytecode);

    // Call double(5) and expect 10
    let mut calldata = selector("double(uint256)").to_vec();
    calldata.extend_from_slice(&encode_u256(5));

    let (ok, out) = evm.call(calldata);
    assert!(ok, "double(5) reverted");
    assert_eq!(decode_u256(&out), 10, "double(5) should be 10");
}

#[test]
fn test_calldata_two_args() {
    std::fs::write("/tmp/test_add.edge", "abi IAdd { fn add(a: u256, b: u256) -> (u256); }\ncontract Add {\n    pub fn add(a: u256, b: u256) -> (u256) {\n        return a + b;\n    }\n}\n").unwrap();

    let path = std::path::PathBuf::from("/tmp/test_add.edge");
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    let bytecode = output.bytecode.expect("no bytecode produced");

    let mut evm = EvmHandle::new(bytecode);

    // Call add(3, 5) and expect 8
    let mut calldata = selector("add(uint256,uint256)").to_vec();
    calldata.extend_from_slice(&encode_u256(3));
    calldata.extend_from_slice(&encode_u256(5));

    let (ok, out) = evm.call(calldata);
    assert!(ok, "add(3, 5) reverted");
    assert_eq!(decode_u256(&out), 8, "add(3, 5) should be 8");
}

#[test]
fn test_calldata_three_args() {
    std::fs::write("/tmp/test_sum.edge", "abi ISum { fn sum(a: u256, b: u256, c: u256) -> (u256); }\ncontract Sum {\n    pub fn sum(a: u256, b: u256, c: u256) -> (u256) {\n        return a + b + c;\n    }\n}\n").unwrap();

    let path = std::path::PathBuf::from("/tmp/test_sum.edge");
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    let bytecode = output.bytecode.expect("no bytecode produced");

    let mut evm = EvmHandle::new(bytecode);

    // Call sum(1, 2, 3) and expect 6
    let mut calldata = selector("sum(uint256,uint256,uint256)").to_vec();
    calldata.extend_from_slice(&encode_u256(1));
    calldata.extend_from_slice(&encode_u256(2));
    calldata.extend_from_slice(&encode_u256(3));

    let (ok, out) = evm.call(calldata);
    assert!(ok, "sum(1, 2, 3) reverted");
    assert_eq!(decode_u256(&out), 6, "sum(1, 2, 3) should be 6");
}
