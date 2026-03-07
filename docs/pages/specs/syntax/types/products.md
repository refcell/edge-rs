---
title: Product Types
---

# Product Types

The product type is a compound type composed of none or more internal types.

## Signature

```text
<struct_field_signature> ::= <ident> ":" <type_signature> ;
<struct_signature> ::=
    ["packed"] "{"
        [<struct_field_signature> ("," <struct_field_signature>)* [","]]
    "}" ;
<tuple_signature> ::= ["packed"] "(" <type_signature> ("," <type_signature>)* [","] ")" ;
```

Dependencies:

* `<ident>`
* `<type_parameters>`
* `<type_signature>`

## Instantiation

```text
<struct_field_instantiation> ::= <ident> ":" <expr> ;

<struct_instantiation> ::=
    [<data_location>] <struct_signature> "{"
        [<struct_field_instantiation> ("," <struct_field_instantiation>)* [","]]
    "}" ;

<tuple_instantiation> ::= [<data_location>] <ident> "(" [<expr> ("," <expr>)* [","]] ")" ;
```

Dependencies:

* `<ident>`
* `<expr>`
* `<data_location>`

The `<struct_instantiation>` is an instantiation, or creation, of a struct. It may
optionally include a data location annotation, however the semantic rules for this
are in the data location semantic rules. It is instantiated by the struct identifier
followed by a comma separated list of field name and value pairs delimited by curly braces.

The `<tuple_instantiation>` is an instantiation, or creation, of a tuple. It may
optionally include a data location annotation, however the semantic rules for this
are in the data location semantic rules. It is instantiated by a comma separated list
of expressions delimited by parenthesis.

## Field Access

```text
<struct_field_access> ::= <ident> "." <ident> ;
<tuple_field_access> ::= <ident> "." <dec_char> ;
```

Dependencies:

* `<ident>`
* `<dec_char>`

The `<struct_field_access>` is written as the struct's identifier followed by the
field's identifier separated by a period.

The `<tuple_field_access>` is written as the tuple's identifier followed by the
field's index separated by a period.

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

Prefixing the signature with the "packed" keyword will pack the fields by their
bitsize, otherwise each field is padded to its own 256 bit word.

```edge
type Rgb = packed { r: u8, g: u8, b: u8 };

let rgb = Rgb { r: 1, g: 2, b: 3 };
// rgb = 0x010203
```

Instantiation depends on the data location. Structs that can fit into a single word,
either a single field struct or a packed struct with a bitsize sum less than or equal
to 256, sit on the stack by default. Instantiating a struct in memory requires the
memory data location annotation. If a struct that does not fit into a single word
does not have a data location annotation, a compiler error is thrown.

Stack struct instantiation consists of optionally bitpacking fields and leaving the
struct on the stack. Memory instantiation consists of allocating new memory, optionally
bitpacking fields, storing the struct in memory, and leaving the pointer to it on the
stack.

```edge
type MemoryRgb = { r: u8, g: u8, b: u8 };

let memoryRgb = MemoryRgb{ r: 1, g: 2, b: 3 };
// ptr = ..
// mstore(ptr, 1)
// mstore(add(32, ptr), 2)
// mstore(add(64, ptr), 3)
```

Persistent and transient storage structs must be instantiated at the file level. If
anything except zero values are assigned, storage writes will be injected into the initcode
to be run on deployment. A reasonable convention for creating a storage layout without
the contract object abstraction would be to create a Storage type which is a struct,
mapping identifiers to storage slots. Nested structs will also allow granular control
over which variables get packed.

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

Packing rules for buffer locations is to pack everything exactly by its bit length.
Packing rules for map locations is to right-align the first field, for each subsequent
field, if its bitsize fits into the same word as the previous, it is left-shifted to
the first available bits, otherwise, if the bitsize would overflow, it becomes a new
word.

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
