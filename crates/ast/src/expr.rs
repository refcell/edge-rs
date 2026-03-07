//! Expression AST nodes
//!
//! Defines all expression types used in the Edge language.

use edge_types::span::Span;

use crate::{
    lit::Lit,
    op::{BinOp, UnaryOp},
    pattern::UnionPattern,
    ty::Location,
    Ident,
};

/// An expression that produces a value
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value
    Literal(Box<Lit>),

    /// Identifier reference
    Ident(Ident),

    /// Binary operation: left op right
    Binary(Box<Self>, BinOp, Box<Self>, Span),

    /// Unary operation: op expr
    Unary(UnaryOp, Box<Self>, Span),

    /// Ternary operation: cond ? `true_expr` : `false_expr`
    Ternary(Box<Self>, Box<Self>, Box<Self>, Span),

    /// Function call: `func(args...)` or `func::<T, U>(args...)`
    /// The `Vec<TypeSig>` holds explicit type arguments (turbofish), empty if none.
    FunctionCall(Box<Self>, Vec<Self>, Vec<crate::ty::TypeSig>, Span),

    /// Field access: expr.field
    FieldAccess(Box<Self>, Ident, Span),

    /// Tuple field access: expr.0
    TupleFieldAccess(Box<Self>, u64, Span),

    /// Array indexing: `arr[index]` or `arr[start:end]`
    ArrayIndex(Box<Self>, Box<Self>, Option<Box<Self>>, Span),

    /// Struct instantiation: `MyStruct` { field: value, ... }
    StructInstantiation(Option<Location>, Ident, Vec<(Ident, Self)>, Span),

    /// Tuple instantiation: (expr, expr, ...)
    TupleInstantiation(Option<Location>, Vec<Self>, Span),

    /// Array instantiation: [expr, expr, ...]
    ArrayInstantiation(Option<Location>, Vec<Self>, Span),

    /// Union instantiation: `Type::Variant(expr`, ...)
    UnionInstantiation(Ident, Ident, Vec<Self>, Span),

    /// Pattern match: expr matches `Type::Variant`
    PatternMatch(Box<Self>, UnionPattern, Span),

    /// Arrow function: x => { ... } or (x, y) => { ... }
    ArrowFunction(Vec<Ident>, Box<crate::stmt::CodeBlock>, Span),

    /// Parenthesized expression: (expr)
    Paren(Box<Self>, Span),

    /// Compile-time expression: comptime(expr)
    Comptime(Box<Self>, Span),

    /// Path expression: `a::b::c`
    Path(Vec<Ident>, Span),

    /// Builtin call: @builtin(args...)
    At(Ident, Vec<Self>, Span),

    /// Assignment: lhs = rhs
    Assign(Box<Self>, Box<Self>, Span),
}

impl Expr {
    /// Get the span of this expression
    #[allow(clippy::match_same_arms)]
    pub fn span(&self) -> Span {
        match self {
            Self::Literal(lit) => lit.span(),
            Self::Ident(id) => id.span.clone(),
            Self::Binary(_, _, _, span) => span.clone(),
            Self::Unary(_, _, span) => span.clone(),
            Self::Ternary(_, _, _, span) => span.clone(),
            Self::FunctionCall(_, _, _, span) => span.clone(),
            Self::FieldAccess(_, _, span) => span.clone(),
            Self::TupleFieldAccess(_, _, span) => span.clone(),
            Self::ArrayIndex(_, _, _, span) => span.clone(),
            Self::StructInstantiation(_, _, _, span) => span.clone(),
            Self::TupleInstantiation(_, _, span) => span.clone(),
            Self::ArrayInstantiation(_, _, span) => span.clone(),
            Self::UnionInstantiation(_, _, _, span) => span.clone(),
            Self::PatternMatch(_, _, span) => span.clone(),
            Self::ArrowFunction(_, _, span) => span.clone(),
            Self::Paren(_, span) => span.clone(),
            Self::Comptime(_, span) => span.clone(),
            Self::Path(_, span) => span.clone(),
            Self::At(_, _, span) => span.clone(),
            Self::Assign(_, _, span) => span.clone(),
        }
    }
}
