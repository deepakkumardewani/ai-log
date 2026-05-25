# cclog development tasks
# Uses `just` (https://github.com/casey/just) — install via `cargo install just`

default:
    @just --list

# Build debug binary
build:
    cargo build

# Build release binary
build-release:
    cargo build --release

# Format all code
fmt:
    cargo fmt --all

# Run clippy with warnings as errors
clippy:
    cargo clippy --all-targets -- -D warnings

# Run all tests
test:
    cargo test

# CI pipeline: fmt → clippy → test
ci: fmt clippy test
    @echo "CI: all checks passed"

# Check formatting, clippy, and test in one pass (order matches CI)
check:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test
