#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# weavr (cclog) performance benchmark
#
# Benchmarks cclog against the Python claude-code-log on a full-project
# export of ~/.claude/projects/. Uses uvx for zero-install Python runs.
#
# Usage: just bench   (or  ./scripts/bench.sh)
# ---------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BENCH_DIR="${PROJECT_DIR}/target/bench"
mkdir -p "$BENCH_DIR"

# ---- configuration ---------------------------------------------------------
WARMUP="${WARMUP:-1}"           # hyperfine warmup runs
RUNS="${RUNS:-3}"               # measurement runs
PROJECTS_DIR="${PROJECTS_DIR:-$HOME/.claude/projects}"
PYTHON_TOOL="${PYTHON_TOOL:-uvx claude-code-log@latest}"

# ---- helpers ---------------------------------------------------------------
red()   { echo "$(tput setaf 1)$*$(tput sgr0)"; }
green() { echo "$(tput setaf 2)$*$(tput sgr0)"; }
cyan()  { echo "$(tput setaf 6)$*$(tput sgr0)"; }

# ---- build release binary --------------------------------------------------
echo ""
cyan "=== Building release binary ==="
cd "$PROJECT_DIR"
cargo build --release 2>&1 | tail -1
BIN="${PROJECT_DIR}/target/release/cclog"
echo "  binary: $BIN"

# ---- check data ------------------------------------------------------------
if [ ! -d "$PROJECTS_DIR" ]; then
    red "ERROR: projects directory not found: $PROJECTS_DIR"
    exit 1
fi

JSONL_COUNT=$(find "$PROJECTS_DIR" -name '*.jsonl' 2>/dev/null | wc -l | tr -d ' ')
echo "  projects dir: $PROJECTS_DIR ($JSONL_COUNT JSONL files)"

# ---- bench: cclog (Rust) ---------------------------------------------------
echo ""
cyan "=== Benchmark: cclog (Rust) ==="
hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$BENCH_DIR/cclog.json" \
    --export-markdown "$BENCH_DIR/cclog.md" \
    --prepare 'rm -rf '"$BENCH_DIR"'/cclog-out' \
    --command-name "cclog (Rust)" \
    "$BIN --all-projects --projects-dir '$PROJECTS_DIR' --output-dir '$BENCH_DIR/cclog-out' --no-cache 2>/dev/null"

# ---- bench: claude-code-log (Python) ---------------------------------------
echo ""
cyan "=== Benchmark: claude-code-log (Python) ==="
hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$BENCH_DIR/python.json" \
    --export-markdown "$BENCH_DIR/python.md" \
    --prepare 'rm -rf '"$BENCH_DIR"'/python-out ~/.cache/claude-code-log 2>/dev/null' \
    --command-name "claude-code-log (Python)" \
    "$PYTHON_TOOL '$PROJECTS_DIR/' --all-projects -o '$BENCH_DIR/python-out' --no-individual-sessions 2>/dev/null"

# ---- summary ---------------------------------------------------------------
echo ""
cyan "=== Comparison ==="
printf "%-30s %s\n" "Tool" "Mean time"
echo "----------------------------------------"
if [ -f "$BENCH_DIR/cclog.md" ]; then
    cclog_mean=$(grep -oE '[0-9]+\.[0-9]+ s' "$BENCH_DIR/cclog.md" | head -1 || echo "N/A")
    printf "%-30s %s\n" "cclog (Rust)" "$cclog_mean"
fi
if [ -f "$BENCH_DIR/python.md" ]; then
    python_mean=$(grep -oE '[0-9]+\.[0-9]+ s' "$BENCH_DIR/python.md" | head -1 || echo "N/A")
    printf "%-30s %s\n" "claude-code-log (Python)" "$python_mean"
fi
echo ""
cyan "Full results: $BENCH_DIR/"
