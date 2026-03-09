#![allow(missing_docs)]

//! Regression test for full_math (bisect1) - Arg DUP depth 0 crash at O3.
//!
//! This test is ignored because it triggers a pre-existing bug where
//! single-function contracts at O3 produce an Arg DUP depth 0 panic.

use crate::helpers::*;

const CONTRACT: &str = "examples/tests/test_full_math.edge";

#[test]
#[ignore = "Arg DUP depth 0 crash at O3 - pre-existing bug"]
fn test_full_math_mul_div() {
    for_all_opt_levels(CONTRACT, |h, o| {
        // mul_div(6, 7, 3) = (6*7) / 3 = 14
        let mut cd = selector("mul_div(uint256,uint256,uint256)").to_vec();
        cd.extend(encode_u256(6));
        cd.extend(encode_u256(7));
        cd.extend(encode_u256(3));
        let r = h.call(cd);
        assert!(r.success, "mul_div reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 14, "6*7/3=14 at O{o}");
    });
}
