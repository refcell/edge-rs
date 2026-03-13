# Built In

Built-in functionality refers to functionality that is only available
during the compiler runtime and not the EVM runtime that is otherwise
inaccessible through the language's syntax.

Macros contain their own syntax and semantics, however, comptime
functionality and built-in assistants cover most of the use cases for
macros without leaving the language's native syntax.

## Runtime Builtins

### `@size_of`

```
@size_of::<T>() -> u256
```

Returns the size in bytes of type `T`. For primitive types this is the
ABI-encoded word size (32 bytes for `u256`, `address`, etc.).

### `@alloc`

```
@alloc(size_bytes: u256) -> u256
```

Allocates `size_bytes` of dynamic memory at runtime and returns a pointer
to the start of the region. Uses MSIZE-based pointer arithmetic to ensure
the returned region does not overlap with any other allocation.

`@alloc` is the foundation for dynamically-sized data structures like
`Vec<T>`. It is used in conjunction with the `&dm` data location annotation
(see [Data Locations](syntax/locations.md)).

## EVM Environment Builtins

These builtins expose EVM execution environment values directly. Each
returns a `u256`.

| builtin              | EVM opcode       | description                                |
|----------------------|------------------|--------------------------------------------|
| `@caller()`          | `CALLER`         | address of the direct caller               |
| `@callvalue()`       | `CALLVALUE`      | wei sent with the call                     |
| `@calldatasize()`    | `CALLDATASIZE`   | size of calldata in bytes                  |
| `@origin()`          | `ORIGIN`         | transaction origin address                 |
| `@gasprice()`        | `GASPRICE`       | gas price of the transaction               |
| `@coinbase()`        | `COINBASE`       | block coinbase address                     |
| `@timestamp()`       | `TIMESTAMP`      | block timestamp                            |
| `@number()`          | `NUMBER`         | block number                               |
| `@gaslimit()`        | `GASLIMIT`       | block gas limit                            |
| `@chainid()`         | `CHAINID`        | chain ID                                   |
| `@selfbalance()`     | `SELFBALANCE`    | balance of the current contract            |
| `@basefee()`         | `BASEFEE`        | block base fee                             |
| `@gas()`             | `GAS`            | remaining gas                              |
| `@address()`         | `ADDRESS`        | address of the current contract            |
| `@codesize()`        | `CODESIZE`       | size of the contract's code                |
| `@returndatasize()`  | `RETURNDATASIZE` | size of the return data from the last call |

## Types

### PrimitiveType

```
type PrimitiveType;
```

### StructType

```
type StructType;
```

### EnumType

```
type EnumType;
```

### UnionType

```
type UnionType;
```

### FunctionType

```
type FunctionType;
```

### TypeInfo

```
type TypeInfo =
    | Primitive(PrimitiveType)
    | Struct(StructType)
    | Enum(EnumType)
    | Union(UnionType)
    | Function(FunctionType);
```

### HardFork

```
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

## Comptime Builtins (Future Work)

The following builtins are planned but not yet implemented:

```
@typeInfo
@typeInfo(typeSignature) -> TypeInfo;
```

The typeInfo function takes a single `<type_signature>` as an argument and returns
a union of types, TypeInfo.

```
@bitsize
@bitsize(typeSignature) -> u256;
```

The bitsize function takes a single `<type_signature>` as an argument and returns
an integer indicating the bitsize of the underlying type.

```
@fields
@fields(structType) -> [T, N];
```

The fields function takes a single StructType as an argument and returns an array
of type signatures of length N where N is the number of fields in the struct.

```
@compilerError
@compilerError(errorMessage);
```

The compilerError function takes a single string as an argument and throws an
error at compile time with the provided message.

```
@hardFork
@hardFork() -> HardFork;
```

The hardFork function returns an enumeration of the built in HardFork type.
This is derived from the compiler configuration.

```
@bytecode
@bytecode(T -> U) -> Bytes;
```

The bytecode function takes an arbitrary function and returns its bytecode
in Bytes.
