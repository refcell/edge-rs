---
title: Implementation
---

# Implementation

Implementation blocks enable method-call syntax and trait satisfaction.

## Implementation block

```text
<impl_block> ::=
    "impl" <identifier> [<type_parameters>] [":" <identifier> [<type_parameters>]] "{"
        (
            | <function_assignment>
            | <constant_assignment>
            | <type_assignment>
        )*
    "}" ;
```

Dependencies:

* `<identifier>`
* `<type_parameters>`
* `<function_assignment>`
* `<constant_assignment>`
* `<type_assignment>`

The `<impl_block>` maps to `ImplBlock` in the AST:

- `ty_name: Ident` — the type being implemented
- `type_params: Vec<TypeParam>` — type parameters brought into scope
- `trait_impl: Option<(Ident, Vec<TypeParam>)>` — optional trait being satisfied
- `items: Vec<ImplItem>` — function, constant, and type assignments

Each body item maps to an `ImplItem` variant:

| Syntax | AST variant |
|---|---|
| `fn name(...) { ... }` | `ImplItem::FnAssign` |
| `const NAME: T = expr;` | `ImplItem::ConstAssign` |
| `type Name = T;` | `ImplItem::TypeAssign` |

The trait clause uses `:` (not `for`):

```edge
impl MyType<T> : MyTrait<T> {
    fn method(self) -> T { ... }
}
```

## Semantics

Associated functions, constants, and types are defined for a given type.
If the type contains generics in any of its internal assignments, the
type parameters must be brought into scope by annotating them directly
following the type's identifier.

If the impl block satisfies a trait's interface, only functions, constants,
and types declared in the trait may be defined. All undefaulted trait
declarations must be assigned in the impl block.
