---
title: Built-In
---

# Built-In

Built-in functionality refers to functionality that is only available
during the compiler runtime and not the EVM runtime that is otherwise
inaccessible through the language's syntax.

Macros contain their own syntax and semantics, however, comptime
functionality and built-in assistants cover most of the use cases for
macros without leaving the language's native syntax.

## Types

### PrimitiveType

```edge
type PrimitiveType;
```

### StructType

```edge
type StructType;
```

### EnumType

```edge
type EnumType;
```

### UnionType

```edge
type UnionType;
```

### FunctionType

```edge
type FunctionType;
```

### TypeInfo

```edge
type TypeInfo =
    | Primitive(PrimitiveType)
    | Struct(StructType)
    | Enum(EnumType)
    | Union(UnionType)
    | Function(FunctionType);
```

### HardFork

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

```edge
@typeInfo
@typeInfo(typeSignature) -> TypeInfo;
```

The typeInfo function takes a single `<type_signature>` as an argument and returns
a union of types, TypeInfo.

```edge
@bitsize
@bitsize(typeSignature) -> u256;
```

The bitsize function takes a single `<type_signature>` as an argument and returns
an integer indicating the bitsize of the underlying type.

```edge
@fields
@fields(structType) -> [T, N];
```

The fields function takes a single StructType as an argument and returns an array
of type signatures of length N where N is the number of fields in the struct.

```edge
@compilerError
@compilerError(errorMessage);
```

The compilerError function takes a single string as an argument and throws an
error at compile time with the provided message.

```edge
@hardFork
@hardFork() -> HardFork;
```

The hardFork function returns an enumeration of the built in HardFork type.
This is derived from the compiler configuration.

```edge
@bytecode
@bytecode(T -> U) -> Bytes;
```

The bytecode function takes an arbitrary function and returns its bytecode
in Bytes.
