---
title: Array types
---

# Array types

The array type is a fixed-length list of elements of a single type.

## Signature

```text
<array_signature> ::= ["packed"] "[" <type_signature> ";" <expression> "]" ;
```

Dependencies:

* `<type_signature>`
* `<expression>`

The `<array_signature>` consists of an optional `packed` keyword, a type
signature and a size expression separated by a semicolon, delimited by
brackets. It maps to `TypeSig::Array` or `TypeSig::PackedArray` depending
on the `packed` prefix.

:::warning
Packed array IR lowering is not yet implemented. The `packed` keyword is
accepted by the parser but currently has no effect on code generation.
:::

## Instantiation

```text
<array_instantiation> ::= [<data_location>] "[" [<expression> ("," <expression>)* [","]] "]" ;
```

Dependencies:

* `<data_location>`
* `<expression>`

The `<array_instantiation>` is an optional data location annotation followed
by a comma-separated list of expressions delimited by brackets. It produces
`Expr::ArrayInstantiation(location, elements, span)`.

## Element access

```text
<array_element_access> ::= <expression> "[" <expression> [":" <expression>] "]" ;
```

Dependencies:

* `<expression>`

Array element access is a postfix operation on any expression. A single index
returns one element. When a second expression follows separated by `:`, a
slice is returned. Both forms produce `Expr::ArrayIndex(expr, index, end, span)`
where `end` is `Some(...)` for slices.

## Examples

```edge
type TwoElementIntegerArray = [u8; 2];
type TwoElementPackedIntegerArray = packed [u8; 2];

const arr: TwoElementIntegerArray = [1, 2];

const elem: u8 = arr[0];
```

## Semantics

### Instantiation

Instantiation of a fixed-length array stores one element per 32-byte word in
either data location.

### Access

Array element access depends on whether the second expression is included.
A single expression returns that element. With a colon-separated second
expression, a pointer of the same data location is returned. The resulting
array type has the same element type but a size equal to `end - start`.

:::warning
Bounds checking is not yet implemented. Out-of-bounds array accesses are
currently undefined behavior at runtime.
:::
