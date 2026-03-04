//! Token Module
//!
//! Handles the token kinds to represent edge source code.

use std::{fmt, fmt::Write};

use alloy_primitives::B256;
use crate::span::Span;

pub mod keywords;
pub mod locations;
pub mod operators;
pub mod types;

pub use keywords::*;
pub use locations::*;
pub use operators::*;
pub use types::*;

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
    /// An Operator
    Operator(Operator),
    /// A Data Type
    DataType(DataType),
    /// Keyword Identifier
    Keyword(Keyword),
    /// Pointer data location
    Pointer(Location),
    /// A Comment
    Comment(String),
    /// Whitespace
    Whitespace,
    /// A numeric literal
    Literal(B256),
    /// A string literal
    StringLiteral(String),
    /// An Identifier
    Ident(String),
    /// An open parenthesis
    OpenParen,
    /// A close parenthesis
    CloseParen,
    /// An open brace
    OpenBrace,
    /// A close brace
    CloseBrace,
    /// An open bracket
    OpenBracket,
    /// A close bracket
    CloseBracket,
    /// A comma
    Comma,
    /// A colon
    Colon,
    /// A semicolon
    Semicolon,
    /// Arrow (->)
    Arrow,
    /// Fat arrow (=>)
    FatArrow,
    /// Double colon (::)
    DoubleColon,
    /// Dot (.)
    Dot,
    /// Question (?)
    Question,
    /// At (@)
    At,
}

impl TokenKind {
    /// Transform a single char TokenKind into a Token given a single position
    pub fn into_single_span(self, position: u32) -> Token {
        self.into_span(position, position)
    }

    /// Transform a TokenKind into a Token given a start and end position
    pub fn into_span(self, start: u32, end: u32) -> Token {
        Token {
            kind: self,
            span: Span {
                start: start as usize,
                end: end as usize,
                file: None,
            },
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = match self {
            TokenKind::Eof => "EOF",
            TokenKind::Operator(o) => return write!(f, "{o}"),
            TokenKind::DataType(d) => return write!(f, "{d}"),
            TokenKind::Comment(s) => return write!(f, "Comment({s})"),
            TokenKind::Keyword(k) => return write!(f, "{k}"),
            TokenKind::Pointer(l) => return write!(f, "{l}"),
            TokenKind::Literal(l) => {
                let mut s = String::new();
                for b in l.iter() {
                    let _ = write!(&mut s, "{b:02x}");
                }
                return write!(f, "{s}");
            }
            TokenKind::StringLiteral(s) => return write!(f, "\"{s}\""),
            TokenKind::Whitespace => " ",
            TokenKind::Ident(s) => return write!(f, "{s}"),
            TokenKind::OpenBrace => "{",
            TokenKind::CloseBrace => "}",
            TokenKind::OpenParen => "(",
            TokenKind::CloseParen => ")",
            TokenKind::OpenBracket => "[",
            TokenKind::CloseBracket => "]",
            TokenKind::Comma => ",",
            TokenKind::Colon => ":",
            TokenKind::Semicolon => ";",
            TokenKind::Arrow => "->",
            TokenKind::FatArrow => "=>",
            TokenKind::DoubleColon => "::",
            TokenKind::Dot => ".",
            TokenKind::Question => "?",
            TokenKind::At => "@",
        };
        write!(f, "{x}")
    }
}
