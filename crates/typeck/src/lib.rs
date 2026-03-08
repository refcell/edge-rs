//! Edge Language Type Checker
#![warn(missing_docs)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod abi;
pub mod checker;
pub mod error;

pub use abi::{
    extract_abi, AbiFunctionEntry, AbiEntry, AbiEventEntry, AbiEventParam, AbiParam,
    StateMutability,
};
pub use checker::{CheckedProgram, ConstValue, ContractInfo, FnInfo, StorageLayout, TypeChecker};
pub use error::TypeCheckError;
