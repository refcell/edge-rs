---
title: Array Types
---

# Array Types

The array type is a list of elements of a single type.

## Signature

```text
<array_signature> ::= ["packed"] "[" <type_signature> ";" <expr> "]" ;
```

Dependencies:

* `<type_signature>`
* `<expr>`

The `<array_signature>` consists of an optional "packed" keyword prefix
to a type signature and expression separated by a colon, delimited by brackets.

## Instantiation

```text
<array_instantiation> ::= [<data_location>] "[" <expr> ("," <expr>)* [","] "]" ;
```

Dependencies:

* `<data_location>`
* `<expr>`

The `<array_instantiation>` is an optional data location annotation followed
by a comma separated list of expressions delimited by brackets.

## Element Access

```text
<array_element_access> ::= <ident> "[" <expr> [":" <expr>] "]" ;
```

Dependencies:

* `<ident>`
* `<expr>`

The `<array_element_access>` is the array's identifier followed a
bracket-delimited expression and optionally a second expression, colon
separated.

## Examples

```edge
type TwoElementIntegerArray = [u8; 2];
type TwoElementPackedIntegerArray = packed [u8; 2];

const arr: TwoElementIntegerArray = [1, 2];

const elem: u8 = arr[0];
```

## Semantics

### Instantiation

Instantiation of a fixed-length array stores one element per 32 byte word in
either data location. The only difference between data locations in terms of
instantiation behavior is if all elements of the array are populated with
constant values and the array belongs in memory, a performance optimization
may include code-copying an instance of the constant array from the bytecode
into memory.

### Access

Array element access depends on whether the second expression is included.
If a single expression is inside the access brackets, the single element is
returned from the array. If a second expression follows the first with a colon
in between, a pointer of the same data location is returned. The type of the
new array pointer is the same type but the size is now the size of the second
expression's value minus the first expression's value. If the index values are
known at compile time and are greater than or equal to the array's length, a
compiler error is thrown, else a bounds check against the array's length is
added into the runtime bytecode.
