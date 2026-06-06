# weavr development tasks
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

# Run coverage and print overall + per-core-module percentages
coverage:
    cargo llvm-cov --html --open=false 2>/dev/null; \
    cargo llvm-cov --summary-only 2>&1 | grep -E '(^| )src/' || true; \
    echo "---"; \
    cargo llvm-cov --summary-only 2>&1 | tail -5

# CI coverage gate: fails if total line coverage < 80%
coverage-ci:
    @cov=$$(cargo llvm-cov --summary-only 2>&1 | grep -oE '[0-9]+(\.[0-9]+)?%' | head -1 | tr -d '%'); \
    echo "Total line coverage: $${cov}%"; \
    if [ "$$(echo "$${cov} < 80" | bc 2>/dev/null || echo 0)" = "1" ]; then \
        echo "ERROR: Coverage $${cov}% is below 80% threshold"; \
        exit 1; \
    else \
        echo "Coverage gate passed (>= 80%)"; \
    fi

# Run performance benchmark (requires hyperfine: brew install hyperfine)
bench:
    @./scripts/bench.sh
