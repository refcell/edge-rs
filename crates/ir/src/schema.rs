//! Rust mirror of `schema.egg` — pure data definitions for the EVM IR.
//!
//! Every type here corresponds 1:1 with an egglog datatype or constructor
//! defined in `schema.egg`. This module contains **only** data definitions
//! and display impls — no conversion logic.

use std::rc::Rc;

/// Reference-counted expression for sharing subexpressions in the IR DAG.
pub type RcExpr = Rc<EvmExpr>;

// ============================================================
// Types
// ============================================================

/// EVM base types (primitives).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EvmBaseType {
    /// Unsigned integer with bit width (8, 16, ..., 256)
    UIntT(u16),
    /// Signed integer with bit width (8, 16, ..., 256)
    IntT(u16),
    /// Fixed bytes with byte count (1..32)
    BytesT(u8),
    /// Address (160-bit)
    AddrT,
    /// Boolean
    BoolT,
    /// Unit/void
    UnitT,
    /// State token (for ordering side effects)
    StateT,
}

/// EVM types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EvmType {
    /// A primitive type
    Base(EvmBaseType),
    /// A tuple type (flat — no nested tuples)
    TupleT(Vec<EvmBaseType>),
}

// ============================================================
// Data Locations
// ============================================================

/// Where data lives in the EVM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataLocation {
    /// Persistent storage (SLOAD/SSTORE)
    Storage,
    /// Transient storage (TLOAD/TSTORE)
    Transient,
    /// Memory (MLOAD/MSTORE)
    Memory,
    /// Calldata (CALLDATALOAD)
    Calldata,
    /// Return data buffer
    Returndata,
    /// Stack (default)
    Stack,
}

// ============================================================
// Constants
// ============================================================

/// Constant values in the IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EvmConstant {
    /// Small integers that fit in i64
    SmallInt(i64),
    /// Large integers as hex strings (for full u256 range)
    LargeInt(String),
    /// Boolean constants
    Bool(bool),
    /// Address constants as hex strings
    Addr(String),
}

// ============================================================
// Context / Assumptions
// ============================================================

/// Contextual information for where an expression lives.
/// Used by egglog for context-sensitive optimizations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvmContext {
    /// Inside a named function
    InFunction(String),
    /// Inside a conditional branch (true/false, predicate, input)
    InBranch(bool, RcExpr, RcExpr),
    /// Inside a loop (input, pred_output)
    InLoop(RcExpr, RcExpr),
}

// ============================================================
// Operators
// ============================================================

/// Binary operators (two operands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmBinaryOp {
    // Arithmetic
    /// Addition
    Add,
    /// Subtraction
    Sub,
    /// Multiplication
    Mul,
    /// Unsigned division
    Div,
    /// Signed division
    SDiv,
    /// Unsigned modulo
    Mod,
    /// Signed modulo
    SMod,
    /// Exponentiation
    Exp,

    // Comparison
    /// Unsigned less than
    Lt,
    /// Unsigned greater than
    Gt,
    /// Signed less than
    SLt,
    /// Signed greater than
    SGt,
    /// Equality
    Eq,

    // Bitwise
    /// Bitwise AND
    And,
    /// Bitwise OR
    Or,
    /// Bitwise XOR
    Xor,
    /// Shift left
    Shl,
    /// Logical shift right
    Shr,
    /// Arithmetic shift right
    Sar,
    /// Get byte at position
    Byte,

    // Logical (high-level, lowered during codegen)
    /// Logical AND (short-circuit)
    LogAnd,
    /// Logical OR (short-circuit)
    LogOr,

    // Storage/memory reads (take state, return (value, state))
    /// Storage load: (slot, state) -> (value, state)
    SLoad,
    /// Transient storage load
    TLoad,
    /// Memory load: (offset, state) -> (value, state)
    MLoad,
    /// Calldata load: (offset, state) -> value
    CalldataLoad,
}

/// Unary operators (one operand).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmUnaryOp {
    /// Check if zero (returns 1 if zero, 0 otherwise)
    IsZero,
    /// Bitwise NOT
    Not,
    /// Arithmetic negation (0 - x)
    Neg,
    /// Sign extend
    SignExtend,
}

/// Ternary operators (three operands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmTernaryOp {
    /// Storage write: (slot, value, state) -> state
    SStore,
    /// Transient storage write: (slot, value, state) -> state
    TStore,
    /// Memory write: (offset, value, state) -> state
    MStore,
    /// Memory write byte: (offset, value, state) -> state
    MStore8,
    /// Keccak256 hash: (offset, size, state) -> hash (reads from memory state)
    Keccak256,
    /// Conditional select: (cond, true_val, false_val) -> val
    Select,
}

// ============================================================
// Environment Operations
// ============================================================

/// EVM environment/block context reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmEnvOp {
    /// msg.sender
    Caller,
    /// msg.value
    CallValue,
    /// calldata size in bytes
    CallDataSize,
    /// tx.origin
    Origin,
    /// tx.gasprice
    GasPrice,
    /// block hash of given block number
    BlockHash,
    /// block.coinbase
    Coinbase,
    /// block.timestamp
    Timestamp,
    /// block.number
    Number,
    /// block.gaslimit
    GasLimit,
    /// chain ID
    ChainId,
    /// contract's ETH balance
    SelfBalance,
    /// block.basefee
    BaseFee,
    /// remaining gas
    Gas,
    /// contract's own address
    Address,
    /// balance of given address
    Balance,
    /// contract code size
    CodeSize,
    /// return data size from last call
    ReturnDataSize,
}

// ============================================================
// Expressions
// ============================================================

/// The core IR expression type. Every node in the IR DAG is an `EvmExpr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvmExpr {
    // ---- Leaf nodes ----
    /// Current function's argument (tupled if multiple params)
    Arg(EvmType, EvmContext),
    /// Constant value
    Const(EvmConstant, EvmType, EvmContext),
    /// Empty tuple
    Empty(EvmType, EvmContext),

    // ---- Operators ----
    /// Binary operation
    Bop(EvmBinaryOp, RcExpr, RcExpr),
    /// Unary operation
    Uop(EvmUnaryOp, RcExpr),
    /// Ternary operation
    Top(EvmTernaryOp, RcExpr, RcExpr, RcExpr),

    // ---- Tuple operations ----
    /// Get element from tuple at static index
    Get(RcExpr, usize),
    /// Single-element tuple
    Single(RcExpr),
    /// Concatenate two tuples
    Concat(RcExpr, RcExpr),

    // ---- Control flow ----
    /// If-then-else: (pred, inputs, then_body, else_body)
    If(RcExpr, RcExpr, RcExpr, RcExpr),
    /// Do-while loop: (inputs, pred_and_body)
    DoWhile(RcExpr, RcExpr),

    // ---- EVM environment ----
    /// Nullary environment read: (op, state) -> (value, state)
    EnvRead(EvmEnvOp, RcExpr),
    /// Unary environment read: (op, arg, state) -> (value, state)
    EnvRead1(EvmEnvOp, RcExpr, RcExpr),

    // ---- EVM-specific ----
    /// Log event: (topic_count, topics, data, state) -> state
    Log(usize, Vec<RcExpr>, RcExpr, RcExpr),
    /// Revert: (offset, size, state) -> !
    Revert(RcExpr, RcExpr, RcExpr),
    /// Return: (offset, size, state) -> !
    ReturnOp(RcExpr, RcExpr, RcExpr),
    /// External call: (target, value, args_offset, args_len, ret_offset, ret_len, state)
    ExtCall(RcExpr, RcExpr, RcExpr, RcExpr, RcExpr, RcExpr, RcExpr),
    /// Internal function call: (name, args) -> result
    Call(String, RcExpr),
    /// Function selector constant (4-byte keccak256 of signature)
    Selector(String),

    /// Let binding: compute value once, reference via Var(name) in body
    LetBind(String, RcExpr, RcExpr),
    /// Variable reference to a LetBind
    Var(String),

    // ---- Top-level ----
    /// Function: (name, input_type, output_type, body)
    Function(String, EvmType, EvmType, RcExpr),
    /// Storage field: (name, slot_index, type)
    StorageField(String, usize, EvmType),
}

// ============================================================
// Program-level structures
// ============================================================

/// A compiled contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmContract {
    /// Contract name
    pub name: String,
    /// Storage field definitions (StorageField nodes)
    pub storage_fields: Vec<RcExpr>,
    /// Constructor body
    pub constructor: RcExpr,
    /// Runtime body (dispatcher + function bodies)
    pub runtime: RcExpr,
}

/// A complete program (one or more contracts + free functions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmProgram {
    /// Contracts defined in the program
    pub contracts: Vec<EvmContract>,
    /// Free-standing functions (outside contracts)
    pub free_functions: Vec<RcExpr>,
}

// ============================================================
// Display implementations for debugging
// ============================================================

impl std::fmt::Display for EvmBaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UIntT(n) => write!(f, "u{n}"),
            Self::IntT(n) => write!(f, "i{n}"),
            Self::BytesT(n) => write!(f, "b{n}"),
            Self::AddrT => write!(f, "addr"),
            Self::BoolT => write!(f, "bool"),
            Self::UnitT => write!(f, "unit"),
            Self::StateT => write!(f, "state"),
        }
    }
}

impl std::fmt::Display for EvmBinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Add => "ADD",
            Self::Sub => "SUB",
            Self::Mul => "MUL",
            Self::Div => "DIV",
            Self::SDiv => "SDIV",
            Self::Mod => "MOD",
            Self::SMod => "SMOD",
            Self::Exp => "EXP",
            Self::Lt => "LT",
            Self::Gt => "GT",
            Self::SLt => "SLT",
            Self::SGt => "SGT",
            Self::Eq => "EQ",
            Self::And => "AND",
            Self::Or => "OR",
            Self::Xor => "XOR",
            Self::Shl => "SHL",
            Self::Shr => "SHR",
            Self::Sar => "SAR",
            Self::Byte => "BYTE",
            Self::LogAnd => "LOGAND",
            Self::LogOr => "LOGOR",
            Self::SLoad => "SLOAD",
            Self::TLoad => "TLOAD",
            Self::MLoad => "MLOAD",
            Self::CalldataLoad => "CALLDATALOAD",
        };
        write!(f, "{s}")
    }
}

impl std::fmt::Display for EvmUnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::IsZero => "ISZERO",
            Self::Not => "NOT",
            Self::Neg => "NEG",
            Self::SignExtend => "SIGNEXTEND",
        };
        write!(f, "{s}")
    }
}

impl std::fmt::Display for EvmTernaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::SStore => "SSTORE",
            Self::TStore => "TSTORE",
            Self::MStore => "MSTORE",
            Self::MStore8 => "MSTORE8",
            Self::Keccak256 => "KECCAK256",
            Self::Select => "SELECT",
        };
        write!(f, "{s}")
    }
}

impl std::fmt::Display for EvmEnvOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Caller => "CALLER",
            Self::CallValue => "CALLVALUE",
            Self::CallDataSize => "CALLDATASIZE",
            Self::Origin => "ORIGIN",
            Self::GasPrice => "GASPRICE",
            Self::BlockHash => "BLOCKHASH",
            Self::Coinbase => "COINBASE",
            Self::Timestamp => "TIMESTAMP",
            Self::Number => "NUMBER",
            Self::GasLimit => "GASLIMIT",
            Self::ChainId => "CHAINID",
            Self::SelfBalance => "SELFBALANCE",
            Self::BaseFee => "BASEFEE",
            Self::Gas => "GAS",
            Self::Address => "ADDRESS",
            Self::Balance => "BALANCE",
            Self::CodeSize => "CODESIZE",
            Self::ReturnDataSize => "RETURNDATASIZE",
        };
        write!(f, "{s}")
    }
}
