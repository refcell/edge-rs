//! Spec-compliance stress tests covering all features that compile end-to-end.
//! Tests operators, functions, control flow, storage, events, builtins, constants.

use alloy_primitives::{Address, U256};
use edge_evm_tests::{abi_decode_u256, abi_encode_address, abi_encode_u256, EvmTestHost};

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn decode(data: &[u8]) -> U256 {
    abi_decode_u256(data)
}

fn u(val: u64) -> U256 {
    U256::from(val)
}

fn encode(val: u64) -> Vec<u8> {
    abi_encode_u256(U256::from(val))
}

fn encode2(a: u64, b: u64) -> Vec<u8> {
    let mut v = encode(a);
    v.extend_from_slice(&encode(b));
    v
}

fn encode3(a: u64, b: u64, c: u64) -> Vec<u8> {
    let mut v = encode(a);
    v.extend_from_slice(&encode(b));
    v.extend_from_slice(&encode(c));
    v
}

// ═══════════════════════════════════════════════════════════════════
// test_operators
// ═══════════════════════════════════════════════════════════════════

fn deploy_operators() -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/test_operators.edge", 0)
}

#[test]
fn op_add() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_add(uint256,uint256)", &encode2(10, 20));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(30));
}

#[test]
fn op_sub() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_sub(uint256,uint256)", &encode2(50, 20));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(30));
}

#[test]
fn op_mul() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_mul(uint256,uint256)", &encode2(7, 8));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(56));
}

#[test]
fn op_div() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_div(uint256,uint256)", &encode2(100, 3));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(33)); // integer division
}

#[test]
fn op_mod() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_mod(uint256,uint256)", &encode2(100, 7));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(2)); // 100 % 7 = 2
}

#[test]
fn op_exp() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_exp(uint256,uint256)", &encode2(2, 10));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1024));
}

#[test]
fn op_bitwise_and() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_and(uint256,uint256)", &encode2(0xff, 0x0f));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0x0f));
}

#[test]
fn op_bitwise_or() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_or(uint256,uint256)", &encode2(0xf0, 0x0f));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0xff));
}

#[test]
fn op_bitwise_xor() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_xor(uint256,uint256)", &encode2(0xff, 0x0f));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0xf0));
}

#[test]
fn op_shl() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_shl(uint256,uint256)", &encode2(1, 8));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(256));
}

#[test]
fn op_shr() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_shr(uint256,uint256)", &encode2(256, 4));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(16));
}

#[test]
fn op_not() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_not(uint256)", &encode(0));
    assert!(r.success);
    // ~0 in 256-bit = all 1s = 2^256 - 1
    assert_eq!(decode(&r.output), U256::MAX);
}

#[test]
fn op_eq_true() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_eq(uint256,uint256)", &encode2(42, 42));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));
}

#[test]
fn op_eq_false() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_eq(uint256,uint256)", &encode2(42, 43));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn op_neq() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_neq(uint256,uint256)", &encode2(10, 20));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("test_neq(uint256,uint256)", &encode2(10, 10));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn op_lt() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_lt(uint256,uint256)", &encode2(5, 10));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("test_lt(uint256,uint256)", &encode2(10, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));

    let r = h.call_fn("test_lt(uint256,uint256)", &encode2(5, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn op_gt() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_gt(uint256,uint256)", &encode2(10, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("test_gt(uint256,uint256)", &encode2(5, 10));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn op_lte() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_lte(uint256,uint256)", &encode2(5, 10));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("test_lte(uint256,uint256)", &encode2(5, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("test_lte(uint256,uint256)", &encode2(10, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn op_gte() {
    let mut h = deploy_operators();
    let r = h.call_fn("test_gte(uint256,uint256)", &encode2(10, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("test_gte(uint256,uint256)", &encode2(5, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("test_gte(uint256,uint256)", &encode2(5, 10));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn op_precedence() {
    let mut h = deploy_operators();
    // a + b * c = 2 + 3 * 4 = 2 + 12 = 14 (mul before add)
    let r = h.call_fn(
        "test_precedence(uint256,uint256,uint256)",
        &encode3(2, 3, 4),
    );
    assert!(r.success);
    assert_eq!(decode(&r.output), u(14));
}

#[test]
fn op_complex_expr() {
    let mut h = deploy_operators();
    // (a*a + b*b) / (a + b) = (9 + 16) / (3 + 4) = 25 / 7 = 3
    let r = h.call_fn("test_complex_expr(uint256,uint256)", &encode2(3, 4));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(3));
}

// ═══════════════════════════════════════════════════════════════════
// test_functions
// ═══════════════════════════════════════════════════════════════════

fn deploy_functions() -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/test_functions.edge", 0)
}

#[test]
fn fn_double() {
    let mut h = deploy_functions();
    let r = h.call_fn("call_double(uint256)", &encode(7));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(14));
}

#[test]
fn fn_add_three() {
    let mut h = deploy_functions();
    let r = h.call_fn(
        "call_add_three(uint256,uint256,uint256)",
        &encode3(10, 20, 30),
    );
    assert!(r.success);
    assert_eq!(decode(&r.output), u(60));
}

#[test]
fn fn_chain() {
    let mut h = deploy_functions();
    // square(double(5)) = square(10) = 100
    let r = h.call_fn("call_chain(uint256)", &encode(5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(100));
}

#[test]
fn fn_max() {
    let mut h = deploy_functions();
    let r = h.call_fn("call_max(uint256,uint256)", &encode2(10, 20));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(20));

    let r = h.call_fn("call_max(uint256,uint256)", &encode2(30, 5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(30));
}

#[test]
fn fn_clamp() {
    let mut h = deploy_functions();
    // clamp(5, 10, 100) = max(5, 10) = 10, min(10, 100) = 10
    let r = h.call_fn("call_clamp(uint256,uint256,uint256)", &encode3(5, 10, 100));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(10));

    // clamp(50, 10, 100) = max(50, 10) = 50, min(50, 100) = 50
    let r = h.call_fn("call_clamp(uint256,uint256,uint256)", &encode3(50, 10, 100));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(50));

    // clamp(200, 10, 100) = max(200, 10) = 200, min(200, 100) = 100
    let r = h.call_fn(
        "call_clamp(uint256,uint256,uint256)",
        &encode3(200, 10, 100),
    );
    assert!(r.success);
    assert_eq!(decode(&r.output), u(100));
}

// ═══════════════════════════════════════════════════════════════════
// test_control_flow (storage-based loops — should work today)
// ═══════════════════════════════════════════════════════════════════

fn deploy_control_flow() -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/test_control_flow.edge", 0)
}

#[test]
fn cf_if_else_return() {
    let mut h = deploy_control_flow();
    // x=200 > 100 → 200*2 = 400
    let r = h.call_fn("if_else_return(uint256)", &encode(200));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(400));

    // x=50 <= 100 → 50+10 = 60
    let r = h.call_fn("if_else_return(uint256)", &encode(50));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(60));
}

#[test]
fn cf_nested_if() {
    let mut h = deploy_control_flow();
    let r = h.call_fn("nested_if(uint256)", &encode(3000));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(3));

    let r = h.call_fn("nested_if(uint256)", &encode(1500));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(2));

    let r = h.call_fn("nested_if(uint256)", &encode(750));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    let r = h.call_fn("nested_if(uint256)", &encode(100));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn cf_multi_branch_return() {
    let mut h = deploy_control_flow();
    let cases = [(0, 100), (1, 200), (2, 300), (3, 400), (99, 999)];
    for (input, expected) in cases {
        let r = h.call_fn("multi_branch_return(uint256)", &encode(input));
        assert!(r.success, "multi_branch_return({input}) failed");
        assert_eq!(
            decode(&r.output),
            u(expected),
            "multi_branch_return({input})"
        );
    }
}

#[test]
fn cf_storage_while_sum() {
    let mut h = deploy_control_flow();
    // sum(1..10) = 55
    let r = h.call_fn("storage_while_sum(uint256)", &encode(10));
    println!(
        "r: {:?}",
        (
            alloy_primitives::hex::encode(edge_evm_tests::fn_selector(
                "storage_while_sum(uint256)"
            )),
            alloy_primitives::hex::encode(encode(10)),
            &r
        )
    );
    assert!(r.success, "storage_while_sum failed: {:?}", r.output);
    assert_eq!(decode(&r.output), u(55));
}

#[test]
fn cf_storage_for_factorial() {
    let mut h = deploy_control_flow();
    // 5! = 120
    let r = h.call_fn("storage_for_factorial(uint256)", &encode(5));
    assert!(r.success, "storage_for_factorial failed: {:?}", r.output);
    assert_eq!(decode(&r.output), u(120));
}

#[test]
fn cf_early_return_loop() {
    let mut h = deploy_control_flow();
    // First power of 2 >= 100 → 128
    let r = h.call_fn("early_return_loop(uint256)", &encode(100));
    assert!(r.success, "early_return_loop failed: {:?}", r.output);
    assert_eq!(decode(&r.output), u(128));

    // First power of 2 >= 1 → 1
    let r = h.call_fn("early_return_loop(uint256)", &encode(1));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));
}

// ═══════════════════════════════════════════════════════════════════
// test_storage_heavy
// ═══════════════════════════════════════════════════════════════════

fn deploy_storage_heavy() -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/test_storage_heavy.edge", 0)
}

fn encode5(a: u64, b: u64, c: u64, d: u64, e: u64) -> Vec<u8> {
    let mut v = encode(a);
    v.extend_from_slice(&encode(b));
    v.extend_from_slice(&encode(c));
    v.extend_from_slice(&encode(d));
    v.extend_from_slice(&encode(e));
    v
}

fn addr(b: u8) -> Address {
    Address::from([b; 20])
}

#[test]
fn storage_set_and_sum() {
    let mut h = deploy_storage_heavy();
    let r = h.call_fn(
        "set_all(uint256,uint256,uint256,uint256,uint256)",
        &encode5(10, 20, 30, 40, 50),
    );
    assert!(r.success, "set_all failed");

    let r = h.call_fn("get_sum()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(150));
}

#[test]
fn storage_get_field() {
    let mut h = deploy_storage_heavy();
    h.call_fn(
        "set_all(uint256,uint256,uint256,uint256,uint256)",
        &encode5(100, 200, 300, 400, 500),
    );

    for (idx, expected) in [(0, 100), (1, 200), (2, 300), (3, 400), (4, 500)] {
        let r = h.call_fn("get_field(uint256)", &encode(idx));
        assert!(r.success);
        assert_eq!(decode(&r.output), u(expected), "get_field({idx})");
    }
}

#[test]
fn storage_mapping_ops() {
    let mut h = deploy_storage_heavy();
    let alice = addr(0x0A);

    // Set and get
    let mut args = abi_encode_address(alice);
    args.extend_from_slice(&encode(1000));
    h.call_fn("map_set(address,uint256)", &args);

    let r = h.call_fn("map_get(address)", &abi_encode_address(alice));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1000));

    // Add
    let mut args = abi_encode_address(alice);
    args.extend_from_slice(&encode(500));
    h.call_fn("map_add(address,uint256)", &args);

    let r = h.call_fn("map_get(address)", &abi_encode_address(alice));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1500));
}

#[test]
fn storage_nested_mapping() {
    let mut h = deploy_storage_heavy();
    let alice = addr(0x0A);
    let bob = addr(0x0B);

    // Set nested mapping
    let mut args = abi_encode_address(alice);
    args.extend_from_slice(&abi_encode_address(bob));
    args.extend_from_slice(&encode(999));
    h.call_fn("nested_map_set(address,address,uint256)", &args);

    // Get nested mapping
    let mut args = abi_encode_address(alice);
    args.extend_from_slice(&abi_encode_address(bob));
    let r = h.call_fn("nested_map_get(address,address)", &args);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(999));
}

#[test]
fn storage_complex_op() {
    let mut h = deploy_storage_heavy();
    let alice = addr(0x0A);

    // Set field_a = 10
    h.call_fn(
        "set_all(uint256,uint256,uint256,uint256,uint256)",
        &encode5(10, 0, 0, 0, 0),
    );

    // complex_storage_op: balances[key] = balances[key] + amount + field_a
    // balances[alice] = 0 + 100 + 10 = 110, field_e becomes 1
    let mut args = abi_encode_address(alice);
    args.extend_from_slice(&encode(100));
    let r = h.call_fn("complex_storage_op(address,uint256)", &args);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(110));
}

// ═══════════════════════════════════════════════════════════════════
// test_events_heavy
// ═══════════════════════════════════════════════════════════════════

fn deploy_events() -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/test_events_heavy.edge", 0)
}

#[test]
fn events_no_indexed() {
    let mut h = deploy_events();
    let r = h.call_fn("emit_no_indexed(uint256)", &encode(42));
    assert!(r.success);
    assert_eq!(r.logs.len(), 1);
    // LOG1: 1 topic (event sig hash)
    assert_eq!(r.logs[0].topics().len(), 1);

    // Verify marker was set
    let r = h.call_fn("get_marker()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));
}

#[test]
fn events_one_indexed() {
    let mut h = deploy_events();
    let r = h.call_fn("emit_one_indexed(uint256,uint256)", &encode2(99, 42));
    assert!(r.success);
    assert_eq!(r.logs.len(), 1);
    // LOG2: 2 topics (event sig + indexed key)
    assert_eq!(r.logs[0].topics().len(), 2);
}

#[test]
fn events_two_indexed() {
    let mut h = deploy_events();
    let from = addr(0x0A);
    let to = addr(0x0B);
    let mut args = abi_encode_address(from);
    args.extend_from_slice(&abi_encode_address(to));
    args.extend_from_slice(&encode(1000));
    let r = h.call_fn("emit_two_indexed(address,address,uint256)", &args);
    assert!(r.success);
    assert_eq!(r.logs.len(), 1);
    // LOG3: 3 topics (event sig + from + to)
    assert_eq!(r.logs[0].topics().len(), 3);
}

#[test]
fn events_three_indexed() {
    let mut h = deploy_events();
    let r = h.call_fn(
        "emit_three_indexed(uint256,uint256,uint256)",
        &encode3(1, 2, 3),
    );
    assert!(r.success);
    assert_eq!(r.logs.len(), 1);
    // LOG4: 4 topics (event sig + a + b + c)
    assert_eq!(r.logs[0].topics().len(), 4);
}

#[test]
fn events_multiple_emits() {
    let mut h = deploy_events();
    let r = h.call_fn("emit_multiple(uint256,uint256)", &encode2(10, 20));
    assert!(r.success);
    // 3 events emitted
    assert_eq!(r.logs.len(), 3);

    let r = h.call_fn("get_marker()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(6));
}

// ═══════════════════════════════════════════════════════════════════
// test_builtins
// ═══════════════════════════════════════════════════════════════════

fn deploy_builtins() -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/test_builtins.edge", 0)
}

#[test]
fn builtin_caller() {
    let mut h = deploy_builtins();
    let r = h.call_fn("get_caller()", &[]);
    assert!(r.success);
    // Caller is Address::from([0x01; 20])
    let caller_addr = h.caller();
    let returned = Address::from_slice(&r.output[12..32]);
    assert_eq!(returned, caller_addr);
}

#[test]
fn builtin_is_caller() {
    let mut h = deploy_builtins();
    let caller = h.caller();
    let r = h.call_fn("is_caller(address)", &abi_encode_address(caller));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));

    // Wrong caller
    let r = h.call_fn("is_caller(address)", &abi_encode_address(addr(0xFF)));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0));
}

#[test]
fn builtin_address() {
    let mut h = deploy_builtins();
    let contract_addr = h.address();
    let r = h.call_fn("get_address()", &[]);
    assert!(r.success);
    let returned = Address::from_slice(&r.output[12..32]);
    assert_eq!(returned, contract_addr);
}

#[test]
fn builtin_check_no_value() {
    let mut h = deploy_builtins();
    let r = h.call_fn("check_no_value()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1));
}

// ═══════════════════════════════════════════════════════════════════
// test_constants
// ═══════════════════════════════════════════════════════════════════

fn deploy_constants() -> EvmTestHost {
    EvmTestHost::deploy_edge("../../examples/test_constants.edge", 0)
}

#[test]
fn const_decimal() {
    let mut h = deploy_constants();
    let r = h.call_fn("get_decimal()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(42));
}

#[test]
fn const_hex() {
    let mut h = deploy_constants();
    let r = h.call_fn("get_hex()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(0xff));
}

#[test]
fn const_large_decimal() {
    let mut h = deploy_constants();
    let r = h.call_fn("get_large_decimal()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(1_000_000));
}

#[test]
fn const_literal_arithmetic() {
    let mut h = deploy_constants();
    // 100 + 200 * 3 = 100 + 600 = 700
    let r = h.call_fn("literal_arithmetic()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(700));
}

#[test]
fn const_hex_arithmetic() {
    let mut h = deploy_constants();
    // 0x10 + 0x20 = 16 + 32 = 48
    let r = h.call_fn("hex_arithmetic()", &[]);
    assert!(r.success);
    assert_eq!(decode(&r.output), u(48));
}

#[test]
fn const_mixed_literals() {
    let mut h = deploy_constants();
    // 5 * 1000 + 42 = 5042
    let r = h.call_fn("mixed_literals(uint256)", &encode(5));
    assert!(r.success);
    assert_eq!(decode(&r.output), u(5042));
}
