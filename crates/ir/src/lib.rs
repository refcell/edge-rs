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

pub mod ast_helpers;
pub mod cleanup;
pub mod costs;
pub mod optimizations;
pub mod schedule;
pub mod schema;
pub mod sexp;
pub mod storage_hoist;
pub mod to_egglog;
pub mod u256_sort;
pub mod var_opt;

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
        include_str!("optimizations/range_analysis.egg"),
        include_str!("optimizations/u256_const_fold.egg"),
        include_str!("optimizations/type_propagation.egg"),
        include_str!("optimizations/checked_arithmetic.egg"),
        include_str!("optimizations/cse.egg"),
        &schedule::rulesets(),
    ]
    .join("\n")
}

/// Create an egglog EGraph with the U256 sort registered.
pub fn create_egraph() -> egglog::EGraph {
    let mut egraph = egglog::EGraph::default();
    egraph
        .add_arcsort(
            std::sync::Arc::new(u256_sort::U256Sort),
            egglog::ast::Span::Panic,
        )
        .expect("U256 sort registration");
    egraph
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
    let mut ir_program = lowering.lower_program(program)?;

    // 2. Variable optimizations (store-forwarding, dead elim, inlining, const prop)
    // Runs at ALL optimization levels since these are cheap deterministic transforms.
    var_opt::optimize_program(&mut ir_program);

    // 3. Storage optimizations:
    //    a) Hoist storage ops out of loops (LICM) — egglog can't model iteration
    storage_hoist::hoist_program(&mut ir_program);

    if optimization_level == 0 {
        //    b) At O0 only: forward SStore→SLoad in straight-line code (no egglog)
        storage_hoist::forward_stores_program(&mut ir_program);
        return Ok(ir_program);
    }
    // At O1+, egglog handles SStore→SLoad forwarding via cross-slot rules in storage.egg

    // 2. Optimize each contract's runtime through egglog equality saturation
    let schedule = schedule::make_schedule(optimization_level);
    let mut optimized_contracts = Vec::new();

    for contract in &ir_program.contracts {
        let runtime_sexp = sexp::expr_to_sexp(&contract.runtime);

        // Collect immutable variable names for bound propagation in egglog
        let immutable_vars = var_opt::collect_immutable_vars(&contract.runtime);
        let immutable_facts: String = immutable_vars
            .iter()
            .map(|name| format!("(ImmutableVar \"{}\")\n", name))
            .collect();

        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n\n{}\n{}\n\n(extract __runtime)\n",
            prologue(optimize_for),
            runtime_sexp,
            immutable_facts,
            schedule
        );

        let mut egraph = create_egraph();
        let outputs = egraph
            .parse_and_run_program(None, &egglog_program)
            .map_err(|e| IrError::Egglog(format!("{e}")))?;

        // The last output is the extracted expression from (extract __runtime)
        let extracted_sexp = outputs
            .last()
            .ok_or_else(|| IrError::Extraction("no output from extract".to_owned()))?;

        tracing::info!("Optimized contract {} at -O{}", contract.name, optimization_level);

        let mut optimized_runtime = sexp::sexp_to_expr(extracted_sexp)?;

        // Post-egglog cleanup: simplify state params and remove dead code
        optimized_runtime = cleanup::cleanup_expr_pub(&optimized_runtime);

        optimized_contracts.push(EvmContract {
            name: contract.name.clone(),
            storage_fields: contract.storage_fields.clone(),
            constructor: contract.constructor.clone(),
            runtime: optimized_runtime,
        });
    }

    let mut result = EvmProgram {
        contracts: optimized_contracts,
        free_functions: ir_program.free_functions,
    };

    // Post-egglog: forward SStore→SLoad and eliminate dead stores in straight-line code.
    // Egglog's storage-opt rules only handle state-threaded SStore chains, not Concat-chained
    // SStores (which use Arg(StateT) as state). This pass handles the Concat case.
    storage_hoist::forward_stores_program(&mut result);

    Ok(result)
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

        let mut egraph = create_egraph();
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
    fn test_checked_elision_via_range_analysis() {
        // Verify that range analysis elides CheckedAdd→Add when bounds prove safety.
        // Input: (calldataload(4) & 0xFF) + 1 — max value 256, no overflow possible.
        let program = format!(
            "{}\n\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __test (Bop (OpCheckedAdd)
                (Bop (OpAnd)
                    (Bop (OpCalldataLoad)
                        (Const (SmallInt 4) (Base (UIntT 256)) (InFunction "test"))
                        (Arg (Base (StateT)) (InFunction "test")))
                    (Const (SmallInt 255) (Base (UIntT 256)) (InFunction "test")))
                (Const (SmallInt 1) (Base (UIntT 256)) (InFunction "test"))))"#,
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))",
            "(extract __test)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        assert!(
            !extracted.contains("OpCheckedAdd"),
            "CheckedAdd should have been elided to Add, got: {}", extracted
        );
    }

    #[test]
    fn test_checked_not_elided_without_bounds() {
        // Verify CheckedAdd is NOT elided when operands have no tight bounds.
        // Input: calldataload(4) + calldataload(36) — both are full u256 range.
        let program = format!(
            "{}\n\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __test (Bop (OpCheckedAdd)
                (Bop (OpCalldataLoad)
                    (Const (SmallInt 4) (Base (UIntT 256)) (InFunction "test"))
                    (Arg (Base (StateT)) (InFunction "test")))
                (Bop (OpCalldataLoad)
                    (Const (SmallInt 36) (Base (UIntT 256)) (InFunction "test"))
                    (Arg (Base (StateT)) (InFunction "test")))))"#,
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))",
            "(extract __test)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        assert!(
            extracted.contains("OpCheckedAdd"),
            "CheckedAdd should NOT be elided without tight bounds, got: {}", extracted
        );
    }

    #[test]
    fn test_checked_elision_cascading() {
        // Verify cascading elision: (x & 0xff) + 1 + 2
        // First add: max(255+1)=256, no overflow → elided
        // Second add needs bounds from first result: max(256+2)=258 → elided
        let program = format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __masked (Bop (OpAnd)
                (Bop (OpCalldataLoad)
                    (Const (SmallInt 4) (Base (UIntT 256)) (InFunction "test"))
                    (Arg (Base (StateT)) (InFunction "test")))
                (Const (SmallInt 255) (Base (UIntT 256)) (InFunction "test"))))"#,
            r#"(let __plus1 (Bop (OpCheckedAdd) __masked
                (Const (SmallInt 1) (Base (UIntT 256)) (InFunction "test"))))"#,
            r#"(let __plus3 (Bop (OpCheckedAdd) __plus1
                (Const (SmallInt 2) (Base (UIntT 256)) (InFunction "test"))))"#,
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))\n(extract __plus3)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        assert!(
            !extracted.contains("OpCheckedAdd"),
            "Both CheckedAdds should be elided via cascading bounds, got: {}", extracted
        );
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
        eprintln!("sexp (first 500): {}", &runtime_sexp[..runtime_sexp.len().min(500)]);

        let schedule = schedule::make_schedule(1);
        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n\n{}\n\n(extract __runtime)\n",
            prologue(OptimizeFor::Gas),
            runtime_sexp,
            schedule
        );

        let mut egraph = create_egraph();
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
