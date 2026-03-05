//! Benchmarks for the edgec compiler pipeline stages.
#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use edge_driver::compiler::Compiler;
use edge_lexer::lexer::Lexer;
use edge_typeck::TypeChecker;

const COUNTER_SRC: &str = include_str!("../../../examples/counter.edge");
const ERC20_SRC: &str = include_str!("../../../examples/erc20.edge");
const ERC20_FULL_SRC: &str = include_str!("../../../examples/tokens/erc20.edge");

const INPUTS: &[(&str, &str)] = &[
    ("counter", COUNTER_SRC),
    ("erc20", ERC20_SRC),
    ("erc20_full", ERC20_FULL_SRC),
];

/// Benchmark just the lexing phase (tokenization only).
fn bench_lex(c: &mut Criterion) {
    let mut group = c.benchmark_group("lex");

    for (name, src) in INPUTS {
        group.bench_with_input(BenchmarkId::from_parameter(name), src, |b, src| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(src));
                // Collect all tokens, discarding errors
                lexer.filter_map(|r| r.ok()).count()
            });
        });
    }

    group.finish();
}

/// Benchmark the parse phase (lexing + AST construction).
fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    for (name, src) in INPUTS {
        group.bench_with_input(BenchmarkId::from_parameter(name), src, |b, src| {
            b.iter(|| edge_parser::parse(black_box(src)).unwrap());
        });
    }

    group.finish();
}

/// Benchmark the type-check phase alone (AST pre-parsed outside timing loop).
fn bench_typeck(c: &mut Criterion) {
    let mut group = c.benchmark_group("typeck");

    for (name, src) in INPUTS {
        let ast = edge_parser::parse(src).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(name), &ast, |b, ast| {
            b.iter(|| TypeChecker::new().check(black_box(ast)).unwrap());
        });
    }

    group.finish();
}

/// Benchmark the full pipeline: parse → typeck → lower → codegen.
fn bench_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");

    for (name, src) in INPUTS {
        group.bench_with_input(BenchmarkId::from_parameter(name), src, |b, src| {
            b.iter(|| {
                let mut compiler = Compiler::from_source(black_box(*src));
                compiler.compile().unwrap()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_lex,
    bench_parse,
    bench_typeck,
    bench_pipeline
);
criterion_main!(benches);
