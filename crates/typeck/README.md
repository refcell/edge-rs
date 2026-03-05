# edge-typeck

Type checking and name resolution pass for the Edge compiler. Walks the AST to resolve type information, compute storage layouts, and generate ABI function selectors.

## Pipeline Position

```
source -> lexer -> parser -> AST -> typeck -> IR -> codegen -> driver -> bytecode
                                    ^^^^^^^
```

## What It Does

- Resolves contract declarations and their fields, functions, and constants
- Assigns storage slots to `&s` (storage pointer) fields
- Computes 4-byte ABI selectors via `keccak256("name(types...)")`
- Wraps top-level functions in a synthetic `__module__` contract

## Key Types

- **`TypeChecker`** -- Stateless checker; call `check(&Program)` to produce a `CheckedProgram`
- **`CheckedProgram`** -- Output of type checking; holds a list of `ContractInfo`
- **`ContractInfo`** -- A contract's name, storage layout, functions, and constants
- **`FnInfo`** -- Per-function metadata: name, selector, params, returns, visibility, body
- **`StorageLayout`** -- Ordered map from field name to u256 storage slot index
- **`ConstValue`** -- A compile-time constant (name + resolved value)
- **`TypeCheckError`** -- Errors: undefined symbols, type mismatches, missing contracts

## Usage

```rust,no_run
use edge_typeck::TypeChecker;
use edge_parser::parse;

let program = parse("contract C { pub fn get() -> (u256) { return 0; } }").unwrap();
let checked = TypeChecker::new().check(&program).unwrap();

for contract in &checked.contracts {
    for func in &contract.functions {
        println!("{}: selector {:?}", func.name, func.selector);
    }
}
```

## Integration

- **Input**: `edge_ast::Program` (from `edge-parser`)
- **Output**: `CheckedProgram` (consumed by `edge-ir` lowerer and `edge-driver`)
- **Dependencies**: `edge-ast`, `edge-types`, `alloy-primitives`, `tiny-keccak`
