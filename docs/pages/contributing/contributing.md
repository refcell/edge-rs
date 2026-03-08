---
title: Contributing
---

# Contributing

This document covers contribution guidelines for [edge-rs](https://github.com/refcell/edge-rs).

## Issues

Search [existing issues](https://github.com/refcell/edge-rs/issues) before
opening a new one. When reporting a bug, include the `edgec --version` output,
the source file that triggered the problem, and the full error output.

## Pull requests

1. Fork the repository and create a branch from `main`.
2. Build the project: `just build` (or `cargo build --workspace`).
3. Run the test suite before submitting: `just test` (or `cargo test --workspace`).
4. Run the linter: `just lint`.
5. Open a pull request against `main` with a clear description of the change.

## Development workflow

The repository includes a [`Justfile`](https://github.com/casey/just) with
common workflows:

```bash
just build           # build all crates
just test            # run all tests
just lint            # run all lints (format, clippy, deny, docs)
just e2e             # run end-to-end tests
just bench           # run benchmarks
just check-examples  # parse all example contracts
just check-stdlib    # parse all stdlib contracts
just docs            # serve the documentation site locally
just docs-build      # build the documentation site
```

:::note
`just lint` runs four checks: `check-format` (requires `cargo +nightly fmt`),
`check-clippy`, `check-deny`, and `check-docs`. All four must pass for CI to
be green.
:::

## Crate structure

The compiler is organized into these crates:

| Crate | Path | Purpose |
|---|---|---|
| `edge-lexer` | `crates/lexer/` | Tokenization |
| `edge-parser` | `crates/parser/` | Parsing |
| `edge-ast` | `crates/ast/` | AST types |
| `edge-types` | `crates/types/` | Shared type definitions |
| `edge-typeck` | `crates/typeck/` | Type checking |
| `edge-diagnostics` | `crates/diagnostics/` | Error reporting |
| `edge-ir` | `crates/ir/` | IR lowering + egglog optimization |
| `edge-codegen` | `crates/codegen/` | Bytecode generation + optimizer |
| `edge-driver` | `crates/driver/` | Pipeline orchestration |
| `edge-lsp` | `crates/lsp/` | Language server |
| `edge-evm-tests` | `crates/evm-tests/` | EVM test host |
| `edge-bench` | `crates/bench/` | Benchmarks |
| `edge-e2e` | `crates/e2e/` | End-to-end tests |

For a detailed walkthrough of the compiler pipeline, see
[Compiler Architecture](/compiler/overview).

## Labels

Issues and PRs are tagged to indicate their status and area:

- **bug** — something is broken
- **enhancement** — new feature or improvement
- **documentation** — docs-only changes
- **good first issue** — suitable for first-time contributors

## Assistance

For questions or discussion, open an issue on
[GitHub](https://github.com/refcell/edge-rs/issues) or see the
[Contact](/contact/contact) page for other ways to reach the team.
