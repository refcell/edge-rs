//! Data Types

use std::fmt;

/// A kind of data type
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum DataType {
    /// A 32-bit signed integer
    I32,
    /// A 64-bit signed integer
    I64,
    /// A 32-bit unsigned integer
    U32,
    /// A 64-bit unsigned integer
    U64,
    /// A 32-bit floating point number
    F32,
    /// A 64-bit floating point number
    F64,
    /// A boolean
    Bool,
    /// A character
    Char,
    /// A string
    String,
    /// A byte
    Byte,
    /// A pointer
    Pointer(Box<DataType>),
    /// A function
    Function(Vec<DataType>, Box<DataType>),
    /// A tuple
    Tuple(Vec<DataType>),
    /// A struct
    Struct(Vec<(String, DataType)>),
    /// An array
    Array(Box<DataType>, usize),
    /// A slice
    Slice(Box<DataType>),
    /// A reference
    Reference(Box<DataType>),
    /// A unit
    Unit,
    /// An unknown data type
    Unknown,
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::I32 => write!(f, "i32"),
            DataType::I64 => write!(f, "i64"),
            DataType::U32 => write!(f, "u32"),
            DataType::U64 => write!(f, "u64"),
            DataType::F32 => write!(f, "f32"),
            DataType::F64 => write!(f, "f64"),
            DataType::Bool => write!(f, "bool"),
            DataType::Char => write!(f, "char"),
            DataType::String => write!(f, "string"),
            DataType::Byte => write!(f, "byte"),
            DataType::Pointer(t) => write!(f, "*{}", t),
            DataType::Function(args, ret) => {
                write!(f, "fn(")?;
                for (i, arg) in args.iter().enumerate() {
                    write!(f, "{}", arg)?;
                    if i < args.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ") -> {}", ret)
            }
            DataType::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    write!(f, "{}", t)?;
                    if i < types.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")
            }
            DataType::Struct(fields) => {
                write!(f, "{{")?;
                for (i, (name, t)) in fields.iter().enumerate() {
                    write!(f, "{}: {}", name, t)?;
                    if i < fields.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "}}")
            }
            DataType::Array(t, len) => write!(f, "[{}; {}]", t, len),
            DataType::Slice(t) => write!(f, "[{}]", t),
            DataType::Reference(t) => write!(f, "&{}", t),
            DataType::Unit => write!(f, "()"),
            DataType::Unknown => write!(f, "unknown"),
        }
    }
}
