---
title: Tooling Overview
---

# Tooling overview

Edge's tooling is centered around the compiler CLI, the installer, and the
language server.

## `edgec`

`edgec` is the main command-line entry point for the compiler. It can compile
contracts directly to EVM bytecode or stop after earlier phases for inspection.

```bash
edgec examples/counter.edge
edgec lex examples/counter.edge
edgec parse examples/counter.edge
edgec check examples/counter.edge
edgec lsp
```

### Subcommands

| Subcommand | Description |
|---|---|
| `lex <FILE>` | Lex file and print tokens (debug output) |
| `parse <FILE>` | Parse file and print AST (debug output) |
| `check <FILE>` | Compile for errors without producing output |
| `lsp` | Start the LSP server over stdin/stdout |

### Compiler flags

| Flag / Option | Short | Default | Description |
|---|---|---|---|
| `<FILE>` | — | — | Source file to compile (outputs hex bytecode to stdout) |
| `--output` | `-o` | — | Write raw bytecode bytes to file (requires FILE) |
| `--emit <KIND>` | — | `bytecode` | `tokens` / `ast` / `ir` / `pretty-ir` / `asm` / `bytecode` |
| `-O <LEVEL>` | — | `0` | Optimization level (0–3) |
| `--optimize-for` | — | `gas` | Optimization target: `gas` or `size` |
| `--std-path` | — | — | Filesystem stdlib path (also: `EDGE_STD_PATH` env var) |
| `--verbose` | `-v` | — | Verbosity; repeat for more: `-v`=WARN, `-vv`=INFO, `-vvv`=DEBUG, `-vvvv`=TRACE |
| `--version` | — | — | Print version and exit |
| `--help` | `-h` | — | Print help |

### Verbosity levels

| `-v` count | Log level | Notes |
|---|---|---|
| 0 | (off) | No tracing output |
| 1 | `WARN` | |
| 2 | `INFO` | |
| 3 | `DEBUG` | |
| 4+ | `TRACE` | egglog also set to TRACE (otherwise WARN) |

### Emit output behavior

| Emit | Stdout | File (`-o`) |
|---|---|---|
| `tokens` | Debug print of each Token | — |
| `ast` | Debug print of Program | — |
| `ir` | S-expression format | — |
| `pretty-ir` | Pretty-printed IR | — |
| `asm` | Labeled block assembly | — |
| `bytecode` | `0x<hex>` string | Raw bytes |

## `edgeup`

`edgeup` is the Edge toolchain manager. Install it first, then use it to
install and manage `edgec` versions.

```bash
# 1. Install edgeup
curl -fsSL https://raw.githubusercontent.com/refcell/edge-rs/main/etc/install.sh | sh

# 2. Install the Edge compiler
edgeup install
```

**Supported platforms:** Linux x86_64, macOS x86_64, macOS arm64.
Windows is not supported.

`edgeup` detects your shell (bash, zsh, or fish) and appends
`~/.edgeup/bin` to your `PATH` in the appropriate RC file (`~/.bashrc`,
`~/.zshrc`, or `~/.config/fish/config.fish`). Restart your shell or run
the printed `source` command after installation.

### Directory layout

```
~/.edgeup/
  bin/
    edgec          ← symlink → versions/{tag}/edgec
  versions/
    v0.1.6/
      edgec        ← actual binary (chmod 755)
    v0.1.7/
      edgec
```

### `edgeup` subcommands

| Subcommand | Description |
|---|---|
| `install [VERSION]` | Download and install Edge toolchain (default: latest) |
| `update` | Alias for `install` — installs latest version |
| `list` | List all installed versions |
| `use <VERSION>` | Switch active version (updates symlink) |
| `uninstall [VERSION]` | Remove a version, or all if omitted |
| `self-update` | Update `edgeup` itself to the latest release |
| `version` | Print the `edgeup` version |

## LSP

Edge ships an LSP server for editor integration:

```bash
edgec lsp
```

The server communicates over stdin/stdout and provides parse and type-check
diagnostics with precise source spans.

:::warning
Hover, completions, and go-to-definition are not yet implemented. The LSP
currently only reports parse errors and type-check errors.
:::

## Repository utilities

The repository ships a [`Justfile`](https://github.com/refcell/edge-rs/blob/main/Justfile)
with common contributor workflows:

| Command | Description |
|---|---|
| `just build` | Build all crates (`cargo build --workspace`) |
| `just test` | Run all tests (`cargo test --workspace`) |
| `just lint` | Run all lints (format, clippy, deny, docs) |
| `just e2e` | Run end-to-end tests |
| `just bench` | Run benchmarks |
| `just docs` | Serve the Vocs documentation site locally |
| `just docs-build` | Build the documentation site |
| `just check-examples` | Parse all example contracts |
| `just check-stdlib` | Parse all stdlib contracts |

## Reference material

For runnable contracts and language samples, see the
[`examples/`](https://github.com/refcell/edge-rs/tree/main/examples) and
[`std/`](https://github.com/refcell/edge-rs/tree/main/std) directories.
