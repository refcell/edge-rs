---
title: Built-in
---

# Built-in

Built-in functionality refers to features available during compilation that
are otherwise inaccessible through the language's regular syntax.

The parser accepts any `@identifier` form without validation; unknown builtin
names are caught during IR lowering (semantic analysis), not parsing.

## EVM environment builtins

These builtins read EVM execution context values. Each compiles to a single
`EnvRead` IR node and a corresponding EVM opcode:

| Builtin | EVM opcode | Returns |
|---------|------------|---------|
| `@caller` | `CALLER` | Address of the direct caller |
| `@callvalue` | `CALLVALUE` | Wei sent with the call |
| `@value` | `CALLVALUE` | Alias for `@callvalue` |
| `@calldatasize` | `CALLDATASIZE` | Size of calldata in bytes |
| `@origin` | `ORIGIN` | Transaction originator address |
| `@gasprice` | `GASPRICE` | Gas price of the transaction |
| `@coinbase` | `COINBASE` | Current block's beneficiary address |
| `@timestamp` | `TIMESTAMP` | Current block's timestamp |
| `@number` | `NUMBER` | Current block number |
| `@gaslimit` | `GASLIMIT` | Current block's gas limit |
| `@chainid` | `CHAINID` | Chain ID (EIP-155) |
| `@selfbalance` | `SELFBALANCE` | Balance of the executing contract |
| `@basefee` | `BASEFEE` | Current block's base fee (EIP-1559) |
| `@gas` | `GAS` | Remaining gas |
| `@address` | `ADDRESS` | Address of the executing contract |
| `@codesize` | `CODESIZE` | Size of the executing contract's code |
| `@returndatasize` | `RETURNDATASIZE` | Size of the last call's return data |

All EVM environment builtins are zero-argument. Parentheses are optional:
both `@caller` and `@caller()` are valid. Arguments passed to them are
currently ignored.

```edge
fn checkCaller() {
    if @caller == 0x0000000000000000000000000000000000000000 {
        revert();
    }
}
```

## Comptime builtins

These builtins execute at compile time and are used for type introspection,
compile-time assertions, and code generation.

### Types

```edge
type PrimitiveType;
type StructType;
type UnionType;
type FunctionType;

type TypeInfo =
    | Primitive(PrimitiveType)
    | Struct(StructType)
    | Union(UnionType)
    | Function(FunctionType);
```

:::note
`TypeInfo` does not include an `Enum` variant. In Edge, enums are a subset of
union types (unions where no variant carries data). They are represented as
`Union(UnionType)` in the type system — there is no distinct enum concept at the
AST or IR level.
:::

```edge
type HardFork =
    | Frontier
    | Homestead
    | Dao
    | Tangerine
    | SpuriousDragon
    | Byzantium
    | Constantinople
    | Petersburg
    | Istanbul
    | MuirGlacier
    | Berlin
    | London
    | ArrowGlacier
    | GrayGlacier
    | Paris
    | Shanghai
    | Cancun;
```

### Functions

#### `@typeInfo`

```edge
@typeInfo(typeSignature) -> TypeInfo;
```

Takes a single type signature as an argument and returns a `TypeInfo` union
describing the kind of the type.

#### `@bitsize`

```edge
@bitsize(typeSignature) -> u256;
```

Takes a single type signature as an argument and returns the bitsize of the
underlying type.

#### `@fields`

```edge
@fields(structType) -> [T, N];
```

Takes a single `StructType` as an argument and returns an array of type
signatures of length N, where N is the number of fields in the struct.

#### `@compilerError`

```edge
@compilerError(errorMessage);
```

Emits a compile-time error with the provided message. Useful in `comptime`
branches to enforce invariants.

#### `@hardFork`

```edge
@hardFork() -> HardFork;
```

Returns the target hard fork from the compiler configuration as a `HardFork`
union value.

#### `@bytecode`

```edge
@bytecode(T -> U) -> Bytes;
```

Takes an arbitrary function and returns its compiled bytecode as a `Bytes`
value. `Bytes` is an opaque compiler-internal type representing a sequence of
raw bytes; it is not a user-definable Edge type.
