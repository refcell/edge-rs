---
title: Semantics
---

# Semantics

The semantics section covers language features that are not tied to specific
syntax constructs — general compiler behaviors, name resolution, and access
control.

- [Codesize & optimization](/specs/semantics/codesize)
- [Namespaces](/specs/semantics/namespaces)
- [Scoping](/specs/semantics/scoping)
- [Visibility](/specs/semantics/visibility)

## Optimization overview

Edge compiles through a functional IR (intermediate representation) that is
optimized before code generation. The optimization pipeline has three phases:

1. **Pre-egglog (`var_opt` + storage LICM)** — Tree-level transforms that
   require occurrence counting: dead variable elimination, single-use inlining,
   last-store forwarding, constant propagation, function inlining (O1+), and
   early drop insertion. **Storage LICM** (`storage_hoist`) hoists
   loop-invariant `SLOAD`/`SSTORE` accesses out of loop bodies *before* egglog
   runs.

2. **Equality saturation (egglog)** — The core optimization engine. Applies
   ~333 rewrite rules across 12 rule files in an iterative schedule determined
   by the optimization level (O0–O3). Discovers algebraic equivalences,
   eliminates dead code, folds 256-bit constants, propagates types and value
   ranges, eliminates redundant storage accesses, and elides checked arithmetic
   when provably safe.

3. **Post-egglog** — State-chain cleanup, dead code after halting instructions,
   store forwarding (O0 only), and dead function elimination.

The compiler supports two cost models: `--optimize-for gas` (default) minimizes
EVM execution cost; `--optimize-for size` minimizes instruction count. See
[Codesize & optimization](/specs/semantics/codesize) for full details.
