//! Shared test helpers for e2e execution tests.

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

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn compile_contract_opt(relative_path: &str, opt_level: u8) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    config.optimization_level = opt_level;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
}

pub fn selector(sig: &str) -> [u8; 4] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

pub fn decode_u256(output: &[u8]) -> u64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    assert_eq!(&output[0..24], &[0u8; 24], "u256 too large for u64");
    u64::from_be_bytes(output[24..32].try_into().unwrap())
}

pub fn encode_u256(val: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&val.to_be_bytes());
    out
}

pub const fn encode_b32(val: [u8; 32]) -> [u8; 32] {
    val
}

pub fn calldata(sel: [u8; 4], args: &[[u8; 32]]) -> Vec<u8> {
    let mut cd = sel.to_vec();
    for a in args {
        cd.extend_from_slice(a);
    }
    cd
}

pub const CALLER: Address = Address::ZERO;

pub type TestDb = CacheDB<EmptyDB>;
pub type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

pub struct EvmHandle {
    pub evm: TestEvm,
    pub contract: Address,
    pub nonce: u64,
}

impl EvmHandle {
    pub fn new(deploy_bytecode: Vec<u8>) -> Self {
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

    pub fn call(&mut self, calldata: Vec<u8>) -> (bool, Vec<u8>) {
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

pub fn for_all_opt_levels(contract_path: &str, test_fn: impl Fn(&mut EvmHandle, u8) + Sync) {
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..=3)
            .map(|opt| {
                let test_fn = &test_fn;
                let path = contract_path;
                s.spawn(move || {
                    let bc = compile_contract_opt(path, opt);
                    let mut h = EvmHandle::new(bc);
                    test_fn(&mut h, opt);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
}
