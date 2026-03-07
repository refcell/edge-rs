#![allow(missing_docs)]

//! Execution-level tests for packed struct bit-packing.
//!
//! Uses a revm Inspector to capture EVM MSTORE operations, verifying that
//! packed structs store fields in a single packed word rather than
//! one-word-per-field. Tests run at O0, O1, O2, and O3.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use revm::{
    context::{Context, TxEnv},
    database::{CacheDB, EmptyDB},
    handler::MainnetContext,
    interpreter::{interpreter::EthInterpreter, interpreter_types::Jumps, Interpreter},
    primitives::{Address, Bytes, TxKind, U256},
    state::AccountInfo,
    ExecuteCommitEvm, Inspector, MainBuilder, MainContext, MainnetEvm,
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

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const CALLER: Address = Address::ZERO;

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

// =============================================================================
// Memory-capturing Inspector
// =============================================================================

type MStoreLog = Vec<(usize, [u8; 32])>;
type MStoreCapture = Rc<RefCell<MStoreLog>>;

/// Inspector that captures MSTORE args from the stack BEFORE execution.
/// Records (offset, value) for each MSTORE opcode.
struct MStoreInspector {
    capture: MStoreCapture,
    /// Pending MSTORE: set in `step` (pre-exec), flushed in `step_end` (post-exec)
    pending: Option<(usize, [u8; 32])>,
}

impl MStoreInspector {
    const fn new(capture: MStoreCapture) -> Self {
        Self {
            capture,
            pending: None,
        }
    }
}

impl<CTX> Inspector<CTX, EthInterpreter> for MStoreInspector {
    fn step(&mut self, interp: &mut Interpreter<EthInterpreter>, _context: &mut CTX) {
        let opcode = interp.bytecode.opcode();
        // MSTORE = 0x52: stack[0] = offset, stack[1] = value
        if opcode == 0x52 {
            if let (Ok(offset), Ok(value)) = (interp.stack.peek(0), interp.stack.peek(1)) {
                let off: usize = offset.to::<usize>();
                let val_bytes: [u8; 32] = value.to_be_bytes();
                self.pending = Some((off, val_bytes));
            }
        }
    }

    fn step_end(&mut self, _interp: &mut Interpreter<EthInterpreter>, _context: &mut CTX) {
        if let Some(entry) = self.pending.take() {
            self.capture.borrow_mut().push(entry);
        }
    }
}

// =============================================================================
// EVM handles
// =============================================================================

/// Standard EVM handle (no inspector).
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

/// Deploy a contract and call a function with MSTORE inspection.
/// Returns (success, output, mstores) where mstores is Vec<(offset, 32-byte value)>.
fn call_with_inspector(deploy_bytecode: &[u8], calldata: Vec<u8>) -> (bool, Vec<u8>, MStoreLog) {
    let capture: MStoreCapture = Rc::new(RefCell::new(Vec::new()));

    let mut db = CacheDB::<EmptyDB>::default();
    db.insert_account_info(
        CALLER,
        AccountInfo {
            balance: U256::from(u64::MAX),
            nonce: 0,
            ..Default::default()
        },
    );

    // Build with inspector from the start (inspector is a no-op during deployment)
    let inspector = MStoreInspector::new(Rc::clone(&capture));
    let mut evm = Context::mainnet()
        .with_db(db)
        .build_mainnet_with_inspector(inspector);

    // Deploy
    let deploy_tx = TxEnv::builder()
        .caller(CALLER)
        .kind(TxKind::Create)
        .data(Bytes::from(deploy_bytecode.to_vec()))
        .gas_limit(10_000_000)
        .nonce(0)
        .build()
        .unwrap();

    use revm::inspector::InspectCommitEvm;
    let result = evm.inspect_tx_commit(deploy_tx).unwrap();
    assert!(result.is_success(), "Deployment failed: {result:#?}");
    let contract = CALLER.create(0);

    // Clear capture from deployment MSTOREs
    capture.borrow_mut().clear();

    // Call with inspection
    let call_tx = TxEnv::builder()
        .caller(CALLER)
        .kind(TxKind::Call(contract))
        .data(Bytes::from(calldata))
        .nonce(1)
        .gas_limit(10_000_000)
        .build()
        .unwrap();

    let result = evm.inspect_tx_commit(call_tx).unwrap();
    let success = result.is_success();
    let output = result.output().map(|b| b.to_vec()).unwrap_or_default();
    let mstores = capture.borrow().clone();
    (success, output, mstores)
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
// Packed struct execution tests
// =============================================================================

const PACKED: &str = "examples/tests/test_packed_structs.edge";

// ----- Value correctness tests -----

#[test]
fn test_packed_rgb_r() {
    for_all_opt_levels(PACKED, |h, o| {
        let (ok, out) = h.call(calldata(selector("packed_rgb_r()"), &[]));
        assert!(ok, "packed_rgb_r() reverted at O{o}");
        assert_eq!(decode_u256(&out), 1, "packed_rgb_r() wrong at O{o}");
    });
}

#[test]
fn test_packed_rgb_g() {
    for_all_opt_levels(PACKED, |h, o| {
        let (ok, out) = h.call(calldata(selector("packed_rgb_g()"), &[]));
        assert!(ok, "packed_rgb_g() reverted at O{o}");
        assert_eq!(decode_u256(&out), 2, "packed_rgb_g() wrong at O{o}");
    });
}

#[test]
fn test_packed_rgb_b() {
    for_all_opt_levels(PACKED, |h, o| {
        let (ok, out) = h.call(calldata(selector("packed_rgb_b()"), &[]));
        assert!(ok, "packed_rgb_b() reverted at O{o}");
        assert_eq!(decode_u256(&out), 3, "packed_rgb_b() wrong at O{o}");
    });
}

#[test]
fn test_packed_pair_a() {
    for_all_opt_levels(PACKED, |h, o| {
        let (ok, out) = h.call(calldata(selector("packed_pair_a()"), &[]));
        assert!(ok, "packed_pair_a() reverted at O{o}");
        assert_eq!(decode_u256(&out), 42, "packed_pair_a() wrong at O{o}");
    });
}

#[test]
fn test_packed_pair_b() {
    for_all_opt_levels(PACKED, |h, o| {
        let (ok, out) = h.call(calldata(selector("packed_pair_b()"), &[]));
        assert!(ok, "packed_pair_b() reverted at O{o}");
        assert_eq!(decode_u256(&out), 99, "packed_pair_b() wrong at O{o}");
    });
}

#[test]
fn test_packed_field_sum() {
    for_all_opt_levels(PACKED, |h, o| {
        let (ok, out) = h.call(calldata(selector("packed_field_sum()"), &[]));
        assert!(ok, "packed_field_sum() reverted at O{o}");
        assert_eq!(decode_u256(&out), 60, "packed_field_sum() wrong at O{o}");
    });
}

#[test]
fn test_packed_two_structs() {
    for_all_opt_levels(PACKED, |h, o| {
        let (ok, out) = h.call(calldata(selector("packed_two_structs()"), &[]));
        assert!(ok, "packed_two_structs() reverted at O{o}");
        assert_eq!(decode_u256(&out), 31, "packed_two_structs() wrong at O{o}");
    });
}

// ----- Memory layout verification tests (inspector-based) -----

#[test]
fn test_packed_rgb_memory_word() {
    // Verify the actual packed word stored in memory via MSTORE inspection.
    // Rgb { r: 1, g: 2, b: 3 } → packed word = (1 << 16) | (2 << 8) | 3 = 0x010203
    let bc = compile_contract_opt(PACKED, 0);
    let (ok, out, mstores) = call_with_inspector(&bc, calldata(selector("packed_rgb_r()"), &[]));
    assert!(ok, "packed_rgb_r() reverted");
    assert_eq!(decode_u256(&out), 1, "packed_rgb_r() wrong value");

    let expected_packed = U256::from(0x010203u64);
    let found = mstores
        .iter()
        .any(|(_offset, val)| U256::from_be_bytes(*val) == expected_packed);
    assert!(
        found,
        "Expected packed word 0x010203 in MSTOREs, but not found.\nMSTOREs:\n{}",
        mstores
            .iter()
            .map(|(off, val)| format!("  offset={off}, value=0x{}", hex_encode(val)))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_packed_pair_memory_word() {
    // Verify the actual packed word for Pair128 { a: 42, b: 99 }.
    // a (u128) at bits [255:128], b (u128) at bits [127:0]
    // packed = (42 << 128) | 99
    let bc = compile_contract_opt(PACKED, 0);
    let (ok, out, mstores) = call_with_inspector(&bc, calldata(selector("packed_pair_a()"), &[]));
    assert!(ok, "packed_pair_a() reverted");
    assert_eq!(decode_u256(&out), 42, "packed_pair_a() wrong value");

    let expected_packed = (U256::from(42u64) << 128) | U256::from(99u64);
    let found = mstores
        .iter()
        .any(|(_offset, val)| U256::from_be_bytes(*val) == expected_packed);
    assert!(
        found,
        "Expected packed word (42<<128)|99 in MSTOREs, but not found.\nMSTOREs:\n{}",
        mstores
            .iter()
            .map(|(off, val)| format!("  offset={off}, value=0x{}", hex_encode(val)))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
