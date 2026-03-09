#![allow(missing_docs)]

//! Execution-level tests for packed structs in transient storage (&t).
//!
//! Mirrors `packed_storage_exec.rs` but uses transient storage (TLOAD/TSTORE).

use crate::helpers::*;

const PACKED_TRANSIENT: &str = "examples/tests/test_packed_transient.edge";

// ----- Whole-struct write + sub-field reads -----

#[test]
fn test_packed_transient_read_r() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("store_and_read_r()"), &[]));
        assert!(r.success, "store_and_read_r() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 10, "store_and_read_r() wrong at O{o}");
    });
}

#[test]
fn test_packed_transient_read_g() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("store_and_read_g()"), &[]));
        assert!(r.success, "store_and_read_g() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 20, "store_and_read_g() wrong at O{o}");
    });
}

#[test]
fn test_packed_transient_read_b() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("store_and_read_b()"), &[]));
        assert!(r.success, "store_and_read_b() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 30, "store_and_read_b() wrong at O{o}");
    });
}

#[test]
fn test_packed_transient_read_sum() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("store_and_read_sum()"), &[]));
        assert!(r.success, "store_and_read_sum() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 60, "store_and_read_sum() wrong at O{o}");
    });
}

#[test]
fn test_packed_transient_pair_a() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("store_pair_read_a()"), &[]));
        assert!(r.success, "store_pair_read_a() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 42, "store_pair_read_a() wrong at O{o}");
    });
}

#[test]
fn test_packed_transient_pair_b() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("store_pair_read_b()"), &[]));
        assert!(r.success, "store_pair_read_b() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 99, "store_pair_read_b() wrong at O{o}");
    });
}

// ----- Sub-field writes -----

#[test]
fn test_packed_transient_write_subfield() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("write_subfield_r()"), &[]));
        assert!(r.success, "write_subfield_r() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 99, "write_subfield_r() wrong at O{o}");
    });
}

#[test]
fn test_packed_transient_write_preserves_other_fields() {
    for_all_opt_levels(PACKED_TRANSIENT, |h, o| {
        let r = h.call(calldata(selector("write_subfield_preserves()"), &[]));
        assert!(r.success, "write_subfield_preserves() reverted at O{o}");
        assert_eq!(
            decode_u256(&r.output),
            50,
            "write_subfield_preserves() wrong at O{o}"
        );
    });
}
