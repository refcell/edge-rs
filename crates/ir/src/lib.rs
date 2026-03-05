//! Edge Language IR and Lowering
#![warn(missing_docs)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod instruction;
pub mod lower;
pub mod program;

pub use instruction::IrInstruction;
pub use lower::{EventMeta, FnMeta, LowerError, Lowerer, StorageSlots};
pub use program::{IrContract, IrFunction, IrProgram};
