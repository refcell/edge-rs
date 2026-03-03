# edge-ast

Abstract Syntax Tree (AST) for the Edge language.

This crate defines all the AST node types that represent the syntactic structure of Edge source code. The AST is produced by the parser and consumed by code generation and semantic analysis phases of the compiler.

## Module Organization

- **`ty`**: Type signatures (primitives, arrays, structs, tuples, unions, functions, etc.)
- **`expr`**: Expression nodes (literals, binary ops, function calls, instantiations, etc.)
- **`stmt`**: Statement nodes (declarations, assignments, control flow, etc.)
- **`item`**: Top-level items (functions, types, traits, impls, contracts, modules)
- **`lit`**: Literal values (integers, strings, booleans, hex, binary)
- **`op`**: Operators (binary and unary)
- **`pattern`**: Pattern matching (union patterns, match arms)

## Design Principles

- All AST nodes store their source location (`Span`) for error reporting
- All types derive `Debug`, `Clone`, and `PartialEq`
- The AST is fully immutable
- Forward references are supported via `Ident` for deferred resolution
- Data locations (stack, memory, calldata, etc.) are explicitly tracked for EVM semantics

## Example

```rust
use edge_ast::{Program, Stmt, Expr, Ident};

// The AST is typically constructed by the parser
// and then traversed for semantic analysis
```

## Integration

The `edge-ast` crate is:
- **Produced by**: `edge-parser`
- **Consumed by**: Semantic analysis and code generation phases
- **Depends on**: `edge-types` (for `Span` and other basic types)
