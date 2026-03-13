#![allow(missing_docs)]

//! Gas benchmarking for Vec operations using the existing test_vec.edge contract.
//! Uses the test harness to measure actual execution gas at each opt level.

use crate::helpers::*;

const CONTRACT: &str = "examples/tests/test_vec.edge";

/// Detailed gas analysis of Vec test functions.
#[test]
fn bench_vec_gas_breakdown() {
    let sigs = [
        "test_new_and_push()",
        "test_get()",
        "test_set()",
        "test_grow()",
        "test_index()",
    ];

    eprintln!("\n╔═══════════════════════════════════════════════════════════════╗");
    eprintln!("║         Vec<u256> Gas Analysis — Execution Gas Only          ║");
    eprintln!("║  (tx base + calldata intrinsic stripped)                     ║");
    eprintln!("╠════════════════════╦════════╦════════╦════════╦═════════════╣");
    eprintln!("║ Function           ║   O0   ║   O1   ║   O2   ║   O3        ║");
    eprintln!("╠════════════════════╬════════╬════════╬════════╬═════════════╣");

    for sig in &sigs {
        let mut gases = [0u64; 4];
        for opt in 0..=3u8 {
            let bc = compile_contract_opt(CONTRACT, opt);
            let mut h = EvmHandle::new(bc);
            let sel = selector(sig);
            let cd = calldata(sel, &[]);
            let r = h.call(cd.clone());
            assert!(r.success, "{sig} reverted at O{opt}; gas={}", r.gas_used);
            gases[opt as usize] = execution_gas(r.gas_used, &cd);
        }
        eprintln!(
            "║ {:18} ║ {:>6} ║ {:>6} ║ {:>6} ║ {:>6}      ║",
            sig.trim_end_matches("()"),
            gases[0], gases[1], gases[2], gases[3]
        );
    }
    eprintln!("╚════════════════════╩════════╩════════╩════════╩═════════════╝");
    eprintln!();

    // Now show what operations each test does for context:
    eprintln!("Operation breakdown:");
    eprintln!("  test_new_and_push: new(4) + 3×push + len read");
    eprintln!("  test_get:          new(4) + 3×push + get(1)");
    eprintln!("  test_set:          new(4) + 3×push + set(1,999) + get(1)");
    eprintln!("  test_grow:         new(2) + 5×push (triggers grow) + 5×get + 4×add");
    eprintln!("  test_index:        new(4) + 2×push + v[1] (Index trait)");
}
