---
title: ABI
---

# ABI

The application binary interface is both a construct to
generate a JSON ABI by the compiler as well as a subtyping
construct for contract objects.

## Declaration

```text
<abi_declaration> ::=
    "abi" <ident> [":" <ident> ("&" <ident>)*] "{"
        (
            ["mut"] <function_declaration> ";"
        )*
    "}" ;
```

Dependencies:

* `<ident>`
* `<function_declaration>`

The `<abi_declaration>` is prefixed with "abi",
followed by its identifier, then an optional
colon and list of ampersand separated identifiers,
and finally a series of zero or more function
declarations optionally prefixed by "mut" and
delimited by curly braces.

## Semantics

The optional "mut" keyword indicates whether the
function will mutate the state of the smart contract
or the EVM. This allows contracts to determine
whether to use the call or staticcall instruction
to interface with a target conforming to the given
ABI.

The optional ampersand separated list of identifiers
represents other ABI identifiers to enable ABI
subtyping.

:::note
Todo: revisit ABI subtyping here and decide whether traits cover the same use
case.
:::
