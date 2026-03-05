# Plan: EVM Semantic Testing for Edge Compiler

## Inspiration

Solidity's compiler has "semantic tests" — self-contained test files with an expected-output section:
```
// function(type,type): arg1, arg2 -> expected_return
// ~ emit Event(types): indexed1, indexed2, data
```
They deploy bytecode on an EVM host, execute calls, and assert return values, events, and storage. This is the gold standard for compiler correctness testing.

## Approach

Create a Rust integration test crate (`crates/evm-tests/`) that:
1. Compiles `.edge` files to deployment bytecode using the Edge compiler
2. Deploys that bytecode on **revm** (Rust EVM implementation)
3. Sends ABI-encoded function calls and asserts return values, storage state, and emitted events

This gives us end-to-end correctness testing: source → compiler → bytecode → actual EVM execution → correct behavior.

No Solidity needed. We ARE the compiler — we just need to verify our output runs correctly on an EVM.

---

## Step 1: Create `crates/evm-tests/` crate

New crate with:
- `Cargo.toml` — depends on `revm`, `alloy-primitives`, `alloy-sol-types`, `edge-driver`
- `src/lib.rs` — `EvmTestHost` helper struct

**`EvmTestHost`** provides:
```rust
pub struct EvmTestHost { /* revm::Evm instance */ }

impl EvmTestHost {
    /// Compile an .edge file and deploy the bytecode. Returns deployed address.
    fn deploy_edge(path: &str, opt_level: u8) -> Self;

    /// Call a function by selector + ABI-encoded args. Returns (success, output_bytes).
    fn call(&mut self, selector: [u8; 4], args: &[u8]) -> CallResult;

    /// Call a function by signature string, e.g. "transfer(address,uint256)"
    fn call_fn(&mut self, sig: &str, args: &[u8]) -> CallResult;

    /// Read a storage slot directly
    fn sload(&self, slot: U256) -> U256;

    /// Get emitted logs from the last call
    fn logs(&self) -> &[Log];
}

pub struct CallResult {
    pub success: bool,
    pub output: Vec<u8>,
    pub gas_used: u64,
    pub logs: Vec<Log>,
}
```

---

## Step 2: Counter contract tests

`tests/counter.rs`:

```
deploy counter.edge
call get() -> 0
call increment()
call get() -> 1
call increment()
call get() -> 2
call decrement()
call get() -> 1
call reset()
call get() -> 0
```

Tests cover: deployment, SLOAD, SSTORE, arithmetic, function dispatch.

---

## Step 3: ERC20 contract tests

`tests/erc20.rs`:

```
deploy erc20.edge
call totalSupply() -> 0
call balanceOf(deployer) -> 0

// Test transfer
// (Note: Edge ERC20 doesn't have a mint in constructor,
//  so we'd need to either add one or test with what we have)
call transfer(addr2, 100) -> true  // will underflow since no balance
// ... or we add a public mint to a test-specific .edge file

call approve(addr2, 500) -> true
call allowance(deployer, addr2) -> 500

// Check events emitted
assert last call emitted Approval(deployer, addr2, 500)
```

We'll likely create a `test_erc20.edge` variant that exposes `_mint` publicly so we can seed balances.

---

## Step 4: Optimized vs unoptimized equivalence

For each contract, run the same test sequence at -O0, -O1, -O2 and assert identical return values and storage state. This catches optimizer bugs.

---

## Step 5: Property-based / fuzz tests (stretch)

Use `proptest` or `arbitrary` to generate random:
- Transfer amounts
- Sequences of increment/decrement calls
- Approval/transferFrom sequences

Assert invariants:
- Counter: `get()` always equals number of increments minus decrements
- ERC20: `sum(balances) == totalSupply` after any sequence of transfers

---

## Files to Create/Modify

| File | Change |
|------|--------|
| `crates/evm-tests/Cargo.toml` | **New** — revm, alloy-primitives, alloy-sol-types, edge-driver deps |
| `crates/evm-tests/src/lib.rs` | **New** — EvmTestHost helper |
| `crates/evm-tests/tests/counter.rs` | **New** — Counter semantic tests |
| `crates/evm-tests/tests/erc20.rs` | **New** — ERC20 semantic tests |
| `examples/test_erc20.edge` | **New** (maybe) — ERC20 variant with public mint |
| `Cargo.toml` | Add `crates/evm-tests` to workspace members |

---

## Key Design Decisions

1. **revm** over Foundry/forge: We're a Rust project. revm gives us a native Rust EVM we can call directly in tests — no subprocess, no Solidity dependency, no FFI. It's what Foundry itself uses under the hood.

2. **No Solidity reference**: We don't compare against Solidity output. We test against the EVM specification directly — "does our bytecode do the right thing when executed?" This is more fundamental and doesn't couple us to Solidity's behavior.

3. **Test .edge files directly**: Compile during the test, not from pre-built bytecode. This means test failures point to compiler regressions immediately.

4. **Separate crate**: Keeps the heavy `revm` dependency out of the main compiler. Tests only.

---

## Verification

1. `cargo build -p edge-evm-tests` — compiles
2. `cargo test -p edge-evm-tests` — all semantic tests pass
3. Counter: deploy + all 4 functions verified via EVM execution
4. ERC20: deploy + transfer/approve/allowance verified via EVM execution
5. O0 vs O1 vs O2 produce identical results for same inputs
