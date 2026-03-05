# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.1.6] - 2025-03-04

### Added

- Full compilation pipeline: lex, parse, type-check, IR lower, codegen producing EVM bytecode
- Driver crate (`edge-driver`) orchestrating all compiler phases with early-exit emit modes (tokens, AST, bytecode)
- Type checker (`edge-typeck`) with storage layout computation, ABI selector generation, and synthetic `__module__` contracts for top-level functions
- IR lowerer (`edge-ir`) translating AST to stack-based instructions with symbolic label support
- EVM code generator (`edge-codegen`) with two-pass assembler, selector dispatcher, and label resolution
- Diagnostics crate (`edge-diagnostics`) for structured error reporting with source spans and pretty-printed output
- Calldata argument loading from ABI-standard offsets (4 + 32*i) for function parameters
- If/else control flow with `JUMPI`/`JUMPDEST` label chains
- Storage variable support via `SLOAD`/`SSTORE` with slot assignments from typeck
- Builtin intrinsics: `@caller()`, `@value()`, `@timestamp()`, `@blocknumber()`
- Example contracts: `counter.edge`, `expressions.edge`, `erc20.edge`
- End-to-end acceptance tests (`edge-e2e`) running compiled bytecode on an in-memory REVM instance
- E2E tests for counter (increment, decrement, get, reset, stateful sequences)
- E2E tests for expressions (arithmetic, comparisons, bitwise, operator precedence)
- E2E tests for calldata argument passing (single, two, and three arguments)
- E2E test scaffolding for ERC-20 (compile check, mint, transfer, approve, transferFrom)
- Integration with `alloy-primitives` for `Selector` and `Address` types
- Workspace-level Clippy and rustdoc lint configuration
