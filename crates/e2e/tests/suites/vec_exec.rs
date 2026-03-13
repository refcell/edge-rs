#![allow(missing_docs)]

//! Execution-level tests for Vec<T> dynamic memory allocation.

use crate::helpers::*;

const CONTRACT: &str = "examples/tests/test_vec.edge";

#[test]
fn test_vec_new_and_push() {
    for_all_opt_levels(CONTRACT, |evm, opt| {
        let r = evm.call(calldata(selector("test_new_and_push()"), &[]));
        assert!(r.success, "test_new_and_push() reverted at O{opt}");
        assert_eq!(
            decode_u256(&r.output),
            3,
            "len should be 3 after 3 pushes (O{opt})"
        );
    });
}

#[test]
fn test_vec_get() {
    for_all_opt_levels(CONTRACT, |evm, opt| {
        let r = evm.call(calldata(selector("test_get()"), &[]));
        assert!(r.success, "test_get() reverted at O{opt}");
        assert_eq!(
            decode_u256(&r.output),
            200,
            "get(1) should return second element (200) (O{opt})"
        );
    });
}

#[test]
fn test_vec_set() {
    for_all_opt_levels(CONTRACT, |evm, opt| {
        let r = evm.call(calldata(selector("test_set()"), &[]));
        assert!(
            r.success,
            "test_set() reverted at O{opt}; gas_used={}",
            r.gas_used
        );
        assert_eq!(
            decode_u256(&r.output),
            999,
            "set(1, 999) then get(1) should return 999 (O{opt})"
        );
    });
}

#[test]
fn test_vec_grow() {
    for_all_opt_levels(CONTRACT, |evm, opt| {
        let r = evm.call(calldata(selector("test_grow()"), &[]));
        assert!(r.success, "test_grow() reverted at O{opt}");
        assert_eq!(
            decode_u256(&r.output),
            15,
            "sum of elements 1..5 should be 15 after growth (O{opt})"
        );
    });
}

#[test]
fn test_vec_zero() {
    for_all_opt_levels(CONTRACT, |evm, opt| {
        let r = evm.call(calldata(selector("test_zero_array()"), &[]));
        assert!(r.success, "test_zero_array() reverted at O{opt}");
    });
}

#[test]
fn test_vec_index() {
    for_all_opt_levels(CONTRACT, |evm, opt| {
        let r = evm.call(calldata(selector("test_index()"), &[]));
        assert!(
            r.success,
            "test_index() reverted at O{opt}; gas_used={}",
            r.gas_used
        );
        assert_eq!(decode_u256(&r.output), 84, "v[1] should return 84 (O{opt})");
    });
}
