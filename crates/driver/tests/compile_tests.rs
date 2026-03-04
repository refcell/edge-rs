//! End-to-end integration tests for the Edge compiler pipeline.

use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};

/// Helper to compile a file and return the bytecode.
fn compile_to_bytecode(path: &str, opt_level: u8) -> Vec<u8> {
    let mut config = CompilerConfig::new(PathBuf::from(path));
    config.optimization_level = opt_level;
    let mut compiler = Compiler::new(config).expect("failed to create compiler");
    let output = compiler.compile().expect("compilation failed");
    output.bytecode.expect("no bytecode generated")
}

/// Helper to compile a file and return the IR.
fn compile_to_ir(path: &str, opt_level: u8) -> edge_ir::EvmProgram {
    let mut config = CompilerConfig::new(PathBuf::from(path));
    config.emit = EmitKind::Ir;
    config.optimization_level = opt_level;
    let mut compiler = Compiler::new(config).expect("failed to create compiler");
    let output = compiler.compile().expect("compilation failed");
    output.ir.expect("no IR generated")
}

fn bytecode_to_hex(bytecode: &[u8]) -> String {
    bytecode.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_contains_opcode(hex: &str, opcode_hex: &str) -> bool {
    hex.contains(opcode_hex)
}

// ============================================================
// Counter.edge tests
// ============================================================

#[test]
fn counter_compiles_o0() {
    let bytecode = compile_to_bytecode("../../examples/counter.edge", 0);
    assert!(!bytecode.is_empty(), "counter bytecode should not be empty");
}

#[test]
fn counter_compiles_o1() {
    let bytecode = compile_to_bytecode("../../examples/counter.edge", 1);
    assert!(!bytecode.is_empty(), "counter -O1 bytecode should not be empty");
}

#[test]
fn counter_has_correct_selectors() {
    let hex = bytecode_to_hex(&compile_to_bytecode("../../examples/counter.edge", 0));
    // increment() selector: d09de08a
    assert!(hex.contains("d09de08a"), "missing increment() selector");
    // decrement() selector: 2baeceb7
    assert!(hex.contains("2baeceb7"), "missing decrement() selector");
    // get() selector: 6d4ce63c
    assert!(hex.contains("6d4ce63c"), "missing get() selector");
    // reset() selector: d826f88f
    assert!(hex.contains("d826f88f"), "missing reset() selector");
}

#[test]
fn counter_has_expected_opcodes() {
    let hex = bytecode_to_hex(&compile_to_bytecode("../../examples/counter.edge", 0));
    // SLOAD = 0x54
    assert!(hex_contains_opcode(&hex, "54"), "missing SLOAD");
    // SSTORE = 0x55
    assert!(hex_contains_opcode(&hex, "55"), "missing SSTORE");
    // RETURN = 0xf3
    assert!(hex_contains_opcode(&hex, "f3"), "missing RETURN");
}

#[test]
fn counter_ir_output() {
    let ir = compile_to_ir("../../examples/counter.edge", 0);
    assert_eq!(ir.contracts.len(), 1, "should have exactly one contract");
    assert_eq!(ir.contracts[0].name, "Counter");

    // Verify the IR can be converted to s-expression
    let sexp = edge_ir::sexp::expr_to_sexp(&ir.contracts[0].runtime);
    assert!(!sexp.is_empty(), "IR s-expression should not be empty");
    assert!(sexp.contains("Concat"), "runtime should use Concat for dispatch");
}

// ============================================================
// ERC20.edge tests
// ============================================================

#[test]
fn erc20_compiles_o0() {
    let bytecode = compile_to_bytecode("../../examples/erc20.edge", 0);
    assert!(!bytecode.is_empty(), "erc20 bytecode should not be empty");
}

#[test]
fn erc20_compiles_o1() {
    let bytecode = compile_to_bytecode("../../examples/erc20.edge", 1);
    assert!(!bytecode.is_empty(), "erc20 -O1 bytecode should not be empty");
}

#[test]
fn erc20_has_correct_selectors() {
    let hex = bytecode_to_hex(&compile_to_bytecode("../../examples/erc20.edge", 0));
    // totalSupply() selector: 18160ddd
    assert!(hex.contains("18160ddd"), "missing totalSupply() selector");
    // balanceOf(address) selector: 70a08231
    assert!(hex.contains("70a08231"), "missing balanceOf() selector");
    // transfer(address,uint256) selector: a9059cbb
    assert!(hex.contains("a9059cbb"), "missing transfer() selector");
    // allowance(address,address) selector: dd62ed3e
    assert!(hex.contains("dd62ed3e"), "missing allowance() selector");
    // approve(address,uint256) selector: 095ea7b3
    assert!(hex.contains("095ea7b3"), "missing approve() selector");
    // transferFrom(address,address,uint256) selector: 23b872dd
    assert!(hex.contains("23b872dd"), "missing transferFrom() selector");
}

#[test]
fn erc20_has_expected_opcodes() {
    let hex = bytecode_to_hex(&compile_to_bytecode("../../examples/erc20.edge", 0));
    // KECCAK256 = 0x20
    assert!(hex_contains_opcode(&hex, "20"), "missing KECCAK256");
    // SLOAD = 0x54
    assert!(hex_contains_opcode(&hex, "54"), "missing SLOAD");
    // SSTORE = 0x55
    assert!(hex_contains_opcode(&hex, "55"), "missing SSTORE");
    // LOG3 = 0xa3 (events with 3 topics)
    assert!(hex_contains_opcode(&hex, "a3"), "missing LOG3");
}

#[test]
fn erc20_ir_output() {
    let ir = compile_to_ir("../../examples/erc20.edge", 0);
    assert_eq!(ir.contracts.len(), 1, "should have exactly one contract");
    assert_eq!(ir.contracts[0].name, "ERC20");

    let sexp = edge_ir::sexp::expr_to_sexp(&ir.contracts[0].runtime);
    assert!(!sexp.is_empty(), "IR s-expression should not be empty");
}

// ============================================================
// Golden file tests
// ============================================================

#[test]
fn counter_golden_o0() {
    let bytecode = compile_to_bytecode("../../examples/counter.edge", 0);
    let hex = bytecode_to_hex(&bytecode);
    let golden = include_str!("golden/counter_o0.hex").trim();
    assert_eq!(hex, golden, "counter.edge -O0 bytecode changed from golden file");
}

#[test]
fn erc20_golden_o0() {
    let bytecode = compile_to_bytecode("../../examples/erc20.edge", 0);
    let hex = bytecode_to_hex(&bytecode);
    let golden = include_str!("golden/erc20_o0.hex").trim();
    assert_eq!(hex, golden, "erc20.edge -O0 bytecode changed from golden file");
}

// ============================================================
// Optimization regression tests
// ============================================================

#[test]
fn optimizable_compiles_o0() {
    let bytecode = compile_to_bytecode("../../examples/optimizable.edge", 0);
    assert!(!bytecode.is_empty());
}

#[test]
fn optimizable_o2_smaller_than_o0() {
    let o0 = compile_to_bytecode("../../examples/optimizable.edge", 0);
    let o2 = compile_to_bytecode("../../examples/optimizable.edge", 2);
    assert!(
        o2.len() < o0.len(),
        "O2 bytecode ({} bytes) should be smaller than O0 ({} bytes)",
        o2.len(),
        o0.len()
    );
}

#[test]
fn optimizable_has_correct_selectors() {
    let hex = bytecode_to_hex(&compile_to_bytecode("../../examples/optimizable.edge", 0));
    // get() selector: 6d4ce63c
    assert!(hex.contains("6d4ce63c"), "missing get() selector");
}

// ============================================================
// O1 roundtrip tests
// ============================================================

#[test]
fn counter_o1_roundtrip_valid() {
    let bytecode = compile_to_bytecode("../../examples/counter.edge", 1);
    let hex = bytecode_to_hex(&bytecode);
    // After optimization, selectors should still be present
    assert!(hex.contains("d09de08a"), "O1: missing increment() selector");
    assert!(hex.contains("6d4ce63c"), "O1: missing get() selector");
}

#[test]
fn erc20_o1_roundtrip_valid() {
    let bytecode = compile_to_bytecode("../../examples/erc20.edge", 1);
    let hex = bytecode_to_hex(&bytecode);
    assert!(hex.contains("18160ddd"), "O1: missing totalSupply() selector");
    assert!(hex.contains("a9059cbb"), "O1: missing transfer() selector");
}
