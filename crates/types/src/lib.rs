#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

/// Span contains the logic for handling source code spans.
pub mod span;

/// Module containing all the tokens used to represent edge source code.
pub mod tokens;

/// File source objects and utilities.
pub mod source;

/// Time utilities.
pub mod time;

/// The prelude re-exports commonly used types across various modules in this crate.
pub mod prelude {
    pub use crate::source::*;
    pub use crate::span::*;
    pub use crate::time::*;
    pub use crate::tokens::*;
}
