#![allow(missing_docs)]

//! Execution-level correctness tests for inline assembly.
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

fn for_all_opt_levels(contract_path: &str, test_fn: impl Fn(&mut EvmHandle, u8)) {
    for opt in 0..=3 {
        let bc = compile_contract_opt(contract_path, opt);
        let mut h = EvmHandle::new(bc);
        test_fn(&mut h, opt);
    }
}

const ASM: &str = "examples/tests/test_inline_asm.edge";

#[test]
fn test_asm_add() {
    for_all_opt_levels(ASM, |h, o| {
        let (ok, out) = h.call(selector("asm_add()").to_vec());
        assert!(ok, "asm_add reverted at O{o}");
        assert_eq!(decode_u256(&out), 3, "1+2=3 at O{o}");
    });
}

#[test]
fn test_asm_mul_add() {
    for_all_opt_levels(ASM, |h, o| {
        let (ok, out) = h.call(selector("asm_mul_add()").to_vec());
        assert!(ok, "asm_mul_add reverted at O{o}");
        assert_eq!(decode_u256(&out), 7, "2*3+1=7 at O{o}");
    });
}

#[test]
fn test_asm_identity() {
    for_all_opt_levels(ASM, |h, o| {
        let (ok, out) = h.call(selector("asm_identity()").to_vec());
        assert!(ok, "asm_identity reverted at O{o}");
        assert_eq!(decode_u256(&out), 99, "identity(99)=99 at O{o}");
    });
}

#[test]
fn test_asm_hex_literal() {
    for_all_opt_levels(ASM, |h, o| {
        let (ok, out) = h.call(selector("asm_hex_literal()").to_vec());
        assert!(ok, "asm_hex_literal reverted at O{o}");
        assert_eq!(decode_u256(&out), 255, "0xff=255 at O{o}");
    });
}

#[test]
fn test_asm_caller() {
    for_all_opt_levels(ASM, |h, o| {
        let (ok, out) = h.call(selector("asm_caller()").to_vec());
        assert!(ok, "asm_caller reverted at O{o}");
        // CALLER opcode returns msg.sender — which is Address::ZERO in our test setup
        assert_eq!(decode_u256(&out), 0, "caller should be 0 at O{o}");
    });
}

#[test]
fn test_asm_local_var() {
    for_all_opt_levels(ASM, |h, o| {
        let (ok, out) = h.call(selector("asm_local_var()").to_vec());
        assert!(ok, "asm_local_var reverted at O{o}");
        assert_eq!(decode_u256(&out), 30, "10+20=30 at O{o}");
    });
}

#[test]
fn test_asm_computed_local() {
    for_all_opt_levels(ASM, |h, o| {
        let (ok, out) = h.call(selector("asm_computed_local()").to_vec());
        assert!(ok, "asm_computed_local reverted at O{o}");
        assert_eq!(decode_u256(&out), 50, "(3+7)*5=50 at O{o}");
    });
}
