---
title: Namespaces
---

# Namespaces

A namespace contains valid identifiers for items that may be used.
Edge uses a hierarchical module-based namespace system.

## Module namespaces

Each file is implicitly a module, forming a namespace for all items declared
within it. Items in a module are accessed using `::` path syntax:

```edge
// Access an item from another module
super::moduleA::TypeA
```

Items must be explicitly imported into the current scope with `use` before they
can be referenced by their short name. See [Scoping](/specs/semantics/scoping)
for details on how `use`, `super::`, and `pub use` bring items into scope.

## Item namespaces

The following kinds of items occupy the module namespace:

- **Types** — declared with `type`
- **Functions** — declared with `fn`
- **Constants** — declared with `const`
- **Traits** — declared with `trait`
- **Implementation blocks** — declared with `impl`
- **Submodules** — declared with `mod`

## Storage field namespaces

Contract storage fields are declared with `let` and a data location annotation,
mapping a field name to a sequential storage slot and a type:

```edge
contract Token {
    let balance: &s u256;
    let owner: &s addr;
}
```

Storage field names occupy a separate namespace from local variables and
functions. At the IR level, each storage field is represented as a
`StorageField(name, slot_index, type)` node. Name resolution maps the source
field name to its concrete slot index at compile time (slots are assigned
sequentially starting at 0).

## Name resolution at the IR level

By the time source code is lowered to the Edge IR, all names are fully resolved:

- Function calls are represented as `Call("fully_resolved_name", args)`.
- Local variables are represented as `LetBind("unique_name", ...)` and `Var("unique_name")`.
- Storage fields are represented as `StorageField("name", slot_index, type)`.

The IR uses plain strings for all names. Name uniqueness (preventing collisions
between locals in different scopes or after function inlining) is the
responsibility of the frontend lowering pass, which renames variables as needed.

When function inlining runs (at optimization level O1+), local variable names
in inlined function bodies are renamed with a unique suffix (e.g., `_s0`, `_s1`)
to prevent collisions with names at the call site.

:::note
The full cross-module name resolution rules and the interaction of namespaces
with `pub use` re-exports are still being expanded in the specification.
:::
