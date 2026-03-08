---
title: Basics
---

# Basics

This page walks through the fundamental building blocks of Edge: contracts, storage, functions, types, expressions, and transient storage. All code snippets are exact copies from files in [`examples/`](https://github.com/refcell/edge-rs/tree/main/examples).

## A simple counter contract

From `examples/counter.edge` — the simplest possible contract: an on-chain counter.

```edge
abi ICounter {
    fn increment();
    fn decrement();
    fn get() -> (u256);
    fn reset();
}

contract Counter {
    // Storage slot for the counter value
    let count: &s u256;

    // Increment the counter by 1
    pub fn increment() {
        let val: u256 = count + 1;
        count = val;
    }

    // Decrement the counter by 1 (saturating at 0)
    pub fn decrement() {
        let val: u256 = count - 1;
        count = val;
    }

    // Return the current count
    pub fn get() -> (u256) {
        return count;
    }

    // Reset the counter to zero
    pub fn reset() {
        count = 0;
    }
}
```

:::warning
The `decrement` function performs `count - 1` with no underflow guard. On the EVM, unsigned subtraction past zero reverts — the counter does **not** saturate at 0. The upstream source comment is misleading.
:::

Key points:

- `let count: &s u256` declares a **persistent storage** field with the `&s` (storage) annotation.
- `pub fn` marks a function as publicly callable. Inside a `contract` block, `pub fn` implicitly creates a dispatch entry (equivalent to `pub ext fn`).
- Return types use parentheses: `-> (u256)`.
- The `abi` block declares the external interface; the `contract` block implements it.

## Primitive types and data locations

From `examples/types.edge` — a tour of all primitive types and data location annotations:

```edge
// Primitive types
let a: u8;
let b: u256;
let c: i128;
let d: b32;
let e: addr;
let f: bool;
let g: bit;

// Data location annotations
let stored: &s u256;        // storage
let transient_val: &t u256; // transient storage (EIP-1153)
let in_memory: &m u256;     // memory

// Type aliases
type TokenId = u256;
type Owner = addr;
type Balance = u256;

// Constants
const ZERO: u256 = 0;
const MAX_SUPPLY: u256 = 1000000;
```

| Annotation | Opcodes | Lifetime |
|------------|---------|----------|
| `&s` | SLOAD / SSTORE | Persists across transactions |
| `&t` | TLOAD / TSTORE (EIP-1153) | Cleared after each transaction |
| `&m` | MLOAD / MSTORE | Within execution only |

## Expressions and operators

From `examples/expressions.edge` — all operator categories:

```edge
// --- Arithmetic ---

fn arithmetic(a: u256, b: u256) -> (u256) {
    let sum: u256 = a + b;
    let diff: u256 = a - b;
    let product: u256 = a * b;
    let quotient: u256 = a / b;
    let remainder: u256 = a % b;
    let power: u256 = a ** b;

    return sum;
}

// --- Comparison and logic ---

fn comparisons(x: u256, y: u256) -> (bool) {
    let eq: bool = x == y;
    let lt: bool = x < y;
    let gt: bool = x > y;

    return eq;
}

// --- Bitwise ---

fn bitwise(x: u256, y: u256) -> (u256) {
    let and_result: u256 = x & y;
    let or_result: u256 = x | y;
    let xor_result: u256 = x ^ y;
    let shifted: u256 = x << 2;

    return and_result;
}

// --- Nested expressions ---

fn complex(a: u256, b: u256, c: u256) -> (u256) {
    return a + b * c;
}
```

:::note
These are free functions (not inside a `contract` block). Edge supports top-level function definitions.
:::

## Transient storage

From `examples/transient.edge` — transient storage (`&t`) is erased at the end of every transaction (EIP-1153). It uses `TLOAD`/`TSTORE` opcodes (100 gas each) and is useful for reentrancy locks and within-transaction caches.

```edge
contract ReentrancyGuard {
    // Transient lock — cleared automatically after each tx
    let locked: &t u256;

    // Persistent counter
    let count: &s u256;

    // Set the transient lock
    pub fn enter() {
        locked = 1;
    }

    // Clear the transient lock
    pub fn exit() {
        locked = 0;
    }

    // Read the lock state
    pub fn get_locked() -> (u256) {
        return locked;
    }

    // Increment the persistent counter
    pub fn increment() {
        count = count + 1;
    }

    // Read the persistent counter
    pub fn get_count() -> (u256) {
        return count;
    }
}
```

Mix `&s` and `&t` in the same contract as needed. The compiler generates the appropriate opcodes for each.

## Built-in globals

Two commonly used EVM builtins:

| Builtin | Type | Description |
|---------|------|-------------|
| `@caller()` | `addr` | Address of the immediate caller (`msg.sender`) |
| `@callvalue()` | `u256` | ETH value sent with the call (`msg.value`) |

The `@` prefix distinguishes built-in context accessors from regular function calls.

## Quick reference

| Concept | Syntax |
|---------|--------|
| Persistent storage field | `let x: &s u256;` |
| Transient storage field | `let x: &t u256;` |
| Memory field | `let x: &m u256;` |
| Type alias | `type TokenId = u256;` |
| Constant | `const ZERO: u256 = 0;` |
| Public function | `pub fn name() { ... }` |
| Function with return | `pub fn get() -> (u256) { return x; }` |
| ABI definition | `abi IName { fn foo() -> (u256); }` |
| Caller address | `@caller()` |
| ETH value sent | `@callvalue()` |
