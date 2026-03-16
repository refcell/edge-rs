---
title: Standard Library
---

# Standard Library

The Edge standard library provides traits, types, and data structures
that are automatically available or importable via `use std::*`.

## Operator Traits (`std::ops`)

These traits enable operator syntax for user-defined types. Import them
with `use std::ops::TraitName;` and implement them on your type.

### Arithmetic

```edge
trait Add {
    fn add(self, rhs: Self) -> (Self);
}

trait Sub {
    fn sub(self, rhs: Self) -> (Self);
}

trait Mul {
    fn mul(self, rhs: Self) -> (Self);
}

trait Div {
    fn div(self, rhs: Self) -> (Self);
}

trait Mod {
    fn mod_(self, rhs: Self) -> (Self);
}
```

When implemented, these traits dispatch from the corresponding binary
operators (`+`, `-`, `*`, `/`, `%`). For primitive types, the compiler
provides built-in implementations with checked overflow behavior.

### Unchecked Arithmetic

```edge
trait UnsafeAdd {
    fn unsafe_add(self, rhs: Self) -> (Self);
}

trait UnsafeSub {
    fn unsafe_sub(self, rhs: Self) -> (Self);
}

trait UnsafeMul {
    fn unsafe_mul(self, rhs: Self) -> (Self);
}
```

These bypass overflow and underflow checks. The compiler provides
built-in implementations for all primitive integer types. Use these
in performance-critical code where overflow is provably impossible
(e.g., internal pointer arithmetic).

### Comparison

```edge
trait Eq {
    fn eq(self, rhs: Self) -> (bool);
}

trait Ord {
    fn lt(self, rhs: Self) -> (bool);
    fn gt(self, rhs: Self) -> (bool);
    fn le(self, rhs: Self) -> (bool);
    fn ge(self, rhs: Self) -> (bool);
}
```

`Eq` dispatches from `==`. `Ord` dispatches from `<`, `>`, `<=`, `>=`.

### Indexing

```edge
trait Index<Idx, Output> {
    fn index(self, index: Idx) -> (Output);
}
```

Dispatches from the `[]` operator. `Idx` is the key/index type,
`Output` is the return type. Used by `Vec<T>` and `Map<K, V>`.

## Storage & Memory Traits (`std::ops`)

These traits control how types are stored to and loaded from different
data locations. The compiler provides built-in implementations for
primitive types.

### Storage

```edge
trait Sstore {
    fn sstore(self, base_slot: u256);
}

trait Sload {
    fn sload(base_slot: u256) -> Self;
}
```

Control how values are written to (`SSTORE`) and read from (`SLOAD`)
persistent storage. `Map<K, V>` overrides `Sload` to return the slot
itself (identity), enabling nested map composition.

### Slot Derivation

```edge
trait UniqueSlot {
    fn derive_slot(self, base_slot: u256) -> u256;
}
```

Derives a storage slot from a key and a base slot. Required for `Map`
keys. The compiler provides default implementations for primitive types
using `keccak256(abi.encode(key, base_slot))`, but users can implement
this trait on their own types to define custom slot derivation logic.

### Memory

```edge
trait Mstore {
    fn mstore(self, offset: u256);
}

trait Mload {
    fn mload(offset: u256) -> Self;
}

trait Mcopy {
    fn mcopy(self, dest: u256, size: u256);
}
```

Control how values interact with EVM memory. Used internally by `Vec<T>`
and other memory-backed data structures.

## Generic Types

### `Option<T>`

```edge
type Option<T> = None | Some(T);
```

A sum type representing an optional value. `None` indicates absence,
`Some(T)` wraps a present value.

### `Result<T, E>`

```edge
type Result<T, E> = Ok(T) | Err(E);
```

A sum type for fallible operations. `Ok(T)` wraps a success value,
`Err(E)` wraps an error value.

### `Vec<T>`

```edge
type Vec<T> = { len: u256, capacity: u256 };
```

A dynamically-allocated, growable array backed by `&dm` (dynamic memory).

**Memory layout** (contiguous):
```
[len (32 bytes), capacity (32 bytes), elem0, elem1, ...]
 ^--- pointer
```

**Construction**:
```edge
let v: &dm Vec<u256> = Vec::new(4);  // initial capacity of 4
```

**Methods**:

| method                     | description                                 |
|----------------------------|---------------------------------------------|
| `Vec::new(cap) -> u256`   | Allocate a new Vec with initial capacity    |
| `v.len() -> u256`         | Current number of elements                  |
| `v.capacity() -> u256`    | Current allocated capacity                  |
| `v.push(val)`             | Append element, growing if needed           |
| `v.pop() -> T`            | Remove and return last element (reverts if empty) |
| `v.get(index) -> T`       | Read element at index (reverts if out of bounds)  |
| `v.set(index, val)`       | Write element at index (reverts if out of bounds) |
| `v.grow(new_cap)`         | Reallocate to larger capacity               |

**Index trait**: `Vec<T>` implements `Index<u256, T>`, so `v[i]` is
equivalent to `v.get(i)`.

**Growth**: When `push` exceeds capacity, `grow` allocates a new region
via `@alloc`, copies existing data with `MCOPY`, and transparently
updates the caller's pointer via `&dm` aliasing.

### `Map<K, V>`

```edge
type Map<K: UniqueSlot, V: Sload & Sstore> = ();
```

A storage mapping type. At runtime, a `Map` is just a `u256` representing
its base storage slot — it is a zero-storage type with no runtime overhead.

**Trait bounds**: Keys must implement `UniqueSlot` (for slot derivation
via keccak256). Values must implement `Sload` and `Sstore`.

**Usage**:
```edge
contract MyContract {
    let balances: &s Map<address, u256>;
    let allowances: &s Map<address, Map<address, u256>>;

    pub fn get_balance(owner: address) -> u256 {
        self.balances.get(owner)
    }

    pub fn set_balance(owner: address, val: u256) {
        self.balances.set(owner, val);
    }

    pub fn get_allowance(owner: address, spender: address) -> u256 {
        self.allowances[owner][spender]
    }
}
```

**Methods**:

| method                  | description                        |
|-------------------------|------------------------------------|
| `m.get(key) -> V`      | Derive slot from key and SLOAD     |
| `m.set(key, val)`      | Derive slot from key and SSTORE    |

**Index trait**: `Map<K, V>` implements `Index<K, V>`, so `m[key]` is
equivalent to `m.get(key)`.

**Nested maps**: `Map<K, Map<K2, V>>` works because `Map` implements
`Sload` as identity — "loading" an inner Map just passes through the
derived slot without an actual `SLOAD`. This means `m[k1][k2]` performs
exactly one `SLOAD` (at the leaf), with two keccak256 slot derivations.

**Slot derivation**: For a key `k` and base slot `s`, the storage slot
is `keccak256(abi.encode(k, s))`. This matches the Solidity mapping
layout convention.
