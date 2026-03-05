# edge-ir

Intermediate representation and AST-to-IR lowering for the Edge compiler. Translates typed AST nodes into a flat sequence of EVM-oriented instructions with symbolic labels.

## Pipeline Position

```
source -> lexer -> parser -> AST -> typeck -> IR -> codegen -> driver -> bytecode
                                              ^^
```

## What It Does

- Lowers expressions, statements, and control flow into stack-based IR instructions
- Allocates local variables in linear memory (32-byte slots)
- Loads function parameters from calldata at ABI-standard offsets
- Emits symbolic `PushLabel` / `JumpDest` pairs for branches (resolved later by codegen)
- Handles storage reads/writes via `SLoad` / `SStore` with slot numbers from typeck
- Maps `@caller()`, `@value()`, `@timestamp()` builtins to their EVM opcodes

## Key Types

- **`Lowerer`** -- Stateful lowerer; takes storage slots and function metadata, produces `IrProgram`
- **`IrProgram`** -- Collection of `IrContract` values
- **`IrContract`** -- A named contract with its lowered functions
- **`IrFunction`** -- Name, ABI selector, visibility, instruction body, and local memory size
- **`IrInstruction`** -- Enum of ~40 EVM-like operations (Push, Pop, Add, Jump, SLoad, etc.)
- **`FnMeta`** -- Function metadata passed into the lowerer (name, selector, visibility, params)
- **`LowerError`** -- Unsupported expressions/statements, undefined variables

## Usage

```rust,no_run
use edge_ir::{Lowerer, FnMeta};
use indexmap::IndexMap;

let storage_slots = IndexMap::new();
let fn_metas = vec![FnMeta {
    name: "get".into(),
    selector: [0x6d, 0x4c, 0xe6, 0x3c].into(),
    is_pub: true,
    params: vec![],
}];

let lowerer = Lowerer::new(storage_slots, fn_metas);
// let ir_program = lowerer.lower(&ast_program).unwrap();
```

## Integration

- **Input**: `edge_ast::Program` + `FnMeta` from `edge-typeck`
- **Output**: `IrProgram` (consumed by `edge-codegen`)
- **Dependencies**: `edge-ast`, `alloy-primitives`, `indexmap`
