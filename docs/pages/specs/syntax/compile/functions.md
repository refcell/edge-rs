---
title: Compile Time Functions
---

# Compile Time Functions

```text
<comptime_function> ::= "comptime" <function_assignment> ;
```

Dependencies:

* `<function_assignment>`

The `<comptime_function>` is a function that is evaluated at
compile time. It is defined as the "comptime" keyword followed
by a `<function_assignment>`.

## Semantics

Since comptime must be resolved at compile time, the function
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
