//! IR instruction types

/// A single IR instruction mapping closely to EVM opcodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrInstruction {
    // ── Stack manipulation ────────────────────────────────
    /// Push bytes onto the stack (1-32 bytes)
    Push(Vec<u8>),
    /// Pop top of stack
    Pop,
    /// Duplicate nth stack item (1-indexed)
    Dup(u8),
    /// Swap top with nth stack item (1-indexed)
    Swap(u8),

    // ── Arithmetic ────────────────────────────────────────
    /// ADD
    Add,
    /// SUB
    Sub,
    /// MUL
    Mul,
    /// DIV
    Div,
    /// MOD
    Mod,

    // ── Comparison ────────────────────────────────────────
    /// LT
    Lt,
    /// GT
    Gt,
    /// EQ
    Eq,
    /// ISZERO
    IsZero,

    // ── Bitwise ──────────────────────────────────────────
    /// AND
    And,
    /// OR
    Or,
    /// XOR
    Xor,
    /// NOT
    Not,
    /// SHL
    Shl,
    /// SHR
    Shr,

    // ── Storage ──────────────────────────────────────────
    /// Load from storage (slot number already on stack)
    SLoad,
    /// Store to storage (value then slot on stack)
    SStore,

    // ── Memory ───────────────────────────────────────────
    /// Load 32 bytes from memory (offset on stack)
    MLoad,
    /// Store 32 bytes to memory (value then offset on stack)
    MStore,

    // ── Calldata ─────────────────────────────────────────
    /// Load 32 bytes from calldata at offset
    CallDataLoad,
    /// Get size of calldata
    CallDataSize,

    // ── Context ──────────────────────────────────────────
    /// Get the caller address
    Caller,
    /// Get call value (msg.value)
    CallValue,
    /// Get block number
    Number,
    /// Get block timestamp
    Timestamp,

    // ── Hashing ──────────────────────────────────────────
    /// SHA3/KECCAK256 (offset, size on stack → hash)
    Keccak256,

    // ── Logging ──────────────────────────────────────────
    /// LOG0..LOG4 (n = number of topics)
    Log(u8),

    // ── Control flow ─────────────────────────────────────
    /// Unconditional jump (destination on stack)
    Jump,
    /// Conditional jump (destination, condition on stack)
    JumpI,
    /// Jump destination marker with label name
    JumpDest(String),
    /// Push the byte offset of a label (resolved in codegen)
    PushLabel(String),

    // ── Return ───────────────────────────────────────────
    /// Return (offset, size on stack)
    Return,
    /// Revert (offset, size on stack)
    Revert,
    /// Stop execution
    Stop,
}
