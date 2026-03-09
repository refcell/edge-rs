#![allow(missing_docs)]

use crate::helpers::*;
use revm::{
    context::{Context, TxEnv},
    database::{CacheDB, EmptyDB},
    primitives::{Address, Bytes, TxKind},
    state::AccountInfo,
    ExecuteCommitEvm, MainBuilder, MainContext,
};

// =============================================================================
// Tests
// =============================================================================

#[test]
fn test_counter_selectors() {
    assert_eq!(selector("increment()"), [0xd0, 0x9d, 0xe0, 0x8a]);
    assert_eq!(selector("decrement()"), [0x2b, 0xae, 0xce, 0xb7]);
    assert_eq!(selector("get()"), [0x6d, 0x4c, 0xe6, 0x3c]);
    assert_eq!(selector("reset()"), [0xd8, 0x26, 0xf8, 0x8f]);
}

#[test]
fn test_counter_compiles() {
    let bytecode = compile_contract("examples/counter.edge");
    assert!(!bytecode.is_empty(), "counter.edge produced empty bytecode");
    assert!(bytecode.len() > 10, "bytecode too short to be valid");
}

#[test]
fn test_counter_encode_call() {
    let sel = selector("get()");
    let cd = calldata(sel, &[]);
    assert_eq!(cd.len(), 4, "calldata should be 4 bytes for get()");
    assert_eq!(cd, vec![0x6d, 0x4c, 0xe6, 0x3c]);
}

/// Minimal sanity check: a contract that just does MSTORE + RETURN works in revm.
#[test]
fn test_revm_return_works() {
    // Bytecode: PUSH1 42, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    // Returns a 32-byte big-endian uint with value 42 in the last byte.
    let bytecode_bytes = vec![
        0x60, 0x2a, // PUSH1 42
        0x60, 0x00, // PUSH1 0
        0x52, // MSTORE (mem[0] = 42)
        0x60, 0x20, // PUSH1 32
        0x60, 0x00, // PUSH1 0
        0xf3, // RETURN(0, 32)
    ];
    // This is raw runtime bytecode (no constructor), so insert directly as code
    let addr = Address::new([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x01,
    ]);
    let code = revm::state::Bytecode::new_legacy(Bytes::from(bytecode_bytes));
    let account = AccountInfo::default().with_code(code);
    let mut db = CacheDB::<EmptyDB>::default();
    db.insert_account_info(addr, account);
    let mut evm = Context::mainnet().with_db(db).build_mainnet();

    let tx = TxEnv::builder()
        .caller(Address::ZERO)
        .kind(TxKind::Call(addr))
        .data(Bytes::new())
        .build()
        .unwrap();
    let result = evm.transact_commit(tx).unwrap();
    assert!(result.is_success(), "minimal RETURN should succeed");
    let out = result.output().unwrap();
    assert_eq!(out.len(), 32, "should return 32 bytes");
    assert_eq!(out[31], 42, "last byte should be 42");
}

/// A freshly deployed counter starts at zero.
#[test]
fn test_counter_get_initial_zero() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    let r = evm.call(calldata(selector("get()"), &[]));
    assert!(r.success, "get() reverted on fresh contract");
    assert_eq!(decode_u256(&r.output), 0, "initial count should be 0");
}

/// Calling `increment()` once should make `get()` return 1.
#[test]
fn test_counter_increment_and_get() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    let r = evm.call(calldata(selector("increment()"), &[]));
    assert!(r.success, "increment() reverted");

    let r = evm.call(calldata(selector("get()"), &[]));
    assert!(r.success, "get() reverted after increment");
    assert_eq!(
        decode_u256(&r.output),
        1,
        "count should be 1 after one increment"
    );
}

/// Full stateful sequence: increment twice, check 2, reset, check 0.
#[test]
fn test_counter_stateful_sequence() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    // Increment twice
    let r = evm.call(calldata(selector("increment()"), &[]));
    assert!(r.success, "first increment() reverted");
    let r = evm.call(calldata(selector("increment()"), &[]));
    assert!(r.success, "second increment() reverted");

    let r = evm.call(calldata(selector("get()"), &[]));
    assert!(r.success, "get() reverted after 2 increments");
    assert_eq!(
        decode_u256(&r.output),
        2,
        "count should be 2 after two increments"
    );

    // Reset to zero
    let r = evm.call(calldata(selector("reset()"), &[]));
    assert!(r.success, "reset() reverted");

    let r = evm.call(calldata(selector("get()"), &[]));
    assert!(r.success, "get() reverted after reset");
    assert_eq!(decode_u256(&r.output), 0, "count should be 0 after reset");
}

/// An unknown selector should hit the revert fallback in the dispatcher.
#[test]
fn test_counter_unknown_selector_reverts() {
    let bytecode = compile_contract("examples/counter.edge");
    let mut evm = EvmHandle::new(bytecode);

    let r = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!r.success, "unknown selector should revert");
}
