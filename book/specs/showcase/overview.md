# Syntax Showcase

The following are Edge Language source code examples, organized by category.
Full source files are in the [`examples/`](https://github.com/refcell/edge2/tree/main/examples) directory.

## Basics

* [Basics](./basics.md): Basic Edge constructs — variables, functions, contracts, storage
* [ERC20](./erc20.md): Sugared and desugared ERC-20 examples

## Introductory Examples

These top-level example files cover the fundamentals of Edge syntax:

| Example | What it covers |
|---------|----------------|
| `counter.edge` | `abi`, `contract`, `&s` storage pointers, `pub fn` visibility |
| `erc20.edge` | `event`, `indexed`, `map<K,V>` storage mappings, `emit` |
| `expressions.edge` | All arithmetic, comparison, bitwise, and logical operators |
| `types.edge` | Primitive types (`u8`..`u256`, `i128`, `b32`, `addr`, `bool`), data locations (`&s`, `&cd`), `type` aliases, `const` |

## Token Standards

Full implementations of ERC token standards:

| Example | Standard |
|---------|----------|
| `tokens/erc20.edge` | ERC-20 with `trait` hooks, `mod`/`use` imports, `_mint`/`_burn`/`_transfer` |
| `tokens/erc721.edge` | ERC-721 with `IERC721Receiver` callback and `abi`/`trait`/`impl` |
| `tokens/erc4626.edge` | ERC-4626 tokenized vault with multi-module composition |
| `tokens/weth.edge` | Wrapped ETH demonstrating `@caller()`, `@callvalue()` builtins |

## Library Primitives

Foundational modules imported by other examples:

| Example | Purpose |
|---------|---------|
| `lib/math.edge` | WAD/RAY fixed-point math, safe arithmetic |
| `lib/auth.edge` | Ownership traits (`IOwned`, `IAuth`) and contracts (`Owned`, `Auth`) |
| `lib/safe_transfer.edge` | Safe ERC-20 and ETH transfer helpers |

## Utility Libraries

Stateless utility functions:

| Example | Purpose |
|---------|---------|
| `utils/merkle.edge` | Merkle proof verification with fixed arrays and bitwise ops |
| `utils/bits.edge` | Bit manipulation: popcount, leading/trailing zeros |
| `utils/bytes.edge` | Bytes32 utilities: address extraction, packing, masking |

## Access Control

| Example | Pattern |
|---------|---------|
| `access/ownable.edge` | Single-owner with 2-step transfer, `trait`/`impl` |
| `access/roles.edge` | Role-based access control with nested `map<addr, map<b32, bool>>` |
| `access/pausable.edge` | Pausable pattern with `bool` state machine |

## Finance / DeFi

| Example | Pattern |
|---------|---------|
| `finance/amm.edge` | Constant product AMM (x * y = k) |
| `finance/staking.edge` | ERC-20 staking with per-second rewards |
| `finance/multisig.edge` | N-of-M multisig with sum types and `match` |

## Design Patterns

| Example | Pattern |
|---------|---------|
| `patterns/reentrancy_guard.edge` | Reentrancy protection with `&s`/`&t` and `comptime if @hardFork()` |
| `patterns/timelock.edge` | Time-locked operations with sum types carrying data |
| `patterns/factory.edge` | CREATE2 deterministic deployment factory |

## Type System Deep Dives

Dedicated examples for each major type system feature:

| Example | Feature |
|---------|---------|
| `types/structs.edge` | Product types: structs, packed structs, tuples, generic structs |
| `types/enums.edge` | Sum types: enums, unions with data, `Option<T>`, `Result<T>` |
| `types/generics.edge` | Generics, trait constraints, monomorphization |
| `types/arrays.edge` | Fixed arrays, packed arrays, slices, iteration |
| `types/comptime.edge` | Compile-time evaluation: `const`, `comptime fn`, `@hardFork()`, `@bitsize()` |
