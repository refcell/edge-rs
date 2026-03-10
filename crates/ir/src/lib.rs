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
pub mod mem_region;
pub mod optimizations;
pub mod pretty;
pub mod schedule;
pub mod schema;
pub mod sexp;
pub mod storage_hoist;
pub mod to_egglog;
pub mod u256_sort;
pub mod var_opt;

use std::rc::Rc;

pub use costs::OptimizeFor;
use schema::{EvmBaseType, EvmConstant, EvmType};
pub use schema::{EvmContract, EvmExpr, EvmProgram, RcExpr};

/// Errors that can occur during IR lowering or optimization.
#[derive(Debug, thiserror::Error)]
pub enum IrError {
    /// Error during AST lowering
    #[error("lowering error: {0}")]
    Lowering(String),
    /// Error during AST lowering with source span for diagnostics
    #[error("{message}")]
    LoweringSpanned {
        /// The error message
        message: String,
        /// Source location where the error occurred
        span: edge_types::span::Span,
    },
    /// Rich diagnostic error with multiple labels and notes
    #[error("{}", .0.message)]
    Diagnostic(
        /// The full diagnostic with labels, notes, and severity
        edge_diagnostics::Diagnostic,
    ),
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
        include_str!("optimizations/inline.egg"),
        include_str!("optimizations/const_prop.egg"),
        &schedule::rulesets(),
    ]
    .join("\n")
}

/// Create an egglog `EGraph` with the U256 sort registered.
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
    let pipeline_start = std::time::Instant::now();

    // 1. Lower AST -> IR structs
    let t = std::time::Instant::now();
    let mut lowering = to_egglog::AstToEgglog::new();
    let mut ir_program = lowering.lower_program(program)?;
    tracing::debug!("  lowering: {:?}", t.elapsed());

    // 2. Variable optimizations (store-forwarding, dead elim, inlining, const prop)
    // Runs at ALL optimization levels since these are cheap deterministic transforms.
    // Monomorphization only at O1+ (egglog decides inlining); at O0 keep original calls.
    let t = std::time::Instant::now();
    var_opt::optimize_program(&mut ir_program, optimization_level);
    tracing::debug!("  var_opt: {:?}", t.elapsed());

    // 3. Storage optimizations:
    //    a) Hoist storage ops out of loops (LICM) — egglog can't model iteration
    let t = std::time::Instant::now();
    storage_hoist::hoist_program(&mut ir_program);
    tracing::debug!("  storage_hoist: {:?}", t.elapsed());

    // 4. Resolve symbolic MemRegion nodes to concrete offsets.
    // Runs before egglog so that Add(Const, Const) patterns from
    // region+field offsets get folded by egglog's constant folding.
    let t = std::time::Instant::now();
    mem_region::assign_program_offsets(&mut ir_program);
    tracing::debug!("  mem_region: {:?}", t.elapsed());

    if optimization_level == 0 {
        //    b) At O0 only: forward SStore→SLoad in straight-line code (no egglog)
        let t = std::time::Instant::now();
        storage_hoist::forward_stores_program(&mut ir_program);
        tracing::debug!("  forward_stores: {:?}", t.elapsed());

        let t = std::time::Instant::now();
        var_opt::tighten_drops_program(&mut ir_program);
        tracing::debug!("  tighten_drops: {:?}", t.elapsed());

        let t = std::time::Instant::now();
        var_opt::dead_store_elim_program(&mut ir_program);
        tracing::debug!("  dead_store_elim: {:?}", t.elapsed());

        tracing::debug!("  total IR pipeline: {:?}", pipeline_start.elapsed());
        return Ok(ir_program);
    }
    // At O1+, egglog handles SStore→SLoad forwarding via cross-slot rules in storage.egg

    // 2. Optimize each contract's runtime through egglog equality saturation
    let t_egglog_total = std::time::Instant::now();
    let schedule = schedule::make_schedule(optimization_level);
    let mut optimized_contracts = Vec::new();

    for contract in &ir_program.contracts {
        let t_contract = std::time::Instant::now();
        let runtime_sexp = sexp::expr_to_sexp(&contract.runtime);

        // Collect immutable variable names for bound propagation in egglog
        let immutable_vars = var_opt::collect_immutable_vars(&contract.runtime);
        let immutable_facts: String = immutable_vars
            .iter()
            .map(|name| format!("(ImmutableVar \"{name}\")\n"))
            .collect();

        // Include internal function definitions in the same egraph so that
        // the inline rule (Call + Function → body) can fire.
        let mut func_lets = String::new();
        for (i, func) in contract.internal_functions.iter().enumerate() {
            let func_sexp = sexp::expr_to_sexp(func);
            func_lets.push_str(&format!("(let __fn_{i} {func_sexp})\n"));
        }

        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n{}\n{}\n{}\n\n(extract __runtime)\n",
            prologue(optimize_for),
            runtime_sexp,
            func_lets,
            immutable_facts,
            schedule
        );

        let t_egg = std::time::Instant::now();
        let mut egraph = create_egraph();
        let outputs = egraph
            .parse_and_run_program(None, &egglog_program)
            .map_err(|e| IrError::Egglog(format!("{e}")))?;
        tracing::debug!("    egglog run ({}): {:?}", contract.name, t_egg.elapsed());

        // The last output is the extracted expression from (extract __runtime)
        let extracted_sexp = outputs
            .last()
            .ok_or_else(|| IrError::Extraction("no output from extract".to_owned()))?;

        tracing::info!(
            "Optimized contract {} at -O{}",
            contract.name,
            optimization_level
        );

        let mut optimized_runtime = sexp::sexp_to_expr(extracted_sexp)?;

        // Check for compile-time-detectable constant overflows in narrow types.
        // This catches overflow revealed by egglog const-folding (e.g. through
        // inlined constants). The lowering-time check catches literal cases with
        // source spans; this is the fallback for optimization-revealed cases.
        let overflow_errors = check_const_overflow(&optimized_runtime);
        if !overflow_errors.is_empty() {
            let mut diag = edge_diagnostics::Diagnostic::error(
                "arithmetic overflow detected after optimization",
            );
            for err in &overflow_errors {
                diag = diag.with_note(err.clone());
            }
            return Err(IrError::Diagnostic(diag));
        }

        // Post-egglog cleanup: simplify state params and remove dead code
        optimized_runtime = cleanup::cleanup_expr_pub(&optimized_runtime);

        // Only keep internal functions still referenced (directly or transitively)
        // by Call nodes in the optimized runtime. Monomorphized functions that
        // were inlined by egglog are no longer needed.
        let mut referenced = collect_call_names(&optimized_runtime);
        // Transitively collect: if a kept function calls another, keep that too
        loop {
            let mut new_names = std::collections::HashSet::new();
            for func in &contract.internal_functions {
                if let EvmExpr::Function(name, _, _, body) = func.as_ref() {
                    if referenced.contains(name.as_str()) {
                        for n in collect_call_names(body) {
                            if !referenced.contains(&n) {
                                new_names.insert(n);
                            }
                        }
                    }
                }
            }
            if new_names.is_empty() {
                break;
            }
            referenced.extend(new_names);
        }
        let mut optimized_functions = Vec::new();
        for func in &contract.internal_functions {
            let name = match func.as_ref() {
                EvmExpr::Function(n, ..) => n,
                _ => continue,
            };
            if !referenced.contains(name.as_str()) {
                continue;
            }
            let func_sexp = sexp::expr_to_sexp(func);
            let func_program = format!(
                "{}\n\n(let __func {})\n\n{}\n\n(extract __func)\n",
                prologue(optimize_for),
                func_sexp,
                schedule
            );
            let mut func_egraph = create_egraph();
            let func_outputs = func_egraph
                .parse_and_run_program(None, &func_program)
                .map_err(|e| IrError::Egglog(format!("{e}")))?;
            let func_extracted = func_outputs
                .last()
                .ok_or_else(|| IrError::Extraction("no output from func extract".to_owned()))?;
            let optimized_func = sexp::sexp_to_expr(func_extracted)?;
            let optimized_func = cleanup::cleanup_expr_pub(&optimized_func);
            optimized_functions.push(optimized_func);
        }

        tracing::debug!(
            "    contract {} total: {:?}",
            contract.name,
            t_contract.elapsed()
        );
        optimized_contracts.push(EvmContract {
            name: contract.name.clone(),
            storage_fields: contract.storage_fields.clone(),
            constructor: Rc::clone(&contract.constructor),
            runtime: optimized_runtime,
            internal_functions: optimized_functions,
            memory_high_water: contract.memory_high_water,
        });
    }
    tracing::debug!("  egglog total: {:?}", t_egglog_total.elapsed());

    let mut result = EvmProgram {
        contracts: optimized_contracts,
        free_functions: ir_program.free_functions,
        warnings: ir_program.warnings,
    };

    // Post-egglog: forward SStore→SLoad and eliminate dead stores in straight-line code.
    // Egglog's storage-opt rules only handle state-threaded SStore chains, not Concat-chained
    // SStores (which use Arg(StateT) as state). This pass handles the Concat case.
    storage_hoist::forward_stores_program(&mut result);

    let t = std::time::Instant::now();
    var_opt::tighten_drops_program(&mut result);
    tracing::debug!("  tighten_drops: {:?}", t.elapsed());

    // Eliminate dead stores (write-before-write with no intervening read)
    let t = std::time::Instant::now();
    var_opt::dead_store_elim_program(&mut result);
    tracing::debug!("  dead_store_elim: {:?}", t.elapsed());

    // Deduplicate CalldataLoad nodes (hoist repeated loads into LetBind vars)
    let t = std::time::Instant::now();
    var_opt::calldataload_cse_program(&mut result);
    tracing::debug!("  calldataload_cse: {:?}", t.elapsed());

    tracing::debug!("  total IR pipeline: {:?}", pipeline_start.elapsed());

    Ok(result)
}

/// Collect all function names referenced by `Call` nodes in an expression.
fn collect_call_names(expr: &schema::RcExpr) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    collect_call_names_rec(expr, &mut names);
    names
}

fn collect_call_names_rec(expr: &schema::RcExpr, names: &mut std::collections::HashSet<String>) {
    match expr.as_ref() {
        EvmExpr::Call(name, args) => {
            names.insert(name.clone());
            for a in args {
                collect_call_names_rec(a, names);
            }
        }
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) | EvmExpr::DoWhile(a, b) => {
            collect_call_names_rec(a, names);
            collect_call_names_rec(b, names);
        }
        EvmExpr::Uop(_, a) | EvmExpr::VarStore(_, a) => {
            collect_call_names_rec(a, names);
        }
        EvmExpr::Get(a, _) => collect_call_names_rec(a, names),
        EvmExpr::If(c, i, t, e) => {
            collect_call_names_rec(c, names);
            collect_call_names_rec(i, names);
            collect_call_names_rec(t, names);
            collect_call_names_rec(e, names);
        }
        EvmExpr::LetBind(_, init, body) => {
            collect_call_names_rec(init, names);
            collect_call_names_rec(body, names);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_call_names_rec(a, names);
            collect_call_names_rec(b, names);
            collect_call_names_rec(c, names);
        }
        EvmExpr::Log(_, topics, d, s, st) => {
            for t in topics {
                collect_call_names_rec(t, names);
            }
            collect_call_names_rec(d, names);
            collect_call_names_rec(s, names);
            collect_call_names_rec(st, names);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            collect_call_names_rec(a, names);
            collect_call_names_rec(b, names);
            collect_call_names_rec(c, names);
            collect_call_names_rec(d, names);
            collect_call_names_rec(e, names);
            collect_call_names_rec(f, names);
            collect_call_names_rec(g, names);
        }
        EvmExpr::Function(_, _, _, body) => collect_call_names_rec(body, names),
        _ => {}
    }
}

/// Check for constant values that overflow their declared narrow type.
///
/// After egglog optimization + extraction, walk the IR tree and look for
/// `Const(val, UIntT(N))` where `N < 256` and `val >= 2^N`. This detects
/// compile-time-provable overflows like `250u8 + 250u8`.
fn check_const_overflow(expr: &schema::RcExpr) -> Vec<String> {
    let mut errors = Vec::new();
    check_const_overflow_rec(expr, &mut errors);
    errors
}

fn check_const_overflow_rec(expr: &schema::RcExpr, errors: &mut Vec<String>) {
    match expr.as_ref() {
        EvmExpr::Const(val, EvmType::Base(EvmBaseType::UIntT(width)), _) if *width < 256 => {
            let max_val = if *width == 0 {
                0u128
            } else {
                (1u128 << *width) - 1
            };
            let exceeds = match val {
                EvmConstant::SmallInt(n) => {
                    if *n < 0 {
                        true // negative value in unsigned type
                    } else {
                        *n as u128 > max_val
                    }
                }
                EvmConstant::LargeInt(hex) => {
                    // LargeInt always exceeds narrow types (it's > i64::MAX)
                    // unless the hex happens to be small. Parse and check.
                    ruint::aliases::U256::from_str_radix(hex, 16)
                        .map(|v| v > ruint::aliases::U256::from(max_val))
                        .unwrap_or(false)
                }
                _ => false,
            };
            if exceeds {
                errors.push(format!(
                    "constant value {} overflows u{} (max {})",
                    match val {
                        EvmConstant::SmallInt(n) => format!("{n}"),
                        EvmConstant::LargeInt(hex) => format!("0x{hex}"),
                        _ => "?".to_string(),
                    },
                    width,
                    max_val
                ));
            }
        }
        EvmExpr::Const(val, EvmType::Base(EvmBaseType::IntT(width)), _) if *width < 256 => {
            let half = if *width <= 1 {
                1i128
            } else {
                1i128 << (*width - 1)
            };
            let min_val = -half;
            let max_val = half - 1;
            let exceeds = match val {
                EvmConstant::SmallInt(n) => (*n as i128) < min_val || (*n as i128) > max_val,
                _ => false,
            };
            if exceeds {
                errors.push(format!(
                    "constant value {} overflows i{} (range {}..={})",
                    match val {
                        EvmConstant::SmallInt(n) => format!("{n}"),
                        _ => "?".to_string(),
                    },
                    width,
                    min_val,
                    max_val
                ));
            }
        }
        _ => {}
    }

    // Recurse into children
    match expr.as_ref() {
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) | EvmExpr::DoWhile(a, b) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(b, errors);
        }
        EvmExpr::Uop(_, a) | EvmExpr::VarStore(_, a) | EvmExpr::Get(a, _) => {
            check_const_overflow_rec(a, errors);
        }
        EvmExpr::If(c, i, t, e) => {
            check_const_overflow_rec(c, errors);
            check_const_overflow_rec(i, errors);
            check_const_overflow_rec(t, errors);
            check_const_overflow_rec(e, errors);
        }
        EvmExpr::LetBind(_, init, body) => {
            check_const_overflow_rec(init, errors);
            check_const_overflow_rec(body, errors);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(b, errors);
            check_const_overflow_rec(c, errors);
        }
        EvmExpr::Log(_, topics, d, s, st) => {
            for t in topics {
                check_const_overflow_rec(t, errors);
            }
            check_const_overflow_rec(d, errors);
            check_const_overflow_rec(s, errors);
            check_const_overflow_rec(st, errors);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(b, errors);
            check_const_overflow_rec(c, errors);
            check_const_overflow_rec(d, errors);
            check_const_overflow_rec(e, errors);
            check_const_overflow_rec(f, errors);
            check_const_overflow_rec(g, errors);
        }
        EvmExpr::EnvRead(_, s) => check_const_overflow_rec(s, errors),
        EvmExpr::EnvRead1(_, a, s) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(s, errors);
        }
        EvmExpr::Function(_, _, _, body) => check_const_overflow_rec(body, errors),
        EvmExpr::Call(_, args) => {
            for a in args {
                check_const_overflow_rec(a, errors);
            }
        }
        _ => {}
    }
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
            "CheckedAdd should have been elided to Add, got: {extracted}"
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
            "CheckedAdd should NOT be elided without tight bounds, got: {extracted}"
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
            "Both CheckedAdds should be elided via cascading bounds, got: {extracted}"
        );
    }

    #[test]
    fn test_const_prop_simple() {
        // LetBind("x", Const(1), LetBind("y", Var("x"), Var("y")))
        // Should flatten to just Const(1)
        let program = format!(
            "{}\n\n{}\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __test (LetBind "x"
                (Const (SmallInt 1) (Base (UIntT 256)) (InFunction "test"))
                (LetBind "y"
                    (Var "x")
                    (Var "y"))))"#,
            "(ImmutableVar \"x\")\n(ImmutableVar \"y\")",
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run const-prop) (run u256-const-fold)))
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))",
            "(extract __test)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        eprintln!("const_prop result: {extracted}");
        // Should not contain LetBind or Var — everything propagated to Const(1)
        assert!(
            !extracted.contains("LetBind"),
            "LetBinds should be eliminated by const-prop, got: {extracted}"
        );
        assert!(
            !extracted.contains("Var"),
            "Vars should be replaced by const, got: {extracted}"
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
