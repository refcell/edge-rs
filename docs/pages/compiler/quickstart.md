---
title: Quickstart
---

# Quickstart

This page gives you the fastest path from a fresh checkout to compiling an
`.edge` contract. For a deeper walkthrough of the internals, see
[Compiler Architecture](/compiler/overview).

## Install

Install `edgeup`, the Edge toolchain manager:

```bash
curl -fsSL https://raw.githubusercontent.com/refcell/edge-rs/main/etc/install.sh | sh
```

Then install the Edge compiler:

```bash
edgeup install
```

`edgeup` detects your shell (bash, zsh, or fish) and appends the toolchain
directory to your `PATH` automatically. Restart your shell or run the printed
`source` command before continuing.

**Supported platforms:** Linux x86_64, macOS x86_64, macOS arm64 (Apple
Silicon). Windows is not supported.

## Compile a contract

Pass a source file directly to `edgec` to compile it. By default, the compiler
prints EVM bytecode as a hex string to stdout.

```bash
edgec examples/counter.edge
edgec examples/counter.edge -o counter.bin
edgec -v examples/expressions.edge
```

## Inspect intermediate stages

The compiler can stop after earlier phases of the pipeline using subcommands
or the `--emit` flag:

```bash
# Subcommands
edgec lex   examples/counter.edge   # print tokens
edgec parse examples/counter.edge   # print AST
edgec check examples/counter.edge   # type-check only, no output

# --emit flag variants
edgec --emit tokens    examples/counter.edge   # lex only
edgec --emit ast       examples/counter.edge   # parse only
edgec --emit ir        examples/counter.edge   # IR (s-expression)
edgec --emit pretty-ir examples/counter.edge   # IR (pretty-printed)
edgec --emit asm       examples/counter.edge   # pre-final assembly
edgec --emit bytecode  examples/counter.edge   # EVM bytecode (default)
```

## ABI output

Passing `--emit abi` prints the contract ABI as JSON to stdout, compatible with the Ethereum ABI specification. This is useful for generating interface files consumed by frontends, deployment scripts, and other tooling.

```bash
edgec --emit abi examples/counter.edge
# [{"type":"function","name":"increment","inputs":[],"outputs":[],"stateMutability":"view"}, ...]
```

## Standard JSON I/O

The `edgec standard-json` command implements the standard JSON IPC protocol used by Foundry and solc-compatible toolchains. It reads a JSON request from stdin containing source files and compiler settings, compiles every source, and writes a JSON response to stdout with ABI and bytecode fields for each contract. The command always exits 0; compilation errors are reported inside the JSON response rather than as a non-zero exit code. This is the interface that the `foundry-compilers` crate uses to drive external compilers.

```bash
echo '{"language":"Edge","sources":{"counter.edge":{"content":"..."}}}' | edgec standard-json
# {"sources":{"counter.edge":{"id":0}},"contracts":{"counter.edge":{"Counter":{"abi":[...],"evm":{"bytecode":{"object":"604d..."},...}}}}}
```

## Optimization flags

Control the optimization level and target metric:

```bash
edgec -O2 examples/counter.edge                        # optimization level 0–3 (default: 0)
edgec --optimize-for size examples/counter.edge        # optimize for bytecode size
edgec --optimize-for gas  examples/counter.edge        # optimize for gas cost (default)
```

## Standard library path

The compiler embeds the Edge standard library at build time. To use a local
checkout of the stdlib instead, set `--std-path` or the `EDGE_STD_PATH`
environment variable:

```bash
edgec --std-path ./std examples/counter.edge
EDGE_STD_PATH=./std edgec examples/counter.edge
```

## Language server

Edge ships an LSP server for editor integration. Start it with:

```bash
edgec lsp
```

The server communicates over stdin/stdout and provides parse and type-check
diagnostics with precise source spans.

:::warning
Hover, completions, and go-to-definition are not yet implemented.
:::

## Explore the reference programs

The repository includes a growing set of example contracts under
[`examples/`](https://github.com/refcell/edge-rs/tree/main/examples), ranging
from small syntax samples to larger token-style contracts.
