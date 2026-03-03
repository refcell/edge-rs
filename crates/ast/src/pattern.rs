//! Pattern matching AST nodes
//!
//! Defines patterns used in match statements and pattern matching expressions.

use crate::Ident;
use edge_types::span::Span;

/// A union pattern for matching
#[derive(Debug, Clone, PartialEq)]
pub struct UnionPattern {
    /// Union type name
    pub union_name: Ident,
    /// Member name within the union
    pub member_name: Ident,
    /// Bindings for inner values
    pub bindings: Vec<Ident>,
    /// Source span
    pub span: Span,
}

/// A single match arm pattern
#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    /// Union variant pattern
    Union(UnionPattern),
    /// Identifier pattern (binds a value)
    Ident(Ident),
    /// Wildcard pattern (_)
    Wildcard,
}

impl MatchPattern {
    /// Get the span of this pattern
    pub fn span(&self) -> Span {
        match self {
            MatchPattern::Union(p) => p.span.clone(),
            MatchPattern::Ident(id) => id.span.clone(),
            MatchPattern::Wildcard => Span::EOF,
        }
    }
}

/// A single arm in a match statement
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// The pattern to match
    pub pattern: MatchPattern,
    /// The code block to execute
    pub body: crate::stmt::CodeBlock,
}
