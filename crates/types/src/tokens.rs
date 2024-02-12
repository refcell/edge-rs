use crate::span::Span;
use std::{fmt, fmt::Write};

type Literal = [u8; 32];

/// An Edge Token
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// An associated Span
    pub span: Span,
}

impl Token {
    /// Instantiates a new Token given its [TokenKind] and [Span].
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kind of token
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum TokenKind {
    /// EOF Token
    Eof,
    /// A Comment
    Comment(String),
    /// A Contract Token,
    Contract,
    /// An Identifier
    Ident(String),
    /// An open brace
    OpenBrace,
    /// A close brace
    CloseBrace,
}

impl TokenKind {
    /// Transform a single char TokenKind into a Token given a single position
    pub fn into_single_span(self, position: u32) -> Token {
        self.into_span(position, position)
    }

    /// Transform a TokenKind into a Token given a start and end position
    pub fn into_span(self, start: u32, end: u32) -> Token {
        Token { kind: self, span: Span { start: start as usize, end: end as usize, file: None } }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = match self {
            TokenKind::Eof => "EOF",
            TokenKind::Comment(s) => return write!(f, "Comment({s})"),
            TokenKind::Contract => "contract",
            TokenKind::Ident(s) => return write!(f, "{s}"),
            TokenKind::OpenBrace => "{",
            TokenKind::CloseBrace => "}",
        };
        write!(f, "{x}")
    }
}
