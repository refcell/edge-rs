//! Literal AST nodes
//!
//! Defines literal values that appear in source code.

use crate::ty::PrimitiveType;
use edge_types::span::Span;

/// A literal value
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// Integer literal: 42, 42u8, etc.
    Int(u64, Option<PrimitiveType>, Span),
    /// String literal: "hello"
    Str(String, Span),
    /// Boolean literal: true or false
    Bool(bool, Span),
    /// Hexadecimal bytes: 0xDEADBEEF
    Hex(Vec<u8>, Span),
    /// Binary bytes: 0b10101010
    Bin(Vec<u8>, Span),
}

impl Lit {
    /// Get the span of this literal
    pub fn span(&self) -> Span {
        match self {
            Lit::Int(_, _, span) => span.clone(),
            Lit::Str(_, span) => span.clone(),
            Lit::Bool(_, span) => span.clone(),
            Lit::Hex(_, span) => span.clone(),
            Lit::Bin(_, span) => span.clone(),
        }
    }
}
