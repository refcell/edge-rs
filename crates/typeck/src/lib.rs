//! Edge Language Type Checker
#![warn(missing_docs)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod checker;
pub mod error;

pub use checker::{CheckedProgram, ConstValue, ContractInfo, FnInfo, StorageLayout, TypeChecker};
pub use error::TypeCheckError;
