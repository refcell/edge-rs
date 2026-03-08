---
title: Compile-time branching
---

# Compile-time branching

```text
<comptime_branch> ::= "comptime" <stmt> ;
```

Dependencies:

* `<stmt>`

The `<comptime_branch>` produces `Stmt::ComptimeBranch(Box<Stmt>)`. The
`comptime` keyword may precede any statement, but meaningful conditional
compilation only occurs with branching statements (`if`, `if matches`,
`match`).

## Semantics

Since comptime must be resolved at compile time, the branching expression
must itself be a literal, constant, or expression resolvable at compile
time. Branches that are not matched will be removed from the compiled
output.

```edge
use std::{
    builtin::HardFork,
    op::{tstore, tload, sstore, sload},
};

const SLOT: u256 = 0;

fn store(value: u256) {
    comptime if (@hardFork() == HardFork::Cancun) {
        tstore(SLOT, value);
    } else {
        sstore(SLOT, value);
    }
}

fn load() -> u256 {
    comptime match @hardFork() {
        HardFork::Cancun => tload(SLOT),
        _ => sload(SLOT),
    }
}
```
