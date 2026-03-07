# Built In

Built-in functionality refers to functionality that is only available
during the compiler runtime and not the EVM runtime that is otherwise
inaccessible through the language's syntax.

Macros contain their own syntax and semantics, however, comptime
functionality and built-in assistants cover most of the use cases for
macros without leaving the language's native syntax.

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

### Functions

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
