---
title: Contract objects
---

# Contract objects

Contract objects serve as an object-like interface to contract constructs.

## Declaration

```text
<contract_field_declaration> ::= "let" <identifier> ":" <type_signature> ";" ;
<contract_const_declaration> ::= "const" <identifier> [":" <type_signature>] "=" <expression> ";" ;
<contract_function_declaration> ::= ["pub"] ["ext"] ["mut"] (<function_assignment> | <function_declaration> ";") ;

<contract_declaration> ::=
    "contract" <identifier> "{"
        <contract_field_declaration>*
        <contract_const_declaration>*
        <contract_function_declaration>*
    "}" ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`
* `<expression>`
* `<function_declaration>`
* `<function_assignment>`

The `<contract_declaration>` maps to `ContractDecl` in the AST:

- `name: Ident`
- `fields: Vec<(Ident, TypeSig)>` — storage fields
- `consts: Vec<(ConstDecl, Expr)>` — contract constants
- `functions: Vec<ContractFnDecl>` — inline functions

Each `<contract_function_declaration>` maps to `ContractFnDecl`:

- `name: Ident`
- `params: Vec<(Ident, TypeSig)>`
- `returns: Vec<TypeSig>`
- `is_pub: bool`, `is_ext: bool`, `is_mut: bool`
- `body: Option<CodeBlock>` — `None` for declaration-only functions

The `pub`, `ext`, and `mut` keywords are **independent** — each sets a
separate boolean flag in the AST.

## Implementation

```text
<contract_impl_block> ::=
    "impl" <identifier> [":" <identifier>] "{"
        (["pub"] ["ext"] ["mut"] <function_assignment>)*
    "}" ;
```

Dependencies:

* `<identifier>`
* `<function_assignment>`

The `<contract_impl_block>` maps to `ContractImpl`:

- `contract_name: Ident`
- `abi_impl: Option<Ident>` — ABI being satisfied
- `functions: Vec<ContractFnDecl>` — all with `body: Some(...)`

If the impl block includes `: AbiName`, it satisfies that ABI's interface.

## Examples

```edge
contract ERC20 {
    let balances: &s map<addr, u256>;
    let totalSupply: &s u256;

    const DECIMALS: u8 = 18;

    pub ext fn decimals() -> u8 {
        return DECIMALS;
    }
}

impl ERC20 : IERC20 {
    pub ext fn totalSupply() -> u256 {
        return self.totalSupply;
    }

    pub ext mut fn transfer(to: addr, amount: u256) -> bool {
        // ...
    }
}
```

## Semantics

The contract object desugars to a single main function and storage layout
with a dispatcher.

Contract field declarations create the storage layout starting at slot zero,
incrementing by one for each field. Fields are never packed; storage packing
may be achieved by declaring contract fields as packed structs or tuples.
Fields annotated with the `&s` (persistent storage) or `&t` (transient
storage, EIP-1153) location receive sequential storage slots. Fields with
other location annotations do not participate in storage slot assignment.

:::warning[Confusing AST naming]
The `&s` location in the AST maps to `Location::Stack` (see `crates/ast/src/ty.rs`).
Despite the enum variant name, this represents **persistent contract storage**
(EVM `SSTORE`/`SLOAD`), not the EVM execution stack. The naming is a historical
artifact and may be renamed in a future refactor.
:::

Contract implementation blocks contain definitions of external functions.
If the impl block includes `: AbiName`, it satisfies that ABI's interface.
The `ext` keyword indicates the function is exposed via the contract's
dispatcher. The `mut` keyword indicates the function may mutate EVM state;
non-`mut` functions may be called with `staticcall`.

### Constructor

The contract compiler generates a separate constructor (init code) that
runs once at deployment time. The constructor body initializes storage
fields and any contract-level constants before the runtime bytecode is
deployed on-chain.

:::warning
It has not yet been decided whether plain types with storage annotations
fully subsume the contract object abstraction. The contract system may be
revised.
:::
