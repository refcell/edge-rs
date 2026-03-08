---
title: Loops
---

# Loops

Loops are blocks of code that may be executed repeatedly based on conditions.

## Loop control

```text
<loop_break> ::= "break" ";" ;
<loop_continue> ::= "continue" ";" ;
```

The `break` keyword exits the loop immediately. The `continue` keyword
skips to the next iteration.

:::warning
`break` and `continue` are parsed into the AST but **silently dropped
during IR lowering**. Using them in a loop will compile as if the statement
were absent. This is a known limitation.
:::

## Loop block

```text
<loop_block> ::= "{" ((<expression> | <stmt> | <loop_break> | <loop_continue>) ";")* "}" ;
```

Dependencies:

* `<expression>`
* `<stmt>`

The `<loop_block>` maps to `LoopBlock { items: Vec<LoopItem>, span }`.
Each item is a `LoopItem` variant:

| Variant | Description |
|---|---|
| `LoopItem::Expr(Expr)` | Expression |
| `LoopItem::Stmt(Box<Stmt>)` | Statement |
| `LoopItem::Break(Span)` | `break` |
| `LoopItem::Continue(Span)` | `continue` |

:::note
Loop blocks have their own `LoopItem` enum with dedicated `Break`/`Continue`
variants, separate from `Stmt::Break`/`Stmt::Continue`. The top-level
statement variants exist for `break`/`continue` outside loops (which would
be a semantic error), while `LoopItem` variants are used inside loop bodies.
:::

## Core loop

```text
<core_loop> ::= "loop" <loop_block> ;
```

The simplest loop form. Produces `Stmt::Loop(LoopBlock)`. At the IR level,
all loop forms are lowered to a `DoWhile` representation.

## For loop

```text
<for_loop> ::= "for" "(" [<stmt> | <expression>] ";" [<expression>] ";" [<stmt> | <expression>] ")" <loop_block> ;
```

Dependencies:

* `<expression>`
* `<stmt>`

Produces `Stmt::ForLoop(init, condition, update, body)` where each of
`init`, `condition`, and `update` is individually optional.

## While loop

```text
<while_loop> ::= "while" "(" <expression> ")" <loop_block> ;
```

Dependencies:

* `<expression>`

Produces `Stmt::WhileLoop(condition, body)`.

## Do-while loop

```text
<do_while_loop> ::= "do" <loop_block> "while" "(" <expression> ")" ";" ;
```

Dependencies:

* `<expression>`

Produces `Stmt::DoWhile(body, condition)`. The body executes at least once
before the condition is checked.

:::note
The `do-while` loop requires a trailing semicolon after the closing
parenthesis. This distinguishes it from other loop forms and matches the
parser's expectation.
:::

## Examples

```edge
fn example() {
    // core loop
    let mut i = 0;
    loop {
        if (i >= 10) { return; }
        i = i + 1;
    }

    // for loop
    for (let mut j = 0; j < 10; j = j + 1) {
        // ...
    }

    // while loop
    let mut k = 0;
    while (k < 10) {
        k = k + 1;
    }

    // do-while loop
    let mut m = 0;
    do {
        m = m + 1;
    } while (m < 10);
}
```

## Semantics

:::warning
Loop semantics (desugaring rules, IR lowering details) are still under
construction.
:::
