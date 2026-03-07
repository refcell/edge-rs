# Type Assignment

## Signature

```
<type_signature> ::=
    | <array_signature>
    | <struct_signature>
    | <tuple_signature>
    | <union_signature>
    | <function_signature>
    | <ident>
    | (<ident> [<type_parameters>]) ;
```

Dependencies:

* `<array_signature>`
* `<struct_signature>`
* `<tuple_signature>`
* `<union_signature>`
* `<function_signature>`
* `<ident>`
* `<type_parameters>`

Type assignments assign identifiers to type signatures. It may have a struct, tuple, union,
or function signature as well as an identifier followed by optional type parameters.

## Declaration

```
<type_declaration> ::= ["pub"] "type" <ident> [<type_parameters>] 
```

Dependencies:

* `<ident>`
* `<type_parameters>`

The `<type_declaration>` is prefixed with "type" and contains an identifier with optional type parameters.

## Assignment

```
<type_assignment> ::= <type_declaration> "=" <type_signature> ;
```

The `<type_assignment>` is a type declaration followed by a type signature separated by an assignment operator.

## Semantics

Type assignment entails creating an identifier associated with a certain data structure or existing type.
If the assignment is to an existing data type, it contains the same fields or members, if any, and exposes the
same associated items, if any.

```
type MyCustomType = packed (u8, u8, u8);
type MyCustomAlias = MyCustomType;

fn increment(rgb: MyCustomType) -> MyCustomType {
    return (rgb.0 + 1, rgb.1 + 1, rgb.2 + 1);
}

increment(MyCustomType(1, 2, 3));
increment(MyCustomAlias(1, 2, 3));
```

A way to create a wrapper around an existing type without exposing the existing type's external interface,
the type may be wrapped in parenthesis, creating a "tuple" of one element, which comes without overhead.

```
type MyCustomType = packed (u8, u8, u8);
type MyNewCustomType = (MyCustomType);
```
