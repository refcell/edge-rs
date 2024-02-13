#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod bytes;
pub mod source;
pub mod span;
pub mod time;
pub mod tokens;

/// The prelude re-exports commonly used types across various modules in this crate.
pub mod prelude {
    pub use crate::bytes::*;
    pub use crate::source::*;
    pub use crate::span::*;
    pub use crate::time::*;
    pub use crate::tokens::*;
}
