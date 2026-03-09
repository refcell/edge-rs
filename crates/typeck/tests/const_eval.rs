#![allow(missing_docs)]

use edge_typeck::{ConstValue, TypeChecker};

/// Helper: parse + typecheck Edge source and return const values.
fn compile_consts(source: &str) -> Vec<ConstValue> {
    let ast = edge_parser::parse(source).expect("parse failed");
    let checked = TypeChecker::new().check(&ast).expect("typecheck failed");

    checked
        .contracts
        .into_iter()
        .flat_map(|c| c.consts)
        .collect()
}

/// Convert a [u8; 32] big-endian value to a hex string for easier assertions.
fn to_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn large_literal_above_u64_max() {
    // 10^21 = 0x3635C9ADC5DEA00000 — larger than u64::MAX
    let consts = compile_consts(
        r#"contract Test {
            const X: u256 = 1000000000000000000000;
        }"#,
    );
    assert_eq!(consts.len(), 1);
    assert_eq!(consts[0].name, "X");

    let hex = to_hex(&consts[0].value);
    assert!(
        hex.ends_with("3635c9adc5dea00000"),
        "expected 10^21, got 0x{hex}"
    );
}

#[test]
fn const_arithmetic_multiplication() {
    let consts = compile_consts(
        r#"contract Test {
            const A: u256 = 100;
            const B: u256 = A * A;
        }"#,
    );
    assert_eq!(consts.len(), 2);
    assert_eq!(consts[1].name, "B");
    // 100 * 100 = 10000 = 0x2710
    assert_eq!(consts[1].value[31], 0x10);
    assert_eq!(consts[1].value[30], 0x27);
    assert!(consts[1].value[..30].iter().all(|&b| b == 0));
}

#[test]
fn shift_left_128_not_zero() {
    let consts = compile_consts(
        r#"contract Test {
            const S: u256 = 1 << 128;
        }"#,
    );
    assert_eq!(consts.len(), 1);
    assert_eq!(consts[0].name, "S");
    assert!(
        consts[0].value.iter().any(|&b| b != 0),
        "1 << 128 should not be zero"
    );
    // Byte at index 15 (big-endian): bit 128 = byte 16 from end = index 15
    assert_eq!(consts[0].value[15], 1, "bit 128 should be set");
    assert!(consts[0].value[16..].iter().all(|&b| b == 0));
}

#[test]
fn small_value_u8() {
    let consts = compile_consts(
        r#"contract Test {
            const X: u8 = 42;
        }"#,
    );
    assert_eq!(consts.len(), 1);
    assert_eq!(consts[0].name, "X");
    assert_eq!(consts[0].value[31], 42);
    assert!(consts[0].value[..31].iter().all(|&b| b == 0));
}
