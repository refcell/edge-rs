---
title: ERC20
---

# ERC20

A complete ERC-20 fungible token in Edge. This page covers two implementations:

1. **`examples/erc20.edge`** — a minimal, self-contained ERC-20 (shown in the [Complete contract](#complete-contract) section and used for the walkthrough below).
2. **`std/tokens/erc20.edge`** — the full standard library implementation with metadata getters, the `MAX_UINT` infinite-approval pattern, and `mod`/`use` imports.

Source files: [`examples/erc20.edge`](https://github.com/refcell/edge-rs/blob/main/examples/erc20.edge) · [`std/tokens/erc20.edge`](https://github.com/refcell/edge-rs/blob/main/std/tokens/erc20.edge)

:::note
The walkthrough below uses code from `examples/erc20.edge` unless explicitly noted otherwise. Where the standard library version differs, the difference is called out.
:::

---

## Events

Events are declared at the top level with the `event` keyword. Fields marked `indexed` appear in log topics and are filterable on-chain.

```edge
event Transfer(indexed from: addr, indexed to: addr, amount: u256);
event Approval(indexed owner: addr, indexed spender: addr, amount: u256);
```

- `Transfer` — emitted on every token movement, including mint (`from = 0`) and burn (`to = 0`).
- `Approval` — emitted when an owner sets a spender's allowance.

---

## External interface

The `abi` block defines the ERC-20 public interface.

```edge
abi IERC20 {
    fn totalSupply() -> (u256);
    fn balanceOf(account: addr) -> (u256);
    fn transfer(to: addr, amount: u256) -> (bool);
    fn allowance(owner: addr, spender: addr) -> (u256);
    fn approve(spender: addr, amount: u256) -> (bool);
    fn transferFrom(from: addr, to: addr, amount: u256) -> (bool);
}
```

:::note
The standard library version (`std/tokens/erc20.edge`) extends this ABI with `fn name() -> (b32)`, `fn symbol() -> (b32)`, and `fn decimals() -> (u8)`.
:::

---

## Contract and storage layout

The `contract` block holds persistent storage fields and all function definitions. Storage fields use the `&s` (storage pointer) qualifier.

```edge
contract ERC20 {
    const DECIMALS: u8 = 18;

    let name: &s b32;
    let symbol: &s b32;
    let total_supply: &s u256;
    let balances: &s map<addr, u256>;
    let allowances: &s map<addr, map<addr, u256>>;

    // ... functions follow
}
```

**Edge-specific features here:**
- `&s` — marks a field as contract storage (persists on-chain).
- `map<K, V>` — Edge's built-in mapping type, equivalent to Solidity's `mapping(K => V)`.
- `map<addr, map<addr, u256>>` — nested mapping for two-dimensional allowance lookup.
- `b32` — a 32-byte value, used for compact string storage (name, symbol).
- `const` — compile-time constant, not stored on-chain.

:::note
The standard library version also declares `const MAX_UINT: u256 = 0xfff...fff;` as a sentinel for infinite allowances (solmate pattern).
:::

---

## Public read functions

```edge
pub fn totalSupply() -> (u256) {
    return total_supply;
}

pub fn balanceOf(account: addr) -> (u256) {
    return balances[account];
}

pub fn allowance(owner: addr, spender: addr) -> (u256) {
    return allowances[owner][spender];
}
```

Map fields are accessed with `map[key]` syntax. Nested maps use chained brackets: `allowances[owner][spender]`.

---

## Transfer

`transfer` moves tokens from the caller to a recipient. The caller's address is retrieved via `@caller()`.

```edge
pub fn transfer(to: addr, amount: u256) -> (bool) {
    let from: addr = @caller();
    _transfer(from, to, amount);
    return true;
}
```

The actual balance update and event emission are delegated to the internal `_transfer` helper.

---

## Approve

`approve` sets how many tokens a spender may transfer on the caller's behalf.

```edge
pub fn approve(spender: addr, amount: u256) -> (bool) {
    let owner: addr = @caller();
    _approve(owner, spender, amount);
    return true;
}
```

---

## TransferFrom

`transferFrom` lets an approved spender move tokens from another account. It checks and decrements the allowance, then calls `_transfer`.

```edge
pub fn transferFrom(from: addr, to: addr, amount: u256) -> (bool) {
    let caller: addr = @caller();
    let current_allowance: u256 = allowances[from][caller];
    allowances[from][caller] = current_allowance - amount;
    _transfer(from, to, amount);
    return true;
}
```

:::note
The standard library version (`std/tokens/erc20.edge`) adds the solmate infinite-approval optimization: if `current_allowance == MAX_UINT`, the allowance is not decremented, saving a storage write.
:::

---

## Internal helpers

Internal functions (no `pub`) are only callable from within the contract.

### `_transfer`

```edge
fn _transfer(from: addr, to: addr, amount: u256) {
    let from_balance: u256 = balances[from];
    balances[from] = from_balance - amount;
    balances[to] = balances[to] + amount;
    emit Transfer(from, to, amount);
}
```

Subtraction reverts on underflow — no explicit balance check needed. Events are emitted with `emit EventName(args...)`.

### `_approve`

```edge
fn _approve(owner: addr, spender: addr, amount: u256) {
    allowances[owner][spender] = amount;
    emit Approval(owner, spender, amount);
}
```

### `_mint`

```edge
fn _mint(to: addr, amount: u256) {
    total_supply = total_supply + amount;
    balances[to] = balances[to] + amount;
    emit Transfer(0, to, amount);
}
```

Mint is represented as a transfer from the zero address (`0`).

### `_burn`

```edge
fn _burn(from: addr, amount: u256) {
    balances[from] = balances[from] - amount;
    total_supply = total_supply - amount;
    emit Transfer(from, 0, amount);
}
```

Burn is represented as a transfer to the zero address (`0`). Underflow on `balances[from]` provides implicit balance enforcement.

---

## Complete contract

The full minimal ERC-20 from `examples/erc20.edge`:

```edge
event Transfer(indexed from: addr, indexed to: addr, amount: u256);
event Approval(indexed owner: addr, indexed spender: addr, amount: u256);

abi IERC20 {
    fn totalSupply() -> (u256);
    fn balanceOf(account: addr) -> (u256);
    fn transfer(to: addr, amount: u256) -> (bool);
    fn allowance(owner: addr, spender: addr) -> (u256);
    fn approve(spender: addr, amount: u256) -> (bool);
    fn transferFrom(from: addr, to: addr, amount: u256) -> (bool);
}

contract ERC20 {
    const DECIMALS: u8 = 18;

    let name: &s b32;
    let symbol: &s b32;
    let total_supply: &s u256;
    let balances: &s map<addr, u256>;
    let allowances: &s map<addr, map<addr, u256>>;

    pub fn totalSupply() -> (u256) {
        return total_supply;
    }

    pub fn balanceOf(account: addr) -> (u256) {
        return balances[account];
    }

    pub fn transfer(to: addr, amount: u256) -> (bool) {
        let from: addr = @caller();
        _transfer(from, to, amount);
        return true;
    }

    pub fn allowance(owner: addr, spender: addr) -> (u256) {
        return allowances[owner][spender];
    }

    pub fn approve(spender: addr, amount: u256) -> (bool) {
        let owner: addr = @caller();
        _approve(owner, spender, amount);
        return true;
    }

    pub fn transferFrom(from: addr, to: addr, amount: u256) -> (bool) {
        let caller: addr = @caller();
        let current_allowance: u256 = allowances[from][caller];
        allowances[from][caller] = current_allowance - amount;
        _transfer(from, to, amount);
        return true;
    }

    fn _transfer(from: addr, to: addr, amount: u256) {
        let from_balance: u256 = balances[from];
        balances[from] = from_balance - amount;
        balances[to] = balances[to] + amount;
        emit Transfer(from, to, amount);
    }

    fn _approve(owner: addr, spender: addr, amount: u256) {
        allowances[owner][spender] = amount;
        emit Approval(owner, spender, amount);
    }

    fn _mint(to: addr, amount: u256) {
        total_supply = total_supply + amount;
        balances[to] = balances[to] + amount;
        emit Transfer(0, to, amount);
    }

    fn _burn(from: addr, amount: u256) {
        balances[from] = balances[from] - amount;
        total_supply = total_supply - amount;
        emit Transfer(from, 0, amount);
    }
}
```

---

## Edge syntax summary

| Feature | Edge syntax | Notes |
|---|---|---|
| Storage field | `let x: &s T` | `&s` makes it persistent |
| Mapping | `map<K, V>` | Indexed with `map[key]` |
| Nested mapping | `map<K, map<K2, V>>` | Accessed as `map[k1][k2]` |
| Caller address | `@caller()` | Built-in context accessor |
| Emit event | `emit Transfer(from, to, amount)` | Positional args match declaration |
| Event declaration | `event Transfer(indexed from: addr, ...)` | `indexed` fields go into log topics |
| ABI definition | `abi IERC20 { fn ... }` | Defines external call interface |
| Public function | `pub fn name() -> (b32)` | Callable externally |
| Internal function | `fn _transfer(...)` | No `pub`, contract-internal only |
| Constant | `const DECIMALS: u8 = 18` | Compile-time, not stored |
