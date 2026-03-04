//! Optimization schedule configuration.
//!
//! Maps optimization levels to sequences of egglog rulesets to run.

/// Generate the egglog ruleset scheduling code.
///
/// This defines the order and iteration limits for each optimization pass.
pub fn rulesets() -> String {
    String::new()
}

/// Build an egglog schedule string for the given optimization level.
///
/// - Level 0: no optimization (caller should skip egglog entirely)
/// - Level 1: peepholes + constant folding only
/// - Level 2: full suite
/// - Level 3: aggressive (full suite + more iterations)
pub fn make_schedule(optimization_level: u8) -> String {
    match optimization_level {
        0 => String::new(),
        1 => {
            // Fast, safe optimizations only
            "(run-schedule
                (repeat 3
                    (run peepholes)))".to_owned()
        }
        2 => {
            // Full optimization suite
            "(run-schedule
                (repeat 5
                    (seq
                        (run peepholes)
                        (run arithmetic-opt)
                        (run storage-opt)
                        (run memory-opt)
                        (run dead-code)
                        (run cse-rules))))".to_owned()
        }
        _ => {
            // Aggressive: more iterations
            "(run-schedule
                (repeat 10
                    (seq
                        (run peepholes)
                        (run arithmetic-opt)
                        (run storage-opt)
                        (run memory-opt)
                        (run dead-code)
                        (run cse-rules))))".to_owned()
        }
    }
}
