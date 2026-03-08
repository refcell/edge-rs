# edge-driver

Compiler driver that orchestrates the full Edge compilation pipeline. Reads source files and runs each phase in sequence: lex, parse, type-check, lower to IR, and emit EVM bytecode. The `standard_json` module provides a Solidity-compatible standard JSON I/O interface so that build tools like Foundry can drive `edgec` with a single JSON blob on stdin.

## Pipeline Position

```
source -> lexer -> parser -> AST -> typeck -> IR -> codegen -> driver -> bytecode
                                                               ^^^^^^
```

## What It Does

- Reads `.edge` source from disk via `CompilerConfig`
- Runs the lexer, parser, type checker, IR lowerer, and code generator in order
- Supports early-exit emit modes: `Tokens`, `Ast`, or full `Bytecode`
- Converts between `edge-typeck`, `edge-ir`, and `edge-codegen` types
- Collects diagnostics in a `Session` and reports errors with source context

## Key Types

- **`Compiler`** -- Main entry point; constructed with a `CompilerConfig`, drives `compile()`
- **`CompileOutput`** -- Holds optional tokens, AST, ABI, or bytecode depending on emit mode
- **`CompileError`** -- Covers I/O, lex, parse, type-check, IR lowering, and codegen failures
- **`CompilerConfig`** -- Input file path, output path, emit kind, optimization level, verbose flag
- **`EmitKind`** -- What the compiler should produce: `Tokens`, `Ast`, `Abi`, or `Bytecode`
- **`Session`** -- Per-compilation state: config, source text, and accumulated diagnostics
- **`StandardJsonInput`** -- Deserialized standard JSON request (sources, settings)
- **`StandardJsonOutput`** -- Serialized standard JSON response (contracts, errors)
- **`compile_standard_json`** -- Compiles all sources from a `StandardJsonInput` and returns a `StandardJsonOutput`

## Usage

```rust,no_run
use std::path::PathBuf;
use edge_driver::{compiler::Compiler, config::CompilerConfig};

let config = CompilerConfig::new(PathBuf::from("examples/counter.edge"));
let mut compiler = Compiler::new(config).unwrap();
let output = compiler.compile().unwrap();

if let Some(bytecode) = output.bytecode {
    println!("{}", hex::encode(&bytecode));
}

if let Some(abi) = &output.abi {
    println!("{}", serde_json::to_string_pretty(abi).unwrap());
}
```

The `compile_standard_json` function accepts a `StandardJsonInput` and returns a `StandardJsonOutput` containing ABI and bytecode for every contract in the input. Errors are reported inside the output rather than as Rust-level failures.

```rust,no_run
use edge_driver::standard_json::{compile_standard_json, StandardJsonInput};

let json_str = r#"{"language":"Edge","sources":{"counter.edge":{"content":"..."}}}"#;
let input: StandardJsonInput = serde_json::from_str(json_str).unwrap();
let output = compile_standard_json(input);
println!("{}", serde_json::to_string_pretty(&output).unwrap());
```

## Integration

- **Input**: `.edge` source file on disk
- **Output**: `CompileOutput` (tokens, AST, or bytecode)
- **Dependencies**: `edge-lexer`, `edge-parser`, `edge-ast`, `edge-typeck`, `edge-ir`, `edge-codegen`, `edge-diagnostics`, `edge-types`
