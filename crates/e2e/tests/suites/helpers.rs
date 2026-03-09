//! Shared test helpers for e2e execution tests.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Mutex, Once},
};

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

// ═══════════════════════════════════════════════════════════════════════════
// Gas snapshot recording
// ═══════════════════════════════════════════════════════════════════════════

static GAS_ENTRIES: Mutex<BTreeMap<String, [u64; 4]>> = Mutex::new(BTreeMap::new());
static REGISTER_ATEXIT: Once = Once::new();
static SELECTOR_SIGS: Mutex<BTreeMap<[u8; 4], String>> = Mutex::new(BTreeMap::new());

extern "C" fn flush_gas_snapshot() {
    let entries = GAS_ENTRIES.lock().unwrap();
    if entries.is_empty() {
        return;
    }

    let dir = Path::new("/tmp/edge-gas");
    std::fs::create_dir_all(dir).ok();

    let mut content = String::new();
    for (name, gas) in entries.iter() {
        let vals: Vec<String> = gas
            .iter()
            .map(|&g| {
                if g == u64::MAX {
                    "-".to_string()
                } else {
                    g.to_string()
                }
            })
            .collect();
        content.push_str(&format!(
            "{name}, {}, {}, {}, {}\n",
            vals[0], vals[1], vals[2], vals[3]
        ));
    }
    std::fs::write(dir.join("e2e.csv"), content).ok();
}

fn ensure_atexit() {
    REGISTER_ATEXIT.call_once(|| {
        extern "C" {
            fn atexit(f: extern "C" fn()) -> std::ffi::c_int;
        }
        // SAFETY: `flush_gas_snapshot` is an extern "C" fn with no captures;
        // registering it with libc `atexit` is safe.
        unsafe {
            atexit(flush_gas_snapshot);
        }
    });
}

fn calldata_intrinsic_gas(cd: &[u8]) -> u64 {
    cd.iter().map(|&b| if b == 0 { 4u64 } else { 16u64 }).sum()
}

fn execution_gas(gas_used: u64, cd: &[u8]) -> u64 {
    gas_used
        .saturating_sub(21000)
        .saturating_sub(calldata_intrinsic_gas(cd))
}

fn record_gas(label: &str, opt_level: u8, cd: &[u8], gas_used: u64) {
    if opt_level > 3 {
        return;
    }
    ensure_atexit();
    let exec = execution_gas(gas_used, cd);
    let mut map = GAS_ENTRIES.lock().unwrap();
    let entry = map.entry(label.to_string()).or_insert([u64::MAX; 4]);
    let slot = &mut entry[opt_level as usize];
    // Keep the maximum across calls (worst case when multiple tests hit
    // the same selector with different args / calldata costs).
    if *slot == u64::MAX {
        *slot = exec;
    } else {
        *slot = (*slot).max(exec);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Compilation helpers
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Compile a contract at the default optimization level (O0).
pub(crate) fn compile_contract(relative_path: &str) -> Vec<u8> {
    compile_contract_opt(relative_path, 0)
}

/// Compile a contract at a specific optimization level.
pub(crate) fn compile_contract_opt(relative_path: &str, opt_level: u8) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    config.optimization_level = opt_level;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
}

/// Compile a specific named contract from a multi-contract file.
pub(crate) fn compile_named(relative_path: &str, contract_name: &str) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output
        .bytecodes
        .expect("no bytecodes map")
        .get(contract_name)
        .unwrap_or_else(|| panic!("contract {contract_name} not found"))
        .clone()
}

// ═══════════════════════════════════════════════════════════════════════════
// ABI helpers
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) fn selector(sig: &str) -> [u8; 4] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    let sel = [out[0], out[1], out[2], out[3]];
    SELECTOR_SIGS
        .lock()
        .unwrap()
        .entry(sel)
        .or_insert_with(|| sig.to_string());
    sel
}

/// Compute the full 32-byte keccak256 hash of a signature (for event topic matching).
pub(crate) fn event_sig(sig: &str) -> [u8; 32] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

pub(crate) fn decode_u256(output: &[u8]) -> u64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    assert_eq!(&output[0..24], &[0u8; 24], "u256 too large for u64");
    u64::from_be_bytes(output[24..32].try_into().unwrap())
}

pub(crate) fn decode_bool(output: &[u8]) -> bool {
    decode_u256(output) != 0
}

pub(crate) fn decode_address(output: &[u8]) -> [u8; 20] {
    assert!(output.len() >= 32, "return value too short");
    output[12..32].try_into().unwrap()
}

pub(crate) fn encode_u256(val: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&val.to_be_bytes());
    out
}

pub(crate) fn encode_address(addr: [u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(&addr);
    out
}

pub(crate) const fn encode_b32(val: [u8; 32]) -> [u8; 32] {
    val
}

pub(crate) fn calldata(sel: [u8; 4], args: &[[u8; 32]]) -> Vec<u8> {
    let mut cd = sel.to_vec();
    for a in args {
        cd.extend_from_slice(a);
    }
    cd
}

fn label_from_path(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// Call result types
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub(crate) struct LogEntry {
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct CallResult {
    pub success: bool,
    pub output: Vec<u8>,
    pub logs: Vec<LogEntry>,
}

// ═══════════════════════════════════════════════════════════════════════════
// EVM handle
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) const CALLER: Address = Address::ZERO;

pub(crate) type TestDb = CacheDB<EmptyDB>;
pub(crate) type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

pub(crate) struct EvmHandle {
    pub evm: TestEvm,
    pub contract: Address,
    pub nonce: u64,
    opt_level: u8,
    contract_label: Option<String>,
}

impl EvmHandle {
    pub(crate) fn new(deploy_bytecode: Vec<u8>) -> Self {
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
            opt_level: 0,
            contract_label: None,
        }
    }

    pub(crate) fn call(&mut self, cd: Vec<u8>) -> CallResult {
        self.call_with_value(cd, 0)
    }

    pub(crate) fn call_with_value(&mut self, cd: Vec<u8>, value: u64) -> CallResult {
        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Call(self.contract))
            .data(Bytes::from(cd.clone()))
            .value(U256::from(value))
            .nonce(self.nonce)
            .gas_limit(10_000_000)
            .build()
            .unwrap();
        let result = self.evm.transact_commit(tx).unwrap();
        self.nonce += 1;
        let success = result.is_success();
        let gas_used = result.gas_used();
        let output = result.output().map(|b| b.to_vec()).unwrap_or_default();
        let logs = result
            .logs()
            .iter()
            .map(|l| LogEntry {
                topics: l.data.topics().iter().map(|t| t.0).collect(),
                data: l.data.data.to_vec(),
            })
            .collect();

        if success {
            if let Some(ref label) = self.contract_label {
                if cd.len() >= 4 {
                    let sel = [cd[0], cd[1], cd[2], cd[3]];
                    let sig = SELECTOR_SIGS.lock().unwrap().get(&sel).cloned();
                    if let Some(sig) = sig {
                        let key = format!("{label}::{sig}");
                        record_gas(&key, self.opt_level, &cd, gas_used);
                    }
                }
            }
        }

        CallResult {
            success,
            output,
            logs,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn call_fn(&mut self, sig: &str, args: &[[u8; 32]]) -> CallResult {
        let cd = calldata(selector(sig), args);
        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Call(self.contract))
            .data(Bytes::from(cd.clone()))
            .nonce(self.nonce)
            .gas_limit(10_000_000)
            .build()
            .unwrap();
        let result = self.evm.transact_commit(tx).unwrap();
        self.nonce += 1;
        let success = result.is_success();
        let gas_used = result.gas_used();
        let output = result.output().map(|b| b.to_vec()).unwrap_or_default();
        let logs = result
            .logs()
            .iter()
            .map(|l| LogEntry {
                topics: l.data.topics().iter().map(|t| t.0).collect(),
                data: l.data.data.to_vec(),
            })
            .collect();

        if success {
            if let Some(ref label) = self.contract_label {
                let key = format!("{label}::{sig}");
                record_gas(&key, self.opt_level, &cd, gas_used);
            }
        }

        CallResult {
            success,
            output,
            logs,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test runners
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) fn for_all_opt_levels(contract_path: &str, test_fn: impl Fn(&mut EvmHandle, u8) + Sync) {
    let label = label_from_path(contract_path);
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..=3)
            .map(|opt| {
                let test_fn = &test_fn;
                let path = contract_path;
                let label = &label;
                s.spawn(move || {
                    let bc = compile_contract_opt(path, opt);
                    let mut h = EvmHandle::new(bc);
                    h.opt_level = opt;
                    h.contract_label = Some(label.clone());
                    test_fn(&mut h, opt);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
}
