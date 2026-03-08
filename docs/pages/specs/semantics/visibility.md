---
title: Visibility
---

# Visibility

Visibility controls which items are accessible from outside their declaring
scope. Edge has four visibility levels:

| Modifier | Visibility |
|----------|------------|
| _(none)_ | **Private** — accessible only within the declaring scope (module or implementation block) |
| `pub` | **Public** — accessible from sibling modules; inside a `contract` block, also generates a dispatch entry (equivalent to `pub ext`) |
| `pub ext` | **External** — explicitly callable via EVM ABI dispatch; generates a function selector entry in the contract's dispatch table |
| `pub mut` | **Mutable public** — accessible externally with write (state-mutating) permissions |

## Private (default)

Items with no visibility modifier are private to their declaring module or
implementation block. They are not accessible from other modules and are not
exported in any ABI.

```edge
fn helperFunction() -> u256 {
    // Only accessible within this module
    return 42;
}
```

## `pub` — Module-public

The `pub` modifier makes an item accessible from other modules. It can also be
used with `use` to re-export an imported item:

```edge
mod moduleA {
    pub type TypeA = u256;
}

mod moduleB {
    // Re-export TypeA so it is accessible as moduleB::TypeA
    pub use super::moduleA::TypeA;
}
```

## `pub ext` — External (ABI-visible)

The `pub ext` modifier marks a function as externally callable via the EVM ABI.
The compiler generates a 4-byte function selector (keccak256 of the function
signature) and adds a dispatch entry in the contract's runtime that routes
incoming calls to this function.

```edge
contract Token {
    let balance: &s u256;

    pub ext fn transfer(to: addr, amount: u256) -> bool {
        // Callable by anyone via EVM CALL
        return true;
    }
}
```

Both `pub` and `pub ext` functions receive selector entries in the dispatch
table. Inside a `contract` block, `pub fn` implicitly creates a dispatch entry
(equivalent to `pub ext fn`). Private functions (no modifier) are not reachable
from outside the contract unless inlined.

:::warning
The `pub ext` modifier is parsed and used for dispatch table generation, but
full ABI metadata emission (JSON ABI) is not yet implemented. The modifier
correctly generates selector entries and calldata decoding in the compiled
bytecode.
:::

## `pub mut` — Mutable public

The `pub mut` modifier marks a function as externally callable with permission
to mutate contract state. This is a subtype of external visibility that
additionally signals state-mutating intent, which is reflected in the ABI
and affects tooling (e.g., transaction simulation, static analysis).

```edge
contract Owned {
    let owner: &s addr;

    pub mut fn setOwner(newOwner: addr) {
        // State-mutating external function
    }
}
```

:::warning
The `pub mut` modifier is parsed and treated as an external function for
dispatch purposes. Distinguishing `pub ext` (view) from `pub mut` (mutating)
in the generated ABI metadata is not yet fully implemented — both currently
generate dispatch entries with identical codegen behavior.
:::

## Visibility and EVM dispatch

At the IR level, the contract runtime is structured as a dispatcher that reads
the first 4 bytes of calldata (the function selector), matches it against
registered selectors, and jumps to the corresponding function body. Only
`pub`, `pub ext`, and `pub mut` functions all generate selector entries.
Private functions (no modifier) are internal and may be inlined by the
optimizer.

The dispatch strategy depends on the number of public functions:

| Public functions | Strategy |
|------------------|----------|
| < 4 | Linear if-else chain — O(N) |
| ≥ 4 | Balanced binary search tree — O(log N) |

:::note
The complete visibility rules, including interaction with trait implementations
and cross-contract calls, are still being finalized in the specification.
:::
