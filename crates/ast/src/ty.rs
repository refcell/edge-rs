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
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Stack => write!(f, "&s"),
            Location::Transient => write!(f, "&t"),
            Location::Memory => write!(f, "&m"),
            Location::Calldata => write!(f, "&cd"),
            Location::Returndata => write!(f, "&rd"),
            Location::ImmutableCode => write!(f, "&ic"),
            Location::ExternalCode => write!(f, "&ec"),
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
            PrimitiveType::UInt(n) => write!(f, "u{}", n),
            PrimitiveType::Int(n) => write!(f, "i{}", n),
            PrimitiveType::FixedBytes(n) => write!(f, "b{}", n),
            PrimitiveType::Address => write!(f, "addr"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Bit => write!(f, "bit"),
        }
    }
}

/// A full type signature
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSig {
    /// Primitive type
    Primitive(PrimitiveType),
    /// Array type: [T; N]
    Array(Box<TypeSig>, Box<crate::Expr>),
    /// Packed array: packed [T; N]
    PackedArray(Box<TypeSig>, Box<crate::Expr>),
    /// Struct type: { field: T, ... }
    Struct(Vec<StructField>),
    /// Packed struct type
    PackedStruct(Vec<StructField>),
    /// Tuple type: (T, T, ...)
    Tuple(Vec<TypeSig>),
    /// Packed tuple type
    PackedTuple(Vec<TypeSig>),
    /// Union/Sum type: A | B(T) | ...
    Union(Vec<UnionMember>),
    /// Function type: T -> U
    Function(Box<TypeSig>, Box<TypeSig>),
    /// Named type (possibly with type parameters): MyType<T, U>
    Named(Ident, Vec<TypeSig>),
    /// Pointer type: &location ptr
    Pointer(Location, Box<TypeSig>),
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
