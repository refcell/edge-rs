//! EVM semantic test harness for the Edge compiler.
//!
//! Provides [`EvmTestHost`] which compiles `.edge` files, deploys the resulting
//! bytecode on an in-memory revm instance, and executes calls against it.

#![allow(missing_docs)]

use std::path::PathBuf;

use alloy_primitives::{Address, Bytes, Log, U256};
use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{
        ExecutionResult, Output, TransactTo,
    },
    Evm,
};

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

/// In-memory EVM test host for semantic testing of Edge contracts.
#[derive(Debug)]
pub struct EvmTestHost {
    db: CacheDB<EmptyDB>,
    contract: Address,
    caller: Address,
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
        DeployResult { host, init_code_size, deploy_gas }
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
        DeployResult { host, init_code_size, deploy_gas }
    }

    /// Deploy raw bytecode and return a test host.
    pub fn deploy_bytecode(bytecode: &[u8]) -> Self {
        Self::deploy_bytecode_measured(bytecode).0
    }

    /// Deploy raw bytecode, returning (host, deploy_gas).
    fn deploy_bytecode_measured(bytecode: &[u8]) -> (Self, u64) {
        let caller = Address::from([0x01; 20]);
        let mut db = CacheDB::new(EmptyDB::default());

        let caller_info = revm::primitives::AccountInfo {
            balance: U256::from(1_000_000_000_000_000_000u128),
            nonce: 0,
            ..Default::default()
        };
        db.insert_account_info(caller, caller_info);

        let result = {
            let mut evm = Evm::builder()
                .with_db(&mut db)
                .modify_tx_env(|tx| {
                    tx.caller = caller;
                    tx.transact_to = TransactTo::Create;
                    tx.data = Bytes::copy_from_slice(bytecode);
                    tx.gas_limit = 10_000_000;
                    tx.gas_price = U256::ZERO;
                })
                .modify_block_env(|blk| {
                    blk.basefee = U256::ZERO;
                })
                .build();
            evm.transact_commit().expect("deploy transaction failed")
        };

        let (contract, deploy_gas) = match result {
            ExecutionResult::Success {
                output: Output::Create(_, Some(addr)),
                gas_used,
                ..
            } => (addr, gas_used),
            other => panic!("Deployment failed: {other:#?}"),
        };

        (Self { db, contract, caller }, deploy_gas)
    }

    /// The deployed contract address.
    pub const fn address(&self) -> Address {
        self.contract
    }

    /// Size of the deployed runtime bytecode in bytes.
    pub fn runtime_code_size(&self) -> usize {
        self.db
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
        let mut evm = Evm::builder()
            .with_db(&mut self.db)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TransactTo::Call(self.contract);
                tx.data = Bytes::copy_from_slice(calldata);
                tx.gas_limit = 10_000_000;
                tx.nonce = None;
                tx.gas_price = U256::ZERO;
            })
            .modify_block_env(|blk| {
                blk.basefee = U256::ZERO;
            })
            .build();

        let result = evm.transact_commit().expect("call transaction failed");

        match result {
            ExecutionResult::Success {
                output: Output::Call(data),
                gas_used,
                logs,
                ..
            } => CallResult {
                success: true,
                output: data.to_vec(),
                gas_used,
                logs,
            },
            ExecutionResult::Revert { output, gas_used, .. } => CallResult {
                success: false,
                output: output.to_vec(),
                gas_used,
                logs: vec![],
            },
            ExecutionResult::Halt { reason, gas_used, .. } => CallResult {
                success: false,
                output: format!("HALT: {reason:?}").into_bytes(),
                gas_used,
                logs: vec![],
            },
            other => panic!("Unexpected call result: {other:#?}"),
        }
    }

    /// Call using a function signature string to compute the selector.
    /// e.g. `call_fn("transfer(address,uint256)", &args)`
    pub fn call_fn(&mut self, sig: &str, args: &[u8]) -> CallResult {
        let selector = fn_selector(sig);
        self.call(selector, args)
    }

    /// Read a storage slot directly from the database.
    pub fn sload(&self, slot: U256) -> U256 {
        use revm::Database;
        let mut db = self.db.clone();
        db.storage(self.contract, slot)
            .expect("failed to read storage")
    }

    /// Set the caller address for subsequent transactions.
    /// Ensures the new caller has account info in the DB.
    pub fn set_caller(&mut self, caller: Address) {
        // Insert account info for the new caller if not already present
        let caller_info = revm::primitives::AccountInfo {
            balance: U256::from(1_000_000_000_000_000_000u128),
            nonce: 0,
            ..Default::default()
        };
        self.db.insert_account_info(caller, caller_info);
        self.caller = caller;
    }
}

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
    edge_codegen::compile(&ir_program, bytecode_opt_level, optimize_for)
        .expect("codegen failed")
}

/// Compute the 4-byte function selector from a signature like "transfer(address,uint256)".
pub fn fn_selector(sig: &str) -> [u8; 4] {
    use alloy_primitives::keccak256;
    let hash = keccak256(sig.as_bytes());
    let mut sel = [0u8; 4];
    sel.copy_from_slice(&hash[..4]);
    sel
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
