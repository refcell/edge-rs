---
title: Quickstart
---

# Quickstart

This page gives you the fastest path from a fresh checkout to compiling an
`.edge` contract. For a deeper walkthrough of the internals, see
[Compiler Architecture](/compiler/overview).

## Install

Install the Edge toolchain with `edgeup`:

```sh
curl -fsSL https://raw.githubusercontent.com/refcell/edge-rs/main/etc/install.sh | sh
```

Or build the compiler from source:

```sh
cargo install --path bin/edgec
```

## Compile a Contract

Pass a source file directly to `edgec` to compile it. By default, the compiler
prints EVM bytecode as a hex string to stdout.

```sh
edgec examples/counter.edge
edgec examples/counter.edge -o counter.bin
edgec -v examples/expressions.edge
```

## Inspect Intermediate Stages

The compiler can also stop after earlier phases of the pipeline:

```sh
edgec lex examples/counter.edge
edgec parse examples/counter.edge
edgec check examples/counter.edge
```

## Explore the Reference Programs

The repository includes a growing set of example contracts under
[`examples/`](https://github.com/refcell/edge-rs/tree/main/examples), ranging
from small syntax samples to larger token-style contracts.
