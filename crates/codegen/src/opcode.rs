//! EVM opcode definitions.
//!
//! Complete set of EVM opcodes with their byte values, per the
//! [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/) and
//! subsequent EIPs.

/// EVM opcodes with their byte values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum Opcode {
    // 0x00 - Stop and Arithmetic
    Stop = 0x00,
    Add = 0x01,
    Mul = 0x02,
    Sub = 0x03,
    Div = 0x04,
    SDiv = 0x05,
    Mod = 0x06,
    SMod = 0x07,
    AddMod = 0x08,
    MulMod = 0x09,
    Exp = 0x0A,
    SignExtend = 0x0B,

    // 0x10 - Comparison & Bitwise Logic
    Lt = 0x10,
    Gt = 0x11,
    SLt = 0x12,
    SGt = 0x13,
    Eq = 0x14,
    IsZero = 0x15,
    And = 0x16,
    Or = 0x17,
    Xor = 0x18,
    Not = 0x19,
    Byte = 0x1A,
    Shl = 0x1B,
    Shr = 0x1C,
    Sar = 0x1D,

    // 0x20 - Keccak256
    Keccak256 = 0x20,

    // 0x30 - Environmental Information
    Address = 0x30,
    Balance = 0x31,
    Origin = 0x32,
    Caller = 0x33,
    CallValue = 0x34,
    CallDataLoad = 0x35,
    CallDataSize = 0x36,
    CallDataCopy = 0x37,
    CodeSize = 0x38,
    CodeCopy = 0x39,
    GasPrice = 0x3A,
    ExtCodeSize = 0x3B,
    ExtCodeCopy = 0x3C,
    ReturnDataSize = 0x3D,
    ReturnDataCopy = 0x3E,
    ExtCodeHash = 0x3F,

    // 0x40 - Block Information
    BlockHash = 0x40,
    Coinbase = 0x41,
    Timestamp = 0x42,
    Number = 0x43,
    Prevrandao = 0x44,
    GasLimit = 0x45,
    ChainId = 0x46,
    SelfBalance = 0x47,
    BaseFee = 0x48,
    BlobHash = 0x49,
    BlobBaseFee = 0x4A,

    // 0x50 - Stack, Memory, Storage, Flow
    Pop = 0x50,
    MLoad = 0x51,
    MStore = 0x52,
    MStore8 = 0x53,
    SLoad = 0x54,
    SStore = 0x55,
    Jump = 0x56,
    JumpI = 0x57,
    Pc = 0x58,
    MSize = 0x59,
    Gas = 0x5A,
    JumpDest = 0x5B,
    TLoad = 0x5C,
    TStore = 0x5D,
    MCopy = 0x5E,
    Push0 = 0x5F,

    // 0x60-0x7F - Push operations
    Push1 = 0x60,
    Push2 = 0x61,
    Push3 = 0x62,
    Push4 = 0x63,
    Push5 = 0x64,
    Push6 = 0x65,
    Push7 = 0x66,
    Push8 = 0x67,
    Push9 = 0x68,
    Push10 = 0x69,
    Push11 = 0x6A,
    Push12 = 0x6B,
    Push13 = 0x6C,
    Push14 = 0x6D,
    Push15 = 0x6E,
    Push16 = 0x6F,
    Push17 = 0x70,
    Push18 = 0x71,
    Push19 = 0x72,
    Push20 = 0x73,
    Push21 = 0x74,
    Push22 = 0x75,
    Push23 = 0x76,
    Push24 = 0x77,
    Push25 = 0x78,
    Push26 = 0x79,
    Push27 = 0x7A,
    Push28 = 0x7B,
    Push29 = 0x7C,
    Push30 = 0x7D,
    Push31 = 0x7E,
    Push32 = 0x7F,

    // 0x80-0x8F - Duplication operations
    Dup1 = 0x80,
    Dup2 = 0x81,
    Dup3 = 0x82,
    Dup4 = 0x83,
    Dup5 = 0x84,
    Dup6 = 0x85,
    Dup7 = 0x86,
    Dup8 = 0x87,
    Dup9 = 0x88,
    Dup10 = 0x89,
    Dup11 = 0x8A,
    Dup12 = 0x8B,
    Dup13 = 0x8C,
    Dup14 = 0x8D,
    Dup15 = 0x8E,
    Dup16 = 0x8F,

    // 0x90-0x9F - Exchange operations
    Swap1 = 0x90,
    Swap2 = 0x91,
    Swap3 = 0x92,
    Swap4 = 0x93,
    Swap5 = 0x94,
    Swap6 = 0x95,
    Swap7 = 0x96,
    Swap8 = 0x97,
    Swap9 = 0x98,
    Swap10 = 0x99,
    Swap11 = 0x9A,
    Swap12 = 0x9B,
    Swap13 = 0x9C,
    Swap14 = 0x9D,
    Swap15 = 0x9E,
    Swap16 = 0x9F,

    // 0xA0-0xA4 - Logging
    Log0 = 0xA0,
    Log1 = 0xA1,
    Log2 = 0xA2,
    Log3 = 0xA3,
    Log4 = 0xA4,

    // 0xF0-0xFF - System operations
    Create = 0xF0,
    Call = 0xF1,
    CallCode = 0xF2,
    Return = 0xF3,
    DelegateCall = 0xF4,
    Create2 = 0xF5,
    StaticCall = 0xFA,
    Revert = 0xFD,
    Invalid = 0xFE,
    SelfDestruct = 0xFF,
}

impl Opcode {
    /// Get the PUSH opcode for `n` bytes (0 = PUSH0, 1 = PUSH1, ..., 32 = PUSH32).
    ///
    /// # Panics
    /// Panics if `n > 32`.
    pub fn push_n(n: u8) -> Self {
        match n {
            0 => Self::Push0,
            1..=32 => {
                // SAFETY: Push1 through Push32 are contiguous in the enum
                // Push1 = 0x60, Push2 = 0x61, ..., Push32 = 0x7F
                // So push_n(n) = 0x5F + n, which is always a valid Opcode discriminant.
                unsafe { std::mem::transmute::<u8, Self>(0x5Fu8 + n) }
            }
            _ => panic!("PUSH operand size must be 0..=32, got {n}"),
        }
    }

    /// Get the DUP opcode for stack position `n` (1 = DUP1, ..., 16 = DUP16).
    ///
    /// # Panics
    /// Panics if `n < 1` or `n > 16`.
    pub fn dup_n(n: u8) -> Self {
        assert!(
            (1..=16).contains(&n),
            "DUP position must be 1..=16, got {n}"
        );
        // SAFETY: Dup1..Dup16 are contiguous (0x80..0x8F), and the assert guarantees n is 1..=16,
        // so 0x7F + n is always a valid Opcode discriminant.
        unsafe { std::mem::transmute::<u8, Self>(0x7Fu8 + n) }
    }

    /// Get the SWAP opcode for stack position `n` (1 = SWAP1, ..., 16 = SWAP16).
    ///
    /// # Panics
    /// Panics if `n < 1` or `n > 16`.
    pub fn swap_n(n: u8) -> Self {
        assert!(
            (1..=16).contains(&n),
            "SWAP position must be 1..=16, got {n}"
        );
        // SAFETY: Swap1..Swap16 are contiguous (0x90..0x9F), and the assert guarantees n is 1..=16,
        // so 0x8F + n is always a valid Opcode discriminant.
        unsafe { std::mem::transmute::<u8, Self>(0x8Fu8 + n) }
    }

    /// Get the LOG opcode for `n` topics (0 = LOG0, ..., 4 = LOG4).
    ///
    /// # Panics
    /// Panics if `n > 4`.
    pub fn log_n(n: u8) -> Self {
        assert!(n <= 4, "LOG topic count must be 0..=4, got {n}");
        // SAFETY: Log0..Log4 are contiguous (0xA0..0xA4), and the assert guarantees n is 0..=4,
        // so 0xA0 + n is always a valid Opcode discriminant.
        unsafe { std::mem::transmute::<u8, Self>(0xA0u8 + n) }
    }

    /// Get the byte value of this opcode.
    pub const fn byte(self) -> u8 {
        self as u8
    }

    /// Get the number of stack items consumed by this opcode.
    pub const fn stack_inputs(self) -> u8 {
        match self {
            Self::Stop
            | Self::JumpDest
            | Self::Push0
            | Self::Pc
            | Self::MSize
            | Self::Gas
            | Self::Address
            | Self::Origin
            | Self::Caller
            | Self::CallValue
            | Self::CallDataSize
            | Self::CodeSize
            | Self::GasPrice
            | Self::ReturnDataSize
            | Self::Coinbase
            | Self::Timestamp
            | Self::Number
            | Self::Prevrandao
            | Self::GasLimit
            | Self::ChainId
            | Self::SelfBalance
            | Self::BaseFee
            | Self::BlobBaseFee
            | Self::Invalid
            // Push, Dup, and Swap opcodes consume 0 inputs
            | Self::Push1
            | Self::Push2
            | Self::Push3
            | Self::Push4
            | Self::Push5
            | Self::Push6
            | Self::Push7
            | Self::Push8
            | Self::Push9
            | Self::Push10
            | Self::Push11
            | Self::Push12
            | Self::Push13
            | Self::Push14
            | Self::Push15
            | Self::Push16
            | Self::Push17
            | Self::Push18
            | Self::Push19
            | Self::Push20
            | Self::Push21
            | Self::Push22
            | Self::Push23
            | Self::Push24
            | Self::Push25
            | Self::Push26
            | Self::Push27
            | Self::Push28
            | Self::Push29
            | Self::Push30
            | Self::Push31
            | Self::Push32
            | Self::Dup1
            | Self::Dup2
            | Self::Dup3
            | Self::Dup4
            | Self::Dup5
            | Self::Dup6
            | Self::Dup7
            | Self::Dup8
            | Self::Dup9
            | Self::Dup10
            | Self::Dup11
            | Self::Dup12
            | Self::Dup13
            | Self::Dup14
            | Self::Dup15
            | Self::Dup16
            | Self::Swap1
            | Self::Swap2
            | Self::Swap3
            | Self::Swap4
            | Self::Swap5
            | Self::Swap6
            | Self::Swap7
            | Self::Swap8
            | Self::Swap9
            | Self::Swap10
            | Self::Swap11
            | Self::Swap12
            | Self::Swap13
            | Self::Swap14
            | Self::Swap15
            | Self::Swap16 => 0,

            Self::IsZero
            | Self::Not
            | Self::Pop
            | Self::MLoad
            | Self::SLoad
            | Self::Jump
            | Self::TLoad
            | Self::Balance
            | Self::ExtCodeSize
            | Self::ExtCodeHash
            | Self::BlockHash
            | Self::BlobHash
            | Self::CallDataLoad
            | Self::SelfDestruct => 1,

            Self::Add
            | Self::Mul
            | Self::Sub
            | Self::Div
            | Self::SDiv
            | Self::Mod
            | Self::SMod
            | Self::Exp
            | Self::SignExtend
            | Self::Lt
            | Self::Gt
            | Self::SLt
            | Self::SGt
            | Self::Eq
            | Self::And
            | Self::Or
            | Self::Xor
            | Self::Byte
            | Self::Shl
            | Self::Shr
            | Self::Sar
            | Self::Keccak256
            | Self::MStore
            | Self::MStore8
            | Self::SStore
            | Self::JumpI
            | Self::TStore
            | Self::Return
            | Self::Revert
            | Self::Log0 => 2,

            Self::AddMod
            | Self::MulMod
            | Self::CallDataCopy
            | Self::CodeCopy
            | Self::ReturnDataCopy
            | Self::MCopy
            | Self::Create
            | Self::Log1 => 3,

            Self::ExtCodeCopy | Self::Create2 | Self::Log2 => 4,

            Self::Log3 => 5,
            Self::Log4 | Self::DelegateCall | Self::StaticCall => 6,
            Self::Call | Self::CallCode => 7,
        }
    }

    /// Get the number of stack items produced by this opcode.
    pub const fn stack_outputs(self) -> u8 {
        match self {
            // Halting, writes, control flow, logging, memory copy, SWAP — no outputs
            Self::Stop
            | Self::Return
            | Self::Revert
            | Self::Invalid
            | Self::SelfDestruct
            | Self::Pop
            | Self::MStore
            | Self::MStore8
            | Self::SStore
            | Self::TStore
            | Self::Jump
            | Self::JumpI
            | Self::JumpDest
            | Self::Log0
            | Self::Log1
            | Self::Log2
            | Self::Log3
            | Self::Log4
            | Self::CallDataCopy
            | Self::CodeCopy
            | Self::ReturnDataCopy
            | Self::ExtCodeCopy
            | Self::MCopy
            | Self::Swap1
            | Self::Swap2
            | Self::Swap3
            | Self::Swap4
            | Self::Swap5
            | Self::Swap6
            | Self::Swap7
            | Self::Swap8
            | Self::Swap9
            | Self::Swap10
            | Self::Swap11
            | Self::Swap12
            | Self::Swap13
            | Self::Swap14
            | Self::Swap15
            | Self::Swap16 => 0,

            // Everything else produces 1 output (arithmetic, comparisons, env, Push, Dup)
            Self::Add
            | Self::Mul
            | Self::Sub
            | Self::Div
            | Self::SDiv
            | Self::Mod
            | Self::SMod
            | Self::Exp
            | Self::SignExtend
            | Self::AddMod
            | Self::MulMod
            | Self::Lt
            | Self::Gt
            | Self::SLt
            | Self::SGt
            | Self::Eq
            | Self::IsZero
            | Self::And
            | Self::Or
            | Self::Xor
            | Self::Not
            | Self::Byte
            | Self::Shl
            | Self::Shr
            | Self::Sar
            | Self::Keccak256
            | Self::Address
            | Self::Balance
            | Self::Origin
            | Self::Caller
            | Self::CallValue
            | Self::CallDataLoad
            | Self::CallDataSize
            | Self::CodeSize
            | Self::GasPrice
            | Self::ExtCodeSize
            | Self::ExtCodeHash
            | Self::ReturnDataSize
            | Self::BlockHash
            | Self::Coinbase
            | Self::Timestamp
            | Self::Number
            | Self::Prevrandao
            | Self::GasLimit
            | Self::ChainId
            | Self::SelfBalance
            | Self::BaseFee
            | Self::BlobHash
            | Self::BlobBaseFee
            | Self::MLoad
            | Self::SLoad
            | Self::TLoad
            | Self::Pc
            | Self::MSize
            | Self::Gas
            | Self::Create
            | Self::Create2
            | Self::Call
            | Self::CallCode
            | Self::DelegateCall
            | Self::StaticCall
            | Self::Push0
            | Self::Push1
            | Self::Push2
            | Self::Push3
            | Self::Push4
            | Self::Push5
            | Self::Push6
            | Self::Push7
            | Self::Push8
            | Self::Push9
            | Self::Push10
            | Self::Push11
            | Self::Push12
            | Self::Push13
            | Self::Push14
            | Self::Push15
            | Self::Push16
            | Self::Push17
            | Self::Push18
            | Self::Push19
            | Self::Push20
            | Self::Push21
            | Self::Push22
            | Self::Push23
            | Self::Push24
            | Self::Push25
            | Self::Push26
            | Self::Push27
            | Self::Push28
            | Self::Push29
            | Self::Push30
            | Self::Push31
            | Self::Push32
            | Self::Dup1
            | Self::Dup2
            | Self::Dup3
            | Self::Dup4
            | Self::Dup5
            | Self::Dup6
            | Self::Dup7
            | Self::Dup8
            | Self::Dup9
            | Self::Dup10
            | Self::Dup11
            | Self::Dup12
            | Self::Dup13
            | Self::Dup14
            | Self::Dup15
            | Self::Dup16 => 1,
        }
    }
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_values() {
        assert_eq!(Opcode::Stop.byte(), 0x00);
        assert_eq!(Opcode::Add.byte(), 0x01);
        assert_eq!(Opcode::Caller.byte(), 0x33);
        assert_eq!(Opcode::SLoad.byte(), 0x54);
        assert_eq!(Opcode::SStore.byte(), 0x55);
        assert_eq!(Opcode::Push0.byte(), 0x5F);
        assert_eq!(Opcode::Push1.byte(), 0x60);
        assert_eq!(Opcode::Push32.byte(), 0x7F);
        assert_eq!(Opcode::Dup1.byte(), 0x80);
        assert_eq!(Opcode::Swap1.byte(), 0x90);
        assert_eq!(Opcode::Log0.byte(), 0xA0);
        assert_eq!(Opcode::Return.byte(), 0xF3);
        assert_eq!(Opcode::Revert.byte(), 0xFD);
    }

    #[test]
    fn test_push_n() {
        assert_eq!(Opcode::push_n(0), Opcode::Push0);
        assert_eq!(Opcode::push_n(1), Opcode::Push1);
        assert_eq!(Opcode::push_n(32), Opcode::Push32);
    }

    #[test]
    fn test_dup_n() {
        assert_eq!(Opcode::dup_n(1), Opcode::Dup1);
        assert_eq!(Opcode::dup_n(16), Opcode::Dup16);
    }

    #[test]
    fn test_swap_n() {
        assert_eq!(Opcode::swap_n(1), Opcode::Swap1);
        assert_eq!(Opcode::swap_n(16), Opcode::Swap16);
    }

    #[test]
    fn test_log_n() {
        assert_eq!(Opcode::log_n(0), Opcode::Log0);
        assert_eq!(Opcode::log_n(4), Opcode::Log4);
    }
}
