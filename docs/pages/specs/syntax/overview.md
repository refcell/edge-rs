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
