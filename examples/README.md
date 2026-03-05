# Edge Examples

A collection of well-crafted [Edge](https://github.com/refcell/edge2) smart contract patterns,
inspired by [solmate](https://github.com/transmissions11/solmate).
These examples cover the full breadth of the Edge language, from basic syntax to
production-grade token standards, DeFi primitives, and access control patterns.

## Running Examples

Each `.edge` file can be processed with `edgec`:

```sh
# Build the compiler
cargo build --bin edgec

# Print the token stream
cargo run --bin edgec -- lex examples/<file.edge>

# Print the abstract syntax tree
cargo run --bin edgec -- parse examples/<file.edge>

# Compile to bytecode (once codegen is fully implemented)
cargo run --bin edgec -- build examples/<file.edge>
```

---

## Introductory

Top-level files that cover the basics of Edge syntax. Start here.

| File | Description | Key Features |
|------|-------------|--------------|
| [`counter.edge`](./counter.edge) | Simple on-chain counter with increment/decrement/reset | `abi`, `contract`, `&s` storage, `pub fn` |
| [`erc20.edge`](./erc20.edge) | Minimal ERC-20 token implementation | `event`, `indexed`, `map<K,V>`, `emit` |
| [`expressions.edge`](./expressions.edge) | Arithmetic, comparison, bitwise, and nested expressions | `+`, `-`, `*`, `/`, `%`, `**`, `&`, `\|`, `^`, `~`, `>>`, `<<` |
| [`types.edge`](./types.edge) | All primitive types, data locations, type aliases, constants | `u8`..`u256`, `i128`, `b32`, `addr`, `bool`, `&s`, `&cd`, `type` alias |

## Tokens

Full implementations of ERC token standards.

| File | Description | Key Features |
|------|-------------|--------------|
| [`tokens/erc20.edge`](./tokens/erc20.edge) | Full ERC-20 with events, hooks trait, internal `_mint`/`_burn`/`_transfer` | `trait`, `impl`, `mod`/`use`, `event indexed` |
| [`tokens/erc721.edge`](./tokens/erc721.edge) | Full ERC-721 with three-event system and `IERC721Receiver` callback | `abi`, `trait`, `impl`, `map<K,V>` |
| [`tokens/erc4626.edge`](./tokens/erc4626.edge) | ERC-4626 tokenized vault with deposit/withdraw/redeem | `use` imports, `VaultMath` trait, multi-module composition |
| [`tokens/weth.edge`](./tokens/weth.edge) | Wrapped ETH (deposit/withdraw) | `@caller()`, `@callvalue()`, `emit`, `map<addr,u256>` |

## Library Primitives (`lib/`)

Foundational modules imported by other examples.

| File | Description | Key Features |
|------|-------------|--------------|
| [`lib/math.edge`](./lib/math.edge) | WAD/RAY fixed-point math, `mul_div_down`/`mul_div_up`, safe add/sub, `min`/`max`/`clamp` | `const`, arithmetic, pure functions |
| [`lib/auth.edge`](./lib/auth.edge) | `IOwned` and `IAuth` traits; `Owned` (two-step transfer) and `Auth` (owner + authority) | `trait`, `impl`, `event`, `&s` |
| [`lib/safe_transfer.edge`](./lib/safe_transfer.edge) | `SafeERC20` trait and helpers for safe ERC-20 and ETH transfers | `trait`, `abi`, external call patterns |

## Utils

Stateless utility libraries for common operations.

| File | Description | Key Features |
|------|-------------|--------------|
| [`utils/merkle.edge`](./utils/merkle.edge) | Merkle proof verifier | `[b32; 32]` fixed array, `for` loop, `&`, `^` bitwise ops |
| [`utils/bits.edge`](./utils/bits.edge) | Bit manipulation: popcount, leading/trailing zeros, isPowerOfTwo | `&`, `\|`, `~`, `^`, `>>`, `<<`, `**`, `while` loop |
| [`utils/bytes.edge`](./utils/bytes.edge) | Bytes32 utilities: address extraction, packing, masking | `b32`, `addr`, `>>`, `<<`, `&`, `\|`, type conversion |

## Access Control

Ownership, role-based access, and emergency pause patterns.

| File | Description | Key Features |
|------|-------------|--------------|
| [`access/ownable.edge`](./access/ownable.edge) | Single-owner with 2-step ownership transfer | `trait`, `impl`, `event indexed`, `@caller()`, `if`/`else` |
| [`access/roles.edge`](./access/roles.edge) | Multi-role authority (RBAC) with grant/revoke | `map<addr, map<b32, bool>>` nested maps, `const` role IDs |
| [`access/pausable.edge`](./access/pausable.edge) | Pausable contract with pause/unpause | `bool` storage, modifier-like internal functions, state machine |

## Finance

DeFi primitives: AMMs, staking, and multisig wallets.

| File | Description | Key Features |
|------|-------------|--------------|
| [`finance/amm.edge`](./finance/amm.edge) | Constant product AMM (x * y = k) | product types, `mul_div` math, `map<addr,u256>` |
| [`finance/staking.edge`](./finance/staking.edge) | ERC-20 staking with per-second reward distribution | `while` loop, multiple `map`s, time-based arithmetic |
| [`finance/multisig.edge`](./finance/multisig.edge) | N-of-M multisig wallet with proposal lifecycle | `type` sum type (enum), `match` statement, `for` loop |

## Patterns

Reusable smart contract design patterns.

| File | Description | Key Features |
|------|-------------|--------------|
| [`patterns/reentrancy_guard.edge`](./patterns/reentrancy_guard.edge) | Reentrancy protection (persistent and transient storage) | `&s`/`&t` storage, `comptime if`, `@hardFork()` |
| [`patterns/timelock.edge`](./patterns/timelock.edge) | Time-locked operations with proposal lifecycle | sum types with data, `match`, `if matches` pattern |
| [`patterns/factory.edge`](./patterns/factory.edge) | CREATE2 deterministic deployment factory | `addr`, `b32`, `map<b32,addr>`, bitwise address derivation |

## Type System Showcase

Dedicated examples for each major type system feature.

| File | Description | Key Features |
|------|-------------|--------------|
| [`types/structs.edge`](./types/structs.edge) | Product types: structs, packed structs, tuples, generic structs | `type T = { .. }`, `packed`, `(T, U)`, `.` field access, `impl` |
| [`types/enums.edge`](./types/enums.edge) | Sum types: simple enums, unions with data, Option/Result | `type T = A \| B(C)`, `match`, `if matches`, `::` syntax |
| [`types/generics.edge`](./types/generics.edge) | Generics (parametric polymorphism) and trait constraints | `fn f<T>()`, `type S<T>`, `trait C<T>`, `impl S<T>` |
| [`types/arrays.edge`](./types/arrays.edge) | Fixed arrays, packed arrays, slices, iteration | `[u256; 5]`, `packed [u8; 32]`, `arr[i]`, `arr[1:3]`, `for` |
| [`types/comptime.edge`](./types/comptime.edge) | Compile-time evaluation: constants, comptime functions, builtins | `const`, `comptime fn`, `comptime if`, `@hardFork()`, `@bitsize()` |

---

## Import Graph

```
tokens/erc4626 --> lib/math
               --> lib/safe_transfer --> tokens/erc20 (IERC20)
               --> lib/auth          (IOwned)
               --> tokens/erc20      (IERC20)

tokens/erc20   --> lib/math
               --> lib/auth           (IOwned)

tokens/erc721  --> lib/auth           (IOwned)
```

## Quick Start

```sh
# Build the compiler
cargo build --bin edgec

# Introductory examples
cargo run --bin edgec -- lex examples/counter.edge
cargo run --bin edgec -- parse examples/types.edge

# Library modules
cargo run --bin edgec -- lex examples/lib/math.edge
cargo run --bin edgec -- parse examples/lib/auth.edge

# Token contracts
cargo run --bin edgec -- parse examples/tokens/erc20.edge
cargo run --bin edgec -- parse examples/tokens/erc721.edge
cargo run --bin edgec -- parse examples/tokens/erc4626.edge
cargo run --bin edgec -- parse examples/tokens/weth.edge

# Utilities
cargo run --bin edgec -- parse examples/utils/merkle.edge
cargo run --bin edgec -- parse examples/utils/bits.edge

# Access control
cargo run --bin edgec -- parse examples/access/ownable.edge
cargo run --bin edgec -- parse examples/access/roles.edge

# Finance / DeFi
cargo run --bin edgec -- parse examples/finance/amm.edge
cargo run --bin edgec -- parse examples/finance/staking.edge
cargo run --bin edgec -- parse examples/finance/multisig.edge

# Patterns
cargo run --bin edgec -- parse examples/patterns/reentrancy_guard.edge
cargo run --bin edgec -- parse examples/patterns/timelock.edge
cargo run --bin edgec -- parse examples/patterns/factory.edge

# Type system showcase
cargo run --bin edgec -- parse examples/types/structs.edge
cargo run --bin edgec -- parse examples/types/enums.edge
cargo run --bin edgec -- parse examples/types/generics.edge
cargo run --bin edgec -- parse examples/types/arrays.edge
cargo run --bin edgec -- parse examples/types/comptime.edge
```
