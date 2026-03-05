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
/// - Level 1: peepholes + dead code only (fast, safe)
/// - Level 2: full suite with analysis-first scheduling
/// - Level 3+: aggressive (more iterations)
///
/// Scheduling strategy: saturate cheap analysis rulesets (dead-code) before
/// running potentially expensive optimization rulesets (peepholes, arithmetic).
/// This ensures analysis facts are complete before guarded rewrites fire,
/// and prevents wasted work from premature optimization attempts.
pub fn make_schedule(optimization_level: u8) -> String {
    match optimization_level {
        0 => String::new(),
        1 => {
            // Fast, safe optimizations only
            // Saturate analysis first, then peepholes + U256 const fold
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))".to_owned()
        }
        2 => {
            // Full optimization suite with analysis-first scheduling
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 5
                    (seq
                        (run peepholes)
                        (run arithmetic-opt)
                        (run u256-const-fold)
                        (run storage-opt)
                        (run memory-opt)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                        (run cse-rules))))".to_owned()
        }
        _ => {
            // Aggressive: more iterations, saturate analysis each round
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 10
                    (seq
                        (run peepholes)
                        (run arithmetic-opt)
                        (run u256-const-fold)
                        (run storage-opt)
                        (run memory-opt)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                        (run cse-rules))))".to_owned()
        }
    }
}
