---
title: Type assignment
---

# Type assignment

## Signature

```text
<type_signature> ::=
    | <primitive_data_type>
    | <array_signature>
    | <struct_signature>
    | <tuple_signature>
    | <union_signature>
    | <function_signature>
    | <event_signature>
    | <pointer_signature>
    | <identifier>
    | <identifier> "<" <type_signature> ("," <type_signature>)* ">" ;

<pointer_signature> ::= <data_location> <type_signature> ;
```

Dependencies:

* `<primitive_data_type>`
* `<array_signature>`
* `<struct_signature>`
* `<tuple_signature>`
* `<union_signature>`
* `<function_signature>`
* `<event_signature>`
* `<data_location>`
* `<identifier>`

The `<type_signature>` enumerates every form a type can take. It maps
directly to the `TypeSig` enum in the AST. A bare `<identifier>` produces
`TypeSig::Named(ident, [])`, while an identifier with angle-bracketed type
arguments produces `TypeSig::Named(ident, args)`.

The `<pointer_signature>` wraps any type with a data location annotation,
producing `TypeSig::Pointer(location, inner)`.

## Declaration

```text
<type_declaration> ::= ["pub"] "type" <identifier> [<type_parameters>] ;
```

Dependencies:

* `<identifier>`
* `<type_parameters>`

The `<type_declaration>` maps to `TypeDecl` in the AST. It contains a name,
optional type parameters, and a `is_pub` flag.

## Assignment

```text
<type_assignment> ::= <type_declaration> "=" (<type_signature> | <union_signature>) ";" ;
```

The `<type_assignment>` binds a type declaration to a type signature.
When the right-hand side is a `<union_signature>`, the parser uses
`parse_type_sig_or_union()` to accept pipe-separated union variants.

## Semantics

Type assignment creates an identifier associated with a data structure or
existing type. If the assignment targets an existing type, the alias shares
the same fields, members, and associated items.

```edge
type MyCustomType = packed (u8, u8, u8);
type MyCustomAlias = MyCustomType;

fn increment(rgb: MyCustomType) -> MyCustomType {
    return (rgb.0 + 1, rgb.1 + 1, rgb.2 + 1);
}

increment(MyCustomType(1, 2, 3));
increment(MyCustomAlias(1, 2, 3));
```

To create a wrapper around an existing type without exposing its external
interface, the type may be wrapped in parentheses, creating a single-element
tuple with no overhead:

```edge
type MyCustomType = packed (u8, u8, u8);
type MyNewCustomType = (MyCustomType);
```
