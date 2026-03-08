---
title: Syntax Showcase
---

# Syntax showcase

The following are Edge language source code examples, organized by category.
Full source files are in the [`examples/`](https://github.com/refcell/edge-rs/tree/main/examples) directory, with standard library modules in [`std/`](https://github.com/refcell/edge-rs/tree/main/std).

## Basics

* [Basics](/specs/showcase/basics): Core Edge constructs — variables, functions, contracts, storage, types, operators
* [ERC20](/specs/showcase/erc20): A complete ERC-20 token walkthrough

## Introductory examples

These top-level example files cover the fundamentals of Edge syntax:

| Example | What it covers |
|---------|----------------|
| `examples/counter.edge` | `abi`, `contract`, `&s` storage pointers, `pub fn` visibility |
| `examples/erc20.edge` | `event`, `indexed`, `map<K,V>` storage mappings, `emit` |
| `examples/expressions.edge` | Arithmetic, comparison, and bitwise operators |
| `examples/types.edge` | Primitive types (`u8`..`u256`, `i128`, `b32`, `addr`, `bool`, `bit`), data locations (`&s`, `&t`, `&m`), `type` aliases, `const` |
| `examples/transient.edge` | Transient storage (`&t`) with EIP-1153 TLOAD/TSTORE |

## Token standards

Full and partial implementations of ERC token standards:

| File | Standard |
|------|----------|
| `examples/tokens/erc20.edge` | ERC-20 airdrop contract with `mod`/`use` imports |
| `examples/tokens/erc721.edge` | ERC-721 NFT collection with stdlib trait usage |
| `examples/tokens/erc4626.edge` | ERC-4626 tokenized vault with multi-module composition |
| `std/tokens/erc20.edge` | Full ERC-20 reference implementation (solmate pattern) |
| `std/tokens/erc721.edge` | ERC-721 base contract with `abi`/`trait` definitions |
| `std/tokens/erc1155.edge` | ERC-1155 multi-token standard |
| `std/tokens/weth.edge` | Wrapped ETH with `@caller()`, `@callvalue()` builtins |

## Library primitives

Foundational modules imported by other examples:

| File | Purpose |
|------|---------|
| `std/math.edge` | WAD/RAY fixed-point math, safe arithmetic |
| `std/auth.edge` | Ownership traits (`IOwned`, `IAuth`) and contracts (`Owned`, `Auth`) |
| `std/tokens/safe_transfer.edge` | Safe ERC-20 and ETH transfer helpers |

## Utility libraries

Stateless utility functions:

| File | Purpose |
|------|---------|
| `std/utils/merkle.edge` | Merkle proof verification with fixed arrays and bitwise ops |
| `std/utils/bits.edge` | Bit manipulation: popcount, leading/trailing zeros |
| `std/utils/bytes.edge` | Bytes32 utilities: address extraction, packing, masking |

## Access control

| File | Pattern |
|------|---------|
| `std/access/ownable.edge` | Single-owner with 2-step transfer, `trait`/`impl` |
| `std/access/roles.edge` | Role-based access control with nested `map<b32, map<addr, bool>>` |
| `std/access/pausable.edge` | Pausable pattern with `bool` state machine |

## Finance / DeFi

| File | Pattern |
|------|---------|
| `std/finance/amm.edge` | Constant product AMM (x · y = k) |
| `std/finance/staking.edge` | ERC-20 staking with per-second rewards |
| `std/finance/multisig.edge` | N-of-M multisig with sum types and `match` |

## Design patterns

| File | Pattern |
|------|---------|
| `std/patterns/reentrancy_guard.edge` | Reentrancy protection with `&t` transient storage |
| `std/patterns/timelock.edge` | Time-locked operations with sum types carrying data |
| `std/patterns/factory.edge` | CREATE2 deterministic deployment factory |

## Type system deep dives

Dedicated examples for each major type system feature:

| File | Feature |
|------|---------|
| `examples/types/structs.edge` | Product types: structs, packed structs, tuples, generic structs |
| `examples/types/enums.edge` | Sum types: enums, unions with data, `Option<T>`, `Result<T>` |
| `examples/types/generics.edge` | Generics, trait constraints, monomorphization |
| `examples/types/arrays.edge` | Fixed arrays, packed arrays, slices, iteration |
| `examples/types/comptime.edge` | Compile-time evaluation: `const`, `comptime fn` |
