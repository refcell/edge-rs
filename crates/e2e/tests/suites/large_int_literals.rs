#![allow(missing_docs)]

use edge_driver::compiler::Compiler;

fn compile_source(src: &str) -> Vec<u8> {
    let mut compiler = Compiler::from_source(src);
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn large_literal_two_pow_64() {
    let src = r#"
contract Test {
    pub fn get() -> u256 {
        return 18446744073709551616;
    }
}
"#;
    let bytecode = compile_source(src);
    let expected: [u8; 9] = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(
        contains_bytes(&bytecode, &expected),
        "bytecode should contain 2^64: {}",
        to_hex(&bytecode)
    );
}

#[test]
fn large_literal_10_pow_21() {
    let src = r#"
contract Test {
    pub fn get() -> u256 {
        return 1000000000000000000000;
    }
}
"#;
    let bytecode = compile_source(src);
    let expected = from_hex("3635c9adc5dea00000");
    assert!(
        contains_bytes(&bytecode, &expected),
        "bytecode should contain 10^21: {}",
        to_hex(&bytecode)
    );
}

#[test]
fn large_literal_u256_max() {
    let src = r#"
contract Test {
    pub fn get() -> u256 {
        return 115792089237316195423570985008687907853269984665640564039457584007913129639935;
    }
}
"#;
    let bytecode = compile_source(src);
    let expected = [0xffu8; 32];
    assert!(
        contains_bytes(&bytecode, &expected),
        "bytecode should contain 32 bytes of 0xff for MAX_UINT: {}",
        to_hex(&bytecode)
    );
}

#[test]
fn small_literal_not_regressed() {
    let src = r#"
contract Test {
    pub fn get() -> u256 {
        return 42;
    }
}
"#;
    let bytecode = compile_source(src);
    assert!(
        contains_bytes(&bytecode, &[0x60, 0x2a]),
        "bytecode should contain PUSH1 42: {}",
        to_hex(&bytecode)
    );
}
