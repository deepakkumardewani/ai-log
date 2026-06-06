# weavr Code Review Findings

> Phase 1 — Review / Simplify / DRY (T1.1)
> Reviewed: all `src/**` Rust modules + `templates/**` + `assets/**`
> Baseline: `cargo clippy --all-targets -- -D warnings` passes; all 26 tests green.

---

## Findings

### F1 — Dead code: 5 test-only functions suppressed with `#[allow(dead_code)]`

**File:** `src/render/html.rs:421–568`
**Category:** dead
**Functions:** `format_ts`, `render_system_entry`, `render_hook_attachment`, `html_escape_text`, `pretty_json`

These five functions are only called from test code in the same file (confirmed at lines 882, 923, 966). The codebase comment says "will be re-wired in P3/T12" — that plan has been retired. Using `#[allow(dead_code)]` suppresses clippy on them in non-test builds, masking them as live code.

**Proposed fix:** Annotate all five functions (and their call-site tests) with `#[cfg(test)]`. This removes the `#[allow(dead_code)]` suppressions, keeps the tests, and makes the scope explicit.

---

### F2 — Duplication: `html_escape` defined in 4 modules (mandatory extraction)

**Files / lines:**
- `src/render/tools/mod.rs:607` — `fn html_escape(s: &str) -> String`
- `src/render/highlight.rs:39` — `fn html_escape(input: &str) -> String`
- `src/render/diff.rs:157` — `fn html_escape(input: &str) -> String`
- `src/render/html.rs:562` — `fn html_escape_text(s: &str) -> String` (dead-only variant, see F1)

Plus 4 additional **inline** escape chains in `src/render/tools/mod.rs` at lines 138, 152, 198, 235 that bypass the local `html_escape` function entirely:
```rust
text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
```

**Category:** dup (8 sites — mandatory extraction per spec)
**Proposed fix:** Add `pub(crate) fn html_escape(s: &str) -> String` to `src/render/mod.rs`. Update all 3 module-local definitions and 4 inline usages to call the shared one. The `html_escape_text` variant in `html.rs` is removed as part of F1.

---

### F3 — Duplication: `tool-io` row HTML string repeated 14+ times (mandatory extraction)

**File:** `src/render/tools/mod.rs`
**Lines:** 234, 251, 256, 318, 334, 415, 430, 445, 459, 473, 498, 514, 529, 544 (and more)
**Category:** dup (14+ sites — mandatory extraction per spec)

The same HTML pattern is copy-pasted everywhere:
```rust
format!(r#"<div class="tool-io"><span class="tool-io-label">X:</span><span class="tool-io-value">Y</span></div>"#, ...)
```

**Proposed fix:** Extract `fn tool_io_row(label: &str, value: &str) -> String` as a private helper at the top of `render/tools/mod.rs`. Replace all inline occurrences.

---

### F4 — Dead files: `.bak` backup files checked into source tree

**Files:** `src/lib.rs.bak`, `src/main.rs.bak`
**Category:** dead

Leftover backup files from a prior editing session. Not compiled, not referenced, but add noise to the source tree.

**Proposed fix:** Delete both files.

---

### F5 — Clarity: stale phase-numbered comments in `cli.rs`

**File:** `src/cli.rs`
**Lines:** 57, 192, 341
**Category:** clarity

Comments reference a retired internal phase numbering:
- Line 57: `// Phase 6: CLI parity`
- Line 192: `// Single-session export (Phase 3–4)`
- Line 341: `// All-projects pipeline (Phase 5)`

These phase numbers mean nothing to a reader unfamiliar with the original dev log.

**Proposed fix:** Replace with descriptive section comments (e.g. `// Date range filtering`, `// Single-session export`, `// All-projects pipeline`) or remove if self-evident.

---

## Module-by-module status

| Module | Status | Notes |
|---|---|---|
| `src/parser.rs` | ✅ clean | Good error handling, clear naming |
| `src/aggregate.rs` | ✅ clean | Token accumulation logic is clear |
| `src/conversation.rs` | ✅ clean | DFS threading logic is well-structured |
| `src/model/mod.rs` | ✅ clean | — |
| `src/model/entry.rs` | ✅ clean | Manual `Deserialize` impl is justified |
| `src/model/content.rs` | ✅ clean | — |
| `src/model/tool.rs` | ✅ clean | Large but each struct is 1:1 with a tool |
| `src/cli.rs` | ⚠️ F5 | Stale phase comments |
| `src/lib.rs` | ✅ clean | — |
| `src/main.rs` | ✅ clean | — |
| `src/cache.rs` | ✅ clean | SQLite schema + batching is reasonable |
| `src/project.rs` | ✅ clean | — |
| `src/session.rs` | ✅ clean | — |
| `src/dates.rs` | ✅ clean | — |
| `src/assets.rs` | ✅ clean | — |
| `src/render/mod.rs` | ✅ clean (→ F2 fix target) | Will gain `html_escape` |
| `src/render/html.rs` | ⚠️ F1, F2 | Dead-code functions; html_escape duplicate |
| `src/render/diff.rs` | ⚠️ F2 | html_escape duplicate |
| `src/render/highlight.rs` | ⚠️ F2 | html_escape duplicate |
| `src/render/markdown.rs` | ✅ clean | — |
| `src/render/markdown_export.rs` | ✅ clean | — |
| `src/render/pagination.rs` | ✅ clean | — |
| `src/render/project.rs` | ✅ clean | — |
| `src/render/index.rs` | ✅ clean | — |
| `src/render/turn.rs` | ✅ clean | — |
| `src/render/tools/mod.rs` | ⚠️ F2, F3 | html_escape duplicate + tool-io DRY |
| `src/lib.rs.bak` | ⚠️ F4 | Delete |
| `src/main.rs.bak` | ⚠️ F4 | Delete |
| `templates/**` | ✅ clean | Proper template inheritance + `{% include %}` partials; no duplicated markup blocks |
| `assets/index.js` | ✅ clean | — |
| `assets/transcript.js` | ✅ clean | — |
| `assets/tailwind.input.css` | ✅ clean | — |
| `assets/tailwind.config.js` | ✅ clean | — |
| `assets/fonts/` | ✅ clean | — |

> **Note on branding strings:** `transcript.html` uses `id="cclog-modal"` and `base.html` uses `localStorage.getItem('cclog-theme')`. These are functional `cclog` branding references but are deferred to Phase 4 (T4.2 — rename mechanical step), per spec AC4.6.

---

## Summary

| # | Category | Finding | Action |
|---|---|---|---|
| F1 | dead | 5 test-only functions behind `#[allow(dead_code)]` in `render/html.rs` | Move to `#[cfg(test)]` |
| F2 | dup | `html_escape` defined in 4 modules + 4 inline chains (8 sites) | Extract to `render/mod.rs` |
| F3 | dup | `tool-io` row HTML string repeated 14+ times in `render/tools/mod.rs` | Extract `fn tool_io_row` |
| F4 | dead | `src/lib.rs.bak`, `src/main.rs.bak` in source tree | Delete |
| F5 | clarity | Stale phase-numbered comments in `cli.rs` | Replace with descriptive comments |

All issues have concrete, low-risk fixes. No working subsystem requires rewriting.
