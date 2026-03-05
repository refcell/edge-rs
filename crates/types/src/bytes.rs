//! Byte utilities and helpers.

use std::num::ParseIntError;

use alloy_primitives::B256;
use tiny_keccak::{Hasher, Keccak};

/// Convert a **decimal** integer string to a [`B256`].
/// Parses the string as a base-10 integer and stores it big-endian in 32 bytes.
/// i.e. "256" becomes `[0, 0, ..., 0, 1, 0]`
pub fn decimal_str_to_bytes32(s: &str) -> Result<B256, ParseIntError> {
    let clean = s.replace('_', "");
    let value: u128 = clean.parse()?;
    let value_bytes = value.to_be_bytes(); // [u8; 16]
    let mut padded = [0u8; 32];
    padded[16..].copy_from_slice(&value_bytes);
    Ok(B256::from(padded))
}

/// Convert a string slice to a [`B256`].
/// Pads zeros to the left of significant bytes.
/// i.e. 0xa57b becomes `[0, 0, ..., 0, 165, 123]`
pub fn str_to_bytes32(s: &str) -> Result<B256, ParseIntError> {
    let s = format_even_bytes(String::from(s));

    let bytes: Vec<u8> = (0..s.len())
        .step_by(2)
        .map(|c| u8::from_str_radix(&s[c..c + 2], 16))
        .collect::<Result<Vec<u8>, _>>()?;

    let mut padded = [0u8; 32];

    for i in 32 - bytes.len()..32 {
        padded[i] = bytes[bytes.len() - (32 - i)];
    }

    Ok(B256::from(padded))
}

/// Convert a [`B256`] to a bytes string.
pub fn bytes32_to_string(bytes: &B256, prefixed: bool) -> String {
    let mut s = String::default();
    let start = bytes
        .iter()
        .position(|b| *b != 0)
        .unwrap_or(bytes.len() - 1);
    for b in &bytes[start..bytes.len()] {
        s = format!("{s}{:02x}", *b);
    }
    format!("{}{s}", if prefixed { "0x" } else { "" })
}

/// Convert a decimal string to a `[u8; 32]`.
/// Unlike `str_to_bytes32` (which treats input as hex), this parses
/// the string as a base-10 integer.
pub fn decimal_to_bytes32(s: &str) -> [u8; 32] {
    // Parse as u128 first (handles up to ~39 decimal digits)
    if let Ok(val) = s.parse::<u128>() {
        let mut padded = [0u8; 32];
        let bytes = val.to_be_bytes();
        padded[16..32].copy_from_slice(&bytes);
        return padded;
    }
    // Fallback for very large decimals: convert to hex string, then use str_to_bytes32
    let hex = decimal_str_to_hex(s);
    // This is called from the lexer for decimal literals which should always be valid
    str_to_bytes32(&hex)
        .expect("decimal_to_bytes32: invalid decimal string")
        .into()
}

/// Convert a decimal digit string to a hex string (no prefix).
fn decimal_str_to_hex(s: &str) -> String {
    // Manual big-number conversion: multiply-and-add
    let mut result: Vec<u8> = vec![0]; // big-endian bytes
    for ch in s.chars() {
        let digit = ch as u8 - b'0';
        // Multiply result by 10
        let mut carry: u16 = 0;
        for byte in result.iter_mut().rev() {
            let prod = (*byte as u16) * 10 + carry;
            *byte = (prod & 0xff) as u8;
            carry = prod >> 8;
        }
        if carry > 0 {
            result.insert(0, carry as u8);
        }
        // Add digit
        let mut carry: u16 = digit as u16;
        for byte in result.iter_mut().rev() {
            let sum = (*byte as u16) + carry;
            *byte = (sum & 0xff) as u8;
            carry = sum >> 8;
        }
        if carry > 0 {
            result.insert(0, carry as u8);
        }
    }
    // Convert to hex string
    result.iter().map(|b| format!("{b:02x}")).collect()
}

/// Wrapper to convert a hex string to a usize.
pub fn hex_to_usize(s: &str) -> Result<usize, ParseIntError> {
    usize::from_str_radix(s, 16)
}

/// Pad a hex string with n 0 bytes to the left. Will not pad a hex string that has a length
/// greater than or equal to `num_bytes * 2`
pub fn pad_n_bytes(hex: &str, num_bytes: usize) -> String {
    let mut hex = hex.to_owned();
    while hex.len() < num_bytes * 2 {
        hex = format!("0{hex}");
    }
    hex
}

/// Pad odd-length byte string with a leading 0
pub fn format_even_bytes(hex: String) -> String {
    if hex.len() % 2 == 1 {
        format!("0{hex}")
    } else {
        hex
    }
}

/// Convert string slice to `Vec<u8>`, size not capped
pub fn str_to_vec(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    let bytes: Result<Vec<u8>, _> = (0..s.len())
        .step_by(2)
        .map(|c| u8::from_str_radix(&s[c..c + 2], 16))
        .collect();
    bytes
}

/// Hash a string with Keccak256
pub fn hash_bytes(dest: &mut [u8], to_hash: &str) {
    let mut hasher = Keccak::v256();
    hasher.update(to_hash.as_bytes());
    hasher.finalize(dest);
}

/// Converts a literal into its bytecode string representation
pub fn format_literal(hex_literal: String) -> String {
    format!("{:02x}{hex_literal}", 95 + hex_literal.len() / 2)
}
