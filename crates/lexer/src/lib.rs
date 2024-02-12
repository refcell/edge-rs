#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

/// The core lexer module.
pub mod core;

/// Lexer Errors.
pub mod errors;

/// The prelude re-exports commonly used types across various modules in this crate.
pub mod prelude {
    pub use crate::core::*;
    pub use crate::errors::*;
}
