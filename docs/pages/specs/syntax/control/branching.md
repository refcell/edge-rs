---
title: Branching
---

# Branching

Branching refers to blocks of code that may be executed based on a condition.

## If / else if / else

```text
<if_else_branch> ::= "if" "(" <expression> ")" <code_block>
    ("else" "if" "(" <expression> ")" <code_block>)*
    ["else" <code_block>] ;
```

Dependencies:

* `<expression>`
* `<code_block>`

Produces `Stmt::IfElse(Vec<(Expr, CodeBlock)>, Option<CodeBlock>)`. Each
condition-block pair is an element in the vector; the optional else block
is the second field.

## If match

```text
<if_match_branch> ::= "if" <expression> "matches" <union_pattern> <code_block> ;
```

Dependencies:

* `<expression>`
* `<union_pattern>`
* `<code_block>`

:::note[Implementation detail]
`Stmt::IfMatch` exists as a variant in the AST but is **dead code**. The
parser never produces it directly — instead, it always emits `Stmt::IfElse`
with an `Expr::PatternMatch` as the condition. Contributors should be aware
that any logic gated on `Stmt::IfMatch` will not be reached under normal
compilation.
:::

## Pattern match expression

```text
<pattern_match_expr> ::= <expression> "matches" <union_pattern> ;
```

The `matches` keyword produces `Expr::PatternMatch(expr, pattern, span)` and
works as a boolean expression usable anywhere — including as an `if`
condition, ternary operand, or `let` binding value:

```edge
let is_some = value matches Option::Some(x);
```

## Match

```text
<match_pattern> ::= <union_pattern> | <identifier> | "_" ;

<match_arm> ::= <match_pattern> "=>"
    (<code_block> | <expression> | "return" [<expression>]) ;

<match> ::=
    "match" <expression> "{"
        [<match_arm> ("," <match_arm>)* [","]]
    "}" ;
```

Dependencies:

* `<expression>`
* `<union_pattern>`
* `<code_block>`

Each `<match_pattern>` maps to a `MatchPattern` variant:

| Pattern | AST variant |
|---|---|
| `Type::Variant(...)` | `MatchPattern::Union(UnionPattern)` |
| `name` | `MatchPattern::Ident(Ident)` |
| `_` | `MatchPattern::Wildcard` |

Each `<match_arm>` maps to `MatchArm { pattern, body: CodeBlock }`.

:::note
At the AST level, all arm bodies are normalized to `CodeBlock`. Bare
expressions and `return` statements are wrapped in synthetic code blocks
by the parser.
:::

:::warning
Compile-time exhaustiveness checking is not yet implemented. Non-exhaustive
match blocks do not produce a compiler error. If no arm matches at runtime
and no default arm is present, the program reverts.
:::

## Ternary

```text
<ternary> ::= <expression> "?" <expression> ":" <expression> ;
```

Dependencies:

* `<expression>`

Produces `Expr::Ternary(condition, then_expr, else_expr, span)`. The ternary
is right-associative — `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.

## Semantics

### If / else if

The condition expression is evaluated. If it is true, the subsequent block
executes. Otherwise the next `else if` condition is checked. If no condition
is true and an `else` block is present, it executes.

```edge
fn main() {
    let n = 3;

    if (n == 1) {
        // ..
    } else if (n == 2) {
        // ..
    } else {
        // ..
    }
}
```

### If match

The `if match` branch brings into scope any identifiers bound by the
pattern's payload bindings:

```edge
type Union = A(u8) | B;

fn main() {
    let u = Union::A(1);

    if u matches Union::A(n) {
        assert(n == 1);
    }
}
```

### Match

The `match` statement evaluates the target expression and compares it against
each arm's pattern in order. The first matching arm's body executes.

An identifier pattern (`name`) binds the matched value irrefutably.
A wildcard pattern (`_`) discards the value. Both serve as catch-all arms.

```edge
type Ua = A | B;

fn main() {
    let u = Ua::B;

    match u {
        Ua::A => {},
        Ua::B => {},
    }
}
```

:::warning
Type narrowing of wildcard/identifier bindings is not yet implemented.
:::

### Ternary

The condition must evaluate to a boolean. If true, the second expression is
evaluated; otherwise the third.

```edge
fn main() {
    let condition = true;
    let b = condition ? 1 : 2;
}
```

### Short circuiting

For boolean expressions composed of logical operators:

- `expr0 && expr1` — if `expr0` is `false`, short-circuit to `false`
- `expr0 || expr1` — if `expr0` is `true`, short-circuit to `true`

For `if / else if` chains, if an earlier branch is taken, subsequent
conditions are not evaluated.
