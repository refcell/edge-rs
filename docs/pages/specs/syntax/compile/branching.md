---
title: Compile Time Branching
---

# Compile Time Branching

```text
<comptime_branch> ::=
    "comptime" (
        | <if_else_if_branch>
        | <if_match>
        | <match>
        | <ternary>
    ) ;
```

Dependencies:

* `<if_else_if_branch>`
* `<if_match>`
* `<match>`
* `<ternary>`

The `<comptime_branch>` is a branch that is evaluated at compile
time where only the truthy branch is compiled. It is defined as
the "comptime" keyword followed by any of the branches.

## Semantics

Since comptime must be resolved at compile time, the branching
expression must be resolvable at compile time and of type bool.
That is to say the expression must itself be a literal, constant,
or another expression resolvable at compile time.

In the case of compile time branching, branches that are not
matched will be removed from the code at compile time.

```edge
use std::{
    builtin::HardFork,
    op::{tstore, tload, sstore, sload},
};

const LOCK_SLOT: u256 = keccak256("mutex").into() - 1;

enum Lock {
    Locked,
    Unlocked,
}

fn reader() -> (u256 -> u256) {
    match @hardFork() {
        HardFork::Cancun => tload,
        _ => sload,
    }
}

fn writer() -> ((u256, u256) -> ()) {
    match @hardFork() {
        HardFork::Cancun => tstore,
        _ => sstore,
    }
}

fn nonreentrant(action: T -> U) {
    if reader()(LOCK_SLOT) matches Lock::Locked {
        revert();
    }
    writer()(LOCK_SLOT, Lock::Locked);
    let res = action();
    writer()(LOCK_SLOT, Lock::Unlocked);
    return res;
}
```
