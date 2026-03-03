//! Expression AST nodes
//!
//! Defines all expression types used in the Edge language.

use crate::lit::Lit;
use crate::op::{BinOp, UnaryOp};
use crate::pattern::UnionPattern;
use crate::ty::Location;
use crate::Ident;
use edge_types::span::Span;

/// An expression that produces a value
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value
    Literal(Box<Lit>),

    /// Identifier reference
    Ident(Ident),

    /// Binary operation: left op right
    Binary(Box<Expr>, BinOp, Box<Expr>, Span),

    /// Unary operation: op expr
    Unary(UnaryOp, Box<Expr>, Span),

    /// Ternary operation: cond ? true_expr : false_expr
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>, Span),

    /// Function call: func(args...)
    FunctionCall(Box<Expr>, Vec<Expr>, Span),

    /// Field access: expr.field
    FieldAccess(Box<Expr>, Ident, Span),

    /// Tuple field access: expr.0
    TupleFieldAccess(Box<Expr>, u64, Span),

    /// Array indexing: `arr[index]` or `arr[start:end]`
    ArrayIndex(Box<Expr>, Box<Expr>, Option<Box<Expr>>, Span),

    /// Struct instantiation: MyStruct { field: value, ... }
    StructInstantiation(Option<Location>, Ident, Vec<(Ident, Expr)>, Span),

    /// Tuple instantiation: (expr, expr, ...)
    TupleInstantiation(Option<Location>, Vec<Expr>, Span),

    /// Array instantiation: [expr, expr, ...]
    ArrayInstantiation(Option<Location>, Vec<Expr>, Span),

    /// Union instantiation: Type::Variant(expr, ...)
    UnionInstantiation(Ident, Ident, Vec<Expr>, Span),

    /// Pattern match: expr matches Type::Variant
    PatternMatch(Box<Expr>, UnionPattern, Span),

    /// Arrow function: x => { ... } or (x, y) => { ... }
    ArrowFunction(Vec<Ident>, Box<crate::stmt::CodeBlock>, Span),

    /// Parenthesized expression: (expr)
    Paren(Box<Expr>, Span),

    /// Compile-time expression: comptime(expr)
    Comptime(Box<Expr>, Span),

    /// Path expression: a::b::c
    Path(Vec<Ident>, Span),

    /// Builtin call: @builtin(args...)
    At(Ident, Vec<Expr>, Span),

    /// Assignment: lhs = rhs
    Assign(Box<Expr>, Box<Expr>, Span),
}

impl Expr {
    /// Get the span of this expression
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(lit) => lit.span(),
            Expr::Ident(id) => id.span.clone(),
            Expr::Binary(_, _, _, span) => span.clone(),
            Expr::Unary(_, _, span) => span.clone(),
            Expr::Ternary(_, _, _, span) => span.clone(),
            Expr::FunctionCall(_, _, span) => span.clone(),
            Expr::FieldAccess(_, _, span) => span.clone(),
            Expr::TupleFieldAccess(_, _, span) => span.clone(),
            Expr::ArrayIndex(_, _, _, span) => span.clone(),
            Expr::StructInstantiation(_, _, _, span) => span.clone(),
            Expr::TupleInstantiation(_, _, span) => span.clone(),
            Expr::ArrayInstantiation(_, _, span) => span.clone(),
            Expr::UnionInstantiation(_, _, _, span) => span.clone(),
            Expr::PatternMatch(_, _, span) => span.clone(),
            Expr::ArrowFunction(_, _, span) => span.clone(),
            Expr::Paren(_, span) => span.clone(),
            Expr::Comptime(_, span) => span.clone(),
            Expr::Path(_, span) => span.clone(),
            Expr::At(_, _, span) => span.clone(),
            Expr::Assign(_, _, span) => span.clone(),
        }
    }
}
