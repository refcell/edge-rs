#![allow(missing_docs)]

//! Execution-level acceptance tests for utility library contracts.
//!
//! Tests compile math.edge and bits.edge to bytecode, deploy on an
//! in-memory revm EVM, and verify pure-function output correctness.
//!
//! ## Compiler caveats
//! Top-level constants (WAD, ADDR_MASK, etc.) resolve to 0 when referenced
//! by name in function bodies, so WAD-based helpers (wad_mul, wad_div) are
//! not tested here. Functions that only use parameters and integer literals
//! work correctly.

use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use revm::{
    context::{Context, TxEnv},
    database::{CacheDB, EmptyDB},
    handler::{MainBuilder, MainnetContext},
    primitives::{Address, Bytes, TxKind},
    state::{AccountInfo, Bytecode},
    ExecuteCommitEvm, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

// =============================================================================
// Shared helpers
// =============================================================================

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compile_contract(relative_path: &str) -> Vec<u8> {
    let path = workspace_root().join(relative_path);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Bytecode;
    let mut compiler = Compiler::new(config).expect("compiler init failed");
    let output = compiler.compile().expect("compile failed");
    output.bytecode.expect("no bytecode produced")
}

fn selector(sig: &str) -> [u8; 4] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

fn encode_u256(val: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&val.to_be_bytes());
    out
}

fn decode_u256(output: &[u8]) -> u64 {
    assert!(
        output.len() >= 32,
        "return value too short: {} bytes",
        output.len()
    );
    assert_eq!(&output[0..24], &[0u8; 24], "u256 too large for u64");
    u64::from_be_bytes(output[24..32].try_into().unwrap())
}

fn decode_bool(output: &[u8]) -> bool {
    decode_u256(output) != 0
}

const CONTRACT_ADDR: Address = Address::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x00,
]);

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

struct EvmHandle {
    evm: TestEvm,
    nonce: u64,
}

impl EvmHandle {
    fn new(bytecode: Vec<u8>) -> Self {
        let code = Bytecode::new_legacy(Bytes::from(bytecode));
        let account = AccountInfo::default().with_code(code);
        let mut db = CacheDB::<EmptyDB>::default();
        db.insert_account_info(CONTRACT_ADDR, account);
        let evm = Context::mainnet().with_db(db).build_mainnet();
        Self { evm, nonce: 0 }
    }

    fn call(&mut self, calldata: Vec<u8>) -> (bool, Vec<u8>) {
        let tx = TxEnv::builder()
            .caller(Address::ZERO)
            .kind(TxKind::Call(CONTRACT_ADDR))
            .data(Bytes::from(calldata))
            .nonce(self.nonce)
            .build()
            .unwrap();
        let result = self.evm.transact_commit(tx).unwrap();
        self.nonce += 1;
        let success = result.is_success();
        let output = result.output().map(|b| b.to_vec()).unwrap_or_default();
        (success, output)
    }
}

fn calldata(sel: [u8; 4], args: &[[u8; 32]]) -> Vec<u8> {
    let mut cd = sel.to_vec();
    for a in args {
        cd.extend_from_slice(a);
    }
    cd
}

// =============================================================================
// math.edge — safe arithmetic
// =============================================================================

#[test]
fn test_math_safe_add() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("safe_add(uint256,uint256)"),
        &[encode_u256(10), encode_u256(20)],
    ));
    assert!(ok, "safe_add reverted");
    assert_eq!(decode_u256(&out), 30, "safe_add(10, 20) should return 30");
}

#[test]
fn test_math_safe_sub() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("safe_sub(uint256,uint256)"),
        &[encode_u256(30), encode_u256(10)],
    ));
    assert!(ok, "safe_sub reverted");
    assert_eq!(decode_u256(&out), 20, "safe_sub(30, 10) should return 20");
}

#[test]
fn test_math_saturating_sub_underflow() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 5 - 10 would underflow; saturating_sub returns 0
    let (ok, out) = evm.call(calldata(
        selector("saturating_sub(uint256,uint256)"),
        &[encode_u256(5), encode_u256(10)],
    ));
    assert!(ok, "saturating_sub reverted");
    assert_eq!(
        decode_u256(&out),
        0,
        "saturating_sub(5, 10) should return 0"
    );
}

#[test]
fn test_math_saturating_sub_no_underflow() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("saturating_sub(uint256,uint256)"),
        &[encode_u256(10), encode_u256(5)],
    ));
    assert!(ok, "saturating_sub reverted");
    assert_eq!(
        decode_u256(&out),
        5,
        "saturating_sub(10, 5) should return 5"
    );
}

#[test]
fn test_math_max() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("max(uint256,uint256)"),
        &[encode_u256(5), encode_u256(10)],
    ));
    assert!(ok, "max reverted");
    assert_eq!(decode_u256(&out), 10, "max(5, 10) should return 10");
}

#[test]
fn test_math_max_equal() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("max(uint256,uint256)"),
        &[encode_u256(7), encode_u256(7)],
    ));
    assert!(ok, "max reverted");
    assert_eq!(decode_u256(&out), 7, "max(7, 7) should return 7");
}

#[test]
fn test_math_min() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("min(uint256,uint256)"),
        &[encode_u256(5), encode_u256(10)],
    ));
    assert!(ok, "min reverted");
    assert_eq!(decode_u256(&out), 5, "min(5, 10) should return 5");
}

#[test]
fn test_math_clamp_within_range() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("clamp(uint256,uint256,uint256)"),
        &[encode_u256(15), encode_u256(10), encode_u256(20)],
    ));
    assert!(ok, "clamp reverted");
    assert_eq!(decode_u256(&out), 15, "clamp(15, 10, 20) should return 15");
}

#[test]
fn test_math_clamp_below_lo() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("clamp(uint256,uint256,uint256)"),
        &[encode_u256(5), encode_u256(10), encode_u256(20)],
    ));
    assert!(ok, "clamp reverted");
    assert_eq!(
        decode_u256(&out),
        10,
        "clamp(5, 10, 20) should clamp to lo=10"
    );
}

#[test]
fn test_math_clamp_above_hi() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("clamp(uint256,uint256,uint256)"),
        &[encode_u256(25), encode_u256(10), encode_u256(20)],
    ));
    assert!(ok, "clamp reverted");
    assert_eq!(
        decode_u256(&out),
        20,
        "clamp(25, 10, 20) should clamp to hi=20"
    );
}

#[test]
fn test_math_mul_div_down_exact() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 6 * 4 / 3 = 8 (exact)
    let (ok, out) = evm.call(calldata(
        selector("mul_div_down(uint256,uint256,uint256)"),
        &[encode_u256(6), encode_u256(4), encode_u256(3)],
    ));
    assert!(ok, "mul_div_down reverted");
    assert_eq!(
        decode_u256(&out),
        8,
        "mul_div_down(6, 4, 3) should return 8"
    );
}

#[test]
fn test_math_mul_div_down_truncates() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 10 * 3 / 4 = 7.5 → truncates to 7
    let (ok, out) = evm.call(calldata(
        selector("mul_div_down(uint256,uint256,uint256)"),
        &[encode_u256(10), encode_u256(3), encode_u256(4)],
    ));
    assert!(ok, "mul_div_down reverted");
    assert_eq!(
        decode_u256(&out),
        7,
        "mul_div_down(10, 3, 4) should truncate to 7"
    );
}

#[test]
fn test_math_mul_div_up_exact() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 6 * 4 / 3 = 8 (exact, no rounding up)
    let (ok, out) = evm.call(calldata(
        selector("mul_div_up(uint256,uint256,uint256)"),
        &[encode_u256(6), encode_u256(4), encode_u256(3)],
    ));
    assert!(ok, "mul_div_up reverted");
    assert_eq!(decode_u256(&out), 8, "mul_div_up(6, 4, 3) should return 8");
}

#[test]
fn test_math_mul_div_up_rounds_up() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    // 10 * 3 / 4 = 7.5 → rounds up to 8
    let (ok, out) = evm.call(calldata(
        selector("mul_div_up(uint256,uint256,uint256)"),
        &[encode_u256(10), encode_u256(3), encode_u256(4)],
    ));
    assert!(ok, "mul_div_up reverted");
    assert_eq!(
        decode_u256(&out),
        8,
        "mul_div_up(10, 3, 4) should round up to 8"
    );
}

#[test]
fn test_math_unknown_selector_reverts() {
    let bc = compile_contract("examples/lib/math.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}

// =============================================================================
// bits.edge — bitwise operations
// =============================================================================

#[test]
fn test_bits_most_significant_bit_zero() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("most_significant_bit(uint256)"),
        &[encode_u256(0)],
    ));
    assert!(ok, "most_significant_bit reverted");
    assert_eq!(decode_u256(&out), 0, "msb(0) should return 0");
}

#[test]
fn test_bits_most_significant_bit_one() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(
        selector("most_significant_bit(uint256)"),
        &[encode_u256(1)],
    ));
    assert!(ok, "most_significant_bit reverted");
    assert_eq!(decode_u256(&out), 0, "msb(1) should return 0 (bit 0)");
}

#[test]
fn test_bits_most_significant_bit_powers_of_two() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // msb(2) = 1, msb(4) = 2, msb(8) = 3, msb(256) = 8
    for (input, expected) in [(2u64, 1u64), (4, 2), (8, 3), (256, 8)] {
        let (ok, out) = evm.call(calldata(
            selector("most_significant_bit(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(ok, "most_significant_bit({input}) reverted");
        assert_eq!(
            decode_u256(&out),
            expected,
            "msb({input}) should return {expected}"
        );
    }
}

#[test]
fn test_bits_popcount_zero() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("popcount(uint256)"), &[encode_u256(0)]));
    assert!(ok, "popcount reverted");
    assert_eq!(decode_u256(&out), 0, "popcount(0) should return 0");
}

#[test]
fn test_bits_popcount_values() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    for (input, expected) in [(1u64, 1u64), (3, 2), (7, 3), (255, 8)] {
        let (ok, out) = evm.call(calldata(
            selector("popcount(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(ok, "popcount({input}) reverted");
        assert_eq!(
            decode_u256(&out),
            expected,
            "popcount({input}) should return {expected}"
        );
    }
}

#[test]
fn test_bits_is_power_of_two_true() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    for input in [1u64, 2, 4, 8, 16, 256] {
        let (ok, out) = evm.call(calldata(
            selector("is_power_of_two(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(ok, "is_power_of_two({input}) reverted");
        assert!(decode_bool(&out), "is_power_of_two({input}) should be true");
    }
}

#[test]
fn test_bits_is_power_of_two_false() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    for input in [0u64, 3, 5, 6, 7, 9, 10] {
        let (ok, out) = evm.call(calldata(
            selector("is_power_of_two(uint256)"),
            &[encode_u256(input)],
        ));
        assert!(ok, "is_power_of_two({input}) reverted");
        assert!(
            !decode_bool(&out),
            "is_power_of_two({input}) should be false"
        );
    }
}

#[test]
fn test_bits_extract_bit() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // 5 = 0b101: bit0=1, bit1=0, bit2=1
    for (pos, expected) in [(0u64, 1u64), (1, 0), (2, 1), (3, 0)] {
        let (ok, out) = evm.call(calldata(
            selector("extract_bit(uint256,uint256)"),
            &[encode_u256(5), encode_u256(pos)],
        ));
        assert!(ok, "extract_bit(5, {pos}) reverted");
        assert_eq!(
            decode_u256(&out),
            expected,
            "extract_bit(5, {pos}) should return {expected}"
        );
    }
}

#[test]
fn test_bits_set_bit() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // set_bit(4, 0) = 5 (4=100b, set bit 0 → 101b=5)
    let (ok, out) = evm.call(calldata(
        selector("set_bit(uint256,uint256)"),
        &[encode_u256(4), encode_u256(0)],
    ));
    assert!(ok, "set_bit reverted");
    assert_eq!(decode_u256(&out), 5, "set_bit(4, 0) should return 5");
}

#[test]
fn test_bits_clear_bit() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // clear_bit(5, 0) = 4 (5=101b, clear bit 0 → 100b=4)
    let (ok, out) = evm.call(calldata(
        selector("clear_bit(uint256,uint256)"),
        &[encode_u256(5), encode_u256(0)],
    ));
    assert!(ok, "clear_bit reverted");
    assert_eq!(decode_u256(&out), 4, "clear_bit(5, 0) should return 4");
}

#[test]
fn test_bits_toggle_bit() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // toggle_bit(5, 2) = 1 (5=101b, toggle bit 2 → 001b=1)
    let (ok, out) = evm.call(calldata(
        selector("toggle_bit(uint256,uint256)"),
        &[encode_u256(5), encode_u256(2)],
    ));
    assert!(ok, "toggle_bit reverted");
    assert_eq!(decode_u256(&out), 1, "toggle_bit(5, 2) should return 1");
}

#[test]
fn test_bits_least_significant_bit_zero() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);

    // lsb(0) = 256 (no bits set)
    let (ok, out) = evm.call(calldata(
        selector("least_significant_bit(uint256)"),
        &[encode_u256(0)],
    ));
    assert!(ok, "least_significant_bit(0) reverted");
    assert_eq!(
        decode_u256(&out),
        256,
        "lsb(0) should return 256 (sentinel)"
    );
}

#[test]
fn test_bits_unknown_selector_reverts() {
    let bc = compile_contract("examples/utils/bits.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}

// =============================================================================
// bytes.edge — b32 utilities (functions using only parameters work correctly;
//              functions that reference top-level named constants like ADDR_MASK
//              resolve those constants to 0 at this compiler stage)
// =============================================================================

#[test]
fn test_bytes_is_zero_true() {
    let bc = compile_contract("examples/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("is_zero(bytes32)"), &[[0u8; 32]]));
    assert!(ok, "is_zero reverted");
    assert!(decode_bool(&out), "is_zero(0) should return true");
}

#[test]
fn test_bytes_is_zero_false() {
    let bc = compile_contract("examples/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, out) = evm.call(calldata(selector("is_zero(bytes32)"), &[encode_u256(1)]));
    assert!(ok, "is_zero reverted");
    assert!(!decode_bool(&out), "is_zero(1) should return false");
}

#[test]
fn test_bytes_left_pad_zero_shift() {
    let bc = compile_contract("examples/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    // left_pad(0xff, 0) = 0xff (shift by 0 bytes = no shift)
    let (ok, out) = evm.call(calldata(
        selector("left_pad(uint256,uint256)"),
        &[encode_u256(0xff), encode_u256(0)],
    ));
    assert!(ok, "left_pad reverted");
    assert_eq!(
        decode_u256(&out),
        0xff,
        "left_pad(0xff, 0) should return 0xff"
    );
}

#[test]
fn test_bytes_left_pad_one_byte() {
    let bc = compile_contract("examples/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    // left_pad(0x01, 1) = 0x0100 (shift left by 8 bits)
    let (ok, out) = evm.call(calldata(
        selector("left_pad(uint256,uint256)"),
        &[encode_u256(1), encode_u256(1)],
    ));
    assert!(ok, "left_pad reverted");
    assert_eq!(
        decode_u256(&out),
        0x100,
        "left_pad(1, 1) should return 0x100"
    );
}

#[test]
fn test_bytes_unknown_selector_reverts() {
    let bc = compile_contract("examples/utils/bytes.edge");
    let mut evm = EvmHandle::new(bc);
    let (ok, _) = evm.call(vec![0xde, 0xad, 0xbe, 0xef]);
    assert!(!ok, "unknown selector should revert");
}
