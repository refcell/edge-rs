#![allow(missing_docs)]

use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
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

const CALLER: Address = Address::new([0x01; 20]);

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

/// In-memory EVM with a single contract deployed via CREATE.
///
/// Storage changes are committed after each [`call`][EvmHandle::call], so
/// calls are stateful: calls to `_mint` followed by `balanceOf` reflect the minted amount.
struct EvmHandle {
    evm: TestEvm,
    contract: Address,
    /// Tracks caller nonce; `transact_commit` increments it in the DB each call.
    nonce: u64,
}

impl EvmHandle {
    /// Deploy `bytecode` via CREATE and return a ready handle.
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

    /// Call the contract with `calldata`. Returns `(success, return_data)`.
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
// Test helpers for addresses
// =============================================================================

/// Create a test address from a single byte suffix.
const fn test_address(suffix: u8) -> [u8; 20] {
    let mut addr = [0u8; 20];
    addr[19] = suffix;
    addr
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn test_erc20_compiles() {
    let bytecode = compile_contract("examples/erc20.edge");
    assert!(!bytecode.is_empty(), "erc20.edge produced empty bytecode");
    assert!(bytecode.len() > 10, "bytecode too short to be valid");
}

#[test]
fn test_erc20_initial_supply() {
    let bytecode = compile_contract("examples/erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let sel = selector("totalSupply()");
    let calldata = encode_call_with_args(sel, &[]);

    let (ok, out) = evm.call(calldata);
    assert!(ok, "totalSupply() reverted on fresh contract");
    assert_eq!(decode_u256(&out), 0, "initial totalSupply should be 0");
}

#[test]
fn test_erc20_mint_and_balance() {
    let bytecode = compile_contract("examples/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let alice = test_address(0x02);

    // Mint 1000 tokens to alice via public mint().
    let mint_sel = selector("mint(address,uint256)");
    let mint_calldata =
        encode_call_with_args(mint_sel, &[encode_address(alice), encode_u256(1000)]);

    let (ok, _) = evm.call(mint_calldata);
    assert!(ok, "mint(alice, 1000) reverted");

    // Query balance of alice
    let bal_sel = selector("balanceOf(address)");
    let bal_calldata = encode_call_with_args(bal_sel, &[encode_address(alice)]);

    let (ok, out) = evm.call(bal_calldata);
    assert!(ok, "balanceOf(alice) reverted");
    assert_eq!(
        decode_u256(&out),
        1000,
        "alice balance should be 1000 after mint"
    );
}

#[test]
fn test_erc20_transfer() {
    let bytecode = compile_contract("examples/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    // CALLER is [0x01; 20]. transfer() uses @caller() as `from`.
    let bob = test_address(0x02);

    // Mint 1000 tokens to CALLER.
    let mint_sel = selector("mint(address,uint256)");
    let caller_addr: [u8; 20] = [0x01; 20];
    let mint_calldata =
        encode_call_with_args(mint_sel, &[encode_address(caller_addr), encode_u256(1000)]);
    let (ok, _) = evm.call(mint_calldata);
    assert!(ok, "mint(caller, 1000) reverted");

    // Transfer 300 from CALLER to bob via public transfer(to, amount).
    let xfer_sel = selector("transfer(address,uint256)");
    let xfer_calldata = encode_call_with_args(xfer_sel, &[encode_address(bob), encode_u256(300)]);
    let (ok, out) = evm.call(xfer_calldata);
    assert!(ok, "transfer(bob, 300) reverted");
    assert_eq!(decode_u256(&out), 1, "transfer should return true (1)");

    // Check CALLER balance (should be 700)
    let bal_sel = selector("balanceOf(address)");
    let caller_calldata = encode_call_with_args(bal_sel, &[encode_address(caller_addr)]);
    let (ok, out) = evm.call(caller_calldata);
    assert!(ok, "balanceOf(caller) reverted");
    assert_eq!(
        decode_u256(&out),
        700,
        "caller balance should be 700 after transfer"
    );

    // Check bob balance (should be 300)
    let bob_calldata = encode_call_with_args(bal_sel, &[encode_address(bob)]);
    let (ok, out) = evm.call(bob_calldata);
    assert!(ok, "balanceOf(bob) reverted");
    assert_eq!(
        decode_u256(&out),
        300,
        "bob balance should be 300 after transfer"
    );
}

#[test]
fn test_erc20_approve_and_transferfrom() {
    let bytecode = compile_contract("examples/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    // CALLER = [0x01; 20]. approve() uses @caller() as owner,
    // transferFrom() uses @caller() for allowance lookup.
    // Single-caller EVM: self-approve then transferFrom from self.
    let caller_addr: [u8; 20] = [0x01; 20];
    let charlie = test_address(0x03);

    // Mint 1000 tokens to CALLER.
    let mint_sel = selector("mint(address,uint256)");
    let mint_calldata =
        encode_call_with_args(mint_sel, &[encode_address(caller_addr), encode_u256(1000)]);
    let (ok, _) = evm.call(mint_calldata);
    assert!(ok, "mint(caller, 1000) reverted");

    // CALLER approves itself to spend 500 tokens (self-approval).
    // approve(spender=CALLER, 500) → allowances[CALLER][CALLER] = 500
    let approve_sel = selector("approve(address,uint256)");
    let approve_calldata = encode_call_with_args(
        approve_sel,
        &[encode_address(caller_addr), encode_u256(500)],
    );
    let (ok, out) = evm.call(approve_calldata);
    assert!(ok, "approve(caller, 500) reverted");
    assert_eq!(decode_u256(&out), 1, "approve should return true (1)");

    // Check allowance: CALLER→CALLER should be 500
    let allow_sel = selector("allowance(address,address)");
    let allow_calldata = encode_call_with_args(
        allow_sel,
        &[encode_address(caller_addr), encode_address(caller_addr)],
    );
    let (ok, out) = evm.call(allow_calldata);
    assert!(ok, "allowance(caller, caller) reverted");
    assert_eq!(
        decode_u256(&out),
        500,
        "caller should have 500 self-allowance"
    );

    // transferFrom(CALLER, charlie, 300) — spender=@caller()=CALLER
    let xfer_sel = selector("transferFrom(address,address,uint256)");
    let xfer_calldata = encode_call_with_args(
        xfer_sel,
        &[
            encode_address(caller_addr),
            encode_address(charlie),
            encode_u256(300),
        ],
    );
    let (ok, out) = evm.call(xfer_calldata);
    assert!(ok, "transferFrom(caller, charlie, 300) reverted");
    assert_eq!(decode_u256(&out), 1, "transferFrom should return true (1)");

    // Check updated allowance: CALLER→CALLER should be 200 (500 - 300)
    let allow_calldata = encode_call_with_args(
        allow_sel,
        &[encode_address(caller_addr), encode_address(caller_addr)],
    );
    let (ok, out) = evm.call(allow_calldata);
    assert!(ok, "allowance after transferFrom reverted");
    assert_eq!(
        decode_u256(&out),
        200,
        "self-allowance should be 200 after transferFrom"
    );

    // Verify charlie received the tokens
    let bal_sel = selector("balanceOf(address)");
    let charlie_calldata = encode_call_with_args(bal_sel, &[encode_address(charlie)]);
    let (ok, out) = evm.call(charlie_calldata);
    assert!(ok, "balanceOf(charlie) reverted");
    assert_eq!(
        decode_u256(&out),
        300,
        "charlie balance should be 300 after transferFrom"
    );
}

// =============================================================================
// Additional lifecycle tests (using test_erc20.edge)
// =============================================================================

#[test]
fn test_erc20_total_supply_after_mint() {
    let bytecode = compile_contract("examples/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let alice = test_address(0x02);

    // Mint 1000 tokens
    let mint_sel = selector("mint(address,uint256)");
    let (ok, _) = evm.call(encode_call_with_args(
        mint_sel,
        &[encode_address(alice), encode_u256(1000)],
    ));
    assert!(ok, "mint reverted");

    // totalSupply should be 1000
    let (ok, out) = evm.call(encode_call_with_args(selector("totalSupply()"), &[]));
    assert!(ok, "totalSupply() reverted");
    assert_eq!(
        decode_u256(&out),
        1000,
        "totalSupply should be 1000 after mint"
    );
}

#[test]
fn test_erc20_transfer_updates_both_balances() {
    let bytecode = compile_contract("examples/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let caller_addr: [u8; 20] = [0x01; 20];
    let alice = test_address(0x02);

    // Mint 1000 to CALLER
    let (ok, _) = evm.call(encode_call_with_args(
        selector("mint(address,uint256)"),
        &[encode_address(caller_addr), encode_u256(1000)],
    ));
    assert!(ok, "mint reverted");

    // Transfer 400 to alice
    let (ok, _) = evm.call(encode_call_with_args(
        selector("transfer(address,uint256)"),
        &[encode_address(alice), encode_u256(400)],
    ));
    assert!(ok, "transfer reverted");

    // Caller: 600, Alice: 400
    let bal_sel = selector("balanceOf(address)");
    let (ok, out) = evm.call(encode_call_with_args(
        bal_sel,
        &[encode_address(caller_addr)],
    ));
    assert!(ok, "balanceOf(caller) reverted");
    assert_eq!(decode_u256(&out), 600);

    let (ok, out) = evm.call(encode_call_with_args(bal_sel, &[encode_address(alice)]));
    assert!(ok, "balanceOf(alice) reverted");
    assert_eq!(decode_u256(&out), 400);
}

#[test]
fn test_erc20_approve_sets_allowance() {
    let bytecode = compile_contract("examples/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let caller_addr: [u8; 20] = [0x01; 20];
    let alice = test_address(0x02);

    // approve(alice, 500) → allowances[CALLER][alice] = 500
    let (ok, out) = evm.call(encode_call_with_args(
        selector("approve(address,uint256)"),
        &[encode_address(alice), encode_u256(500)],
    ));
    assert!(ok, "approve reverted");
    assert_eq!(decode_u256(&out), 1, "approve should return true");

    // Check allowance
    let (ok, out) = evm.call(encode_call_with_args(
        selector("allowance(address,address)"),
        &[encode_address(caller_addr), encode_address(alice)],
    ));
    assert!(ok, "allowance reverted");
    assert_eq!(decode_u256(&out), 500, "allowance should be 500");
}

#[test]
fn test_erc20_unknown_selector_reverts() {
    let bytecode = compile_contract("examples/test_erc20.edge");
    let mut evm = EvmHandle::new(bytecode);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}
