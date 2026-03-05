//! Tests that range analysis correctly elides checked arithmetic when
//! U256 bounds prove overflow is impossible, and keeps checks when it can't.

use alloy_primitives::U256;
use edge_evm_tests::{abi_decode_u256, abi_encode_u256, EvmTestHost};

const PATH: &str = "../../examples/test_checked_elision.edge";
const SIMPLE: &str = "../../examples/test_elision_simple.edge";

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

// ═══════════════════════════════════════════════════════════════════
// Correctness: functions produce correct results at all opt levels
// ═══════════════════════════════════════════════════════════════════

#[test]
fn add_small_constants_correct() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let r = h.call_fn("add_small_constants(uint256)", &encode(200));
        assert!(r.success, "O{opt}: should succeed");
        assert_eq!(decode(&r.output), u(201), "O{opt}: 200&0xff + 1 = 201");
    }
}

#[test]
fn add_masked_correct() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let r = h.call_fn("add_masked(uint256,uint256)", &encode2(100, 200));
        assert!(r.success, "O{opt}: should succeed");
        assert_eq!(decode(&r.output), u(300), "O{opt}: 100+200=300");
    }
}

#[test]
fn mul_small_correct() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let r = h.call_fn("mul_small(uint256,uint256)", &encode2(7, 9));
        assert!(r.success, "O{opt}: should succeed");
        assert_eq!(decode(&r.output), u(63), "O{opt}: 7*9=63");
    }
}

#[test]
fn cascade_add_correct() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let r = h.call_fn("cascade_add(uint256)", &encode(100));
        assert!(r.success, "O{opt}: should succeed");
        assert_eq!(decode(&r.output), u(103), "O{opt}: 100+1+2=103");
    }
}

#[test]
fn add_unconstrained_correct() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let r = h.call_fn("add_unconstrained(uint256,uint256)", &encode2(10, 20));
        assert!(r.success, "O{opt}: should succeed");
        assert_eq!(decode(&r.output), u(30), "O{opt}: 10+20=30");
    }
}

#[test]
fn mul_unconstrained_correct() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let r = h.call_fn("mul_unconstrained(uint256,uint256)", &encode2(7, 8));
        assert!(r.success, "O{opt}: should succeed");
        assert_eq!(decode(&r.output), u(56), "O{opt}: 7*8=56");
    }
}

// ═══════════════════════════════════════════════════════════════════
// Overflow reverts: unconstrained ops must revert on overflow
// ═══════════════════════════════════════════════════════════════════

#[test]
fn add_overflow_reverts() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let max = abi_encode_u256(U256::MAX);
        let one = encode(1);
        let mut args = max;
        args.extend_from_slice(&one);
        let r = h.call_fn("add_unconstrained(uint256,uint256)", &args);
        assert!(!r.success, "O{opt}: MAX + 1 should revert");
    }
}

#[test]
fn mul_overflow_reverts() {
    for opt in 0..=2 {
        let mut h = EvmTestHost::deploy_edge(PATH, opt);
        let max = abi_encode_u256(U256::MAX);
        let two = encode(2);
        let mut args = max;
        args.extend_from_slice(&two);
        let r = h.call_fn("mul_unconstrained(uint256,uint256)", &args);
        assert!(!r.success, "O{opt}: MAX * 2 should revert");
    }
}

#[test]
fn mul_by_zero_no_revert() {
    let mut h = EvmTestHost::deploy_edge(PATH, 0);
    let zero = encode(0);
    let max = abi_encode_u256(U256::MAX);
    let mut args = zero;
    args.extend_from_slice(&max);
    let r = h.call_fn("mul_unconstrained(uint256,uint256)", &args);
    assert!(r.success, "0 * MAX should not revert");
    assert_eq!(decode(&r.output), u(0));
}

// ═══════════════════════════════════════════════════════════════════
// Elision verification: O1 produces smaller code by removing
// overflow check opcodes when range analysis proves safety
// ═══════════════════════════════════════════════════════════════════

#[test]
fn elision_produces_smaller_code() {
    let mut h0 = EvmTestHost::deploy_edge(SIMPLE, 0);
    let mut h1 = EvmTestHost::deploy_edge(SIMPLE, 1);

    let o0_size = h0.runtime_code_size();
    let o1_size = h1.runtime_code_size();

    assert!(
        o1_size < o0_size,
        "O1 ({} bytes) should be smaller than O0 ({} bytes) — elision removes overflow check",
        o1_size, o0_size,
    );

    // Verify correctness at both levels
    let r0 = h0.call_fn("bounded_add(uint256)", &encode(42));
    let r1 = h1.call_fn("bounded_add(uint256)", &encode(42));
    assert!(r0.success && r1.success);
    assert_eq!(decode(&r0.output), decode(&r1.output));
}

#[test]
fn elision_removes_overflow_check_opcodes() {
    // O0 bytecode should contain GT (0x11) from the overflow check.
    // O1 bytecode should NOT — the check was elided by range analysis.
    let o0 = edge_evm_tests::compile_edge(SIMPLE, 0);
    let o1 = edge_evm_tests::compile_edge(SIMPLE, 1);

    let o0_hex: String = o0.iter().map(|b| format!("{b:02x}")).collect();
    let o1_hex: String = o1.iter().map(|b| format!("{b:02x}")).collect();

    assert!(o0_hex.contains("11"), "O0 should contain GT (0x11) for overflow check");
    assert!(!o1_hex.contains("11"), "O1 should NOT contain GT — check elided");
}

#[test]
fn multi_function_elision_reduces_size() {
    let mut h0 = EvmTestHost::deploy_edge(PATH, 0);
    let mut h1 = EvmTestHost::deploy_edge(PATH, 1);

    assert!(
        h1.runtime_code_size() < h0.runtime_code_size(),
        "O1 ({} bytes) should be smaller than O0 ({} bytes)",
        h1.runtime_code_size(), h0.runtime_code_size(),
    );
}

#[test]
fn unconstrained_keeps_overflow_check_at_o1() {
    // Unconstrained add must still revert on overflow at O1
    let mut h = EvmTestHost::deploy_edge(PATH, 1);
    let max = abi_encode_u256(U256::MAX);
    let one = encode(1);
    let mut args = max;
    args.extend_from_slice(&one);
    let r = h.call_fn("add_unconstrained(uint256,uint256)", &args);
    assert!(!r.success, "unconstrained MAX + 1 should revert even at O1");
}
