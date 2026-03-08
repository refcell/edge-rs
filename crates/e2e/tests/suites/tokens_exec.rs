#![allow(missing_docs)]

//! Execution-level acceptance tests for token contracts.
//!
//! Tests compile weth.edge and amm.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify stateful behaviour.
//!
//! ## Compiler caveats
//! Internal helper functions (_transfer) are lowered to stubs.
//! Multi-value returns (getReserves) return 0 at this compiler stage
//! since tuple instantiation is not yet fully supported.

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

fn encode_address(addr: [u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(&addr);
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

fn decode_bool(output: &[u8]) -> bool {
    decode_u256(output) != 0
}

const CALLER: Address = Address::ZERO;
const ALICE_ADDR: [u8; 20] = {
    let mut a = [0u8; 20];
    a[19] = 0xA1;
    a
};

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
        // Fund caller so it can deploy and send ETH value in deposit tests.
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
        self.call_with_value(calldata, 0)
    }

    fn call_with_value(&mut self, calldata: Vec<u8>, value: u64) -> (bool, Vec<u8>) {
        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Call(self.contract))
            .data(Bytes::from(calldata))
            .value(U256::from(value))
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

// =============================================================================
// WETH
// =============================================================================

#[test]
fn test_weth_total_supply_initially_zero() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(ok, "totalSupply() reverted");
    assert_eq!(decode_u256(&out), 0, "totalSupply should start at 0");
}

#[test]
fn test_weth_balance_of_zero_initially() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(ok, "balanceOf() reverted");
    assert_eq!(decode_u256(&out), 0, "balanceOf should start at 0");
}

#[test]
fn test_weth_approve_returns_true() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("approve(address,uint256)"),
        &[encode_address(ALICE_ADDR), encode_u256(1000)],
    ));
    assert!(ok, "approve() reverted");
    assert!(decode_bool(&out), "approve should return true");
}

#[test]
fn test_weth_allowance_initially_zero() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("allowance(address,address)"),
        &[encode_address([0u8; 20]), encode_address(ALICE_ADDR)],
    ));
    assert!(ok, "allowance() reverted");
    assert_eq!(decode_u256(&out), 0, "allowance should start at 0");
}

#[test]
fn test_weth_deposit_increases_total_supply() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);

    // deposit() — sends ETH value, mints WETH
    let deposit_amount: u64 = 1_000_000;
    let (ok, _) = evm.call_with_value(calldata(selector("deposit()"), &[]), deposit_amount);
    assert!(ok, "deposit() reverted");

    let (ok, out) = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(ok, "totalSupply() reverted after deposit");
    assert_eq!(
        decode_u256(&out),
        deposit_amount,
        "totalSupply should equal deposit amount"
    );
}

#[test]
fn test_weth_deposit_increases_balance() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);

    let deposit_amount: u64 = 500_000;
    // caller is Address::ZERO
    let (ok, _) = evm.call_with_value(calldata(selector("deposit()"), &[]), deposit_amount);
    assert!(ok, "deposit() reverted");

    let (ok, out) = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address([0u8; 20])],
    ));
    assert!(ok, "balanceOf() reverted after deposit");
    assert_eq!(
        decode_u256(&out),
        deposit_amount,
        "balanceOf(caller) should equal deposit amount"
    );
}

#[test]
fn test_weth_unknown_selector_reverts() {
    let bc = compile_contract("std/tokens/weth.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}

// =============================================================================
// AMM
// =============================================================================

#[test]
fn test_amm_total_supply_initially_zero() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(ok, "totalSupply() reverted");
    assert_eq!(decode_u256(&out), 0, "AMM totalSupply should start at 0");
}

#[test]
fn test_amm_balance_of_initially_zero() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("balanceOf(address)"),
        &[encode_address(ALICE_ADDR)],
    ));
    assert!(ok, "balanceOf() reverted");
    assert_eq!(decode_u256(&out), 0, "AMM balanceOf should start at 0");
}

#[test]
fn test_amm_get_reserves_initially_zero() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("getReserves()"), &[]));
    assert!(ok, "getReserves() reverted");
    // Tuple return: 64 bytes (two u256 values)
    assert!(
        out.len() >= 64,
        "getReserves should return 64 bytes, got {}",
        out.len()
    );
    assert_eq!(decode_u256(&out[0..32]), 0, "reserve0 should be 0");
    assert_eq!(decode_u256(&out[32..64]), 0, "reserve1 should be 0");
}

#[test]
fn test_amm_unknown_selector_reverts() {
    let bc = compile_contract("std/finance/amm.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}

// =============================================================================
// ERC721
// =============================================================================

#[test]
fn test_erc721_total_supply_initially_zero() {
    let bc = compile_contract("std/tokens/erc721.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("totalSupply()"), &[]));
    assert!(ok, "totalSupply() reverted");
    assert_eq!(decode_u256(&out), 0, "ERC721 totalSupply should start at 0");
}

#[test]
fn test_erc721_unknown_selector_reverts() {
    let bc = compile_contract("std/tokens/erc721.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}
