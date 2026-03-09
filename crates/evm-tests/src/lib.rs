//! EVM semantic test harness for the Edge compiler.
//!
//! Provides [`EvmTestHost`] which compiles `.edge` files, deploys the resulting
//! bytecode on an in-memory revm instance, and executes calls against it.

#![allow(missing_docs)]

use std::{
    collections::HashMap,
    path::PathBuf,
};

use alloy_primitives::{Address, Bytes, Log, U256};
use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use revm::{
    context::TxEnv,
    database::{CacheDB, EmptyDB},
    handler::MainnetContext,
    primitives::TxKind,
    state::AccountInfo,
    ExecuteCommitEvm, MainBuilder, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

// ═══════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a contract call.
#[derive(Debug)]
pub struct CallResult {
    /// Whether the call succeeded.
    pub success: bool,
    /// Raw return bytes.
    pub output: Vec<u8>,
    /// Gas consumed.
    pub gas_used: u64,
    /// Logs emitted during the call.
    pub logs: Vec<Log>,
}

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

/// In-memory EVM test host for semantic testing of Edge contracts.
pub struct EvmTestHost {
    evm: TestEvm,
    contract: Address,
    caller: Address,
    nonces: HashMap<Address, u64>,
}

impl std::fmt::Debug for EvmTestHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvmTestHost")
            .field("contract", &self.contract)
            .field("caller", &self.caller)
            .field("nonces", &self.nonces)
            .finish()
    }
}

/// Result of a contract deployment.
#[derive(Debug)]
pub struct DeployResult {
    /// The test host with the deployed contract.
    pub host: EvmTestHost,
    /// Size of the deployment (init) bytecode in bytes.
    pub init_code_size: usize,
    /// Gas used for deployment.
    pub deploy_gas: u64,
}

// ═══════════════════════════════════════════════════════════════════════════
// EvmTestHost
// ═══════════════════════════════════════════════════════════════════════════

impl EvmTestHost {
    /// Compile an `.edge` file at the given path and deploy its bytecode.
    pub fn deploy_edge(path: &str, opt_level: u8) -> Self {
        Self::deploy_edge_measured(path, opt_level).host
    }

    /// Compile and deploy, returning deployment metrics alongside the host.
    pub fn deploy_edge_measured(path: &str, opt_level: u8) -> DeployResult {
        let bytecode = compile_edge(path, opt_level);
        let init_code_size = bytecode.len();
        let (host, deploy_gas) = Self::deploy_bytecode_measured(&bytecode);
        DeployResult {
            host,
            init_code_size,
            deploy_gas,
        }
    }

    /// Compile an `.edge` file optimized for size and deploy.
    pub fn deploy_edge_for_size(path: &str, opt_level: u8) -> Self {
        let bytecode = compile_edge_for_size(path, opt_level);
        Self::deploy_bytecode(&bytecode)
    }

    /// Compile optimized for size and deploy, returning deployment metrics.
    pub fn deploy_edge_for_size_measured(path: &str, opt_level: u8) -> DeployResult {
        let bytecode = compile_edge_for_size(path, opt_level);
        let init_code_size = bytecode.len();
        let (host, deploy_gas) = Self::deploy_bytecode_measured(&bytecode);
        DeployResult {
            host,
            init_code_size,
            deploy_gas,
        }
    }

    /// Deploy raw bytecode and return a test host.
    pub fn deploy_bytecode(bytecode: &[u8]) -> Self {
        Self::deploy_bytecode_measured(bytecode).0
    }

    /// Deploy raw bytecode by running init code via CREATE, returning (host, `deploy_gas`).
    fn deploy_bytecode_measured(bytecode: &[u8]) -> (Self, u64) {
        let caller = Address::from([0x01; 20]);

        // Build a CacheDB with funded caller
        let mut db = CacheDB::<EmptyDB>::default();
        let caller_info = AccountInfo {
            balance: U256::from(1_000_000_000_000_000_000u128),
            nonce: 0,
            ..Default::default()
        };
        db.insert_account_info(caller, caller_info);

        // Build the EVM: Context::mainnet() gives EmptyDB, .with_db() swaps in our CacheDB
        let ctx = revm::context::Context::mainnet().with_db(db);
        let mut evm = ctx.build_mainnet();

        let tx = TxEnv::builder()
            .caller(caller)
            .kind(TxKind::Create)
            .data(Bytes::copy_from_slice(bytecode))
            .gas_limit(10_000_000)
            .nonce(0)
            .build()
            .unwrap();

        let result = evm.transact_commit(tx).unwrap();
        let deploy_gas = result.gas_used();
        assert!(result.is_success(), "Deployment failed: {result:#?}");

        // The deployed address for CREATE is derived from (caller, nonce=0)
        let deployed_addr = caller.create(0);

        let mut nonces = HashMap::new();
        nonces.insert(caller, 1);

        let host = Self {
            evm,
            contract: deployed_addr,
            caller,
            nonces,
        };

        (host, deploy_gas)
    }

    /// The deployed contract address.
    pub const fn address(&self) -> Address {
        self.contract
    }

    /// Size of the deployed runtime bytecode in bytes.
    pub fn runtime_code_size(&self) -> usize {
        use revm::context_interface::JournalTr;
        self.evm
            .ctx
            .journaled_state
            .db()
            .cache
            .accounts
            .get(&self.contract)
            .and_then(|acc| acc.info.code.as_ref())
            .map(|code| code.len())
            .unwrap_or(0)
    }

    /// The caller address used for transactions.
    pub const fn caller(&self) -> Address {
        self.caller
    }

    /// Call the contract with a raw 4-byte selector + ABI-encoded args.
    pub fn call(&mut self, selector: [u8; 4], args: &[u8]) -> CallResult {
        let mut calldata = selector.to_vec();
        calldata.extend_from_slice(args);
        self.call_raw(&calldata)
    }

    /// Call the contract with raw calldata bytes.
    pub fn call_raw(&mut self, calldata: &[u8]) -> CallResult {
        let nonce = self.nonces.entry(self.caller).or_insert(0);
        let tx = TxEnv::builder()
            .caller(self.caller)
            .kind(TxKind::Call(self.contract))
            .data(Bytes::copy_from_slice(calldata))
            .gas_limit(10_000_000)
            .nonce(*nonce)
            .build()
            .unwrap();

        let result = self.evm.transact_commit(tx).unwrap();
        *self.nonces.entry(self.caller).or_insert(0) += 1;

        let success = result.is_success();
        let gas_used = result.gas_used();
        let output = result.output().map(|b| b.to_vec()).unwrap_or_default();
        let logs = result.logs().to_vec();

        CallResult {
            success,
            output,
            gas_used,
            logs,
        }
    }

    /// Call using a function signature string.
    ///
    /// e.g. `call_fn("transfer(address,uint256)", &args)`
    pub fn call_fn(&mut self, sig: &str, args: &[u8]) -> CallResult {
        let selector = fn_selector(sig);
        self.call(selector, args)
    }

    /// Read a storage slot directly from the database.
    pub fn sload(&self, slot: U256) -> U256 {
        use revm::{context_interface::JournalTr, Database};
        let mut db = self.evm.ctx.journaled_state.db().clone();
        db.storage(self.contract, slot)
            .expect("failed to read storage")
    }

    /// Set the caller address for subsequent transactions.
    /// Ensures the new caller has account info in the DB (only if not already present).
    pub fn set_caller(&mut self, caller: Address) {
        use revm::{context_interface::JournalTr, Database};
        let db = self.evm.ctx.journaled_state.db_mut();
        // Only insert if the account doesn't already exist
        if db.basic(caller).ok().flatten().is_none() {
            let caller_info = AccountInfo {
                balance: U256::from(1_000_000_000_000_000_000u128),
                nonce: 0,
                ..Default::default()
            };
            db.insert_account_info(caller, caller_info);
        }
        self.caller = caller;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Compilation helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Compile an `.edge` file and return the raw deployment bytecode.
pub fn compile_edge(path: &str, opt_level: u8) -> Vec<u8> {
    let mut config = CompilerConfig::new(PathBuf::from(path));
    config.emit = EmitKind::Bytecode;
    config.optimization_level = opt_level;
    let mut compiler = Compiler::new(config).expect("failed to create compiler");
    let output = compiler.compile().expect("compilation failed");
    output.bytecode.expect("no bytecode generated")
}

/// Compile an `.edge` file optimizing for code size.
pub fn compile_edge_for_size(path: &str, opt_level: u8) -> Vec<u8> {
    let mut config = CompilerConfig::new(PathBuf::from(path));
    config.emit = EmitKind::Bytecode;
    config.optimization_level = opt_level;
    config.optimize_for = edge_driver::config::OptimizeFor::Size;
    let mut compiler = Compiler::new(config).expect("failed to create compiler");
    let output = compiler.compile().expect("compilation failed");
    output.bytecode.expect("no bytecode generated")
}

/// Compile with separate optimization levels for IR and bytecode stages.
/// Useful for isolating bugs to one optimizer layer.
pub fn compile_edge_split(
    path: &str,
    ir_opt_level: u8,
    bytecode_opt_level: u8,
    optimize_for: edge_ir::OptimizeFor,
) -> Vec<u8> {
    let source = std::fs::read_to_string(path).expect("failed to read source");
    let mut parser = edge_parser::Parser::new(&source).expect("failed to create parser");
    let ast = parser.parse().expect("parse failed");
    let ir_program = edge_ir::lower_and_optimize(&ast, ir_opt_level, optimize_for)
        .expect("IR optimization failed");
    edge_codegen::compile(&ir_program, bytecode_opt_level, optimize_for).expect("codegen failed")
}

// ═══════════════════════════════════════════════════════════════════════════
// ABI helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Compute the 4-byte function selector from a signature like "transfer(address,uint256)".
pub fn fn_selector(sig: &str) -> [u8; 4] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

/// ABI-encode a single U256 value (left-padded to 32 bytes).
pub fn abi_encode_u256(val: U256) -> Vec<u8> {
    val.to_be_bytes::<32>().to_vec()
}

/// ABI-encode an address (left-padded to 32 bytes).
pub fn abi_encode_address(addr: Address) -> Vec<u8> {
    let mut buf = [0u8; 32];
    buf[12..].copy_from_slice(addr.as_slice());
    buf.to_vec()
}

/// Decode a U256 from ABI-encoded output (first 32 bytes).
pub fn abi_decode_u256(data: &[u8]) -> U256 {
    if data.len() < 32 {
        return U256::ZERO;
    }
    U256::from_be_slice(&data[..32])
}

/// Decode a bool from ABI-encoded output.
pub fn abi_decode_bool(data: &[u8]) -> bool {
    abi_decode_u256(data) == U256::from(1)
}

// ═══════════════════════════════════════════════════════════════════════════
// Gas helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Compute the EVM calldata intrinsic gas cost.
///
/// Per the yellow paper: 4 gas per zero byte, 16 gas per nonzero byte.
pub fn calldata_intrinsic_gas(selector: [u8; 4], args: &[u8]) -> u64 {
    let mut cost = 0u64;
    for &b in selector.iter().chain(args.iter()) {
        cost += if b == 0 { 4 } else { 16 };
    }
    cost
}

/// Compute the pure execution gas cost from a call result.
///
/// Strips the 21000 base transaction cost and calldata intrinsic cost.
pub fn execution_gas(gas_used: u64, selector: [u8; 4], args: &[u8]) -> u64 {
    gas_used
        .saturating_sub(21000)
        .saturating_sub(calldata_intrinsic_gas(selector, args))
}
