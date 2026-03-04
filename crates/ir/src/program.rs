//! IR program structure

use crate::instruction::IrInstruction;

/// A function in the IR
#[derive(Debug, Clone)]
pub struct IrFunction {
    /// Function name
    pub name: String,
    /// 4-byte ABI selector
    pub selector: [u8; 4],
    /// Whether publicly callable
    pub is_pub: bool,
    /// Instructions in the function body
    pub body: Vec<IrInstruction>,
    /// Number of bytes of memory needed for local variables
    pub local_mem_size: u64,
}

/// A contract in the IR
#[derive(Debug, Clone)]
pub struct IrContract {
    /// Contract name
    pub name: String,
    /// All functions (public and private)
    pub functions: Vec<IrFunction>,
}

/// The full IR program
#[derive(Debug, Clone)]
pub struct IrProgram {
    /// All contracts
    pub contracts: Vec<IrContract>,
}
