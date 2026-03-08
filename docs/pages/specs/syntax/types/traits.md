---
title: Trait constraints
---

# Trait constraints

Traits are interface-like declarations that constrain generic types to
implement specific methods or contain specific properties.

## Declaration

```text
<trait_declaration> ::=
    ["pub"] "trait" <identifier> [<type_parameters>] [":" <identifier> ("+" <identifier>)*] "{"
    (
        | <type_declaration> ";"
        | <type_assignment> ";"
        | <constant_declaration> ";"
        | <constant_assignment> ";"
        | <function_declaration> ";"
        | <function_assignment>
    )*
    "}" ;
```

Dependencies:

* `<identifier>`
* `<type_parameters>`
* `<type_declaration>`
* `<type_assignment>`
* `<constant_declaration>`
* `<constant_assignment>`
* `<function_declaration>`
* `<function_assignment>`

The `<trait_declaration>` maps to `TraitDecl` in the AST. It contains a name,
optional type parameters, optional supertraits, and a body of trait items.
The `is_pub` flag tracks visibility.

Each body item maps to a `TraitItem` variant:

| Syntax | AST variant |
|---|---|
| `type Name;` | `TraitItem::TypeDecl` |
| `type Name = T;` | `TraitItem::TypeAssign` |
| `const NAME: T;` | `TraitItem::ConstDecl` |
| `const NAME: T = expr;` | `TraitItem::ConstAssign` |
| `fn name(...) -> T;` | `TraitItem::FnDecl` |
| `fn name(...) -> T { ... }` | `TraitItem::FnAssign` |

## Supertrait constraints

```text
<supertrait_constraints> ::= ":" <identifier> ("+" <identifier>)* ;
```

Dependencies:

* `<identifier>`

Supertraits are separated by `+`, indicating that all listed traits must be
implemented. This matches the `+` separator used for type parameter bounds
(e.g. `fn f<T: Bar + Baz>()`).

Supertraits are stored as `supertraits: Vec<Ident>` in `TraitDecl`.

## Semantics

Traits can be defined with associated types, constants, and functions. The
trait declaration allows optional assignment for each item as a default.
Declarations without a default assignment must be provided in the
implementation. Default assignments can be overridden in trait
implementations.

Types can depend on trait constraints, and traits can also depend on other
traits (supertraits). Supertraits assert that types implementing a given
trait also implement all of its parent traits.

:::warning
Trait-solving semantics are still being drafted. The compiler does not yet
validate trait implementations or enforce supertrait constraints.
:::
