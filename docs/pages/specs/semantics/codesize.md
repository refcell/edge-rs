---
title: Codesize
---

# Codesize & optimization

This document details Edge's optimization pipeline. The compiler transforms
source programs through a multi-stage IR optimization pipeline before generating
EVM bytecode.

## Optimization levels

Edge supports four optimization levels controlled via `--opt-level N` (default: 0):

| Level | Description | Egglog iterations | Rulesets |
|-------|-------------|-------------------|----------|
| O0 | No equality saturation | 0 (skipped entirely) | Only pre-egglog `var_opt` passes + store forwarding |
| O1 | Fast, safe only | 3 | peepholes, u256-const-fold, const-prop, dead-code, range-analysis, type-propagation |
| O2 | Full suite | 5 | O1 rulesets + arithmetic-opt, storage-opt, memory-opt, cse-rules |
| O3 | Aggressive | 10 | Same rulesets as O2 |

At O0, the program still benefits from pre-egglog variable optimization
(dead variable elimination, store forwarding, constant propagation) but
equality saturation is skipped entirely for maximum compilation speed.

## Gas vs size optimization

Pass `--optimize-for gas` (default) or `--optimize-for size` to control
what the optimizer minimizes:

- **Gas mode** — Each IR node is weighted by its EVM gas cost. Key costs:

  | Node | Gas cost | Note |
  |------|----------|------|
  | `ADD`, `SUB`, `LT`, `GT`, `EQ`, `AND`, `OR`, `XOR`, `SHL`, `SHR`, `SAR`, `BYTE` | 3 | W_verylow |
  | `MUL`, `DIV`, `SDIV`, `MOD`, `SMOD` | 5 | W_low |
  | `EXP` | 60 | 10 + ~50/byte |
  | `CheckedAdd`, `CheckedSub` | 20 | Higher than unchecked to prefer elision |
  | `CheckedMul` | 30 | Higher than unchecked to prefer elision |
  | `SLOAD` | 2100 | Warm SLOAD |
  | `SSTORE` | 5000 | |
  | `TLOAD`, `TSTORE` | 100 | EIP-1153 |
  | `MLOAD`, `MSTORE`, `CalldataLoad` | 3 | |
  | `Call` (internal) | 1,000,000 | Forces extractor to prefer inlined body |
  | `LOG` | 375 | |
  | `ExtCall` | 100 | |
  | `LetBind` | 3 | MSTORE cost |
  | `Var` | 3 | MLOAD cost |
  | `VarStore` | 6 | PUSH offset + MSTORE |

  The optimizer selects the equivalent program form with the lowest total gas.

- **Size mode** — Every IR node costs 1, regardless of opcode. The optimizer
  minimizes instruction count, which reduces deployment cost and helps stay
  within the EVM's 24 KB contract size limit.

Both modes use egglog's `TreeAdditiveCostModel` extractor, which picks the
cheapest equivalent program discovered during equality saturation.

## Pipeline stages

### Stage 1: Pre-egglog variable optimization (`var_opt`)

Runs before equality saturation. Performs tree-level transforms that require
occurrence counting — something egglog pattern matching cannot express directly.
Transforms are applied bottom-up in a single traversal:

| Transform | Condition | Effect |
|-----------|-----------|--------|
| Dead variable elimination | `reads=0, writes=0` | Removes `LetBind`; preserves side effects via `Concat` |
| Single-use inlining | `reads=1, writes=0, not in loop, pure init` | Substitutes init at the single read site, drops `LetBind` |
| Last-store forwarding | `reads=1, writes=1, not in loop` | Forwards `VarStore` value directly to the `Var` read site, eliminates both |
| Constant propagation | `writes=0, not in loop, init is a literal constant` | Substitutes the constant at all read sites, drops `LetBind` |
| Function inlining | O1+ only | Substitutes function body at each call site, renames locals, recurses |
| Early drop insertion | Always (post-optimize) | Inserts `Drop` markers before halting branches for unused variables |

**Function inlining** at O1+ (`inline_calls`): for each `Call("f", args)`, the
compiler looks up the function body, substitutes actual arguments for formal
parameters, renames all local variables with a unique suffix to avoid
collisions, and recursively inlines nested calls in the substituted body.
This eliminates `Call` nodes from the IR tree before egglog runs, enabling
subsequent dead-variable elimination and constant propagation across inlined
boundaries.

### Stage 2: Storage LICM — loop invariant code motion

After `var_opt` and before egglog, `storage_hoist` hoists storage accesses
with constant slot indices out of loop bodies:

1. Emits `let $var = SLOAD(slot)` before the loop.
2. Replaces `SLOAD(slot)` → `$var` and `SSTORE(slot, val)` → `$var = val` inside the loop.
3. Emits `SSTORE(slot, $var)` write-back after the loop (only for slots that were written).

This eliminates repeated expensive `SLOAD`/`SSTORE` opcodes (2100 and 5000 gas
respectively) in hot loops. Both persistent storage (`SLOAD`/`SSTORE`) and
transient storage (`TLOAD`/`TSTORE`) are hoisted. Loops containing external
calls or nested loops are not hoisted, as they may access storage via unknown
aliases.

### Stage 3: Equality saturation (egglog)

The core optimization engine. The IR is serialized to s-expressions and
submitted to an egglog e-graph along with ~330 rewrite rules across 12 files:

| Rule file | Ruleset | Rules | Purpose |
|-----------|---------|-------|---------|
| `peepholes.egg` | `peepholes` | 52 | Algebraic identities (x+0=x, x×1=x, double-negation, etc.) |
| `arithmetic.egg` | `arithmetic-opt` | 31 | Strength reductions, shift/mask patterns |
| `storage.egg` | `storage-opt` | 16 | SStore→SLoad forwarding, cross-slot rules (state-threaded) |
| `memory.egg` | `memory-opt` | 6 | Memory read/write simplifications |
| `dead_code.egg` | `dead-code` | 45 | Dead code and unreachable branch elimination |
| `range_analysis.egg` | `range-analysis` | 64 | Min/max bound propagation for U256 values |
| `u256_const_fold.egg` | `u256-const-fold` | 28 | Constant folding using full 256-bit arithmetic |
| `type_propagation.egg` | `type-propagation` | 59 | Type information propagation (analysis rules) |
| `checked_arithmetic.egg` | `range-analysis` | 27 | Elides `CheckedAdd`/`CheckedSub`/`CheckedMul` → unchecked when range analysis proves no overflow |
| `cse.egg` | `cse-rules` | 0 | See note below |
| `inline.egg` | `peepholes` | 1 | `Call(name, args) + Function(name, ...)` → substituted body |
| `const_prop.egg` | `const-prop` | 1 | Constant propagation through `LetBind`/`Var` chains |

:::note
**`cse.egg` contains 0 rewrite rules.** Common subexpression elimination is
achieved for free via e-graph hash-consing: structurally identical expressions
automatically share the same e-class, so CSE requires no explicit rules. The
file exists as a named placeholder in the ruleset schedule.
:::

Rulesets are run in an analysis-first schedule: cheap analysis rulesets
(`dead-code`, `range-analysis`) saturate first so their facts are available
to guarded rewrite rules in `peepholes`, `arithmetic`, and `checked-arithmetic`.

**Call node costs:** In the egglog cost model, `Call` nodes are assigned a cost
of 1,000,000. This astronomically high cost ensures the extractor always prefers
the inlined function body form over the `Call` form when both are equivalent,
making function inlining effectively unconditional at O1+.

**Immutable variable facts:** Variables that are never mutated (no `VarStore`)
are declared as `(ImmutableVar "name")` facts before egglog runs. This allows
the `const-prop` ruleset to safely propagate their values through `LetBind`/`Var`
chains without worrying about mutation aliasing.

### Stage 4: Post-egglog passes

After egglog extraction, additional passes run:

- **Cleanup** — Simplifies state parameter chains (which become bloated during
  egglog) back to a sentinel placeholder, since codegen does not use state
  parameters for ordering. Also eliminates dead code after halting instructions
  (`RETURN`, `REVERT`) in `Concat` chains.

- **Store forwarding** — Propagates `SSTORE` values forward to subsequent
  `SLOAD`s of the same constant slot in straight-line `Concat` chains and
  eliminates dead intermediate stores. At O0 (when egglog is skipped), this
  is the only storage optimization that runs. At O1+, egglog's `storage-opt`
  ruleset handles equivalent optimizations during equality saturation, and this
  pass handles remaining cases in the post-egglog IR.

- **Dead function elimination** — After the runtime is optimized, the compiler
  collects all `Call` names still present in the runtime (transitively) and
  discards any internal functions no longer referenced. Each surviving function
  is optimized independently through egglog.

## Dead code elimination

Dead code elimination is handled at multiple stages:

- **Egglog `dead-code` ruleset** — Eliminates unreachable branches during
  equality saturation (e.g., `if true { A } else { B }` → `A`).

- **`var_opt` dead variable pass** — Removes `LetBind` nodes whose variable
  is never read or written.

- **Post-egglog cleanup** — Removes instructions following a halting operation
  (`RETURN`/`REVERT`) in a `Concat` chain.

- **Dead function elimination** — Removes internal functions unreachable from
  the contract's runtime after inlining.

## Checked arithmetic

`CheckedAdd`, `CheckedSub`, and `CheckedMul` compile to arithmetic operations
that `REVERT` on overflow. At O1+, the `checked-arithmetic` egglog ruleset uses
range analysis to elide these checks: when bounds propagation proves that no
overflow is possible (e.g., both operands are statically bounded below their
respective overflow thresholds), the checked operation is rewritten to a plain
`Add`/`Sub`/`Mul`. This eliminates the overhead of the overflow check while
preserving safety guarantees.

## Bytecode peephole optimizer

In addition to IR-level optimization, the Edge compiler applies an egglog-based
peephole optimizer at the bytecode level. This pass operates on basic blocks
of generated EVM bytecode and applies 66 rewrite rules across four rulesets:

| Ruleset | Rules | Examples |
|---------|-------|---------|
| `bytecode-peepholes` | 15 | DUP deduplication, SWAP cancellation, commutativity |
| `bytecode-const-fold` | 10 | `PUSH i, PUSH j, ADD` → `PUSH (i+j)` |
| `bytecode-strength-red` | 38 | Identity elimination, MUL→SHL, DIV→SHR, MOD→AND |
| `bytecode-dead-push` | 3 | `PUSH x, POP` → ε |
