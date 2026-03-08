---
title: Product types
---

# Product types

The product type is a compound type composed of zero or more internal types.

## Signature

```text
<struct_field_signature> ::= <identifier> ":" <type_signature> ;

<struct_signature> ::=
    ["packed"] "{"
        [<struct_field_signature> ("," <struct_field_signature>)* [","]]
    "}" ;

<tuple_signature> ::= ["packed"] "(" <type_signature> ("," <type_signature>)* [","] ")" ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`

The `<struct_signature>` maps to `TypeSig::Struct` or `TypeSig::PackedStruct`.
Each field produces a `StructField { name, ty }` in the AST.

The `<tuple_signature>` maps to `TypeSig::Tuple` or `TypeSig::PackedTuple`.

## Instantiation

```text
<struct_field_instantiation> ::= <identifier> ":" <expression> ;

<struct_instantiation> ::=
    [<data_location>] <identifier> "{"
        [<struct_field_instantiation> ("," <struct_field_instantiation>)* [","]]
    "}" ;

<tuple_instantiation> ::= [<data_location>] "(" [<expression> ("," <expression>)* [","]] ")" ;
```

Dependencies:

* `<identifier>`
* `<expression>`
* `<data_location>`

The `<struct_instantiation>` produces `Expr::StructInstantiation(location, name, fields, span)`.
The parser distinguishes struct instantiation from a code block by lookahead:
if the opening `{` is followed by `<identifier> ":"`, it's a struct.

The `<tuple_instantiation>` produces `Expr::TupleInstantiation(location, elements, span)`.
A single expression in parentheses without a trailing comma is parsed as
`Expr::Paren` (grouping), not a tuple.

## Field access

```text
<struct_field_access> ::= <expression> "." <identifier> ;
<tuple_field_access> ::= <expression> "." <dec_char> ;
```

Dependencies:

* `<expression>`
* `<identifier>`

The `<struct_field_access>` produces `Expr::FieldAccess(expr, field, span)`.
The `<tuple_field_access>` produces `Expr::TupleFieldAccess(expr, index, span)`.
Both are postfix operations on any expression, not just identifiers.

## Examples

```edge
type PrimitiveStruct = {
    a: u8,
    b: u8,
    c: u8,
};

const primitiveStruct: PrimitiveStruct = PrimitiveStruct { a: 1, b: 2, c: 3 };

const a = primitiveStruct.a;

type PackedTuple = packed (u8, u8, u8);

const packedTuple: PackedTuple = (1, 2, 3);

const one = packedTuple.0;
```

## Semantics

The struct field signature maps a type identifier to a type signature. The
field may be accessed by the struct's identifier and field identifier separated
by a dot.

Prefixing the signature with the `packed` keyword will pack the fields by their
bit size, otherwise each field is padded to its own 256-bit word.

:::warning
Packed tuple IR lowering is not yet implemented. The `packed` keyword on tuples
is accepted by the parser but currently has no effect on code generation.
:::

```edge
type Rgb = packed { r: u8, g: u8, b: u8 };

let rgb = Rgb { r: 1, g: 2, b: 3 };
// rgb = 0x010203
```

:::warning
Stack-allocated struct optimization (for single-word structs) is not yet
implemented. All struct instantiations currently allocate memory regardless
of size, and no compiler error is generated for missing data location
annotations.
:::

Memory instantiation consists of allocating new memory, optionally
bitpacking fields, storing the struct in memory, and leaving the pointer
to it on the stack.

```edge
type MemoryRgb = { r: u8, g: u8, b: u8 };

let memoryRgb = MemoryRgb{ r: 1, g: 2, b: 3 };
// ptr = ..
// mstore(ptr, 1)
// mstore(add(32, ptr), 2)
// mstore(add(64, ptr), 3)
```

Persistent and transient storage structs must be instantiated at the file
level. If anything except zero values are assigned, storage writes will be
injected into the initcode to be run on deployment.

```edge
type Storage = {
    a: u8,
    b: u8,
    c: packed {
        a: u8,
        b: u8
    }
}

const storage = @default<Storage>();

fn main() {
    storage.a = 1;      // sstore(0, 1)
    storage.b = 2;      // sstore(1, 2)
    storage.c.a = 3;    // ca = shl(8, 3)
    storage.c.b = 4;    // sstore(2, or(ca, 4))
}
```

Packing rules for buffer locations pack everything exactly by its bit length.
Packing rules for map locations right-align the last field; for each preceding
field, left-shift by the combined bit size of all fields to its right. If a
field's bit size would overflow the current word, it begins a new word.

:::warning
Packed struct layout currently supports single-word packing only. Fields whose
combined bit size exceeds 256 bits will not be correctly packed across multiple
words. Multi-word packed structs are a planned feature.
:::

```edge
type Storage = {
    a: u128,
    b: u8,
    c: addr,
    d: u256
}

const storage = Storage {
    a: 1,
    b: 2,
    c: 0x3,
    d: 4,
};
```

| Slot | Value                                                              |
| -----|--------------------------------------------------------------------|
| 0x00 | 0x0000000000000000000000000000000200000000000000000000000000000001 |
| 0x01 | 0x0000000000000000000000000000000000000000000000000000000000000003 |
| 0x02 | 0x0000000000000000000000000000000000000000000000000000000000000004 |
