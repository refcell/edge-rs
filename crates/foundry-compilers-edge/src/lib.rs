//! Foundry compilers plugin for the Edge language.
//!
//! This crate implements the [`foundry_compilers::Compiler`] trait for the Edge
//! language, enabling Foundry to discover, parse, and compile `.edge` source files
//! transparently alongside Solidity contracts.

#![warn(missing_docs)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod compiler;
pub mod contract;
pub mod error;
pub mod input;
pub mod language;
pub mod parser;
pub mod settings;

pub use compiler::EdgeCompiler;
pub use contract::EdgeCompilerContract;
pub use error::EdgeCompilationError;
pub use input::EdgeCompilerInput;
pub use language::EdgeLanguage;
pub use parser::{EdgeParsedSource, EdgeParser};
pub use settings::{EdgeSettings, EdgeSettingsRestrictions};
