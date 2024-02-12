use edge_types::span::{Span, Spanned};

/// A Lexing Error
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LexicalError {
    /// The kind of error
    pub kind: LexicalErrorKind,
    /// The span where the error occurred
    pub span: Span,
}

impl LexicalError {
    /// Public associated function to instatiate a new LexicalError.
    pub fn new(kind: LexicalErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// A Lexical Error Kind
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LexicalErrorKind {
    /// Unexpected end of file
    UnexpectedEof,
    /// Invalid character
    InvalidCharacter(char),
    /// Invalid Array Size
    /// String param expected to be usize parsable
    InvalidArraySize(String),
    /// Invalid Primitive EVM Type
    InvalidPrimitiveType(String),
}

impl Spanned for LexicalError {
    fn span(&self) -> Span {
        self.span.clone()
    }
}
