//! Item AST nodes
//!
//! Defines top-level items like function declarations, type declarations, etc.

use edge_types::span::Span;

use crate::{ty::TypeParam, Ident};

/// A function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    /// Function name
    pub name: Ident,
    /// Type parameters
    pub type_params: Vec<TypeParam>,
    /// Function parameters: (name, type)
    pub params: Vec<(Ident, crate::ty::TypeSig)>,
    /// Return types
    pub returns: Vec<crate::ty::TypeSig>,
    /// Whether function is public
    pub is_pub: bool,
    /// Whether function is external (ext)
    pub is_ext: bool,
    /// Whether function is mutable (mut)
    pub is_mut: bool,
    /// Source span
    pub span: Span,
}

/// A type declaration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDecl {
    /// Type name
    pub name: Ident,
    /// Type parameters
    pub type_params: Vec<TypeParam>,
    /// Whether type is public
    pub is_pub: bool,
    /// Source span
    pub span: Span,
}

/// A trait declaration
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    /// Trait name
    pub name: Ident,
    /// Type parameters
    pub type_params: Vec<TypeParam>,
    /// Supertraits (trait constraints)
    pub supertraits: Vec<Ident>,
    /// Trait items (functions, types, constants)
    pub items: Vec<TraitItem>,
    /// Whether trait is public
    pub is_pub: bool,
    /// Source span
    pub span: Span,
}

/// An item within a trait
#[derive(Debug, Clone, PartialEq)]
pub enum TraitItem {
    /// Type declaration
    TypeDecl(TypeDecl),
    /// Type assignment
    TypeAssign(TypeDecl, crate::ty::TypeSig),
    /// Constant declaration
    ConstDecl(ConstDecl),
    /// Constant assignment
    ConstAssign(ConstDecl, crate::Expr),
    /// Function declaration
    FnDecl(FnDecl),
    /// Function assignment
    FnAssign(FnDecl, crate::stmt::CodeBlock),
}

/// An implementation block
#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    /// Type being implemented
    pub ty_name: Ident,
    /// Type parameters
    pub type_params: Vec<TypeParam>,
    /// Optional trait being implemented
    pub trait_impl: Option<(Ident, Vec<TypeParam>)>,
    /// Implementation items
    pub items: Vec<ImplItem>,
    /// Source span
    pub span: Span,
}

/// An item within an impl block
#[derive(Debug, Clone, PartialEq)]
pub enum ImplItem {
    /// Function assignment
    FnAssign(FnDecl, crate::stmt::CodeBlock),
    /// Constant assignment
    ConstAssign(ConstDecl, crate::Expr),
    /// Type assignment
    TypeAssign(TypeDecl, crate::ty::TypeSig),
}

/// An ABI declaration
#[derive(Debug, Clone, PartialEq)]
pub struct AbiDecl {
    /// ABI name
    pub name: Ident,
    /// Supertrait ABIs (subtyping)
    pub superabis: Vec<Ident>,
    /// Function declarations
    pub functions: Vec<AbiFnDecl>,
    /// Source span
    pub span: Span,
}

/// A function declaration within an ABI
#[derive(Debug, Clone, PartialEq)]
pub struct AbiFnDecl {
    /// Function name
    pub name: Ident,
    /// Parameters
    pub params: Vec<(Ident, crate::ty::TypeSig)>,
    /// Return types
    pub returns: Vec<crate::ty::TypeSig>,
    /// Whether function is mutable
    pub is_mut: bool,
    /// Source span
    pub span: Span,
}

/// A contract declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ContractDecl {
    /// Contract name
    pub name: Ident,
    /// Contract fields (storage layout)
    pub fields: Vec<(Ident, crate::ty::TypeSig)>,
    /// Contract constants
    pub consts: Vec<(ConstDecl, crate::Expr)>,
    /// Contract functions
    pub functions: Vec<ContractFnDecl>,
    /// Source span
    pub span: Span,
}

/// A contract implementation block
#[derive(Debug, Clone, PartialEq)]
pub struct ContractImpl {
    /// Contract type name
    pub contract_name: Ident,
    /// Optional ABI being implemented
    pub abi_impl: Option<Ident>,
    /// Contract functions
    pub functions: Vec<ContractFnDecl>,
    /// Source span
    pub span: Span,
}

/// A function within a contract implementation
#[derive(Debug, Clone, PartialEq)]
pub struct ContractFnDecl {
    /// Function name
    pub name: Ident,
    /// Parameters
    pub params: Vec<(Ident, crate::ty::TypeSig)>,
    /// Return types
    pub returns: Vec<crate::ty::TypeSig>,
    /// Whether function is external
    pub is_ext: bool,
    /// Whether function is public
    pub is_pub: bool,
    /// Whether function is mutable
    pub is_mut: bool,
    /// Function body (if defined inline in the contract)
    pub body: Option<crate::stmt::CodeBlock>,
    /// Source span
    pub span: Span,
}

/// A constant declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    /// Constant name
    pub name: Ident,
    /// Constant type (optional)
    pub ty: Option<crate::ty::TypeSig>,
    /// Source span
    pub span: Span,
}

/// A module declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    /// Module name
    pub name: Ident,
    /// Whether module is public
    pub is_pub: bool,
    /// Module documentation
    pub doc: Option<String>,
    /// Module items
    pub items: Vec<crate::stmt::Stmt>,
    /// Source span
    pub span: Span,
}

/// A module import path component
#[derive(Debug, Clone, PartialEq)]
pub enum ImportPath {
    /// Single identifier
    Ident(Ident),
    /// Nested import: { a, b, ... }
    Nested(Vec<Self>),
    /// All imports: *
    All,
}

/// A module import statement
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleImport {
    /// Module root
    pub root: Ident,
    /// Import path
    pub path: Option<ImportPath>,
    /// Whether import is public
    pub is_pub: bool,
    /// Source span
    pub span: Span,
}

/// An event declaration
#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    /// Event name
    pub name: Ident,
    /// Whether event is anonymous
    pub is_anon: bool,
    /// Event fields
    pub fields: Vec<crate::ty::EventField>,
    /// Source span
    pub span: Span,
}
