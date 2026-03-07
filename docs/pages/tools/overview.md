---
title: Tooling Overview
---

# Tooling Overview

Edge's tooling is currently centered around the compiler CLI, the installer,
and the repository's example contracts.

## `edgec`

`edgec` is the main command-line entry point for the compiler. It can compile
contracts directly to EVM bytecode or stop after earlier phases for inspection.

```sh
edgec examples/counter.edge
edgec lex examples/counter.edge
edgec parse examples/counter.edge
edgec check examples/counter.edge
```

## `edgeup`

The recommended installation path is the `edgeup` script:

```sh
curl -fsSL https://raw.githubusercontent.com/refcell/edge-rs/main/etc/install.sh | sh
```

## Repository Utilities

The repository also ships a [`Justfile`](https://github.com/refcell/edge-rs/blob/main/Justfile)
with common contributor workflows for building, testing, linting, and serving
the documentation site.

## Reference Material

For runnable contracts and language samples, see the
[`examples/`](https://github.com/refcell/edge-rs/tree/main/examples) and
[`std/`](https://github.com/refcell/edge-rs/tree/main/std) directories.
