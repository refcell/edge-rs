//! Data Types

use derive_more::Display;

use super::locations::Location;

/// EVM primitive types per Edge specification
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Display)]
pub enum PrimitiveType {
    /// Unsigned integer: u8, u16, ..., u256 (size in bits, multiple of 8)
    #[display("u{_0}")]
    UInt(u16),
    /// Signed integer: i8, i16, ..., i256 (size in bits, multiple of 8)
    #[display("i{_0}")]
    Int(u16),
    /// Fixed bytes: b1, b2, ..., b32 (size in bytes)
    #[display("b{_0}")]
    FixedBytes(u8),
    /// Address type: 160-bit Ethereum address
    #[display("addr")]
    Address,
    /// Boolean type
    #[display("bool")]
    Bool,
    /// Single bit type
    #[display("bit")]
    Bit,
    /// Pointer type annotated with data location
    #[display("{_0} ptr")]
    Pointer(Location),
}

/// A kind of data type
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Display)]
pub enum DataType {
    /// An EVM primitive type
    #[display("{_0}")]
    Primitive(PrimitiveType),
    /// An unknown data type
    #[display("unknown")]
    Unknown,
}
