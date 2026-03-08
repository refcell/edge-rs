# edge-driver

Compiler driver that orchestrates the full Edge compilation pipeline. Reads source files and runs each phase in sequence: lex, parse, type-check, lower to IR, and emit EVM bytecode.

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
- **`CompileOutput`** -- Holds optional tokens, AST, or bytecode depending on emit mode
- **`CompileError`** -- Covers I/O, lex, parse, type-check, IR lowering, and codegen failures
- **`CompilerConfig`** -- Input file path, output path, emit kind, optimization level, verbose flag
- **`EmitKind`** -- What the compiler should produce: `Tokens`, `Ast`, or `Bytecode`
- **`Session`** -- Per-compilation state: config, source text, and accumulated diagnostics

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
```

## Integration

- **Input**: `.edge` source file on disk
- **Output**: `CompileOutput` (tokens, AST, or bytecode)
- **Dependencies**: `edge-lexer`, `edge-parser`, `edge-ast`, `edge-typeck`, `edge-ir`, `edge-codegen`, `edge-diagnostics`, `edge-types`
