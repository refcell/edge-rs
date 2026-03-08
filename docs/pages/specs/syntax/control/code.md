---
title: Code blocks
---

# Code blocks

A code block is a sequence of items with its own scope. It may appear
standalone or as the body of a function, loop, or branch.

## Declaration

```text
<code_block> ::= "{" ((<statement> | <expression>) ";")* [<expression>] "}" ;
```

Dependencies:

* `<statement>`
* `<expression>`

The `<code_block>` maps to `CodeBlock { stmts: Vec<BlockItem>, span }`.
Each item is either `BlockItem::Stmt(Box<Stmt>)` or `BlockItem::Expr(Expr)`.

## Tail expressions

A code block's final item may omit its trailing semicolon to act as the
block's **return value** (Rust-style tail expression). When the last item
in a `<code_block>` is an `<expression>` with no terminating `;`, the block
evaluates to that expression's value.

```edge
let result = {
    let x = 2;
    x * x   // no semicolon — this is the block's value (4)
};
```

If the trailing semicolon is present, the block evaluates to `unit`
(i.e. the expression's value is discarded).

:::note
At the AST level, tail expressions are not distinguished from regular
expression statements — both are stored as `BlockItem::Expr(expr)`. The
semantic difference (block evaluates to this value) is determined by
position: only the last item in the block, if it is an expression without
a semicolon, acts as the return value.
:::

## Semantics

Code blocks represent a distinct scope. Identifiers declared within a code
block are dropped when the block ends. Blocks may be nested arbitrarily.

Orphan semicolons (e.g. after `match {}`) are silently skipped by the parser.
