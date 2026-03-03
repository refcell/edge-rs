# edge-rs

[![CI]][actions]
[![License]][mit-license]
[![Language]][edge-lang]
[![EVM]][evm-link]
[![Rust]][rust-link]

[CI]: https://img.shields.io/github/actions/workflow/status/refcell/edge-rs/ci.yml?branch=main&label=ci
[actions]: https://github.com/refcell/edge-rs/actions?query=branch%3Amain
[License]: https://img.shields.io/badge/license-MIT-7795AF.svg
[mit-license]: https://github.com/refcell/edge-rs/blob/main/LICENSE.md
[Language]: https://img.shields.io/badge/language-edge-39ff14.svg
[edge-lang]: https://edgelang.netlify.app/
[EVM]: https://img.shields.io/badge/target-EVM-39ff14.svg
[evm-link]: https://ethereum.org/en/developers/docs/evm/
[Rust]: https://img.shields.io/badge/built%20with-rust-orange.svg
[rust-link]: https://www.rust-lang.org/

**edge-rs** is a compiler for the [Edge language](https://edgelang.netlify.app/) — an EVM-targeted DSL for writing smart contracts with a Rust-like type system.

**[Install](#install)**
| [Examples](#examples)
| [Lore](#lore)
| [Contributing](#contributing)
| [License](#license)

## What is Edge?

Edge is a statically-typed, Rust-inspired language that compiles to EVM bytecode. It gives you EVM-native integer and byte types (`u8` through `u256`, `i8` through `i256`, `b1` through `b32`), along with `addr`, `bool`, and `bit` as first-class primitives. Every variable carries an explicit data location annotation — `&s` for storage, `&t` for transient storage, `&m` for memory, `&cd` for calldata — so the compiler always knows exactly where your data lives and can enforce that statically. Contracts and ABIs are declared at the language level rather than bolted on. The type system supports traits, generics, and pattern matching over union types, and a `comptime` keyword lets you run arbitrary code at compile time to generate constants or specialized implementations without runtime cost.

## Install

Install the Edge toolchain via `edgeup`:

```sh
curl -fsSL https://raw.githubusercontent.com/refcell/edge-rs/main/etc/install.sh | sh
```

Or build from source:

```sh
cargo install --path bin/edgec
```

## Examples

See the [`examples/`](./examples/) directory for complete Edge programs.

```sh
# Lex a file (print tokens)
edgec lex examples/counter.edge

# Parse a file (print AST)
edgec parse examples/counter.edge

# Compile a file
edgec build examples/counter.edge
```

## Lore

Edge was conceived by [jtriley](https://github.com/jtriley-eth), an Ethereum developer and EVM language researcher who had spent years studying the design space of smart contract languages. In November 2023 he published ["The Edge Programming Language"](https://jtriley.substack.com/p/the-edge-programming-language) on his Substack, laying out both a diagnosis and a proposed cure.

The diagnosis was blunt. A 2021 Trail of Bits report found that roughly ninety percent of all deployed EVM smart contracts share at least fifty-six percent of their bytecode with other contracts, a sign that the abstraction mechanisms available to developers were badly broken. Teams were copy-pasting instead of composing. The existing languages either gave you Solidity's implicit compiler decisions and sprawling inheritance graphs, or they gave you [Huff](https://huff.sh)'s raw opcodes with no type safety at all. Nothing in between let an experienced engineer write genuinely reusable, auditable, low-overhead code without reaching for assembly.

jtriley had spent significant time inside the Huff ecosystem alongside [refcell](https://github.com/refcell), [clabby](https://github.com/clabby), and others who were core contributors to [huff-rs](https://github.com/huff-language/huff-rs) and the broader [huff-language](https://github.com/huff-language) organization. That work — building [huffmate](https://github.com/huff-language/huffmate), writing optimized contracts directly in opcodes, and pushing up against everything Huff could not express — gave the group a precise understanding of where the abstraction floor needed to be and what it felt like to bump into a language ceiling on production code.

jtriley had also written extensively about this gap, comparing Huff and Yul, cataloguing the ways Solidity quietly allocates memory as if garbage collection exists when it does not, and documenting how its type system makes it hard to transfer assets without inline assembly. His conclusion was that the field had become trapped in what he called status quo ossification: new languages kept arriving as Solidity lookalikes chasing gas efficiency, without offering anything fundamentally new.

Edge was his attempt to start from first principles. The goal was not to replace Solidity for beginners but to give experienced teams a language that combined the granularity of Huff with the type system and compile-time execution of a high-level language. That combination, jtriley argued, would unlock constructs that no existing smart contract language could express at all — type-checked SSTORE2 implementations, in-memory hash maps, compressed ABI encoders, elliptic curve types, nested virtual machines with zero stack overhead. The key insight was that explicit data location annotations for all seven EVM storage areas, paired with parametric polymorphism and a trait system, would let the type checker enforce correctness that developers currently had to maintain by hand or discover through audit.

jtriley presented Edge at Solidity Summit 2023 alongside contributors from other EVM language projects, and the specification has been available at [edge-specification.vercel.app](https://edge-specification.vercel.app/) since the announcement. edge-rs is the Rust implementation of that compiler.

## Contributing

All contributions are welcome. Open an issue or pull request on [GitHub](https://github.com/refcell/edge-rs).

## License

This project is licensed under the [MIT License](LICENSE.md).
