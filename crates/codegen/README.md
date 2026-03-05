# edge-codegen

EVM bytecode emitter for the Edge compiler. Converts IR instructions into executable EVM bytecode with a selector-based dispatcher and two-pass label resolution.

## Pipeline Position

```
source -> lexer -> parser -> AST -> typeck -> IR -> codegen -> driver -> bytecode
                                                    ^^^^^^^
```

## What It Does

- Emits a function dispatcher that reads `calldata[0:4]`, compares against known selectors, and jumps to the matching function body
- Translates each `GenInstr` into raw EVM opcode bytes (`PUSH1`..`PUSH32`, `ADD`, `SSTORE`, etc.)
- Resolves symbolic labels in a two-pass assembler: first pass records label offsets, second pass patches `PUSH2` placeholders with concrete byte positions
- Falls back to `REVERT` for unknown selectors

## Key Types

- **`CodeGenerator`** -- Stateless generator; call `generate(&ContractInput)` to get `Vec<u8>` bytecode
- **`ContractInput`** -- A contract name plus its `FunctionInput` list
- **`FunctionInput`** -- Function name, ABI selector, visibility flag, and `GenInstr` body
- **`GenInstr`** -- Simplified IR instruction enum mirroring `IrInstruction` for codegen consumption
- **`Opcode`** -- Full EVM opcode enum (`#[repr(u8)]`) with `PUSH0`..`PUSH32`, `DUP1`..`DUP16`, `SWAP1`..`SWAP16`, arithmetic, memory, storage, control flow, and logging opcodes
- **`CodeGenError`** -- Undefined labels, unsupported instructions, no public functions

## Bytecode Layout

```
[ dispatcher ] [ fn_foo body ] [ fn_bar body ] ... [ __revert__ fallback ]
```

## Usage

```rust,no_run
use edge_codegen::{CodeGenerator, ContractInput, FunctionInput, GenInstr};

let input = ContractInput {
    name: "MyContract".into(),
    functions: vec![/* ... */],
};

let bytecode = CodeGenerator::new().generate(&input).unwrap();
println!("0x{}", hex::encode(&bytecode));
```

## Integration

- **Input**: `ContractInput` (built from `edge-ir::IrProgram` by the driver)
- **Output**: `Vec<u8>` EVM runtime bytecode
- **Dependencies**: `alloy-primitives`, `indexmap`
