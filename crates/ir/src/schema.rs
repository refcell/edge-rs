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

impl EvmBaseType {
    /// Returns the bit width of this type for packed struct layout.
    pub const fn bit_width(&self) -> u16 {
        match self {
            Self::UIntT(n) | Self::IntT(n) => *n,
            Self::BytesT(n) => (*n as u16) * 8,
            Self::AddrT => 160,
            Self::BoolT => 8,
            Self::UnitT | Self::StateT => 0,
        }
    }
}

/// EVM types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EvmType {
    /// A primitive type
    Base(EvmBaseType),
    /// A tuple type (flat — no nested tuples)
    TupleT(Vec<EvmBaseType>),
    /// A fixed-size array type: element type + length
    ArrayT(EvmBaseType, usize),
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
    /// Inside a loop (input, `pred_output`)
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
    /// Checked addition (reverts on overflow)
    CheckedAdd,
    /// Checked subtraction (reverts on underflow)
    CheckedSub,
    /// Checked multiplication (reverts on overflow)
    CheckedMul,

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
    /// Count leading zeros
    Clz,
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
    /// Conditional select: (cond, `true_val`, `false_val`) -> val
    Select,
    /// Calldata copy: (`dest_offset`, `cd_offset`, `size`) -> state
    /// Copies `size` bytes from calldata at `cd_offset` to memory at `dest_offset`.
    CalldataCopy,
    /// Memory copy: (`dest`, `src`, `size`) -> state
    /// Copies `size` bytes from memory at `src` to memory at `dest`.
    Mcopy,
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
    /// Sequence two expressions (evaluate both, return second)
    Concat(RcExpr, RcExpr),

    // ---- Control flow ----
    /// If-then-else: (pred, inputs, `then_body`, `else_body`)
    If(RcExpr, RcExpr, RcExpr, RcExpr),
    /// Do-while loop: (inputs, `pred_and_body`)
    DoWhile(RcExpr, RcExpr),

    // ---- EVM environment ----
    /// Nullary environment read: (op, state) -> (value, state)
    EnvRead(EvmEnvOp, RcExpr),
    /// Unary environment read: (op, arg, state) -> (value, state)
    EnvRead1(EvmEnvOp, RcExpr, RcExpr),

    // ---- EVM-specific ----
    /// Log event: (`topic_count`, topics, `data_offset`, `data_size`, state) -> state
    Log(usize, Vec<RcExpr>, RcExpr, RcExpr, RcExpr),
    /// Revert: (offset, size, state) -> !
    Revert(RcExpr, RcExpr, RcExpr),
    /// Return: (offset, size, state) -> !
    ReturnOp(RcExpr, RcExpr, RcExpr),
    /// External call: (target, value, `args_offset`, `args_len`, `ret_offset`, `ret_len`, state)
    ExtCall(RcExpr, RcExpr, RcExpr, RcExpr, RcExpr, RcExpr, RcExpr),
    /// Internal function call: (name, args...) -> result
    Call(String, Vec<RcExpr>),
    /// Function selector constant (4-byte keccak256 of signature)
    Selector(String),

    /// Let binding: compute value once, reference via Var(name) in body
    LetBind(String, RcExpr, RcExpr),
    /// Variable reference to a `LetBind`
    Var(String),
    /// Write to a `LetBind` variable's memory slot (mutates the variable in place)
    VarStore(String, RcExpr),
    /// Drop a variable (marks end of lifetime for slot reclamation)
    Drop(String),

    // ---- Top-level ----
    /// Function: (name, `input_type`, `output_type`, body)
    Function(String, EvmType, EvmType, RcExpr),
    /// Storage field: (name, `slot_index`, type)
    StorageField(String, usize, EvmType),

    /// Inline assembly: (`input_exprs`, `encoded_ops_hex`, `num_outputs`)
    /// Input expressions are compiled and pushed to stack before the asm body.
    /// `num_outputs` is how many values the asm block leaves on stack after consuming inputs.
    /// Opaque to egglog — passes through optimization unchanged.
    InlineAsm(Vec<RcExpr>, String, i32),

    /// Symbolic memory region: (`region_id`, `size_words`)
    /// Evaluates to the base memory address of this region at runtime.
    /// Different region IDs are guaranteed to be non-overlapping.
    /// Resolved to a concrete offset by `assign_memory_offsets` after egglog extraction.
    MemRegion(i64, i64),

    /// Dynamic memory allocation: size in bytes → base address.
    /// Uses MSIZE to find the current memory high-water mark and expands memory.
    /// NOT pure — memory expansion is an observable side effect.
    DynAlloc(RcExpr),

    /// Allocate a memory region: (`region_id`, `num_fields`, `is_dynamic`) → base address.
    /// `region_id` is a compile-time unique identifier for this allocation site.
    /// `num_fields` is the number of word-sized fields (may be a constant or expression).
    /// `is_dynamic`: true → runtime MSIZE-based allocation, false → static offset assigned later.
    AllocRegion(i64, RcExpr, bool),

    /// Store to a region field: (`region_id`, `field_index`, value, state) → state.
    /// `field_index` is a compile-time constant (0, 1, 2, ...).
    /// Different region IDs are guaranteed non-overlapping; same region + different field is
    /// guaranteed non-overlapping. Enables symbolic forwarding in egglog.
    RegionStore(i64, i64, RcExpr, RcExpr),

    /// Load from a region field: (`region_id`, `field_index`, state) → value.
    /// Symmetric to `RegionStore`. Egglog can forward through intervening stores to
    /// different regions or different fields of the same region.
    RegionLoad(i64, i64, RcExpr),
}

// ============================================================
// Program-level structures
// ============================================================

/// A compiled contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmContract {
    /// Contract name
    pub name: String,
    /// Storage field definitions (`StorageField` nodes)
    pub storage_fields: Vec<RcExpr>,
    /// Constructor body
    pub constructor: RcExpr,
    /// Runtime body (dispatcher + function bodies)
    pub runtime: RcExpr,
    /// Internal function definitions (Function nodes).
    /// Kept separate from `runtime` so they survive halting-DCE in cleanup.
    /// Compiled as labeled subroutines after the dispatcher.
    pub internal_functions: Vec<RcExpr>,
    /// First free memory offset after IR-allocated regions (arrays, structs, etc.)
    /// Codegen should start `LetBind` variable slots at or above this address.
    pub memory_high_water: usize,
}

/// A complete program (one or more contracts + free functions).
#[derive(Debug, Clone)]
pub struct EvmProgram {
    /// Contracts defined in the program
    pub contracts: Vec<EvmContract>,
    /// Free-standing functions (outside contracts)
    pub free_functions: Vec<RcExpr>,
    /// Compiler warnings collected during lowering
    pub warnings: Vec<edge_diagnostics::Diagnostic>,
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
            Self::CheckedAdd => "CHECKED_ADD",
            Self::CheckedSub => "CHECKED_SUB",
            Self::CheckedMul => "CHECKED_MUL",
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

impl EvmBinaryOp {
    /// Returns true if the second operand is a state token (ignored by codegen).
    pub const fn has_state(&self) -> bool {
        matches!(
            self,
            Self::SLoad | Self::TLoad | Self::MLoad | Self::CalldataLoad
        )
    }
}

impl std::fmt::Display for EvmUnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::IsZero => "ISZERO",
            Self::Not => "NOT",
            Self::Neg => "NEG",
            Self::SignExtend => "SIGNEXTEND",
            Self::Clz => "CLZ",
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
            Self::CalldataCopy => "CALLDATACOPY",
            Self::Mcopy => "MCOPY",
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
