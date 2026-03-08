---
title: Syntax
---

# Syntax

Conceptually, all EVM contracts are single-entry point executables and at compile time,
Edge programs are no different.

Other languages have used primarily the contract-is-an-object paradigm, mapping fields to
storage layouts and methods to "external functions" that may read and write the storage.
Inheritance enables interface constraints, code reuse, and a reasonable model for message
passing that relates to the EVM external call model.

However, this is limited in scope. Conceptually, the contract object paradigm groups stateful
data and functionality, limiting the deployability to the product type. Extending the
deployability to arbitrary data types allows for contracts to be functions, type unions,
product types, and more. While most of these are not particularly useful, this simplifies the
type system as well as opens the design space to new contract paradigms.

The core syntax of Edge is derived from commonly used patterns in modern programming. Functions,
branches, and loops are largely intuitive for engineers with experience in C, Rust, Javascript,
etc. Parametric polymorphism uses syntax similar to Rust and Typescript. Compiler built-in
functions and "comptime" constructs follow the syntax of Zig.

## Top-level items

An Edge source file is a sequence of top-level declarations. The following item kinds are
supported at the top level:

| Keyword | Form | Purpose |
|---------|------|---------|
| `contract` | `contract Name { … }` | Contract definition |
| `fn` | `fn name(…) [-> T] { … }` | Free function |
| `const` | `const NAME[: T] = expr;` | Compile-time constant |
| `let` | `let [mut] name[: T] [= expr];` | Variable declaration |
| `type` | `type Name[<T>] = …;` | Type alias or union type |
| `trait` | `trait Name[<T>] { … }` | Trait definition |
| `impl` | `impl Type[:Trait] { … }` | Implementation block |
| `abi` | `abi Name { … }` | ABI interface declaration |
| `event` | `event Name(…);` | Event declaration |
| `mod` | `mod name;` / `mod name { … }` | Module declaration |
| `use` | `use root::path;` | Module import |

Functions and declarations may be prefixed with `pub` (public visibility). See the
sub-pages for the full grammar of each item kind.

## Keywords

Edge reserves the following 33 keywords:

**Declaration:** `contract`, `type`, `const`, `fn`, `packed`, `trait`, `impl`, `mod`, `use`, `abi`, `event`

**Modifiers:** `pub`, `mut`, `ext`, `indexed`, `anon`, `comptime`

**Control flow:** `return`, `if`, `else`, `match`, `matches`, `for`, `while`, `loop`, `do`, `break`, `continue`

**Variables / scope:** `let`, `Self`, `super`

**Side effects / assembly:** `emit`, `asm`
