//! Statement AST nodes
//!
//! Defines all statement types used in the Edge language.

use crate::item::*;
use crate::pattern::MatchArm;
use crate::Ident;
use edge_types::span::Span;

/// A code block containing statements
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock {
    /// Statements in the block
    pub stmts: Vec<BlockItem>,
    /// Source span
    pub span: Span,
}

/// An item in a code block (statement or expression)
#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    /// A statement
    Stmt(Box<Stmt>),
    /// An expression
    Expr(crate::Expr),
}

/// A loop block (for loops, while loops, etc.)
#[derive(Debug, Clone, PartialEq)]
pub struct LoopBlock {
    /// Items in the loop body
    pub items: Vec<LoopItem>,
    /// Source span
    pub span: Span,
}

/// An item in a loop block
#[derive(Debug, Clone, PartialEq)]
pub enum LoopItem {
    /// Expression or statement
    Expr(crate::Expr),
    /// Statement
    Stmt(Box<Stmt>),
    /// Break statement
    Break(Span),
    /// Continue statement
    Continue(Span),
}

/// A statement
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Variable declaration: let x: T
    VarDecl(Ident, Option<crate::ty::TypeSig>, Span),

    /// Variable assignment: x = expr
    VarAssign(crate::Expr, crate::Expr, Span),

    /// Type assignment: type X = T
    TypeAssign(TypeDecl, crate::ty::TypeSig, Span),

    /// Trait declaration
    TraitDecl(TraitDecl, Span),

    /// Implementation block
    ImplBlock(ImplBlock),

    /// Function assignment: fn name() { ... }
    FnAssign(FnDecl, CodeBlock),

    /// ABI declaration
    AbiDecl(AbiDecl),

    /// Contract declaration
    ContractDecl(ContractDecl),

    /// Contract implementation block
    ContractImpl(ContractImpl),

    /// Constant assignment: const x = expr
    ConstAssign(ConstDecl, crate::Expr, Span),

    /// Module declaration: mod name { ... }
    ModuleDecl(ModuleDecl),

    /// Module import: use path::to::item
    ModuleImport(ModuleImport),

    /// Core loop: loop { ... }
    Loop(LoopBlock),

    /// For loop: for (init; cond; update) { ... }
    ForLoop(Option<Box<Stmt>>, Option<crate::Expr>, Option<Box<Stmt>>, LoopBlock),

    /// While loop: while (cond) { ... }
    WhileLoop(crate::Expr, LoopBlock),

    /// Do-while loop: do { ... } while (cond)
    DoWhile(LoopBlock, crate::Expr),

    /// Code block
    CodeBlock(CodeBlock),

    /// If/else if/else: if (cond) { ... } else if (cond) { ... } else { ... }
    IfElse(Vec<(crate::Expr, CodeBlock)>, Option<CodeBlock>),

    /// If with pattern match: if expr matches pattern { ... }
    IfMatch(crate::Expr, crate::pattern::UnionPattern, CodeBlock),

    /// Match statement: match expr { ... }
    Match(crate::Expr, Vec<MatchArm>),

    /// Compile-time branch: comptime { ... }
    ComptimeBranch(Box<Stmt>),

    /// Compile-time function: comptime fn ...
    ComptimeFn(FnDecl, CodeBlock),

    /// Return statement: `return [expr]`
    Return(Option<crate::Expr>, Span),

    /// Break statement
    Break(Span),

    /// Continue statement
    Continue(Span),

    /// Expression statement
    Expr(crate::Expr),

    /// Event declaration
    EventDecl(EventDecl),
}

impl Stmt {
    /// Get the span of this statement
    pub fn span(&self) -> Span {
        match self {
            Stmt::VarDecl(_, _, span) => span.clone(),
            Stmt::VarAssign(_, _, span) => span.clone(),
            Stmt::TypeAssign(_, _, span) => span.clone(),
            Stmt::TraitDecl(_, span) => span.clone(),
            Stmt::ImplBlock(item) => item.span.clone(),
            Stmt::FnAssign(fn_decl, _) => fn_decl.span.clone(),
            Stmt::AbiDecl(abi) => abi.span.clone(),
            Stmt::ContractDecl(contract) => contract.span.clone(),
            Stmt::ContractImpl(impl_block) => impl_block.span.clone(),
            Stmt::ConstAssign(_, _, span) => span.clone(),
            Stmt::ModuleDecl(module) => module.span.clone(),
            Stmt::ModuleImport(import) => import.span.clone(),
            Stmt::Loop(block) => block.span.clone(),
            Stmt::ForLoop(_, _, _, block) => block.span.clone(),
            Stmt::WhileLoop(_, block) => block.span.clone(),
            Stmt::DoWhile(block, _) => block.span.clone(),
            Stmt::CodeBlock(block) => block.span.clone(),
            Stmt::IfElse(conditions, else_block) => {
                if let Some((_, block)) = conditions.first() {
                    block.span.clone()
                } else if let Some(block) = else_block {
                    block.span.clone()
                } else {
                    Span::EOF
                }
            }
            Stmt::IfMatch(_, _, block) => block.span.clone(),
            Stmt::Match(_, _) => Span::EOF, // TODO: store span in Match
            Stmt::ComptimeBranch(stmt) => stmt.span(),
            Stmt::ComptimeFn(fn_decl, _) => fn_decl.span.clone(),
            Stmt::Return(_, span) => span.clone(),
            Stmt::Break(span) => span.clone(),
            Stmt::Continue(span) => span.clone(),
            Stmt::Expr(expr) => expr.span(),
            Stmt::EventDecl(event) => event.span.clone(),
        }
    }
}
