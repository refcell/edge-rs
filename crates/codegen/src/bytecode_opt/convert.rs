//! Conversion between `AsmInstruction` and egglog s-expressions.

use crate::{
    assembler::AsmInstruction,
    opcode::Opcode,
};

/// Convert a list of `AsmInstruction`s (a basic block body — no labels/jumps)
/// into an egglog s-expression string representing an `ISeq` cons-list.
///
/// The head of the cons-list is the first instruction to execute.
pub(crate) fn instructions_to_sexp(instrs: &[AsmInstruction]) -> String {
    let mut sexp = "(INil)".to_string();
    // Build cons-list from back to front
    for inst in instrs.iter().rev() {
        match inst {
            AsmInstruction::Op(op) => {
                if *op == Opcode::Push0 {
                    sexp = format!("(ICons (IPush0) {sexp})");
                } else if let Some(name) = opcode_to_inst_name(*op) {
                    sexp = format!("(ICons ({name}) {sexp})");
                } else {
                    // Parameterized opcodes
                    let byte = op.byte();
                    if (0x80..=0x8F).contains(&byte) {
                        // DUP1..DUP16
                        let n = byte - 0x7F;
                        sexp = format!("(ICons (IDup {n}) {sexp})");
                    } else if (0x90..=0x9F).contains(&byte) {
                        // SWAP1..SWAP16
                        let n = byte - 0x8F;
                        sexp = format!("(ICons (ISwap {n}) {sexp})");
                    } else if (0xA0..=0xA4).contains(&byte) {
                        // LOG0..LOG4
                        let n = byte - 0xA0;
                        sexp = format!("(ICons (ILog {n}) {sexp})");
                    }
                    // Other opcodes that don't map are skipped (shouldn't happen in basic blocks)
                }
            }
            AsmInstruction::Push(data) => {
                if let Some(val) = try_push_as_i64(data) {
                    sexp = format!("(IPushCons (PushSmall {val}) {sexp})");
                } else {
                    let hex = hex_encode(data);
                    sexp = format!("(IPushCons (PushHex \"{hex}\") {sexp})");
                }
            }
            // Labels, jumps, and comments should not appear in egglog optimization
            AsmInstruction::Label(_) | AsmInstruction::JumpTo(_)
            | AsmInstruction::JumpITo(_) | AsmInstruction::PushLabel(_)
            | AsmInstruction::Comment(_) => {}
        }
    }
    sexp
}

/// Parse an egglog s-expression (extracted best ISeq) back into `AsmInstruction`s.
///
/// The s-expression is a nested cons-list like:
/// `(ICons (IAdd) (IPushCons (PushSmall 42) INil))`
pub(crate) fn sexp_to_instructions(sexp: &str) -> Vec<AsmInstruction> {
    let tokens = tokenize(sexp);
    let (_, instrs) = parse_iseq(&tokens, 0);
    instrs
}

/// Try to interpret push data as a non-negative i64.
/// Returns None if the value is too large or zero (Push0 handles that).
fn try_push_as_i64(data: &[u8]) -> Option<i64> {
    // Must fit in i64 (8 bytes, positive)
    if data.len() > 8 {
        return None;
    }
    let mut val: u64 = 0;
    for &b in data {
        val = (val << 8) | (b as u64);
    }
    // Must fit in positive i64
    if val <= i64::MAX as u64 {
        Some(val as i64)
    } else {
        None
    }
}

/// Encode bytes as a hex string (no 0x prefix).
fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decode a hex string (no 0x prefix) into bytes.
fn hex_decode(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0))
        .collect()
}

/// Convert an i64 value to minimal big-endian push bytes.
fn i64_to_push_bytes(val: i64) -> Vec<u8> {
    if val == 0 {
        // Caller should emit Push0 instead, but handle gracefully
        return vec![0];
    }
    let bytes = (val as u64).to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[start..].to_vec()
}

// --- Simple s-expression tokenizer and parser ---

#[derive(Debug, Clone, PartialEq)]
enum Token {
    LParen,
    RParen,
    Symbol(String),
    Integer(i64),
    Str(String),
}

fn tokenize(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            '"' => {
                i += 1; // skip opening quote
                let start = i;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Str(s));
                i += 1; // skip closing quote
            }
            c if c.is_whitespace() => { i += 1; }
            c if c == '-' || c.is_ascii_digit() => {
                let start = i;
                if c == '-' { i += 1; }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                // Check if next char is alphabetic (it's a symbol like "INil")
                if i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_') {
                    // Not a number, it's a symbol
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' && chars[i] != ')' {
                        i += 1;
                    }
                    let sym: String = chars[start..i].iter().collect();
                    tokens.push(Token::Symbol(sym));
                } else {
                    let num_str: String = chars[start..i].iter().collect();
                    if let Ok(n) = num_str.parse::<i64>() {
                        tokens.push(Token::Integer(n));
                    } else {
                        tokens.push(Token::Symbol(num_str));
                    }
                }
            }
            _ => {
                let start = i;
                while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' && chars[i] != ')' {
                    i += 1;
                }
                let sym: String = chars[start..i].iter().collect();
                tokens.push(Token::Symbol(sym));
            }
        }
    }
    tokens
}

/// Parse an ISeq from tokens starting at position `pos`. Returns (new_pos, instructions).
fn parse_iseq(tokens: &[Token], pos: usize) -> (usize, Vec<AsmInstruction>) {
    if pos >= tokens.len() {
        return (pos, vec![]);
    }

    match &tokens[pos] {
        Token::Symbol(s) if s == "INil" => (pos + 1, vec![]),
        Token::LParen => {
            // (INil) or (ICons <inst> <rest>) or (IPushCons <pushval> <rest>)
            let pos = pos + 1; // skip '('
            if pos >= tokens.len() {
                return (pos, vec![]);
            }
            match &tokens[pos] {
                Token::Symbol(s) if s == "INil" => {
                    let pos = pos + 1;
                    let pos = expect_rparen(tokens, pos);
                    return (pos, vec![]);
                }
                Token::Symbol(s) if s == "ICons" => {
                    let pos = pos + 1;
                    let (pos, inst) = parse_inst(tokens, pos);
                    let (pos, mut rest) = parse_iseq(tokens, pos);
                    let pos = expect_rparen(tokens, pos);
                    let mut result = Vec::with_capacity(1 + rest.len());
                    result.push(inst);
                    result.append(&mut rest);
                    (pos, result)
                }
                Token::Symbol(s) if s == "IPushCons" => {
                    let pos = pos + 1;
                    let (pos, push_inst) = parse_pushval(tokens, pos);
                    let (pos, mut rest) = parse_iseq(tokens, pos);
                    let pos = expect_rparen(tokens, pos);
                    let mut result = Vec::with_capacity(1 + rest.len());
                    result.push(push_inst);
                    result.append(&mut rest);
                    (pos, result)
                }
                Token::Symbol(s) if s == "INil" => {
                    let pos = pos + 1;
                    let pos = expect_rparen(tokens, pos);
                    (pos, vec![])
                }
                _ => (pos, vec![]),
            }
        }
        _ => (pos, vec![]),
    }
}

/// Parse a single Inst from `(InstName)` or `(InstName i64)`.
fn parse_inst(tokens: &[Token], pos: usize) -> (usize, AsmInstruction) {
    if pos >= tokens.len() {
        return (pos, AsmInstruction::Op(Opcode::Invalid));
    }
    match &tokens[pos] {
        Token::LParen => {
            let pos = pos + 1;
            if pos >= tokens.len() {
                return (pos, AsmInstruction::Op(Opcode::Invalid));
            }
            match &tokens[pos] {
                Token::Symbol(name) => {
                    let pos = pos + 1;
                    // Check for parameterized instructions
                    match name.as_str() {
                        "IDup" => {
                            if let Some(Token::Integer(n)) = tokens.get(pos) {
                                let pos = expect_rparen(tokens, pos + 1);
                                (pos, AsmInstruction::Op(Opcode::dup_n(*n as u8)))
                            } else {
                                let pos = expect_rparen(tokens, pos);
                                (pos, AsmInstruction::Op(Opcode::Dup1))
                            }
                        }
                        "ISwap" => {
                            if let Some(Token::Integer(n)) = tokens.get(pos) {
                                let pos = expect_rparen(tokens, pos + 1);
                                (pos, AsmInstruction::Op(Opcode::swap_n(*n as u8)))
                            } else {
                                let pos = expect_rparen(tokens, pos);
                                (pos, AsmInstruction::Op(Opcode::Swap1))
                            }
                        }
                        "ILog" => {
                            if let Some(Token::Integer(n)) = tokens.get(pos) {
                                let pos = expect_rparen(tokens, pos + 1);
                                (pos, AsmInstruction::Op(Opcode::log_n(*n as u8)))
                            } else {
                                let pos = expect_rparen(tokens, pos);
                                (pos, AsmInstruction::Op(Opcode::Log0))
                            }
                        }
                        "IPush0" => {
                            let pos = expect_rparen(tokens, pos);
                            (pos, AsmInstruction::Op(Opcode::Push0))
                        }
                        _ => {
                            let op = inst_name_to_opcode(name);
                            let pos = expect_rparen(tokens, pos);
                            (pos, AsmInstruction::Op(op))
                        }
                    }
                }
                _ => {
                    let pos = expect_rparen(tokens, pos);
                    (pos, AsmInstruction::Op(Opcode::Invalid))
                }
            }
        }
        _ => (pos, AsmInstruction::Op(Opcode::Invalid)),
    }
}

/// Parse a PushVal from `(PushSmall i64)` or `(PushHex "hex")`.
fn parse_pushval(tokens: &[Token], pos: usize) -> (usize, AsmInstruction) {
    if pos >= tokens.len() || tokens[pos] != Token::LParen {
        return (pos, AsmInstruction::Op(Opcode::Push0));
    }
    let pos = pos + 1; // skip '('
    if pos >= tokens.len() {
        return (pos, AsmInstruction::Op(Opcode::Push0));
    }
    match &tokens[pos] {
        Token::Symbol(s) if s == "PushSmall" => {
            let pos = pos + 1;
            if let Some(Token::Integer(val)) = tokens.get(pos) {
                let pos = expect_rparen(tokens, pos + 1);
                if *val == 0 {
                    (pos, AsmInstruction::Op(Opcode::Push0))
                } else {
                    (pos, AsmInstruction::Push(i64_to_push_bytes(*val)))
                }
            } else {
                let pos = expect_rparen(tokens, pos);
                (pos, AsmInstruction::Op(Opcode::Push0))
            }
        }
        Token::Symbol(s) if s == "PushHex" => {
            let pos = pos + 1;
            if let Some(Token::Str(hex)) = tokens.get(pos) {
                let pos = expect_rparen(tokens, pos + 1);
                let data = hex_decode(hex);
                if data.is_empty() || data.iter().all(|&b| b == 0) {
                    (pos, AsmInstruction::Op(Opcode::Push0))
                } else {
                    (pos, AsmInstruction::Push(data))
                }
            } else {
                let pos = expect_rparen(tokens, pos);
                (pos, AsmInstruction::Op(Opcode::Push0))
            }
        }
        _ => {
            let pos = expect_rparen(tokens, pos);
            (pos, AsmInstruction::Op(Opcode::Push0))
        }
    }
}

fn expect_rparen(tokens: &[Token], pos: usize) -> usize {
    if pos < tokens.len() && tokens[pos] == Token::RParen {
        pos + 1
    } else {
        pos
    }
}

/// Map an Opcode to its egglog Inst constructor name.
/// Returns None for parameterized opcodes (Dup/Swap/Log) and Push0.
fn opcode_to_inst_name(op: Opcode) -> Option<&'static str> {
    Some(match op {
        Opcode::Add => "IAdd",
        Opcode::Mul => "IMul",
        Opcode::Sub => "ISub",
        Opcode::Div => "IDiv",
        Opcode::SDiv => "ISDiv",
        Opcode::Mod => "IMod",
        Opcode::SMod => "ISMod",
        Opcode::Exp => "IExp",
        Opcode::AddMod => "IAddMod",
        Opcode::MulMod => "IMulMod",
        Opcode::SignExtend => "ISignExtend",
        Opcode::Lt => "ILt",
        Opcode::Gt => "IGt",
        Opcode::SLt => "ISLt",
        Opcode::SGt => "ISGt",
        Opcode::Eq => "IEq",
        Opcode::IsZero => "IIsZero",
        Opcode::And => "IAnd",
        Opcode::Or => "IOr",
        Opcode::Xor => "IXor",
        Opcode::Not => "INot",
        Opcode::Byte => "IByte",
        Opcode::Shl => "IShl",
        Opcode::Shr => "IShr",
        Opcode::Sar => "ISar",
        Opcode::Keccak256 => "IKeccak256",
        Opcode::Address => "IAddress",
        Opcode::Balance => "IBalance",
        Opcode::Origin => "IOrigin",
        Opcode::Caller => "ICaller",
        Opcode::CallValue => "ICallValue",
        Opcode::CallDataLoad => "ICallDataLoad",
        Opcode::CallDataSize => "ICallDataSize",
        Opcode::CodeSize => "ICodeSize",
        Opcode::GasPrice => "IGasPrice",
        Opcode::ReturnDataSize => "IReturnDataSize",
        Opcode::ExtCodeSize => "IExtCodeSize",
        Opcode::ExtCodeHash => "IExtCodeHash",
        Opcode::BlockHash => "IBlockHash",
        Opcode::Coinbase => "ICoinbase",
        Opcode::Timestamp => "ITimestamp",
        Opcode::Number => "INumber",
        Opcode::Prevrandao => "IPrevrandao",
        Opcode::GasLimit => "IGasLimit",
        Opcode::ChainId => "IChainId",
        Opcode::SelfBalance => "ISelfBalance",
        Opcode::BaseFee => "IBaseFee",
        Opcode::Pop => "IPop",
        Opcode::MLoad => "IMLoad",
        Opcode::MStore => "IMStore",
        Opcode::MStore8 => "IMStore8",
        Opcode::SLoad => "ISLoad",
        Opcode::SStore => "ISStore",
        Opcode::TLoad => "ITLoad",
        Opcode::TStore => "ITStore",
        Opcode::MCopy => "IMCopy",
        Opcode::Create => "ICreate",
        Opcode::Call => "ICall",
        Opcode::CallCode => "ICallCode",
        Opcode::Return => "IReturn",
        Opcode::DelegateCall => "IDelegateCall",
        Opcode::Create2 => "ICreate2",
        Opcode::StaticCall => "IStaticCall",
        Opcode::Revert => "IRevert",
        Opcode::Invalid => "IInvalid",
        Opcode::SelfDestruct => "ISelfDestruct",
        Opcode::Gas => "IGas",
        Opcode::Pc => "IPc",
        Opcode::MSize => "IMSize",
        Opcode::Stop => "IStop",
        // Parameterized and special opcodes handled separately
        Opcode::Push0 => return None,
        _ => return None,
    })
}

/// Map an egglog Inst constructor name back to an Opcode.
fn inst_name_to_opcode(name: &str) -> Opcode {
    match name {
        "IAdd" => Opcode::Add,
        "IMul" => Opcode::Mul,
        "ISub" => Opcode::Sub,
        "IDiv" => Opcode::Div,
        "ISDiv" => Opcode::SDiv,
        "IMod" => Opcode::Mod,
        "ISMod" => Opcode::SMod,
        "IExp" => Opcode::Exp,
        "IAddMod" => Opcode::AddMod,
        "IMulMod" => Opcode::MulMod,
        "ISignExtend" => Opcode::SignExtend,
        "ILt" => Opcode::Lt,
        "IGt" => Opcode::Gt,
        "ISLt" => Opcode::SLt,
        "ISGt" => Opcode::SGt,
        "IEq" => Opcode::Eq,
        "IIsZero" => Opcode::IsZero,
        "IAnd" => Opcode::And,
        "IOr" => Opcode::Or,
        "IXor" => Opcode::Xor,
        "INot" => Opcode::Not,
        "IByte" => Opcode::Byte,
        "IShl" => Opcode::Shl,
        "IShr" => Opcode::Shr,
        "ISar" => Opcode::Sar,
        "IKeccak256" => Opcode::Keccak256,
        "IAddress" => Opcode::Address,
        "IBalance" => Opcode::Balance,
        "IOrigin" => Opcode::Origin,
        "ICaller" => Opcode::Caller,
        "ICallValue" => Opcode::CallValue,
        "ICallDataLoad" => Opcode::CallDataLoad,
        "ICallDataSize" => Opcode::CallDataSize,
        "ICodeSize" => Opcode::CodeSize,
        "IGasPrice" => Opcode::GasPrice,
        "IReturnDataSize" => Opcode::ReturnDataSize,
        "IExtCodeSize" => Opcode::ExtCodeSize,
        "IExtCodeHash" => Opcode::ExtCodeHash,
        "IBlockHash" => Opcode::BlockHash,
        "ICoinbase" => Opcode::Coinbase,
        "ITimestamp" => Opcode::Timestamp,
        "INumber" => Opcode::Number,
        "IPrevrandao" => Opcode::Prevrandao,
        "IGasLimit" => Opcode::GasLimit,
        "IChainId" => Opcode::ChainId,
        "ISelfBalance" => Opcode::SelfBalance,
        "IBaseFee" => Opcode::BaseFee,
        "IPop" => Opcode::Pop,
        "IMLoad" => Opcode::MLoad,
        "IMStore" => Opcode::MStore,
        "IMStore8" => Opcode::MStore8,
        "ISLoad" => Opcode::SLoad,
        "ISStore" => Opcode::SStore,
        "ITLoad" => Opcode::TLoad,
        "ITStore" => Opcode::TStore,
        "IMCopy" => Opcode::MCopy,
        "ICreate" => Opcode::Create,
        "ICall" => Opcode::Call,
        "ICallCode" => Opcode::CallCode,
        "IReturn" => Opcode::Return,
        "IDelegateCall" => Opcode::DelegateCall,
        "ICreate2" => Opcode::Create2,
        "IStaticCall" => Opcode::StaticCall,
        "IRevert" => Opcode::Revert,
        "IInvalid" => Opcode::Invalid,
        "ISelfDestruct" => Opcode::SelfDestruct,
        "IGas" => Opcode::Gas,
        "IPc" => Opcode::Pc,
        "IMSize" => Opcode::MSize,
        "IStop" => Opcode::Stop,
        _ => Opcode::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_simple() {
        let instrs = vec![
            AsmInstruction::Push(vec![0x01]),
            AsmInstruction::Push(vec![0x02]),
            AsmInstruction::Op(Opcode::Add),
        ];
        let sexp = instructions_to_sexp(&instrs);
        let back = sexp_to_instructions(&sexp);
        assert_eq!(back, instrs);
    }

    #[test]
    fn test_roundtrip_push0() {
        let instrs = vec![
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Push0),
            AsmInstruction::Op(Opcode::Add),
        ];
        let sexp = instructions_to_sexp(&instrs);
        assert!(sexp.contains("IPush0"));
        let back = sexp_to_instructions(&sexp);
        assert_eq!(back, instrs);
    }

    #[test]
    fn test_roundtrip_dup_swap() {
        let instrs = vec![
            AsmInstruction::Op(Opcode::Dup1),
            AsmInstruction::Op(Opcode::Swap2),
            AsmInstruction::Op(Opcode::Pop),
        ];
        let sexp = instructions_to_sexp(&instrs);
        assert!(sexp.contains("IDup 1"));
        assert!(sexp.contains("ISwap 2"));
        let back = sexp_to_instructions(&sexp);
        assert_eq!(back, instrs);
    }

    #[test]
    fn test_roundtrip_large_push() {
        // 20-byte address: too large for i64
        let data = vec![0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33, 0x44,
                        0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                        0xee, 0xff];
        let instrs = vec![AsmInstruction::Push(data.clone())];
        let sexp = instructions_to_sexp(&instrs);
        assert!(sexp.contains("PushHex"));
        let back = sexp_to_instructions(&sexp);
        assert_eq!(back, instrs);
    }

    #[test]
    fn test_roundtrip_empty() {
        let instrs: Vec<AsmInstruction> = vec![];
        let sexp = instructions_to_sexp(&instrs);
        assert_eq!(sexp, "(INil)");
        let back = sexp_to_instructions(&sexp);
        assert_eq!(back, instrs);
    }

    #[test]
    fn test_roundtrip_log() {
        let instrs = vec![
            AsmInstruction::Op(Opcode::Log2),
        ];
        let sexp = instructions_to_sexp(&instrs);
        assert!(sexp.contains("ILog 2"));
        let back = sexp_to_instructions(&sexp);
        assert_eq!(back, instrs);
    }

    #[test]
    fn test_try_push_as_i64() {
        assert_eq!(try_push_as_i64(&[0x01]), Some(1));
        assert_eq!(try_push_as_i64(&[0xFF]), Some(255));
        assert_eq!(try_push_as_i64(&[0x01, 0x00]), Some(256));
        // 8 bytes max positive i64
        assert_eq!(try_push_as_i64(&[0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
                   Some(i64::MAX));
        // Too large for i64
        assert_eq!(try_push_as_i64(&[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
                   None);
        // More than 8 bytes
        assert_eq!(try_push_as_i64(&[0x01; 9]), None);
    }
}
