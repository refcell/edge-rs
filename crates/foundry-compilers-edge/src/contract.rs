//! Contract artifact output for compiled Edge files.

use alloy_json_abi::JsonAbi;
use foundry_compilers::{artifacts::BytecodeObject, CompilerContract};

/// A compiled Edge contract artifact.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EdgeCompilerContract {
    /// The contract ABI.
    pub abi: Option<JsonAbi>,
    /// The deployed bytecode.
    pub bytecode: Option<BytecodeObject>,
    /// The runtime bytecode.
    pub runtime_bytecode: Option<BytecodeObject>,
}

impl CompilerContract for EdgeCompilerContract {
    fn abi_ref(&self) -> Option<&JsonAbi> {
        self.abi.as_ref()
    }

    fn bin_ref(&self) -> Option<&BytecodeObject> {
        self.bytecode.as_ref()
    }

    fn bin_runtime_ref(&self) -> Option<&BytecodeObject> {
        self.runtime_bytecode.as_ref()
    }
}
