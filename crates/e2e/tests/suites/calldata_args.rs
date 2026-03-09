#![allow(missing_docs)]

use crate::helpers::*;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};

#[test]
fn test_calldata_single_arg() {
    // Create test contract
    std::fs::write("/tmp/test_double.edge", "abi IDouble { fn double(x: u256) -> (u256); }\ncontract Double {\n    pub fn double(x: u256) -> (u256) {\n        return x + x;\n    }\n}\n").unwrap();

    let path = std::path::PathBuf::from("/tmp/test_double.edge");
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    let bytecode = output.bytecode.expect("no bytecode produced");

    let mut evm = EvmHandle::new(bytecode);

    // Call double(5) and expect 10
    let cd = calldata(selector("double(uint256)"), &[encode_u256(5)]);

    let r = evm.call(cd);
    assert!(r.success, "double(5) reverted");
    assert_eq!(decode_u256(&r.output), 10, "double(5) should be 10");
}

#[test]
fn test_calldata_two_args() {
    std::fs::write("/tmp/test_add.edge", "abi IAdd { fn add(a: u256, b: u256) -> (u256); }\ncontract Add {\n    pub fn add(a: u256, b: u256) -> (u256) {\n        return a + b;\n    }\n}\n").unwrap();

    let path = std::path::PathBuf::from("/tmp/test_add.edge");
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    let bytecode = output.bytecode.expect("no bytecode produced");

    let mut evm = EvmHandle::new(bytecode);

    // Call add(3, 5) and expect 8
    let cd = calldata(selector("add(uint256,uint256)"), &[encode_u256(3), encode_u256(5)]);

    let r = evm.call(cd);
    assert!(r.success, "add(3, 5) reverted");
    assert_eq!(decode_u256(&r.output), 8, "add(3, 5) should be 8");
}

#[test]
fn test_calldata_three_args() {
    std::fs::write("/tmp/test_sum.edge", "abi ISum { fn sum(a: u256, b: u256, c: u256) -> (u256); }\ncontract Sum {\n    pub fn sum(a: u256, b: u256, c: u256) -> (u256) {\n        return a + b + c;\n    }\n}\n").unwrap();

    let path = std::path::PathBuf::from("/tmp/test_sum.edge");
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    let bytecode = output.bytecode.expect("no bytecode produced");

    let mut evm = EvmHandle::new(bytecode);

    // Call sum(1, 2, 3) and expect 6
    let cd = calldata(
        selector("sum(uint256,uint256,uint256)"),
        &[encode_u256(1), encode_u256(2), encode_u256(3)],
    );

    let r = evm.call(cd);
    assert!(r.success, "sum(1, 2, 3) reverted");
    assert_eq!(decode_u256(&r.output), 6, "sum(1, 2, 3) should be 6");
}
