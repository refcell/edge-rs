# Implementation

Implementation blocks enable method-call syntax.

## Implementation Block

```
<impl_block> ::=
    "impl" <ident> [<type_parameters>] [":" <ident> [<type_parameters>]] "{"
        (
            | <function_assignment>
            | <constant_assignment>
            | <type_assignment>
        )*
    "}"
```

Dependencies:

* `<ident>`
* `<type_parameters>`
* `<function_assignment>`
* `<constant_assignment>`
* `<type_assignment>`

The `<impl_block>` is the implementation block for a give
type. The type identifier is optionally followed by type
parameters then optionally followed by a "for" clause.
The "for" clause contains trait identifiers and optional
type parameters for the traits. Followed by this is a list
of function, constant, and type assignments delimited by
curly braces.

## Semantics

Associated functions, constants, and types are defined for a
given type. If the type contains any generics in any of its
internal assignments, the type parameters must be brought
into scope by annotating them directly following the type's
identifier.

If the impl block is to satisfy a trait's interface, the
type's identifier and optional type parameters are followed
by the trait's identifier and optional type parameters. In
this case, only associated functions, constants, and types
that are declared in the trait's declaration may be defined
in the impl block. Additionally, all declarations in a
trait's declaration that are not assigned in the trait's
declaration must be assigned in the impl block for the
given data type.
