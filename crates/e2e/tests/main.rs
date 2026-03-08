#![allow(missing_docs)]

#[path = "suites/access_exec.rs"]
mod access_exec;
#[path = "suites/calldata_args.rs"]
mod calldata_args;
#[path = "suites/counter.rs"]
mod counter;
#[path = "suites/erc20.rs"]
mod erc20;
#[path = "suites/examples.rs"]
mod examples;
#[path = "suites/expressions.rs"]
mod expressions;
#[path = "suites/features_exec.rs"]
mod features_exec;
#[path = "suites/finance_exec.rs"]
mod finance_exec;
#[path = "suites/generics_exec.rs"]
mod generics_exec;
#[path = "suites/generics_negative.rs"]
mod generics_negative;
#[path = "suites/inline_asm_exec.rs"]
mod inline_asm_exec;
#[path = "suites/packed_exec.rs"]
mod packed_exec;
#[path = "suites/packed_storage_exec.rs"]
mod packed_storage_exec;
#[path = "suites/packed_transient_exec.rs"]
mod packed_transient_exec;
#[path = "suites/patterns_exec.rs"]
mod patterns_exec;
#[path = "suites/tokens_exec.rs"]
mod tokens_exec;
#[path = "suites/traits_exec.rs"]
mod traits_exec;
#[path = "suites/traits_negative.rs"]
mod traits_negative;
#[path = "suites/type_demos_exec.rs"]
mod type_demos_exec;
#[path = "suites/types_exec.rs"]
mod types_exec;
#[path = "suites/utils_exec.rs"]
mod utils_exec;
#[path = "suites/warnings.rs"]
mod warnings;

#[path = "suites/large_int_literals.rs"]
mod large_int_literals;
