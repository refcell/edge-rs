---
title: Constants
---

# Constants

## Declaration

```text
<constant_declaration> ::= "const" <identifier> [":" <type_signature>] ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`

The `<constant_declaration>` maps to `ConstDecl { name, ty: Option<TypeSig>, span }`.

## Assignment

```text
<constant_assignment> ::= <constant_declaration> "=" <expression> ";" ;
```

Dependencies:

* `<expression>`

The `<constant_assignment>` produces `Stmt::ConstAssign(ConstDecl, Expr, Span)`.

:::note
The expression must be resolvable at compile time, but this constraint is
enforced semantically, not by the grammar.
:::

## Semantics

Constants must be resolvable at compile time by assigning a literal, another
constant, or an expression that can be evaluated at compile time.

The type of a constant is inferred from its assignment when no explicit type
annotation is provided.

```edge
const A: u8 = 1;
const B = 1;
const C = B;
const D = a();

comptime fn a() -> u8 {
    1
}
```
