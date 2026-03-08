---
title: Primitive types
---

# Primitive types

```text
<integer_size> ::= "8" | "16" | "24" | "32" | "40" | "48" | "56" | "64" | "72" | "80" | "88" | "96"
    | "104" | "112" | "120" | "128" | "136" | "144" | "152" | "160" | "168" | "176" | "184" | "192"
    | "200" | "208" | "216" | "224" | "232" | "240" | "248" | "256" ;

<fixed_bytes_size> ::= "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "10" | "11" | "12"
    | "13" | "14" | "15" | "16" | "17" | "18" | "19" | "20" | "21" | "22" | "23" | "24" | "25"
    | "26" | "27" | "28" | "29" | "30" | "31" | "32" ;

<signed_integer> ::= {"i" <integer_size>} ;
<unsigned_integer> ::= {"u" <integer_size>} ;
<fixed_bytes> ::= {"b" <fixed_bytes_size>} ;
<address> ::= "addr" ;
<boolean> ::= "bool" ;
<bit> ::= "bit" ;

<numeric_type> ::= <signed_integer> | <unsigned_integer> | <fixed_bytes> | <address> ;

<primitive_data_type> ::=
    | <numeric_type>
    | <boolean>
    | <bit> ;
```

The `<primitive_data_type>` covers signed and unsigned integers, booleans,
address, fixed bytes, and the single-bit type. Each maps directly to
`TypeSig::Primitive(PrimitiveType)` in the AST.

:::note
Pointer types (`<data_location> <type_signature>`) are not primitives — they
are a separate `TypeSig::Pointer` variant that wraps any type with a storage
location. See [type assignment](/specs/syntax/types/assignment) for the
`<pointer_signature>` production.
:::

## Examples

```edge
u8
u256
i8
i256
b4
b32
addr
bool
bit
```

## Semantics

Integers occupy the number of bits indicated by their size.
Fixed bytes types occupy the number of bytes indicated by their size,
or `size × 8` bits. Address occupies 160 bits. Booleans occupy eight bits.
Bit occupies a single bit.
