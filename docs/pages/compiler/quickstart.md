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

## ABI Output

Passing `--emit abi` prints the contract ABI as JSON to stdout, compatible with the Ethereum ABI specification. This is useful for generating interface files consumed by frontends, deployment scripts, and other tooling.

```sh
edgec --emit abi examples/counter.edge
# [{"type":"function","name":"increment","inputs":[],"outputs":[],"stateMutability":"view"}, ...]
```

## Standard JSON I/O

The `edgec standard-json` command implements the standard JSON IPC protocol used by Foundry and solc-compatible toolchains. It reads a JSON request from stdin containing source files and compiler settings, compiles every source, and writes a JSON response to stdout with ABI and bytecode fields for each contract. The command always exits 0; compilation errors are reported inside the JSON response rather than as a non-zero exit code. This is the interface that the `foundry-compilers` crate uses to drive external compilers.

```sh
echo '{"language":"Edge","sources":{"counter.edge":{"content":"..."}}}' | edgec standard-json
# {"sources":{"counter.edge":{"id":0}},"contracts":{"counter.edge":{"Counter":{"abi":[...],"evm":{"bytecode":{"object":"604d..."},...}}}}}
```

## Explore the Reference Programs

The repository includes a growing set of example contracts under
[`examples/`](https://github.com/refcell/edge-rs/tree/main/examples), ranging
from small syntax samples to larger token-style contracts.
