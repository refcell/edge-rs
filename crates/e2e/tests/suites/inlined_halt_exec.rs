#![allow(missing_docs)]

//! Regression test for inlined-function halt detection in codegen.
//!
//! `bisect14.edge` triggers a stack-depth mismatch at O3 when the
//! `remaining_reads` consume optimization fires inside dead code
//! produced by inlined returns.

use crate::helpers::*;

const CONTRACT: &str = "examples/tests/test_inlined_halt.edge";

#[test]
fn test_inlined_halt_compute() {
    for_all_opt_levels(CONTRACT, |h, o| {
        // compute(6, 7, 3) = (6*7) / 3 = 14  (prod1==0 branch)
        let mut cd = selector("compute(uint256,uint256,uint256)").to_vec();
        cd.extend(encode_u256(6));
        cd.extend(encode_u256(7));
        cd.extend(encode_u256(3));
        let r = h.call(cd);
        assert!(r.success, "compute reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 14, "6*7/3=14 at O{o}");
    });
}

#[test]
fn test_inlined_halt_wrapper() {
    for_all_opt_levels(CONTRACT, |h, o| {
        let mut cd = selector("wrapper(uint256,uint256,uint256)").to_vec();
        cd.extend(encode_u256(10));
        cd.extend(encode_u256(5));
        cd.extend(encode_u256(2));
        let r = h.call(cd);
        assert!(r.success, "wrapper reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 25, "10*5/2=25 at O{o}");
    });
}
