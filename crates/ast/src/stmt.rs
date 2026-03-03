//! Statement AST nodes
//!
//! Defines all statement types used in the Edge language.

use edge_types::span::Span;

use crate::{
    item::{
        AbiDecl, ConstDecl, ContractDecl, ContractImpl, EventDecl, FnDecl, ImplBlock, ModuleDecl,
        ModuleImport, TraitDecl, TypeDecl,
    },
    pattern::MatchArm,
    Ident,
};

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

    /// Function assignment: fn `name()` { ... }
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

    /// Module import: use `path::to::item`
    ModuleImport(ModuleImport),

    /// Core loop: loop { ... }
    Loop(LoopBlock),

    /// For loop: for (init; cond; update) { ... }
    ForLoop(
        Option<Box<Self>>,
        Option<crate::Expr>,
        Option<Box<Self>>,
        LoopBlock,
    ),

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
    ComptimeBranch(Box<Self>),

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

    /// Emit statement: emit EventName(args...)
    Emit(Ident, Vec<crate::Expr>, Span),
}

impl Stmt {
    /// Get the span of this statement
    #[allow(clippy::match_same_arms)]
    pub fn span(&self) -> Span {
        match self {
            Self::VarDecl(_, _, span) => span.clone(),
            Self::VarAssign(_, _, span) => span.clone(),
            Self::TypeAssign(_, _, span) => span.clone(),
            Self::TraitDecl(_, span) => span.clone(),
            Self::ImplBlock(item) => item.span.clone(),
            Self::FnAssign(fn_decl, _) => fn_decl.span.clone(),
            Self::AbiDecl(abi) => abi.span.clone(),
            Self::ContractDecl(contract) => contract.span.clone(),
            Self::ContractImpl(impl_block) => impl_block.span.clone(),
            Self::ConstAssign(_, _, span) => span.clone(),
            Self::ModuleDecl(module) => module.span.clone(),
            Self::ModuleImport(import) => import.span.clone(),
            Self::Loop(block) => block.span.clone(),
            Self::ForLoop(_, _, _, block) => block.span.clone(),
            Self::WhileLoop(_, block) => block.span.clone(),
            Self::DoWhile(block, _) => block.span.clone(),
            Self::CodeBlock(block) => block.span.clone(),
            Self::IfElse(conditions, else_block) => {
                if let Some((_, block)) = conditions.first() {
                    block.span.clone()
                } else if let Some(block) = else_block {
                    block.span.clone()
                } else {
                    Span::EOF
                }
            }
            Self::IfMatch(_, _, block) => block.span.clone(),
            Self::Match(_, _) => Span::EOF, // TODO: store span in Match
            Self::ComptimeBranch(stmt) => stmt.span(),
            Self::ComptimeFn(fn_decl, _) => fn_decl.span.clone(),
            Self::Return(_, span) => span.clone(),
            Self::Break(span) => span.clone(),
            Self::Continue(span) => span.clone(),
            Self::Expr(expr) => expr.span(),
            Self::EventDecl(event) => event.span.clone(),
            Self::Emit(_, _, span) => span.clone(),
        }
    }
}
