---
title: Inline assembly
---

# Inline assembly

Edge supports inline EVM assembly for low-level control when the high-level
language abstractions are insufficient.

## Opcodes

The following EVM opcodes are accepted in inline assembly blocks. Opcode names
are case-insensitive.

**Arithmetic and logic:**
`stop`, `add`, `mul`, `sub`, `div`, `sdiv`, `mod`, `smod`, `addmod`, `mulmod`,
`exp`, `signextend`, `lt`, `gt`, `slt`, `sgt`, `eq`, `iszero`, `and`, `or`,
`xor`, `not`, `byte`, `shl`, `shr`, `sar`

**Cryptographic:**
`keccak256` (alias: `sha3`)

**Environment:**
`address`, `balance`, `origin`, `caller`, `callvalue`, `calldataload`,
`calldatasize`, `calldatacopy`, `codesize`, `codecopy`, `gasprice`,
`extcodesize`, `extcodecopy`, `returndatasize`, `returndatacopy`,
`extcodehash`

**Block:**
`blockhash`, `coinbase`, `timestamp`, `number`, `prevrandao` (alias:
`difficulty`), `gaslimit`, `chainid`, `selfbalance`, `basefee`, `blobhash`,
`blobbasefee`

**Stack, memory, and storage:**
`pop`, `mload`, `mstore`, `mstore8`, `sload`, `sstore`, `tload`, `tstore`,
`mcopy`

**Flow control:**
`jump`, `jumpi`, `pc`, `msize`, `gas`, `jumpdest`

**Push:**
`push0`, `push1` through `push32`

**Duplication:**
`dup1` through `dup16`

**Exchange:**
`swap1` through `swap16`

**Logging:**
`log0`, `log1`, `log2`, `log3`, `log4`

**System:**
`create`, `call`, `callcode`, `return`, `delegatecall`, `create2`,
`staticcall`, `revert`, `invalid`, `selfdestruct`

In addition to mnemonics, numeric literals and identifiers are accepted (see
grammar below).

### Grammar

```
<opcode> ::=
    <evm_mnemonic>
  | <numeric_literal>
  | <ident> ;
```

Where `<evm_mnemonic>` is any of the opcodes listed above.

## Inline assembly block

```
<assembly_output> ::= <ident> | "_" ;

<inline_assembly> ::=
    "asm"
    "(" [<expr> ("," <expr>)* [","]] ")"
    ["->" "(" [<assembly_output> ("," <assembly_output>)* [","]] ")"]
    "{" (<opcode>)* "}" ;
```

The `<inline_assembly>` consists of the `asm` keyword, followed by a
parenthesized, comma-separated list of input expressions, an optional
`-> (...)` clause listing output names, and a code block containing opcodes.
The entire `-> (...)` clause may be omitted when no outputs are needed.

## Semantics

Arguments are ordered such that the state of the stack at the start of the
block, top to bottom, is the list of arguments, left to right. Identifiers in
the output list are ordered such that the state of the stack at the end of the
assembly block, top to bottom, is the list of outputs, left to right.

```edge
asm (1, 2, 3) -> (a) {
    // stack: [1, 2, 3]
    add         // [3, 3]
    mul         // [9]
}
```

### Numeric literals

Inside the assembly block, numeric literals are implicitly converted into
`PUSH{N}` instructions. Literals are encoded in the smallest `N` by value,
except that leading zeros in hex literals are preserved. For example, `0x0000`
becomes `PUSH2 0x0000` to allow for bytecode padding.

### Identifiers

Identifiers in the assembly body can be:

- **Variables** — resolved to their stack position (scheduled by the compiler).
  Only compile-time constants and stack-allocated variables are supported;
  memory-backed variables must be passed as input arguments.
- **Constants** — replaced with their `PUSH{N}` encoding, same as numeric
  literals.
- **Opcode names** — treated as the corresponding EVM instruction (case-insensitive).

### Outputs

- **Named outputs** (e.g., `a`) are bound as local variables accessible in
  subsequent code.
- **Discarded outputs** (`_`) are popped from the stack.
- **Multiple outputs** (N > 1) are stored to sequential memory slots internally
  and bound as `LetBind` variables via `MLOAD`.

### IR representation

Inline assembly compiles to an `InlineAsm(inputs, hex_bytecode, num_outputs)`
IR node. This node is opaque to the egglog optimizer — it passes through
equality saturation unchanged.

:::note
If the input arguments contain local variables, the stack scheduling required
to construct the pre-assembly stack state may be unprofitable for small assembly
blocks. Consider passing values as immediate literals when possible.
:::
