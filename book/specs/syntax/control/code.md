# Code Block

A code block is a sequence of items with its own scope.
It may be used independently or in tandem with conditional
statements.

## Declaration

```
<code_block> ::= "{" ((<stmt> | <expr>) ";")* "}" ;
```

Dependencies:

* `<stmt>`
* `<expr>`

The `<code_block>` is a semi-colon separated list of
expressions or statements delimited by curly braces.

## Semantics

Code blocks may be contained in loops, branching
statements, or standalone statements.

Code blocks represent a distinct scope. Identifiers
declared in a code block are dropped once the code
block ends.
