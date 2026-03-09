#![allow(missing_docs)]

//! Execution-level tests for generics, impl blocks, and trait impls.
//!
//! Every test runs at O0, O1, O2, and O3 to catch optimizer bugs.

use crate::helpers::*;

// =============================================================================
// Generic function tests (examples/test_generics.edge)
// =============================================================================

const GENERICS: &str = "examples/tests/test_generics.edge";

#[test]
fn test_generic_identity() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_identity()"), &[]));
        assert!(r.success, "test_identity() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 42, "test_identity() wrong at O{o}");
    });
}

#[test]
fn test_generic_max() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_max()"), &[]));
        assert!(r.success, "test_max() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 20, "test_max() wrong at O{o}");
    });
}

#[test]
fn test_generic_min() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_min()"), &[]));
        assert!(r.success, "test_min() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 10, "test_min() wrong at O{o}");
    });
}

#[test]
fn test_generic_entry_value() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_entry_value()"), &[]));
        assert!(r.success, "test_entry_value() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 42, "test_entry_value() wrong at O{o}");
    });
}

#[test]
fn test_generic_entry_key() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_entry_key()"), &[]));
        assert!(r.success, "test_entry_key() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 100, "test_entry_key() wrong at O{o}");
    });
}

#[test]
fn test_generic_result_ok() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_result_ok()"), &[]));
        assert!(r.success, "test_result_ok() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 77, "test_result_ok() wrong at O{o}");
    });
}

#[test]
fn test_generic_result_err() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_result_err()"), &[]));
        assert!(r.success, "test_result_err() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 99, "test_result_err() wrong at O{o}");
    });
}

#[test]
fn test_generic_option_some() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_option_some()"), &[]));
        assert!(r.success, "test_option_some() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 55, "test_option_some() wrong at O{o}");
    });
}

#[test]
fn test_generic_option_none() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_option_none()"), &[]));
        assert!(r.success, "test_option_none() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 0, "test_option_none() wrong at O{o}");
    });
}

#[test]
fn test_turbofish_identity() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_turbofish_identity()"), &[]));
        assert!(r.success, "test_turbofish_identity() reverted at O{o}");
        assert_eq!(
            decode_u256(&r.output),
            42,
            "test_turbofish_identity() wrong at O{o}"
        );
    });
}

#[test]
fn test_turbofish_max() {
    for_all_opt_levels(GENERICS, |h, o| {
        let r = h.call(calldata(selector("test_turbofish_max()"), &[]));
        assert!(r.success, "test_turbofish_max() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 20, "test_turbofish_max() wrong at O{o}");
    });
}

// =============================================================================
// Impl block tests (examples/test_impl.edge)
// =============================================================================

const IMPL: &str = "examples/tests/test_impl.edge";

#[test]
fn test_impl_point_sum() {
    for_all_opt_levels(IMPL, |h, o| {
        let r = h.call(calldata(selector("test_point_sum()"), &[]));
        assert!(r.success, "test_point_sum() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 30, "test_point_sum() wrong at O{o}");
    });
}

#[test]
fn test_impl_point_scale() {
    for_all_opt_levels(IMPL, |h, o| {
        let r = h.call(calldata(selector("test_point_scale()"), &[]));
        assert!(r.success, "test_point_scale() reverted at O{o}");
        // (5 * 3) + (7 * 3) = 15 + 21 = 36
        assert_eq!(decode_u256(&r.output), 36, "test_point_scale() wrong at O{o}");
    });
}

#[test]
fn test_impl_point_x() {
    for_all_opt_levels(IMPL, |h, o| {
        let r = h.call(calldata(selector("test_point_x()"), &[]));
        assert!(r.success, "test_point_x() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 42, "test_point_x() wrong at O{o}");
    });
}

#[test]
fn test_impl_counter_get() {
    for_all_opt_levels(IMPL, |h, o| {
        let r = h.call(calldata(selector("test_counter_get()"), &[]));
        assert!(r.success, "test_counter_get() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 100, "test_counter_get() wrong at O{o}");
    });
}

#[test]
fn test_impl_counter_add() {
    for_all_opt_levels(IMPL, |h, o| {
        let r = h.call(calldata(selector("test_counter_add()"), &[]));
        assert!(r.success, "test_counter_add() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 150, "test_counter_add() wrong at O{o}");
    });
}

// =============================================================================
// Trait impl tests (examples/test_traits.edge)
// =============================================================================

const TRAITS: &str = "examples/tests/test_traits.edge";

#[test]
fn test_trait_double() {
    for_all_opt_levels(TRAITS, |h, o| {
        let r = h.call(calldata(selector("test_double()"), &[]));
        assert!(r.success, "test_double() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 42, "test_double() wrong at O{o}");
    });
}

#[test]
fn test_trait_triple() {
    for_all_opt_levels(TRAITS, |h, o| {
        let r = h.call(calldata(selector("test_triple()"), &[]));
        assert!(r.success, "test_triple() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 30, "test_triple() wrong at O{o}");
    });
}

#[test]
fn test_trait_double_then_triple() {
    for_all_opt_levels(TRAITS, |h, o| {
        let r = h.call(calldata(selector("test_double_then_triple()"), &[]));
        assert!(r.success, "test_double_then_triple() reverted at O{o}");
        // Doubler::double(5) = 10, then * 3 = 30
        assert_eq!(
            decode_u256(&r.output),
            30,
            "test_double_then_triple() wrong at O{o}"
        );
    });
}

#[test]
fn test_trait_double_method() {
    for_all_opt_levels(TRAITS, |h, o| {
        let r = h.call(calldata(selector("test_double_method()"), &[]));
        assert!(r.success, "test_double_method() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 42, "test_double_method() wrong at O{o}");
    });
}

#[test]
fn test_trait_triple_method() {
    for_all_opt_levels(TRAITS, |h, o| {
        let r = h.call(calldata(selector("test_triple_method()"), &[]));
        assert!(r.success, "test_triple_method() reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 30, "test_triple_method() wrong at O{o}");
    });
}

#[test]
fn test_operator_overload_add() {
    for_all_opt_levels(TRAITS, |h, o| {
        let r = h.call(calldata(selector("test_add_overload()"), &[]));
        assert!(r.success, "test_add_overload() reverted at O{o}");
        // Wrapper{10} + Wrapper{32} = Wrapper{42} → .val = 42
        assert_eq!(decode_u256(&r.output), 42, "test_add_overload() wrong at O{o}");
    });
}

#[test]
fn test_operator_overload_eq() {
    for_all_opt_levels(TRAITS, |h, o| {
        let r = h.call(calldata(selector("test_eq_overload()"), &[]));
        assert!(r.success, "test_eq_overload() reverted at O{o}");
        // Wrapper{42} == Wrapper{42} → true → returns 1
        assert_eq!(decode_u256(&r.output), 1, "test_eq_overload() wrong at O{o}");
    });
}
