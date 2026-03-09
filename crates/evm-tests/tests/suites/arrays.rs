//! Array feature tests: memory arrays, storage arrays, bounds checking,
//! iteration, mutation, and slices. Tests at O0, O1, and O2 to catch
//! optimization-related regressions.

use alloy_primitives::U256;
use edge_evm_tests::{abi_decode_u256, abi_encode_u256, EvmTestHost};

const PATH: &str = "../../examples/tests/test_arrays.edge";

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

fn deploy(opt: u8) -> EvmTestHost {
    EvmTestHost::deploy_edge(PATH, opt)
}

// ═══════════════════════════════════════════════════════════════════
// Memory array: basic element access
// ═══════════════════════════════════════════════════════════════════

#[test]
fn element_access() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("element_access()", &[]);
        assert!(r.success, "O{opt}: element_access should succeed");
        assert_eq!(decode(&r.output), u(20), "O{opt}: arr[1] == 20");
    }
}

#[test]
fn read_all() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("read_all()", &[]);
        assert!(r.success, "O{opt}: read_all should succeed");
        assert_eq!(
            decode(&r.output),
            u(1000),
            "O{opt}: 100+200+300+400 == 1000"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// Memory array: write then read
// ═══════════════════════════════════════════════════════════════════

#[test]
fn write_then_read() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("write_then_read()", &[]);
        assert!(r.success, "O{opt}: write_then_read should succeed");
        // arr = [99, 2, 77], sum = 178
        assert_eq!(decode(&r.output), u(178), "O{opt}: 99+2+77 == 178");
    }
}

// ═══════════════════════════════════════════════════════════════════
// Memory array: iteration
// ═══════════════════════════════════════════════════════════════════

#[test]
fn sum_array() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("sum_array()", &[]);
        assert!(r.success, "O{opt}: sum_array should succeed");
        assert_eq!(decode(&r.output), u(100), "O{opt}: 10+20+30+40 == 100");
    }
}

#[test]
fn loop_write_sum() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("loop_write_sum()", &[]);
        assert!(r.success, "O{opt}: loop_write_sum should succeed");
        // arr[i] = i*10: [0,10,20,30,40], sum = 100
        assert_eq!(decode(&r.output), u(100), "O{opt}: 0+10+20+30+40 == 100");
    }
}

#[test]
fn find_max() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("find_max()", &[]);
        assert!(r.success, "O{opt}: find_max should succeed");
        assert_eq!(
            decode(&r.output),
            u(50),
            "O{opt}: max of [30,10,50,20,40] == 50"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// Slice access
// ═══════════════════════════════════════════════════════════════════

#[test]
fn slice_sum() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("slice_sum()", &[]);
        assert!(r.success, "O{opt}: slice_sum should succeed");
        // arr[1:3] = [20, 30], sum = 50
        assert_eq!(decode(&r.output), u(50), "O{opt}: 20+30 == 50");
    }
}

// ═══════════════════════════════════════════════════════════════════
// Storage array: set/get round-trip
// ═══════════════════════════════════════════════════════════════════

#[test]
fn storage_set_get() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // Set values[0] = 42
        let r = h.call_fn("set(uint256,uint256)", &encode2(0, 42));
        assert!(r.success, "O{opt}: set(0, 42) should succeed");

        // Get values[0]
        let r = h.call_fn("get(uint256)", &encode(0));
        assert!(r.success, "O{opt}: get(0) should succeed");
        assert_eq!(decode(&r.output), u(42), "O{opt}: values[0] == 42");
    }
}

#[test]
fn storage_multiple_slots() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // Set several slots
        for i in 0..5u64 {
            let r = h.call_fn("set(uint256,uint256)", &encode2(i, (i + 1) * 100));
            assert!(
                r.success,
                "O{opt}: set({i}, {}) should succeed",
                (i + 1) * 100
            );
        }
        // Read back
        for i in 0..5u64 {
            let r = h.call_fn("get(uint256)", &encode(i));
            assert!(r.success, "O{opt}: get({i}) should succeed");
            assert_eq!(
                decode(&r.output),
                u((i + 1) * 100),
                "O{opt}: values[{i}] == {}",
                (i + 1) * 100
            );
        }
    }
}

#[test]
fn storage_sum() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // Set values[0..5] = [10, 20, 30, 40, 50]
        for i in 0..5u64 {
            let r = h.call_fn("set(uint256,uint256)", &encode2(i, (i + 1) * 10));
            assert!(r.success, "O{opt}: set({i}) should succeed");
        }
        let r = h.call_fn("storage_sum()", &[]);
        assert!(r.success, "O{opt}: storage_sum should succeed");
        assert_eq!(decode(&r.output), u(150), "O{opt}: 10+20+30+40+50 == 150");
    }
}

#[test]
fn storage_overwrite() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // Set then overwrite
        let r = h.call_fn("set(uint256,uint256)", &encode2(2, 100));
        assert!(r.success, "O{opt}: initial set should succeed");
        let r = h.call_fn("set(uint256,uint256)", &encode2(2, 999));
        assert!(r.success, "O{opt}: overwrite should succeed");
        let r = h.call_fn("get(uint256)", &encode(2));
        assert!(r.success, "O{opt}: get after overwrite should succeed");
        assert_eq!(decode(&r.output), u(999), "O{opt}: values[2] == 999");
    }
}

// ═══════════════════════════════════════════════════════════════════
// Bounds checking: storage array OOB reverts
// ═══════════════════════════════════════════════════════════════════

#[test]
fn storage_get_oob_reverts() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // values has length 5, index 5 is OOB
        let r = h.call_fn("get(uint256)", &encode(5));
        assert!(
            !r.success,
            "O{opt}: get(5) should revert (OOB on [u256; 5])"
        );
    }
}

#[test]
fn storage_get_large_index_reverts() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("get(uint256)", &encode(100));
        assert!(!r.success, "O{opt}: get(100) should revert");
    }
}

#[test]
fn storage_set_oob_reverts() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("set(uint256,uint256)", &encode2(5, 42));
        assert!(
            !r.success,
            "O{opt}: set(5, 42) should revert (OOB on [u256; 5])"
        );
    }
}

#[test]
fn storage_boundary_index_succeeds() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // Index 4 is the last valid index for [u256; 5]
        let r = h.call_fn("set(uint256,uint256)", &encode2(4, 777));
        assert!(
            r.success,
            "O{opt}: set(4, 777) should succeed (last valid index)"
        );
        let r = h.call_fn("get(uint256)", &encode(4));
        assert!(r.success, "O{opt}: get(4) should succeed");
        assert_eq!(decode(&r.output), u(777), "O{opt}: values[4] == 777");
    }
}

// ═══════════════════════════════════════════════════════════════════
// Bounds checking: smaller storage array
// ═══════════════════════════════════════════════════════════════════

#[test]
fn small_storage_set_get() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // small is [u256; 3]
        let r = h.call_fn("set_small(uint256,uint256)", &encode2(0, 11));
        assert!(r.success, "O{opt}: set_small(0, 11) should succeed");
        let r = h.call_fn("set_small(uint256,uint256)", &encode2(2, 33));
        assert!(r.success, "O{opt}: set_small(2, 33) should succeed");

        let r = h.call_fn("get_small(uint256)", &encode(0));
        assert!(r.success, "O{opt}: get_small(0) should succeed");
        assert_eq!(decode(&r.output), u(11));

        let r = h.call_fn("get_small(uint256)", &encode(2));
        assert!(r.success, "O{opt}: get_small(2) should succeed");
        assert_eq!(decode(&r.output), u(33));
    }
}

#[test]
fn small_storage_oob_reverts() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        // small is [u256; 3], index 3 is OOB
        let r = h.call_fn("get_small(uint256)", &encode(3));
        assert!(
            !r.success,
            "O{opt}: get_small(3) should revert (OOB on [u256; 3])"
        );

        let r = h.call_fn("set_small(uint256,uint256)", &encode2(3, 42));
        assert!(
            !r.success,
            "O{opt}: set_small(3, 42) should revert (OOB on [u256; 3])"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// Bounds checking: index 0 always valid for non-empty arrays
// ═══════════════════════════════════════════════════════════════════

#[test]
fn index_zero_always_valid() {
    for opt in 0..=2 {
        let mut h = deploy(opt);
        let r = h.call_fn("set(uint256,uint256)", &encode2(0, 1));
        assert!(r.success, "O{opt}: set(0, 1) should always succeed");
        let r = h.call_fn("get(uint256)", &encode(0));
        assert!(r.success, "O{opt}: get(0) should always succeed");
        assert_eq!(decode(&r.output), u(1));
    }
}
