---
title: Compile-time functions
---

# Compile-time functions

```text
<comptime_function> ::= "comptime" <function_assignment> ;
```

Dependencies:

* `<function_assignment>`

The `<comptime_function>` produces `Stmt::ComptimeFn(FnDecl, CodeBlock)`,
which is distinct from `Stmt::FnAssign`. This distinction affects how later
compiler phases evaluate and inline the function.

## Scope restrictions

`comptime fn` is only valid at module/top-level scope. It cannot appear
inside `contract`, `impl`, or `trait` bodies.

## Semantics

Since comptime functions must be resolved at compile time, the function body
must contain only expressions resolvable at compile time.

```edge
comptime fn a() -> u8 {
    1
}

comptime fn b(arg: u8) -> u8 {
    arg * 2
}

comptime fn c(arg: u8) -> u8 {
    a(b(arg))
}

const A = c(1);
const B = c(A);
```
