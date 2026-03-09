#![allow(missing_docs)]

#[path = "suites/arrays.rs"]
mod arrays;
#[path = "suites/checked_elision.rs"]
mod checked_elision;
#[path = "suites/counter.rs"]
mod counter;
#[path = "suites/erc20.rs"]
mod erc20;
#[path = "suites/gas_bench.rs"]
mod gas_bench;
#[path = "suites/gas_stats.rs"]
mod gas_stats;
#[path = "suites/loop_extraction.rs"]
mod loop_extraction;
#[path = "suites/o2_size_debug.rs"]
mod o2_size_debug;
#[path = "suites/opt_efficiency.rs"]
mod opt_efficiency;
#[path = "suites/opt_equivalence.rs"]
mod opt_equivalence;
#[path = "suites/spec_compliance.rs"]
mod spec_compliance;
#[path = "suites/stress.rs"]
mod stress;
#[path = "suites/subroutine.rs"]
mod subroutine;
#[path = "suites/transient.rs"]
mod transient;
