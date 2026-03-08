---
title: Variables
---

# Variables

## Declaration

```text
<variable_declaration> ::= "let" ["mut"] <identifier> [":" <type_signature>] ["=" <expression>] ";" ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`
* `<expression>`

The `<variable_declaration>` marks the declaration of a variable. The optional
`mut` keyword marks the variable as mutable. The variable may optionally be
given a type annotation and/or be assigned at the point of declaration.

:::warning
The `mut` keyword is parsed but not yet tracked in the AST or enforced by the
compiler. All variables are currently mutable regardless of the `mut` annotation.
:::

## Constants

```text
<constant_assignment> ::= "const" <identifier> [":" <type_signature>] "=" <expression> ";" ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`
* `<expression>`

The `<constant_assignment>` declares a compile-time constant. Unlike `let`,
constants require an initializer and are immutable. By convention, constant
names are written in `UPPER_SNAKE_CASE`.

## Assignment

```text
<variable_assignment> ::= <expression> "=" <expression> ";" ;
```

Dependencies:

* `<expression>`

The `<variable_assignment>` assigns a value to a target expression. The
left-hand side is a full expression, supporting simple identifiers as well
as field access (`a.b = x`), array indexing (`arr[i] = x`), and other
assignable forms.

In addition to simple assignment, Edge supports the following compound
assignment operators that combine an arithmetic or bitwise operation with
assignment:

| Operator | Meaning |
|----------|---------|
| `+=`  | add and assign |
| `-=`  | subtract and assign |
| `*=`  | multiply and assign |
| `/=`  | divide and assign |
| `%=`  | modulo and assign |
| `**=` | exponentiate and assign |
| `&=`  | bitwise AND and assign |
| `\|=`  | bitwise OR and assign |
| `^=`  | bitwise XOR and assign |
| `>>=` | right-shift and assign |
| `<<=` | left-shift and assign |

:::note
Assignment also exists as an expression (`Expr::Assign`) at precedence level 0
in the Pratt parser. The statement form `Stmt::VarAssign` and the expression
form `Expr::Assign` both accept a full expression on the left-hand side.
:::
