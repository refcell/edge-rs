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
    cargo nextest run --workspace

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

# Serve the book
book:
    mdbook serve --open
