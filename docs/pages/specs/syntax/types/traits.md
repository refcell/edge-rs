---
title: Trait Constraints
---

# Trait Constraints

Traits are interface-like declarations that constrain generic types to
implement specific methods or contain specific properties.

## Declaration

```text
<trait_declaration> ::=
    ["pub"] "trait" <ident> [<type_parameters>] [<trait_constraints>] "{"
    (
        | <type_declaration>
        | <type_assignment>
        | <constant_declaration>
        | <constant_assignment>
        | <function_declaration>
        | <function_assignment>
    )*
    "}" ;
```

Dependencies:

* `<ident>`
* `<type_parameters>`
* `<type_declaration>`
* `<type_assignment>`
* `<constant_declaration>`
* `<constant_assignment>`
* `<function_declaration>`
* `<function_assignment>`

The `<trait_declaration>` is a declaration of a set of associated
types, constants, and functions that may itself take type parameters
and may be constrained to a super type. Semantics of the declaration
are listed under trait solving rules.

## Constraints

```text
<trait_constraints> ::= ":" <ident> ("&" <ident>)* ;
```

Dependencies:

* `<ident>`

The `<trait_constraints>` contains a colon followed by an
ampersand separated list of identifiers of implemented traits.
The ampersand is meant to indicate that all of the trait
identifiers are implemented for the type.

## Semantics

Traits can be defined with associated types, constants,
and functions. The trait declaration itself allows for
optional assignment for each item as a default. Any
declarations in the trait that are not assigned in the
trait declaration must be assigned in the implementation
of the trait for the data type. Additionally, any assignments
in the trait declaration can be overridden in the trait
implementation.

While types can depend on trait constraints, traits can
also depend on other trait constraints. These assert that
types that implement a given trait also implement its
"super traits".

## Solving

:::note
Todo: trait-solving semantics are still being drafted.
:::
