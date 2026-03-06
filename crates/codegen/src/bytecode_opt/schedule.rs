//! Run-schedules per optimization level.

/// Returns the egglog run-schedule commands for the given optimization level.
/// Returns `None` for O0 (no optimization).
pub(crate) fn schedule_for_level(level: u8) -> Option<String> {
    match level {
        0 => None,
        1 => Some(
            r#"
(run-schedule
  (repeat 3
    (seq
      (run bytecode-peepholes)
      (run bytecode-dead-push))))
"#
            .to_string(),
        ),
        2 => Some(
            r#"
(run-schedule
  (repeat 5
    (seq
      (run bytecode-peepholes)
      (run bytecode-const-fold)
      (run bytecode-strength-red)
      (run bytecode-dead-push))))
"#
            .to_string(),
        ),
        _ => Some(
            r#"
(run-schedule
  (repeat 10
    (seq
      (run bytecode-peepholes)
      (run bytecode-const-fold)
      (run bytecode-strength-red)
      (run bytecode-dead-push))))
"#
            .to_string(),
        ),
    }
}
