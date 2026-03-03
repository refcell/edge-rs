#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all,
)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod expr;
pub mod item;
pub mod lit;
pub mod op;
pub mod pattern;
pub mod stmt;
pub mod ty;

pub use expr::*;
pub use item::*;
pub use lit::*;
pub use op::*;
pub use pattern::*;
pub use stmt::*;
pub use ty::*;

/// A simple string identifier with source location
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    /// The identifier string
    pub name: String,
    /// Source location
    pub span: edge_types::span::Span,
}

/// A top-level program (file)
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Top-level statements
    pub stmts: Vec<crate::stmt::Stmt>,
    /// Source span
    pub span: edge_types::span::Span,
}
