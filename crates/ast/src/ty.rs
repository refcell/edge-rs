//! Type Signature AST nodes
//!
//! Defines all type signatures used in Edge expressions and declarations.

use crate::Ident;

/// A data location annotation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Stack location (&s)
    Stack,
    /// Transient storage (&t)
    Transient,
    /// Memory (&m)
    Memory,
    /// Calldata (&cd)
    Calldata,
    /// Return data (&rd)
    Returndata,
    /// Immutable code (&ic)
    ImmutableCode,
    /// External code (&ec)
    ExternalCode,
    /// Dynamic memory (&dm)
    DynamicMemory,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stack => write!(f, "&s"),
            Self::Transient => write!(f, "&t"),
            Self::Memory => write!(f, "&m"),
            Self::Calldata => write!(f, "&cd"),
            Self::Returndata => write!(f, "&rd"),
            Self::ImmutableCode => write!(f, "&ic"),
            Self::ExternalCode => write!(f, "&ec"),
            Self::DynamicMemory => write!(f, "&dm"),
        }
    }
}

/// Primitive data types (EVM-based)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveType {
    /// Unsigned integer: u8, u16, ..., u256 (size in bits, multiple of 8)
    UInt(u16),
    /// Signed integer: i8, i16, ..., i256 (size in bits, multiple of 8)
    Int(u16),
    /// Fixed bytes: b1, b2, ..., b32 (size in bytes)
    FixedBytes(u8),
    /// Address type (addr)
    Address,
    /// Boolean type (bool)
    Bool,
    /// Single bit type (bit)
    Bit,
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UInt(n) => write!(f, "u{n}"),
            Self::Int(n) => write!(f, "i{n}"),
            Self::FixedBytes(n) => write!(f, "b{n}"),
            Self::Address => write!(f, "addr"),
            Self::Bool => write!(f, "bool"),
            Self::Bit => write!(f, "bit"),
        }
    }
}

/// A full type signature
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSig {
    /// Primitive type
    Primitive(PrimitiveType),
    /// Array type: [T; N]
    Array(Box<Self>, Box<crate::Expr>),
    /// Packed array: packed [T; N]
    PackedArray(Box<Self>, Box<crate::Expr>),
    /// Struct type: { field: T, ... }
    Struct(Vec<StructField>),
    /// Packed struct type
    PackedStruct(Vec<StructField>),
    /// Tuple type: (T, T, ...)
    Tuple(Vec<Self>),
    /// Packed tuple type
    PackedTuple(Vec<Self>),
    /// Union/Sum type: A | B(T) | ...
    Union(Vec<UnionMember>),
    /// Function type: T -> U
    Function(Box<Self>, Box<Self>),
    /// Named type (possibly with type parameters): `MyType`<T, U>
    Named(Ident, Vec<Self>),
    /// Pointer type: &location ptr
    Pointer(Location, Box<Self>),
    /// Event type: `[anon] event { ... }`
    Event(bool, Vec<EventField>),
}

/// A struct field with name and type
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// Field name
    pub name: Ident,
    /// Field type
    pub ty: TypeSig,
}

/// A union/sum type member
#[derive(Debug, Clone, PartialEq)]
pub struct UnionMember {
    /// Member name
    pub name: Ident,
    /// Optional inner type
    pub inner: Option<TypeSig>,
}

/// An event field declaration
#[derive(Debug, Clone, PartialEq)]
pub struct EventField {
    /// Field name
    pub name: Ident,
    /// Whether this field is indexed
    pub indexed: bool,
    /// Field type
    pub ty: TypeSig,
}

/// A type parameter for generics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    /// Parameter name
    pub name: Ident,
    /// Trait constraints on this type parameter
    pub constraints: Vec<Ident>,
}
