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

/// Fixed address where the ERC20 contract is deployed for tests.
/// Using 0x1001 to avoid collisions with other contracts (counter.edge uses 0x1000).
const CONTRACT_ADDR: Address = Address::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x01,
]);

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

/// In-memory EVM with a single contract deployed at [`CONTRACT_ADDR`].
///
/// Storage changes are committed after each [`call`][EvmHandle::call], so
/// calls are stateful: calls to `_mint` followed by `balanceOf` reflect the minted amount.
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
#[ignore = "requires mapping support (task #2) and calldata args (task #1)"]
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
#[ignore = "requires mapping support (task #2) and calldata args (task #1)"]
fn test_erc20_mint_and_balance() {
    let bytecode = compile_contract("examples/erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let alice = test_address(0x01);

    // Mint 1000 tokens to alice.
    // Note: This test assumes the contract provides a _mint function that can be
    // called. Depending on the implementation, this might require an internal call
    // or a public mint function. For now, we assume _mint exists as a public function.
    let mint_sel = selector("_mint(address,uint256)");
    let mint_calldata =
        encode_call_with_args(mint_sel, &[encode_address(alice), encode_u256(1000)]);

    let (ok, _) = evm.call(mint_calldata);
    assert!(ok, "_mint(alice, 1000) reverted");

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
#[ignore = "requires mapping support (task #2) and calldata args (task #1)"]
fn test_erc20_transfer() {
    let bytecode = compile_contract("examples/erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let alice = test_address(0x01);
    let bob = test_address(0x02);

    // Mint 1000 tokens to alice
    let mint_sel = selector("_mint(address,uint256)");
    let mint_calldata =
        encode_call_with_args(mint_sel, &[encode_address(alice), encode_u256(1000)]);
    let (ok, _) = evm.call(mint_calldata);
    assert!(ok, "_mint(alice, 1000) reverted");

    // Transfer 300 from alice to bob.
    // Note: This assumes transfer can be called with the contract acting as alice.
    // In a real test, we'd need to set the caller to alice, but EvmHandle uses fixed caller.
    // For now, we assume _transfer is callable directly for testing.
    let xfer_sel = selector("_transfer(address,address,uint256)");
    let xfer_calldata = encode_call_with_args(
        xfer_sel,
        &[encode_address(alice), encode_address(bob), encode_u256(300)],
    );
    let (ok, _) = evm.call(xfer_calldata);
    assert!(ok, "_transfer(alice, bob, 300) reverted");

    // Check alice balance (should be 700)
    let alice_sel = selector("balanceOf(address)");
    let alice_calldata = encode_call_with_args(alice_sel, &[encode_address(alice)]);
    let (ok, out) = evm.call(alice_calldata);
    assert!(ok, "balanceOf(alice) reverted");
    assert_eq!(
        decode_u256(&out),
        700,
        "alice balance should be 700 after transfer"
    );

    // Check bob balance (should be 300)
    let bob_sel = selector("balanceOf(address)");
    let bob_calldata = encode_call_with_args(bob_sel, &[encode_address(bob)]);
    let (ok, out) = evm.call(bob_calldata);
    assert!(ok, "balanceOf(bob) reverted");
    assert_eq!(
        decode_u256(&out),
        300,
        "bob balance should be 300 after transfer"
    );
}

#[test]
#[ignore = "requires mapping support (task #2) and calldata args (task #1)"]
fn test_erc20_approve_and_transferfrom() {
    let bytecode = compile_contract("examples/erc20.edge");
    let mut evm = EvmHandle::new(bytecode);

    let alice = test_address(0x01);
    let bob = test_address(0x02);
    let charlie = test_address(0x03);

    // Mint 1000 tokens to alice
    let mint_sel = selector("_mint(address,uint256)");
    let mint_calldata =
        encode_call_with_args(mint_sel, &[encode_address(alice), encode_u256(1000)]);
    let (ok, _) = evm.call(mint_calldata);
    assert!(ok, "_mint(alice, 1000) reverted");

    // Alice approves bob to spend 500 tokens.
    // Note: In a real test, we'd call approve with caller=alice.
    // For now, we assume _approve is testable as a public function.
    let approve_sel = selector("_approve(address,address,uint256)");
    let approve_calldata = encode_call_with_args(
        approve_sel,
        &[encode_address(alice), encode_address(bob), encode_u256(500)],
    );
    let (ok, _) = evm.call(approve_calldata);
    assert!(ok, "_approve(alice, bob, 500) reverted");

    // Check allowance: bob should be able to spend 500 from alice
    let allow_sel = selector("allowance(address,address)");
    let allow_calldata =
        encode_call_with_args(allow_sel, &[encode_address(alice), encode_address(bob)]);
    let (ok, out) = evm.call(allow_calldata);
    assert!(ok, "allowance(alice, bob) reverted");
    assert_eq!(
        decode_u256(&out),
        500,
        "bob should have 500 allowance from alice"
    );

    // Bob transfers 300 from alice to charlie using transferFrom.
    // Note: This assumes transferFrom can be called with bob as spender.
    let xfer_sel = selector("transferFrom(address,address,uint256)");
    let xfer_calldata = encode_call_with_args(
        xfer_sel,
        &[
            encode_address(alice),
            encode_address(charlie),
            encode_u256(300),
        ],
    );
    let (ok, _) = evm.call(xfer_calldata);
    assert!(ok, "transferFrom(alice, charlie, 300) reverted");

    // Check updated allowance: bob should have 200 left (500 - 300)
    let allow_sel = selector("allowance(address,address)");
    let allow_calldata =
        encode_call_with_args(allow_sel, &[encode_address(alice), encode_address(bob)]);
    let (ok, out) = evm.call(allow_calldata);
    assert!(ok, "allowance(alice, bob) after transferFrom reverted");
    assert_eq!(
        decode_u256(&out),
        200,
        "bob should have 200 allowance left after transferFrom"
    );

    // Verify charlie received the tokens
    let charlie_sel = selector("balanceOf(address)");
    let charlie_calldata = encode_call_with_args(charlie_sel, &[encode_address(charlie)]);
    let (ok, out) = evm.call(charlie_calldata);
    assert!(ok, "balanceOf(charlie) reverted");
    assert_eq!(
        decode_u256(&out),
        300,
        "charlie balance should be 300 after transferFrom"
    );
}
