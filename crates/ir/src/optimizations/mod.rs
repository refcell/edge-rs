//! Optimization rule files for the egglog-based IR.
//!
//! Each `.egg` file in this directory defines rewrite rules for a
//! specific optimization category. They are loaded via `include_str!()`
//! in `lib.rs` and concatenated into the egglog prologue.
