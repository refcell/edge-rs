//! Parse Errors

use edge_types::span::Span;
use edge_types::tokens::TokenKind;
use std::fmt;

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Parser errors
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseError {
    /// Unexpected token
    #[error("Unexpected token {found} at {span:?}, expected {expected}")]
    UnexpectedToken {
        /// The token we found
        found: String,
        /// The expected token description
        expected: String,
        /// Source span
        span: Span,
    },

    /// Unexpected end of file
    #[error("Unexpected end of file")]
    UnexpectedEof,

    /// Invalid type signature
    #[error("Invalid type signature at {span:?}: {message}")]
    InvalidTypeSig {
        /// Error message
        message: String,
        /// Source span
        span: Span,
    },

    /// Invalid expression
    #[error("Invalid expression at {span:?}: {message}")]
    InvalidExpr {
        /// Error message
        message: String,
        /// Source span
        span: Span,
    },

    /// Invalid statement
    #[error("Invalid statement at {span:?}: {message}")]
    InvalidStmt {
        /// Error message
        message: String,
        /// Source span
        span: Span,
    },

    /// Invalid pattern
    #[error("Invalid pattern at {span:?}: {message}")]
    InvalidPattern {
        /// Error message
        message: String,
        /// Source span
        span: Span,
    },

    /// Lexer error
    #[error("Lexer error: {0}")]
    LexerError(String),
}

impl ParseError {
    /// Create an "unexpected token" error
    pub fn unexpected(found: impl fmt::Display, expected: impl fmt::Display, span: Span) -> Self {
        Self::UnexpectedToken {
            found: found.to_string(),
            expected: expected.to_string(),
            span,
        }
    }

    /// Get the span of this error
    pub fn span(&self) -> Option<&Span> {
        match self {
            Self::UnexpectedToken { span, .. }
            | Self::InvalidTypeSig { span, .. }
            | Self::InvalidExpr { span, .. }
            | Self::InvalidStmt { span, .. }
            | Self::InvalidPattern { span, .. } => Some(span),
            Self::UnexpectedEof | Self::LexerError(_) => None,
        }
    }
}
