---
title: Generics
---

# Generics

Generics are polymorphic types enabling function and type reuse across different types.

## Type parameters

```text
<type_param> ::= <identifier> [":" <identifier> ("+" <identifier>)*] ;

<type_parameters> ::= "<" <type_param> ("," <type_param>)* [","] ">" ;
```

Dependencies:

* `<identifier>`

Each `<type_param>` maps to a `TypeParam { name, constraints }` in
the AST. Trait bound constraints are separated by `+` and stored as
`constraints: Vec<Ident>`.

The `<type_parameters>` is a comma-separated list of individual type
parameters delimited by angle brackets.

## Nested generics

The parser handles `>>` in nested generics (e.g. `map<K, map<K, V>>`) by
splitting the `>>` token into two `>` tokens when closing generic parameter
lists.

## Semantics

Generics are resolved at compile time through monomorphization. Generic
functions and data types are monomorphized into distinct unique functions
and data types. Function duplication can become problematic due to the EVM
bytecode size limit, so a series of steps will be taken to allow for
granular control over bytecode size. Those semantics are defined in the
codesize document.
