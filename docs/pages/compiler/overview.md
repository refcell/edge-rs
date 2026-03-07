---
title: Compiler Architecture
---

# Compiler Architecture

Edge compiles `.edge` source files into EVM bytecode through a multi-stage pipeline. The compiler uses **egglog** (equality saturation) as its core optimization framework at two distinct stages: once on the high-level IR and once on the emitted bytecode.

## Pipeline Overview

```text
Source (.edge)
    │
    ▼
┌──────────┐
│  Lexer   │  crates/lexer/
└────┬─────┘
     │ Vec<Token>
     ▼
┌──────────┐
│  Parser  │  crates/parser/
└────┬─────┘
     │ Program (AST)
     ▼
┌────────────┐
│ Type Check │  crates/typeck/
└────┬───────┘
     │ CheckedProgram (storage layout, selectors)
     ▼
┌────────────────┐
│ AST → IR Lower │  crates/ir/src/to_egglog.rs
└────┬───────────┘
     │ EvmProgram (RcExpr trees)
     ▼
┌───────────────────┐
│ Rust Pre-Passes   │  var_opt, storage_hoist
└────┬──────────────┘
     │ EvmProgram (optimized)
     ▼
┌──────────────────────┐
│ Egglog EqSat (IR)    │  crates/ir/src/optimizations/*.egg
└────┬─────────────────┘  [skipped at O0]
     │ EvmProgram (extracted best-cost)
     ▼
┌─────────┐
│ Cleanup │  crates/ir/src/cleanup.rs
└────┬────┘
     │ EvmProgram (simplified state params)
     ▼
┌────────────────┐
│ Expr Compiler  │  crates/codegen/src/expr_compiler.rs
└────┬───────────┘
     │ Vec<AsmInstruction>
     ▼
┌──────────────────────────┐
│ Egglog EqSat (Bytecode)  │  crates/codegen/src/bytecode_opt/
└────┬─────────────────────┘  [skipped at O0]
     │ Vec<AsmInstruction> (optimized)
     ▼
┌───────────────────────┐
│ Subroutine Extraction │  crates/codegen/src/subroutine_extract.rs
└────┬──────────────────┘  [size mode only, O2+]
     │ Vec<AsmInstruction>
     ▼
┌───────────┐
│ Assembler │  crates/codegen/src/assembler.rs
└────┬──────┘
     │ Vec<u8> (runtime bytecode)
     ▼
┌─────────────────────┐
│ Constructor Wrapper  │  crates/codegen/src/contract.rs
└────┬────────────────┘
     │ Vec<u8> (deployment bytecode)
     ▼
  Final Output
```

## Crate Map

| Crate | Path | Purpose |
|-------|------|---------|
| `edge-lexer` | `crates/lexer/` | Tokenization with context-sensitive disambiguation |
| `edge-parser` | `crates/parser/` | Recursive descent + Pratt expression parsing |
| `edge-ast` | `crates/ast/` | AST type definitions |
| `edge-types` | `crates/types/` | Shared type definitions (tokens, spans, literals) |
| `edge-typeck` | `crates/typeck/` | Type checking, storage layout, selector generation |
| `edge-diagnostics` | `crates/diagnostics/` | Error reporting infrastructure |
| `edge-ir` | `crates/ir/` | Egglog-based IR: lowering, optimization rules, extraction |
| `edge-codegen` | `crates/codegen/` | EVM bytecode generation, bytecode optimizer, assembler |
| `edge-driver` | `crates/driver/` | Pipeline orchestration |
| `edge-evm-tests` | `crates/evm-tests/` | EVM test host (revm-based) |
| `edgec` | `bin/edgec/` | CLI binary |

---

## Stage 1: Lexing

**File:** `crates/lexer/src/lexer.rs`

Converts source text into tokens. The lexer tracks a `Context` (Global vs Contract) to disambiguate EVM type names from opcode names (e.g., `bytes32` as a type vs `byte` as an opcode).

- Hex literal parsing (`0x`/`0b`) requires the `'0'` match arm to come before the generic digit arm
- Lookback token enables context-sensitive tokenization
- Outputs `Iterator<Item = Result<Token, LexError>>`

## Stage 2: Parsing

**File:** `crates/parser/src/parser.rs`

Recursive descent parser with Pratt parsing for operator precedence. Eagerly lexes all tokens into a `Vec<Token>` (dropping whitespace/comments) for O(1) random access.

Key design decisions:
- SHL/SHR operand swap happens at parse time: `a << b` becomes `Bop(Shl, b, a)` to match EVM stack order
- Produces `Program { stmts: Vec<Stmt>, span }`

## Stage 3: Type Checking

**File:** `crates/typeck/src/checker.rs`

Walks the AST, resolves types, computes storage layouts (sequential slot assignment), and generates 4-byte function selectors via `keccak256("name(type1,type2,...)")`.

Output: `CheckedProgram { contracts: Vec<ContractInfo> }` with `StorageLayout { slots: IndexMap<String, u32> }`.

## Stage 4: AST → IR Lowering

**File:** `crates/ir/src/to_egglog.rs` (~1700 lines)

The most complex stage. Converts `edge_ast::Program` into `EvmProgram`, an IR designed for egglog equality saturation.

### Core IR Type: `EvmExpr`

The IR is a tree of `EvmExpr` nodes (30+ variants), reference-counted as `RcExpr = Rc<EvmExpr>`:

| Category | Variants |
|----------|----------|
| Constants | `Const(EvmConstant)`, `Selector(String)` |
| Variables | `LetBind(name, init, body)`, `Var(name)`, `VarStore(name, val)`, `Drop(name)` |
| Operators | `Bop(op, lhs, rhs)`, `Uop(op, arg)`, `Top(op, a, b, c)` |
| Control flow | `If(cond, state, then, else)`, `DoWhile(inputs, pred_and_body)` |
| Sequencing | `Concat(first, second)`, `Empty` |
| Storage | `SLoad`, `SStore`, `TLoad`, `TStore`, `MLoad`, `MStore`, `MStore8` via `Bop`/`Top` |
| Functions | `Function(name, body)`, `Call(name, args)` |
| Effects | `Log(n, topics, data, state)`, `Revert`, `ReturnOp`, `ExtCall` |
| Tuples | `Get(tuple, index)` |
| Context | `Arg(type)`, `EnvRead`, `EnvRead1`, `StorageField` |

### Key Design Decisions

- **State threading**: IR uses explicit `StateT` tokens for side-effect ordering. Codegen ignores state parameters entirely, relying on `Concat` sequencing instead.
- **Memory-backed variables**: `VarDecl` creates `LetBind`/`Var`/`VarStore`/`Drop` nodes. `LetBind(name, init, body)` allocates, `Var(name)` reads, `VarStore(name, val)` writes, `Drop(name)` marks lifetime end.
- **Store-forwarding at source**: First `VarStore` init is extracted directly into `LetBind` when safe, avoiding the `LetBind(x, 0, Concat(VarStore(x, real), ...))` pattern.
- **Function inlining**: Functions are inlined at call sites. `inline_depth` counter ensures `return` inside inlined functions produces just the value (not `RETURN` opcode).
- **Checked arithmetic**: User `+`, `-`, `*` lower to `OpCheckedAdd`/`OpCheckedSub`/`OpCheckedMul`. Internal compiler arithmetic (mapping slots, memory offsets) uses unchecked ops.
- **Mapping slots**: `keccak256(key . base_slot)` — MSTORE key at offset 0, MSTORE slot at offset 32, KECCAK256(0, 64).
- **DoWhile ordering**: `pred_and_body = Concat(body, cond)` — body side effects run before condition re-evaluation.

## Stage 5: Rust Pre-Passes

Two Rust passes run before egglog, handling transforms that pattern matching cannot express:

### 5a: Variable Optimization (`crates/ir/src/var_opt.rs`)

Counting-based transforms requiring occurrence analysis:

| Transform | Description |
|-----------|-------------|
| Dead variable elimination | Remove `LetBind(x, pure_init, body)` where `x` has 0 reads |
| Single-use inlining | Replace `Var(x)` with init value when read_count == 1, not in loop |
| Constant propagation | Propagate constant inits through multi-use variables |
| Allocation analysis | Decide stack vs memory mode per variable |
| Early Drop insertion | Insert `Drop(var)` before RETURN/REVERT in branches that don't reference the variable |
| Immutable var collection | Emit `ImmutableVar` facts for egglog bound propagation |

**Allocation modes:**
- **Stack** (DUP-based, 3 gas/read): `write_count == 0`, not in loop, `read_count <= 8`
- **Memory** (MSTORE/MLOAD, 6 gas each): everything else
- Drop-based free-list reclaims memory slots

### 5b: Storage Hoisting (`crates/ir/src/storage_hoist.rs`)

LICM (Loop-Invariant Code Motion) for `SLoad`/`SStore` in loops:

1. Identifies constant-slot storage ops inside `DoWhile` bodies
2. Hoists them into `LetBind` locals before the loop
3. Replaces `SLoad`/`SStore` with `Var`/`VarStore` inside the loop body
4. Emits write-backs after loop exit

This pass is **critical** because it prevents egglog's storage forwarding rules from firing unsoundly across loop back-edges.

Also performs straight-line `SStore` → `SLoad` forwarding and dead store elimination.

**Bail conditions:** `ExtCall` in loop body, nested loops.

## Stage 6: Egglog Equality Saturation (Stage 1 — IR Level)

**Files:** `crates/ir/src/optimizations/*.egg`, `crates/ir/src/schedule.rs`, `crates/ir/src/costs.rs`

The IR is serialized to S-expressions (`sexp.rs`), fed into an egglog e-graph with ~290 optimization rules across 11 rulesets, and the best-cost result is extracted.

### Schedule by Optimization Level

| Level | Schedule |
|-------|----------|
| O0 | Skip egglog entirely |
| O1 | `saturate(dead-code, range-analysis, type-propagation)` then 3× `peepholes → u256-const-fold → saturate(...)` |
| O2 | `saturate(...)` then 5× `peepholes → arithmetic-opt → u256-const-fold → storage-opt → memory-opt → saturate(...) → cse-rules` |
| O3+ | Same as O2 but 10× iterations |

### Cost Model

Two modes controlled by `--optimize-for`:

| Expression | Gas Cost | Size Cost |
|------------|----------|-----------|
| Cheap arith (ADD, SUB, LT, GT, ...) | 3 | 1 |
| Expensive arith (MUL, DIV, ...) | 5 | 1 |
| CheckedAdd/CheckedSub | 20 | 1 |
| CheckedMul | 30 | 1 |
| SLoad | 2100 | 1 |
| SStore | 5000 | 1 |
| TLoad/TStore | 100 | 1 |
| MLoad/MStore | 3 | 1 |
| Keccak256 | 36 | 1 |
| Const/Selector | 3 | 1 |
| LetBind | 3 | 1 |
| Var | 3 | 1 |
| ExtCall | 100 | 1 |
| Log | 375 | 1 |
| If/DoWhile | 10 | 1 |

In size mode, every node costs 1, minimizing total node count. In gas mode, costs reflect EVM gas prices, so the extractor avoids expensive operations.

### Egglog Optimization Rules

#### Peepholes (`peepholes.egg` — ruleset: `peepholes`)

52 rules for algebraic simplification:

| Category | Examples | Count | `:subsume` |
|----------|---------|-------|------------|
| Identity removal | `x + 0 → x`, `x * 1 → x`, `x - 0 → x`, `x / 1 → x` | 7 | Yes |
| Zero/annihilation | `0 * x → 0`, `0 / x → 0` | 4 | Yes |
| SmallInt const-fold | `SmallInt(i) + SmallInt(j) → SmallInt(i+j)` (all arith ops) | 5 | Yes |
| Comparison fold | `LT(SmallInt i, SmallInt j)` → `0` or `1` | 6 | Yes |
| IsZero of constants | `IsZero(0) → 1`, `IsZero(n) → 0` | 2 | Yes |
| Boolean simplification | `true && x → x`, `false \|\| x → x`, etc. | 6 | Yes |
| Double negation | `IsZero(IsZero(x)) → x`, `NOT(NOT(x)) → x` | 2 | Yes |
| Self-cancellation | `x - x → 0`, `x ^ x → 0`, `x == x → 1` | 3 | No |
| Select simplification | `Select(c, x, x) → x` | 1 | Yes |
| Constant-condition If | `If(true, ...) → then`, `If(false, ...) → else` | 4 | Yes |
| Reassociation | `(x + i) + j → x + (i+j)`, `(x * i) * j → x * (i*j)` | 6 | No |
| Commutativity | `ADD(a, b) ↔ ADD(b, a)` (for ADD, MUL, EQ, AND, OR, XOR) | 6 | No |

Checked arithmetic peepholes (8 additional rules): `CheckedAdd(x, 0) → x`, `CheckedMul(x, 1) → x`, `CheckedSub(x, x) → 0`, etc.

#### Arithmetic Optimization (`arithmetic.egg` — ruleset: `arithmetic-opt`)

31 rules for strength reduction and algebraic identities:

| Category | Examples | Count |
|----------|---------|-------|
| MUL → SHL | `val * 2^n → SHL(n, val)` (computed via `log2`) | 2 |
| DIV → SHR | `val / 2^n → SHR(n, val)` | 1 |
| MOD → AND | `val % 2^n → AND(val, 2^n - 1)` | 1 |
| EXP reduction | `x**0 → 1`, `x**1 → x`, `x**2 → x*x`, `x**3 → x*(x*x)`, `x**4 → (x*x)*(x*x)` | 5 |
| Bitwise identity | `x & x → x`, `x \| x → x` | 2 |
| Bitwise zero/identity | `x ^ 0 → x`, `x & 0 → 0`, `x \| 0 → x` | 6 |
| Shift by zero | `SHL(0, e) → e`, `SHR(0, e) → e` | 3 |
| EQ to ISZERO | `EQ(e, 0) → IsZero(e)` | 2 |
| Absorption laws | `x & (x \| y) → x`, `x \| (x & y) → x` | 4 |
| Bitwise const-fold | `AND/OR/XOR/SHL/SHR(SmallInt, SmallInt) → SmallInt` | 5 |

#### Storage Optimization (`storage.egg` — ruleset: `storage-opt`)

16 rules for storage load/store optimization:

| Rule | Pattern | Result | `:subsume` |
|------|---------|--------|------------|
| Load-after-store | `SLoad(slot, SStore(slot, val, st))` | `val` | Yes |
| Redundant store | `SStore(slot, SLoad(slot, st), st)` | `st` | Yes |
| Dead store | `SStore(slot, v, SStore(slot, v2, st))` | `SStore(slot, v, st)` | Yes |
| Cross-slot load | `SLoad(s1, SStore(s2, v, st))` where s1≠s2 | `SLoad(s1, st)` | No |
| SLoad through MStore | `SLoad(slot, MStore(..))` | `SLoad(slot, state)` | Yes |
| SLoad through TStore | `SLoad(slot, TStore(..))` | `SLoad(slot, state)` | Yes |
| SLoad through Log | `SLoad(slot, Log(..))` | `SLoad(slot, state)` | Yes |

Same 8 rules mirrored for `TLoad`/`TStore` (transient storage).

#### Memory Optimization (`memory.egg` — ruleset: `memory-opt`)

6 rules mirroring storage optimization for memory:

- `MLoad(off, MStore(off, val, st)) → val`
- `MStore(off, MLoad(off, st), st) → st`
- `MStore(off, v, MStore(off, v2, st)) → MStore(off, v, st)`
- `MLoad` forwarded through `SStore`, `TStore`, `Log`

#### Dead Code Elimination (`dead_code.egg` — ruleset: `dead-code`)

~30 `IsPure` analysis rules + 5 elimination rules:

| Rule | Pattern | Result |
|------|---------|--------|
| Empty concat | `Concat(Empty, rest)` | `rest` |
| Empty concat (right) | `Concat(inner, Empty)` | `inner` |
| Pure dead code | `Concat(pure_inner, rest)` | `rest` |
| Nested pure dead | `Concat(Concat(prev, pure), rest)` | `Concat(prev, rest)` |
| Dead variable | `LetBind(x, pure_init, Drop(x))` | `Empty` |

**Pure expressions:** `Const`, `Arg`, `Empty`, `Selector`, `Var`, `Drop`, all `Uop`s, most `Bop`s (including `SLoad`, `TLoad`, `MLoad`), `Keccak256`, `Select`, `EnvRead`. Also `Concat(pure, pure)` and `If(pure, pure, pure)`.

**Not pure:** `SStore`, `TStore`, `MStore`, `MStore8`, `Log`, `Revert`, `ReturnOp`, `ExtCall`, `CheckedAdd/Sub/Mul` (can revert), `VarStore`, `LetBind`.

#### Range Analysis (`range_analysis.egg` — ruleset: `range-analysis`)

~50 analysis rules + 4 guarded rewrites. Uses lattice-based interval tracking:

**Lattice functions:**
- `upper-bound(EvmExpr) → i64` with `:merge (min old new)`
- `lower-bound(EvmExpr) → i64` with `:merge (max old new)`
- `u256-upper-bound`/`u256-lower-bound` for full U256 range
- `max-bits(EvmExpr) → i64`
- Relations: `NonZero(EvmExpr)`, `IsBool(EvmExpr)`

**Bound propagation:**

| Op | Bounds Derived |
|----|----------------|
| AND(a, b) | upper ≤ min(upper(a), upper(b)), lower ≥ 0 |
| OR(a, b) | lower ≥ max(lower(a), lower(b)) |
| SHR(shift, val) | upper ≤ val_upper >> shift_lower |
| MOD(a, b) | [0, upper(b) - 1] |
| DIV(a, b) | lower ≥ lower(a)/upper(b), upper ≤ upper(a)/lower(b) |
| BYTE | [0, 255], max-bits 8 |
| Comparisons | [0, 1], IsBool, max-bits 1 |

**ImmutableVar bound propagation:** When `ImmutableVar(name)` is asserted (by `var_opt`), all bounds from `LetBind` init are propagated to `Var(name)` reads. This enables checked arithmetic elision through variables.

**Analysis-guarded rewrites:**
- `x / x → 1` when `NonZero(x)`
- `x % x → 0` when `NonZero(x)`
- `IsZero(IsZero(x)) → x` when `IsBool(x)`
- `bool & 1 → bool` when `IsBool`

#### U256 Constant Folding (`u256_const_fold.egg` — ruleset: `u256-const-fold`)

26 rules for folding operations on `LargeInt` (U256) constants using a custom egglog U256 sort:

- Arithmetic: ADD, SUB, MUL, DIV, MOD, EXP
- Bitwise: AND, OR, XOR, SHL, SHR
- Comparison: EQ, LT, GT, ISZERO (6 rules with two branches each)
- Power-of-2 strength reduction on U256 values (MUL/DIV/MOD → SHL/SHR/AND)
- `LargeInt → SmallInt` normalization when value fits in i64

#### Checked Arithmetic Elision (`checked_arithmetic.egg` — ruleset: `range-analysis`)

3 key elision rules + ~14 bound propagation rules:

| Rule | Guard | Result |
|------|-------|--------|
| `CheckedAdd(a, b) → Add(a, b)` | `u256-add-no-overflow(upper(a), upper(b))` | Unchecked add |
| `CheckedSub(a, b) → Sub(a, b)` | `u256-sub-no-underflow(lower(a), upper(b))` | Unchecked sub |
| `CheckedMul(a, b) → Mul(a, b)` | `u256-mul-no-overflow(upper(a), upper(b))` | Unchecked mul |

Checked ops also propagate bounds (since no overflow is guaranteed): `CheckedAdd(a, b)` upper = upper(a) + upper(b), enabling cascading elision.

Constant folding for checked ops: `CheckedAdd(const_a, const_b) → LargeInt(a+b)` when overflow check passes.

#### Type Propagation (`type_propagation.egg` — ruleset: `type-propagation`)

42 purely additive analysis rules populating `HasType` and `FunctionHasType` relations. No rewrites. Used by other passes for type-aware optimization.

#### CSE (`cse.egg` — ruleset: `cse-rules`)

No explicit rules. CSE is automatic via egglog's e-graph hash-consing. Commutativity rules in `peepholes.egg` ensure `ADD(a,b)` and `ADD(b,a)` share an e-class.

## Stage 7: Post-Egglog Cleanup

**File:** `crates/ir/src/cleanup.rs`

Two passes after egglog extraction:

1. **State simplification**: Replaces all nested state parameters (massive `SStore`/`SLoad` chains) with a simple `Arg(StateT)` sentinel. Codegen ignores state params entirely.
2. **Dead code after halt**: Removes unreachable code after `ReturnOp`/`Revert` in `Concat` chains.

Also runs straight-line `SStore` → `SLoad` forwarding one more time to catch patterns egglog's cross-slot rules couldn't handle.

## Stage 8: Expression Compiler

**File:** `crates/codegen/src/expr_compiler.rs`

Walks the `EvmExpr` tree and emits EVM opcodes into an `Assembler`. Since the EVM is a stack machine, children are compiled first (postorder), then the operator.

**Key state:**
- `let_bindings: HashMap<String, usize>` — variable name → memory offset
- `next_let_offset: usize` — high-water mark starting at `0x80`
- `free_slots: Vec<usize>` — reclaimed by `Drop`
- `stack_vars: HashMap<String, StackVarInfo>` — for stack-allocated variables
- `stack_depth: usize` — tracks current stack depth for DUP indexing
- `overflow_revert_label` — shared trampoline for all checked arithmetic

**Memory layout:** Fixed offsets starting at `0x80`. No free memory pointer — our codegen never reads `0x40`.

**Checked arithmetic codegen:**
- `CheckedAdd`: `b > result` overflow detection (6 extra opcodes)
- `CheckedSub`: `a < b` pre-check (5 extra opcodes)
- `CheckedMul`: `result/a != b` with `a==0` short-circuit (~12 extra opcodes)
- Shared `overflow_revert` trampoline (`PUSH0, PUSH0, REVERT`) emitted once

**Branch handling:** `compile_if` saves/restores `stack_vars`, `let_bindings`, `free_slots` per branch so that Drop/slot-reuse in one branch doesn't affect the other. Halting branches get special stack depth handling.

## Stage 9: Bytecode Optimization (Stage 2 — Egglog)

**Files:** `crates/codegen/src/bytecode_opt/`

A second egglog pass operating on `AsmInstruction` sequences. Splits code into basic blocks, optimizes each through egglog, and reassembles.

### Schedule

| Level | Schedule |
|-------|----------|
| O0 | None |
| O1 | 3× `bytecode-peepholes → bytecode-dead-push` |
| O2 | 5× `bytecode-peepholes → bytecode-const-fold → bytecode-strength-red → bytecode-dead-push` |
| O3+ | 10× all rulesets |

### Bytecode Rewrite Rules (~68 rules)

| Category | Examples | Count |
|----------|---------|-------|
| DUP dedup | `PUSH x, PUSH x → PUSH x, DUP1` | 3 |
| Cancellation | `SWAPn SWAPn → ε`, `NOT NOT → ε`, `DUPn POP → ε` | 8 |
| Commutative swap elim | `SWAP1 ADD → ADD` (also MUL, AND, OR, XOR, EQ) | 6 |
| Const fold | `PUSH(i) PUSH(j) ADD → PUSH(i+j)` (8 ops) | 10 |
| Strength reduction | `PUSH(0) ADD → ε`, `PUSH(1) MUL → ε`, `PUSH(2) MUL → PUSH(1) SHL` | ~20 |
| MOD → AND | `PUSH(2^n) MOD → PUSH(2^n - 1) AND` | 8 |
| Dead push | `PUSH(x) POP → ε` | 3 |

**Pre-pass:** Dead code elimination after `RETURN`/`REVERT`/`STOP`.
**Post-pass:** Label aliasing (consecutive labels → keep last).

### Bytecode Cost Model

| Tier | Gas | Opcodes |
|------|-----|---------|
| Gzero | 0 | STOP, RETURN, REVERT, INVALID |
| Gbase | 2 | POP, ADDRESS, ORIGIN, CALLER, CALLVALUE, ... |
| Gverylow | 3 | ADD, SUB, LT, GT, EQ, AND, OR, XOR, SHL, SHR, MLOAD, MSTORE, PUSH, DUP, SWAP (default) |
| Glow | 5 | MUL, DIV, SDIV, MOD, SMOD, SIGNEXTEND |
| Medium | 8 | ADDMOD, MULMOD |
| KECCAK256 | 36 | |
| EXP | 60 | |
| Warm access | 100 | BALANCE, EXTCODESIZE, TLOAD, TSTORE, CALL, ... |
| SLOAD | 2100 | |
| SSTORE | 5000 | |
| LOG | 750 | |
| CREATE | 32000 | |

## Stage 10: Subroutine Extraction

**File:** `crates/codegen/src/subroutine_extract.rs`

Size-mode only (O2+). Detects repeated instruction sequences and extracts them into JUMP-based subroutines, trading ~30 gas/call for code size reduction.

**Algorithm:**
1. Find straight-line regions (between labels/jumps)
2. Find all repeated subsequences (min 3 occurrences, min 15 bytes, min 5 instructions)
3. Greedy selection of most profitable non-overlapping candidates
4. Rewrite: replace inline code with calls, append subroutine bodies

**Calling convention:** `PushLabel(ret) + JumpTo(sub) + JUMPDEST(ret)`. Subroutine uses SWAP chains for stack management.

## Stage 11: Assembler

**File:** `crates/codegen/src/assembler.rs`

Converts `AsmInstruction` sequences into final bytecode with label resolution:

- **Short jumps**: `PUSH1` for contracts < 256 bytes, `PUSH2` otherwise
- **Two-pass assembly**: first pass computes label offsets, second pass emits bytes
- **`AsmInstruction` variants**: `Op(Opcode)`, `Push(Vec<u8>)`, `Label(String)`, `JumpTo(String)`, `JumpITo(String)`, `PushLabel(String)`, `Comment(String)`

## Stage 12: Constructor Wrapper

**File:** `crates/codegen/src/contract.rs`

Produces two-part deployment bytecode:

1. **Constructor** (init code): Runs constructor body, then `CODECOPY` + `RETURN` to deploy runtime
2. **Runtime**: Dispatcher + inlined function bodies

**Dispatcher:** Binary search dispatch (BST with `GT` branching) for 4+ functions; linear selector chain below 4. Selectors sorted numerically.

## Egglog Advanced Features

The compiler makes heavy use of egglog's advanced capabilities:

| Feature | Usage |
|---------|-------|
| Merge functions | `upper-bound` uses `:merge (min old new)`, `lower-bound` uses `:merge (max old new)` — lattice semantics |
| Subsumption | `:subsume` on identity removal, constant folding, annihilation rules to keep e-graph lean |
| Computed functions | `log2`, `&`, bitwise ops for generalized power-of-2 detection |
| Custom sorts | `U256Sort` for full 256-bit arithmetic in egglog |
| Sentinel context | `InFunction("__opt__")` for self-cancellation rules (`x-x→0`) |
| Analysis scheduling | `saturate(seq(run dead-code)(run range-analysis))` before optimization rulesets |
| Dual cost models | Parameterized `:cost` annotations on the schema, switched at compile time |

## Summary Statistics

| Component | Rule Count | Rulesets |
|-----------|-----------|---------|
| peepholes.egg | 60 | `peepholes` |
| arithmetic.egg | 31 | `arithmetic-opt` |
| storage.egg | 16 | `storage-opt` |
| memory.egg | 6 | `memory-opt` |
| dead_code.egg | ~35 | `dead-code` |
| range_analysis.egg | ~54 | `range-analysis` |
| u256_const_fold.egg | 26 | `u256-const-fold` |
| type_propagation.egg | 42 | `type-propagation` |
| checked_arithmetic.egg | ~28 | `range-analysis`, `peepholes`, `u256-const-fold` |
| cse.egg | 0 (implicit) | `cse-rules` |
| **Stage 1 Total** | **~298** | **11 rulesets** |
| bytecode rules | ~68 | 4 rulesets |
| **Grand Total** | **~366 rules** | **15 rulesets** |
