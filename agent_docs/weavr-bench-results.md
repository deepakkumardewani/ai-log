# weavr (cclog) — Benchmark Results

> Machine: Mac (Apple Silicon, M-series) · Date: 2026-06-06
> Methodology: hyperfine (warmup=1, runs=3–5), fresh output dir per run, no cache

## Head-to-head: cclog (Rust) vs claude-code-log (Python)

### All projects (10 projects, 160 sessions, 97 MB)

| Tool | Mean time | Std dev | Range |
|------|-----------|---------|-------|
| **cclog (Rust)** | **1.324 s** | ±0.057 s | 1.249–1.384 s |
| claude-code-log (Python) | 24.102 s | ±2.071 s | 21.788–25.781 s |
| **Speedup** | | | **18.2×** |

Commands:
- cclog: `cclog --all-projects --no-cache -o /tmp/out`
- Python: `claude-code-log ~/.claude/projects/ --all-projects -o /tmp/out --no-individual-sessions`

### Single project (42 sessions, 98 MB)

| Tool | Mean time | Std dev | Range |
|------|-----------|---------|-------|
| **cclog (Rust)** | **465.6 ms** | ±5.7 ms | 461.6–472.1 ms |
| claude-code-log (Python) | 9.676 s | ±2.382 s | 7.836–12.366 s |
| **Speedup** | | | **20.8×** |

### Single session (19 MB JSONL, ~500 messages)

| Tool | Mean time | Std dev | Range |
|------|-----------|---------|-------|
| **cclog (Rust)** | **27.5 ms** | ±2.4 ms | 24.5–29.7 ms |
| claude-code-log (Python) | 1.279 s | ±0.344 s | 1.037–1.887 s |
| **Speedup** | | | **46.5×** |

Commands:
- cclog: `cclog export session.jsonl -o out.html`
- Python: `claude-code-log session.jsonl -o out.html`

## Summary

| Mode | cclog (Rust) | Python | Speedup |
|------|-------------|--------|---------|
| All projects | 1.324 s | 24.102 s | **18.2×** |
| Single project | 465.6 ms | 9.676 s | **20.8×** |
| Single session | 27.5 ms | 1.279 s | **46.5×** |

The speedup is largest at the single-session level where overhead (startup,
import, template rendering) dominates the Python runtime. At the all-projects
level cclog still outperforms by 18× despite the workload being I/O-heavy
(160 HTML files written to disk).

## Optimizations applied

1. **session.rs — halved TranscriptEntry clones** (highest impact)
   - Before: each entry cloned twice (BuildContext entry_map + MessageNode)
   - After: cloned once in `entries.iter().cloned()`, then moved via `entry_map.drain()`
   - Savings: ~50% fewer deep clones per entry (Strings, Messages, ContentItems)

2. **render/mod.rs — single-pass HTML escape** (moderate impact)
   - Before: chained `.replace()` calls creating up to 4 intermediate Strings per call
   - After: fast-path skip when no special chars present + single-pass scan with one allocation

3. **model/tool.rs — DRY macro for dispatch** (no perf impact, readability)
   - Reduced ~150 lines of repetitive `match` arms to a `dispatch!` macro

## How to re-run

```sh
just bench          # full hyperfine comparison (cclog + Python)
just ci             # fmt + clippy + 281 tests
```
