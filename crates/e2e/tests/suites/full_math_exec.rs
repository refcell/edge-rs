#![allow(missing_docs)]

//! Regression test for full_math (bisect1) - previously crashed with
//! Arg DUP depth 0 at O3 due to InlineAsm inputs not being traversed
//! by substitute_args during monomorphization.

use crate::helpers::*;

const CONTRACT: &str = "examples/tests/test_full_math.edge";

#[test]
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

#[test]
fn test_full_math_mul_div_slow_path() {
    for_all_opt_levels(CONTRACT, |h, o| {
        // mul_div(MAX_UINT, 2, MAX_UINT) = 2
        // This exercises the 512-bit division (Newton-Raphson) slow path
        // because MAX_UINT * 2 overflows u256, making prod1 != 0.
        let sel = selector("mul_div(uint256,uint256,uint256)");
        let max_u256 = [0xFFu8; 32];
        let mut cd = sel.to_vec();
        cd.extend_from_slice(&max_u256);
        cd.extend(encode_u256(2));
        cd.extend_from_slice(&max_u256);
        let r = h.call(cd);
        assert!(r.success, "mul_div slow path reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 2, "MAX*2/MAX=2 at O{o}");
    });
}
