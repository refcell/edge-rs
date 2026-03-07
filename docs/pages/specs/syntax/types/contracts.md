---
title: Contract Objects
---

# Contract Objects

Contract objects serve as an object-like interface to contract constructs.

## Declaration

```text
<contract_field_declaration> ::= <ident> ":" <type_signature> ;
<contract_declaration> ::=
    "contract" <ident> "{"
        [<contract_field_declaration> ("," <contract_field_declaration>)* [","]]
    "}" ;
```

Dependencies:

* `<ident>`
* `<type_signature>`

The `<contract_field_declaration>` is an identifier and type signature,
separated by a colon.

The `<contract_declaration>` is the contract keyword, followed by its
identifier, followed by a curly brace delimited, comma separated list
of field declarations.

## Implementation

```text
<contract_impl_block> ::=
    "impl" <ident> [":" <ident>] "{"
        (["ext"] ["mut"] <function_declaration>)*
    "}"
```

Dependencies:

* `<ident>`
* `<function_declaration>`

The `<contract_impl_block>` is composed of the "impl" keyword, followed
by its identifier, optionally followed by a colon and abi identifier,
followed by list of function declarations, optionally "ext" and/or "mut",
delimited by curly braces.

## Semantics

The contract object desugars to a single main function and storage
layout with a dispatcher.

Contract field declarations create the storage layout which start at
zero and increment by one for each field. Fields are never packed,
however, storage packing may be achieved by declaring contract fields
as packed structs or tuples.

Contract implementation blocks contain definitions of external
functions in the contract object. If the impl block contains a colon
and identifier, this indicates the impl block is satisfying an abi's
constrained functions. The "ext" keyword indicates the function is
publicly exposed via the contract's dispatcher. The "mut" keyword
indicates the function may mutate the global state in the EVM-sense;
that is to say "mut" functions require a "call" instruction while
those without may use "call" or "staticcall" to interface with the
contract.

:::note
Todo: revisit this section and decide whether plain types cover the same use
case as contract objects.
:::
