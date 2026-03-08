---
title: Specifications
---

# Specifications

## All Edge, no drag.

This document defines Edge, a domain-specific language for the Ethereum Virtual
Machine (EVM).

Edge is a high-level, strongly statically typed, multi-paradigm language. It
provides:

- A thin layer of abstraction over the EVM's instruction set architecture (ISA).
- An extensible polymorphic type system with subtyping.
- First-class support for modules and code reuse.
- Compile-time code execution to fine-tune the compiler's input.

Edge's syntax is similar to Rust and Zig where intuitive, however, the language
is not designed to be a general-purpose language with EVM features as an
afterthought. Rather, it extends the EVM instruction set with a reasonable type
system and syntax sugar over universally understood programming constructs.

### Notation

This specification uses a grammar similar to Extended Backus-Naur Form (EBNF)
with the following rules:

- Non-terminal tokens are wrapped in angle brackets `<ident>`.
- Terminal tokens are wrapped in double quotes `"const"`.
- Optional items are wrapped in brackets `["mut"]`.
- Sequences of zero or more items are wrapped in parentheses and suffixed with a star `("," <ident>)*`.
- Sequences of one or more items are wrapped in parentheses and suffixed with a plus `(<ident>)+`.

In contrast to EBNF, all items are non-atomic: arbitrary whitespace characters
(`\n`, `\t`, `\r`) may surround all tokens unless wrapped with curly braces
`{ "0x" (<hex_digit>)* }`.

Common abbreviations:

- `ident` — identifier
- `expr` — expression
- `stmt` — statement

### Disambiguation

#### Return vs return

The word "return" refers to two different behaviors: returned values from
expressions and the halting return opcode.

When "return" is used, this refers to the values returned from expressions —
the values left on the stack, if any.

When "halting return" is used, this refers to the EVM opcode `RETURN` that
halts execution and returns a value from a slice of memory to the caller of
the current execution context.
