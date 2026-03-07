# Default to display help menu
default:
    @just --list

alias b := build
alias t := test
alias c := clean
alias l := lint
alias book := docs

# Build all
build:
    cargo build --workspace

# Build for CI (release mode)
build-ci:
    cargo build --workspace --release

# Run tests
test:
    cargo test --workspace

# Run tests for CI (using nextest)
test-ci:
    cargo nextest run --workspace --exclude edge-e2e

# Fix formatting
format-fix:
    cargo +nightly fmt --all

# Check formatting
check-format:
    cargo +nightly fmt --all -- --check

# Check clippy
check-clippy:
    cargo clippy --workspace -- -D warnings

# Check clippy for CI
check-clippy-ci:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check docs
check-docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Check deny
check-deny:
    cargo deny check

# Lint everything
lint: check-format check-clippy check-deny check-docs

# Clean
clean:
    cargo clean

# Parse all example contracts with edgec
check-examples:
    #!/usr/bin/env bash
    set -euo pipefail
    failed=0
    while IFS= read -r -d '' f; do
        echo "parsing $f"
        if ! cargo run --bin edgec --quiet -- parse "$f" 2>&1; then
            echo "FAILED: $f"
            failed=1
        fi
    done < <(find examples -name '*.edge' -print0 | sort -z)
    exit $failed

# Parse all stdlib contracts with edgec
check-stdlib:
    #!/usr/bin/env bash
    set -euo pipefail
    failed=0
    while IFS= read -r -d '' f; do
        echo "parsing $f"
        if ! cargo run --bin edgec --quiet -- parse "$f" 2>&1; then
            echo "FAILED: $f"
            failed=1
        fi
    done < <(find std -name '*.edge' -print0 | sort -z)
    exit $failed

# Run acceptance tests
e2e:
    cargo test -p edge-e2e

# Run acceptance tests for CI
e2e-ci:
    cargo nextest run -p edge-e2e

# Run benchmarks
bench:
    cargo bench -p edge-bench

# Run benchmarks with a specific filter (e.g., just bench-filter parse)
bench-filter filter:
    cargo bench -p edge-bench -- {{filter}}

# Install docs dependencies
docs-install:
    npm install

# Serve the Vocs docs site
docs:
    npm run docs:dev

# Build the Vocs docs site
docs-build:
    npm run docs:build

# Validate docs source and rendered output
docs-validate:
    npm run docs:validate
