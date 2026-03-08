---
title: ABI
---

# ABI

The application binary interface is both a construct to generate a JSON ABI
by the compiler and a subtyping construct for contract objects.

## Declaration

```text
<abi_function_declaration> ::=
    ["mut"] "fn" <identifier>
    "(" [(<identifier> ":" <type_signature>) ("," <identifier> ":" <type_signature>)* [","]] ")"
    ["->" ("(" <type_signature> ("," <type_signature>)* [","] ")" | <type_signature>)] ";" ;

<abi_declaration> ::=
    "abi" <identifier> [":" <identifier> ("+" <identifier>)*] "{"
        <abi_function_declaration>*
    "}" ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`

The `<abi_declaration>` maps to `AbiDecl` in the AST:

- `name: Ident`
- `superabis: Vec<Ident>` — parent ABIs for subtyping
- `functions: Vec<AbiFnDecl>`

Each `<abi_function_declaration>` maps to `AbiFnDecl`:

- `name: Ident`
- `params: Vec<(Ident, TypeSig)>`
- `returns: Vec<TypeSig>`
- `is_mut: bool`

:::note
Unlike regular `FnDecl`, `AbiFnDecl` does **not** have `is_pub` or `is_ext`
fields. ABI functions are implicitly external interface declarations — `pub`
and `ext` keywords are not valid inside an ABI block.
:::

The optional `+`-separated list of identifiers after `:` represents
parent ABIs, enabling ABI subtyping. The `+` separator matches the
supertrait syntax on trait declarations.

:::warning
Superabi parsing is not yet implemented in the parser. The `superabis` field
in the AST is always an empty `Vec`. The BNF above reflects the planned syntax.
:::

## Examples

```edge
abi IERC20 {
    fn totalSupply() -> u256;
    fn balanceOf(owner: addr) -> u256;
    mut fn transfer(to: addr, amount: u256) -> bool;
    mut fn approve(spender: addr, amount: u256) -> bool;
}

abi IERC20Metadata : IERC20 {
    fn name() -> u256;
    fn symbol() -> u256;
    fn decimals() -> u8;
}
```

## Semantics

The optional `mut` keyword indicates whether the function will mutate the
state of the smart contract or the EVM. This allows contracts to determine
whether to use the `call` or `staticcall` instruction when interfacing with
a contract conforming to the given ABI.

:::warning
ABI subtyping semantics are still being finalized. It has not yet been decided
whether traits fully subsume this use case.
:::
