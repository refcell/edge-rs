# edge-parser

The Edge language parser. Converts a stream of tokens from the lexer into an abstract syntax tree (AST).

## Features

- Recursive descent parser for statements and declarations
- Pratt parsing for binary expressions with proper operator precedence
- Comprehensive error reporting with source spans
- Support for all Edge language constructs

## Usage

```rust,no_run
use edge_parser::parse;

let source = r#"
fn add(a: u8, b: u8) -> (u8) {
    return a + b;
}
"#;

let program = parse(source).expect("parse failed");
// Use the AST...
```

## Architecture

The parser uses a multi-pass approach:

1. **Lexing**: The input source is lexed into tokens (handled by edge-lexer)
2. **Parsing**: Tokens are parsed into AST nodes:
   - Statements: declarations, assignments, control flow
   - Expressions: binary/unary operations, function calls, literals
   - Types: primitive, composite, and named types
