//! Edge Language Egglog-based Intermediate Representation.
//!
//! This crate provides the IR layer between the Edge AST and EVM bytecode
//! generation. It uses [egglog](https://github.com/egraphs-good/egglog) for
//! equality-saturation-based optimization.
//!
//! # Pipeline
//!
//! ```text
//! edge_ast::Program
//!   -> AstToEgglog::lower_program()   (AST -> egglog Terms)
//!   -> equality saturation             (optimize via rewrite rules)
//!   -> sexp::sexp_to_expr()              (extract optimized IR)
//!   -> EvmProgram                      (ready for codegen)
//! ```

#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
// Allow common patterns in egglog/e-graph IR code:
// - Rc::clone() is idiomatic for reference-counted pointers in e-graph code
// - Doc backticks are not yet consistent across the new IR modules
// - Some functions are not yet const but could be
#![allow(
    clippy::redundant_clone,
    clippy::clone_on_ref_ptr,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::uninlined_format_args,
    clippy::option_if_let_else,
    clippy::type_complexity,
    clippy::match_same_arms,
    clippy::collapsible_match,
    clippy::needless_pass_by_ref_mut,
    clippy::explicit_iter_loop,
    clippy::new_without_default,
    clippy::unnecessary_struct_initialization
)]

pub mod ast_helpers;
pub mod costs;
pub mod optimizations;
pub mod schedule;
pub mod schema;
pub mod sexp;
pub mod to_egglog;

pub use costs::OptimizeFor;
pub use schema::{EvmContract, EvmExpr, EvmProgram, RcExpr};

/// Errors that can occur during IR lowering or optimization.
#[derive(Debug, thiserror::Error)]
pub enum IrError {
    /// Error during AST lowering
    #[error("lowering error: {0}")]
    Lowering(String),
    /// Error during egglog execution
    #[error("egglog error: {0}")]
    Egglog(String),
    /// Error during extraction
    #[error("extraction error: {0}")]
    Extraction(String),
    /// Unsupported feature
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// Build the egglog prologue: schema + all optimization rules.
///
/// This string is prepended to every egglog program before execution.
/// The `optimize_for` parameter controls the `:cost` annotations on the schema.
pub fn prologue(optimize_for: OptimizeFor) -> String {
    let schema = costs::schema_with_costs(include_str!("schema.egg"), optimize_for);
    [
        schema.as_str(),
        include_str!("optimizations/peepholes.egg"),
        include_str!("optimizations/arithmetic.egg"),
        include_str!("optimizations/storage.egg"),
        include_str!("optimizations/memory.egg"),
        include_str!("optimizations/dead_code.egg"),
        include_str!("optimizations/cse.egg"),
        &schedule::rulesets(),
    ]
    .join("\n")
}

/// Lower an AST program to the egglog IR, optionally optimize, and extract.
///
/// This is the main entry point for the IR crate.
///
/// - `optimization_level == 0`: no egglog pass, direct lowering and extraction
/// - `optimization_level >= 1`: run equality saturation with appropriate schedule
pub fn lower_and_optimize(
    program: &edge_ast::Program,
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<EvmProgram, IrError> {
    // 1. Lower AST -> IR structs
    let mut lowering = to_egglog::AstToEgglog::new();
    let ir_program = lowering.lower_program(program)?;

    if optimization_level == 0 {
        // No optimization: return the directly-lowered IR
        return Ok(ir_program);
    }

    // 2. Optimize each contract's runtime through egglog equality saturation
    let schedule = schedule::make_schedule(optimization_level);
    let mut optimized_contracts = Vec::new();

    for contract in &ir_program.contracts {
        let runtime_sexp = sexp::expr_to_sexp(&contract.runtime);
        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n\n{}\n\n(extract __runtime)\n",
            prologue(optimize_for),
            runtime_sexp,
            schedule
        );

        let mut egraph = egglog::EGraph::default();
        let outputs = egraph
            .parse_and_run_program(None, &egglog_program)
            .map_err(|e| IrError::Egglog(format!("{e}")))?;

        // The last output is the extracted expression from (extract __runtime)
        let extracted_sexp = outputs
            .last()
            .ok_or_else(|| IrError::Extraction("no output from extract".to_owned()))?;

        tracing::info!(
            "Optimized contract {} at -O{}",
            contract.name,
            optimization_level
        );

        let optimized_runtime = sexp::sexp_to_expr(extracted_sexp)?;
        optimized_contracts.push(EvmContract {
            name: contract.name.clone(),
            storage_fields: contract.storage_fields.clone(),
            constructor: contract.constructor.clone(),
            runtime: optimized_runtime,
        });
    }

    Ok(EvmProgram {
        contracts: optimized_contracts,
        free_functions: ir_program.free_functions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prologue_parses_gas() {
        let prologue = prologue(OptimizeFor::Gas);
        assert!(!prologue.is_empty(), "Prologue should not be empty");
        assert!(prologue.contains("EvmExpr"));
        assert!(prologue.contains("EvmBinaryOp"));
        assert!(prologue.contains("peepholes"));
        assert!(prologue.contains(":cost"));
    }

    #[test]
    fn test_prologue_parses_size() {
        let prologue = prologue(OptimizeFor::Size);
        assert!(!prologue.is_empty());
        assert!(prologue.contains(":cost 1)"));
    }

    #[test]
    fn test_egglog_roundtrip_erc20() {
        let source = std::fs::read_to_string("../../examples/erc20.edge").unwrap();
        let mut parser = edge_parser::Parser::new(&source).unwrap();
        let ast = parser.parse().unwrap();

        let mut lowering = to_egglog::AstToEgglog::new();
        let ir_program = lowering.lower_program(&ast).unwrap();

        let contract = &ir_program.contracts[0];
        let runtime_sexp = sexp::expr_to_sexp(&contract.runtime);
        assert!(!runtime_sexp.is_empty());

        let schedule = schedule::make_schedule(1);
        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n\n{}\n\n(extract __runtime)\n",
            prologue(OptimizeFor::Gas),
            runtime_sexp,
            schedule
        );

        let mut egraph = egglog::EGraph::default();
        let result = egraph.parse_and_run_program(None, &egglog_program);
        match &result {
            Err(e) => eprintln!("EGGLOG ERROR: {e}"),
            Ok(outputs) => {
                eprintln!("SUCCESS: {} outputs", outputs.len());
                if let Some(last) = outputs.last() {
                    eprintln!("extracted (first 200): {}", &last[..last.len().min(200)]);
                }
            }
        }
        result.unwrap();
    }

    #[test]
    fn test_egglog_roundtrip_counter() {
        let source = std::fs::read_to_string("../../examples/counter.edge").unwrap();
        let mut parser = edge_parser::Parser::new(&source).unwrap();
        let ast = parser.parse().unwrap();

        let mut lowering = to_egglog::AstToEgglog::new();
        let ir_program = lowering.lower_program(&ast).unwrap();

        let contract = &ir_program.contracts[0];
        let runtime_sexp = sexp::expr_to_sexp(&contract.runtime);
        eprintln!("sexp length: {}", runtime_sexp.len());
        eprintln!(
            "sexp (first 500): {}",
            &runtime_sexp[..runtime_sexp.len().min(500)]
        );

        let schedule = schedule::make_schedule(1);
        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n\n{}\n\n(extract __runtime)\n",
            prologue(OptimizeFor::Gas),
            runtime_sexp,
            schedule
        );

        let mut egraph = egglog::EGraph::default();
        let result = egraph.parse_and_run_program(None, &egglog_program);
        match &result {
            Err(e) => eprintln!("EGGLOG ERROR: {e}"),
            Ok(outputs) => {
                eprintln!("SUCCESS: {} outputs", outputs.len());
                if let Some(last) = outputs.last() {
                    eprintln!("extracted (first 200): {}", &last[..last.len().min(200)]);
                }
            }
        }
        result.unwrap();
    }
}
