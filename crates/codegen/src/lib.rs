//! Edge Language EVM Bytecode Emitter
#![warn(missing_docs)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod gen;
pub mod opcode;

pub use gen::{CodeGenError, CodeGenerator, ContractInput, FunctionInput, GenInput, GenInstr};
pub use opcode::Opcode;
