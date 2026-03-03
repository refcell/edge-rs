//! Data Types

use super::locations::Location;
use std::fmt;

/// EVM primitive types per Edge specification
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    /// Unsigned integer: u8, u16, ..., u256 (size in bits, multiple of 8)
    UInt(u16),
    /// Signed integer: i8, i16, ..., i256 (size in bits, multiple of 8)
    Int(u16),
    /// Fixed bytes: b1, b2, ..., b32 (size in bytes)
    FixedBytes(u8),
    /// Address type: 160-bit Ethereum address
    Address,
    /// Boolean type
    Bool,
    /// Single bit type
    Bit,
    /// Pointer type annotated with data location
    Pointer(Location),
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::UInt(n) => write!(f, "u{n}"),
            PrimitiveType::Int(n) => write!(f, "i{n}"),
            PrimitiveType::FixedBytes(n) => write!(f, "b{n}"),
            PrimitiveType::Address => write!(f, "addr"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Bit => write!(f, "bit"),
            PrimitiveType::Pointer(loc) => write!(f, "{} ptr", loc),
        }
    }
}

/// A kind of data type
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum DataType {
    /// An EVM primitive type
    Primitive(PrimitiveType),
    /// An unknown data type
    Unknown,
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Primitive(p) => write!(f, "{}", p),
            DataType::Unknown => write!(f, "unknown"),
        }
    }
}
