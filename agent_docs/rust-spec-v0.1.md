# Spec: claude-code-log (Rust) — v0.1

> Status: **Phase 1 (Specify)** — awaiting human approval before Plan/Tasks/Implement.
> Source of confirmed intent: interview on 2026-05-21 (see decisions below).

## Objective

A production-grade Rust CLI that converts Claude Code transcript JSONL files into
**beautiful, fully self-contained HTML** (and Markdown) using a **custom Tailwind
design** (mockups in `new/*.html` / `new/*.png`). It is a clean-room reimplementation
of the Python `claude-code-log` that the author owns end-to-end.

**User:** anyone who uses Claude Code. Public tool, easy install, day-1 polish.

**Why now:** the author dislikes the current HTML design and wants a Rust codebase
they control rather than restyling the existing Python/Jinja templates.

**Confirmed decisions (binding):**
- Drivers: own a Rust codebase + the redesign + "professional, for everyone, from day 1".
  *Not* primarily a perf/single-binary play, *not* a learn-for-fun toy.
- Quality bar, **staged scope**. Sequence: **A → B → D → C**.
- v0.1 = **pillar A + B only**. Timeline (D) and TUI (C) are later, equally-polished releases.
- Output must be **self-contained / offline-shareable**. Tailwind Play CDN is allowed
  *only* as a temporary scaffold and **must not appear in any shipped release**.
- Cost estimation is **out of scope** (no cost data exists in JSONL).

### v0.1 scope (in)
- **A — Export:** single session and single JSONL file → self-contained `.html` and `.md`.
- **B — Hierarchy:** process `~/.claude/projects/` → per-session files + per-project
  combined page + master `index.html`; SQLite metadata cache.
- New Tailwind design (dark, Material-3 token palette from mockups), self-contained.
- Markdown rendering w/ syntax highlighting; side-by-side Edit diffs; image embedding.
- Runtime JS message-type filtering (client-side, in the static HTML).
- Date-range filtering (natural language). Detail levels + compact mode for `--format md`.

### v0.1 scope (out)
- **C — TUI** (`ratatui`) and **D — interactive timeline** (vis-timeline).
- Cost estimation; non-JSONL data sources; Tailwind Play CDN in releases.
- `--format json` passthrough (defer unless trivial).

## Tech Stack (answers the author's Q5)

| Concern | Crate / tool | Why |
|---|---|---|
| CLI args | `clap` v4 (derive) | Standard, ergonomic, derive matches the flag-heavy surface |
| JSONL parse | `serde` + `serde_json` + `BufReader` | Line-by-line; `#[serde(tag="type")]` enums mirror Pydantic unions |
| Error handling | `thiserror` (lib) + `anyhow` (bin) | Typed errors in core, ergonomic context at the edge |
| HTML templates | **`askama`** (compile-time, type-safe) | "Own the codebase" + compile-time checked; Jinja-like. Fallback: `minijinja` if rapid template iteration hurts |
| Markdown | **`comrak`** (GFM) | Tables/strikethrough/task-lists parity with mistune |
| Syntax highlight | `syntect` | Build-time HTML w/ inline styles → self-contained |
| Diffs (Edit/MultiEdit) | `similar` | Server-side side-by-side diff → static HTML, no JS lib |
| Cache | `rusqlite` (`bundled`) | Static SQLite, no system dep; mirrors Python cache schema |
| Dates (NL) | `chrono` + `interim` (or `chrono-english`) | "yesterday", "last week", "3 days ago" |
| Tailwind | **Tailwind standalone CLI** (no Node) at build time → minified CSS embedded via `include_str!` | Self-contained output without a Node toolchain |
| Fonts/icons | woff2 + Material Symbols subset, base64-embedded in CSS | Offline; CDN only during early scaffolding |
| Snapshot tests | `insta` | HTML regression — the `syrupy` analogue |
| CLI tests | `assert_cmd` + `predicates` | End-to-end binary behavior |
| Logging | `tracing` + `tracing-subscriber` | Structured `--debug` output |
| Parallelism | `rayon` (sessions) | Optional; safe per-file parallelism |
| Progress | `indicatif` | Progress bar for large hierarchies |

## Commands

```bash
# Build / release
cargo build
cargo build --release

# Run (mirrors Python CLI surface)
cargo run -- <INPUT_PATH>                     # file, dir, or project path
cargo run -- --all-projects                   # default when no INPUT_PATH
cargo run -- --session-id <ID|prefix>
cargo run -- --format md --detail user-only --compact
cargo run -- --from-date "yesterday" --to-date "today"
cargo run -- --open-browser

# Quality gates (must pass before any commit)
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo test -- --ignored        # snapshot/integration
cargo insta review             # accept intentional HTML changes
```

### CLI surface (parity targets for v0.1; TUI flag deferred)
`INPUT_PATH` (optional) · `--output/-o` · `--format html|md|markdown` ·
`--all-projects` · `--no-individual-sessions` · `--session-id` ·
`--from-date` · `--to-date` · `--detail full|high|low|minimal|user-only` ·
`--compact` · `--image-export-mode placeholder|embedded|referenced` ·
`--page-size <N>` · `--no-cache` · `--clear-cache` · `--clear-output` ·
`--projects-dir` · `--open-browser` · `--debug`.
Deferred: `--tui` (errors with "coming in a later release").

## Project Structure

```
claude-code-log-rs/            # crate name TBD (see Open Questions)
├── Cargo.toml
├── build.rs                   # runs Tailwind CLI → OUT_DIR css; embeds assets
├── assets/
│   ├── tailwind.input.css     # @tailwind layers + custom component classes
│   ├── tailwind.config.js     # design tokens lifted from new/ mockups
│   └── fonts/                 # woff2 for embedding (Geist, Space Grotesk, JetBrains Mono)
├── src/
│   ├── main.rs                # entry → cli::run()
│   ├── cli.rs                 # clap parse + command dispatch
│   ├── model/                 # serde structs
│   │   ├── entry.rs           # TranscriptEntry enum: User|Assistant|Summary|System|QueueOp|HookAttachment|AwaySummary
│   │   ├── content.rs         # ContentItem enum: Text|Thinking|ToolUse|ToolResult|Image
│   │   └── tool.rs            # typed tool-input structs (Bash, Read, Write, Edit, MultiEdit, ...)
│   ├── parser.rs              # JSONL → Vec<TranscriptEntry> (tolerant of unknown fields)
│   ├── session.rs            # DAG threading via parentUuid, fork/sidechain detection, boundaries
│   ├── aggregate.rs          # token totals, tool-usage counts, virtual file tree, summary matching
│   ├── render/
│   │   ├── html.rs            # askama context assembly → transcript.html / index.html
│   │   ├── markdown.rs        # comrak output + detail/compact handling
│   │   ├── highlight.rs       # syntect wrapper
│   │   └── diff.rs            # similar → side-by-side diff HTML
│   ├── templates/             # askama .html (transcript, index, components/*)
│   ├── cache.rs               # rusqlite metadata cache + invalidation by mtime
│   ├── dates.rs               # natural-language range filtering
│   └── assets.rs              # include_str!/include_bytes! embedded CSS + fonts
├── tests/
│   ├── fixtures/              # JSONL samples (ported from Python test/test_data/)
│   ├── cli.rs                 # assert_cmd integration tests
│   └── snapshots/             # insta .snap files
└── README.md
```

## Data Model (from `new/rust-reimpl-analysis.md`)

- **Entries** (`#[serde(tag = "type")]`): `user`, `assistant`, `summary`, `system`,
  `queue-operation`, `hook-attachment`, `away-summary`. Unknown types → tolerant fallback.
- Top-level fields: `uuid`, `parentUuid`, `timestamp`, `sessionId`, `isSidechain`,
  `agentId`, `cwd`, `gitBranch`, `version`, `message`.
- **Message**: `role`, `model?`, `stop_reason?`, `usage`, `content[]`.
- **UsageInfo**: `input_tokens?`, `output_tokens?`, `cache_creation_input_tokens?`,
  `cache_read_input_tokens?`, `service_tier?`.
- **ContentItem**: `text` | `thinking` | `tool_use` | `tool_result` | `image`.
- **Tools (28 names)** — typed inputs for at least: `Bash, Read, Write, Edit, MultiEdit,
  Glob, Grep, Task/Agent, TodoWrite, AskUserQuestion, WebSearch, WebFetch, ScheduleWakeup,
  CronCreate/List/Delete, Task* , Team*, SendMessage, Skill, ExitPlanMode, Monitor`.
  Unknown tool → generic key/value card. Tool rendering is the main extension point.

> Authoritative references to mirror: `claude_code_log/models.py`,
> `factories/transcript_factory.py`, `factories/tool_factory.py`, `renderer.py`,
> `cache.py`, `templates/transcript.html`, `templates/index.html`, `test/test_data/`.

## Design / Rendering Notes

- Dark theme by default (`<html class="dark">`); Material-3 token palette (`surface*`,
  `on-*`, `primary`, `border #262626`, `background #0A0A0A`).
- Layout: fixed top header (project/session meta + token summary), left sidebar
  (Session History · Table of Contents · File Explorer · Tool Usage), main message column,
  status bar. **Cost line omitted.**
- Message cards: user text, assistant markdown, collapsible thinking, Bash IN/OUT,
  Read path+content, Edit side-by-side diff, Write content, token display, pair-duration,
  error state (red border), images (base64). Error = `tool_result.is_error`.
- Derived data the Python tool does not precompute, required by the design:
  **File Explorer** (collect `file_path` from Read/Write/Edit/MultiEdit/Glob → virtual tree)
  and **Tool Usage** (count `tool_use.name`). "Session Active" = `last_timestamp` within 10m.

## Code Style

```rust
/// Threads a flat entry list into a session DAG via `parentUuid`.
/// Returns roots; forks are attached as additional children.
pub fn build_session_tree(entries: Vec<TranscriptEntry>) -> Result<Vec<MessageNode>, ParseError> {
    let mut by_uuid: HashMap<Uuid, MessageNode> = HashMap::with_capacity(entries.len());
    for entry in entries {
        let node = MessageNode::from_entry(entry)?; // fail fast on malformed entries
        by_uuid.insert(node.uuid, node);
    }
    // ... attach children to parents; collect parentless roots
    Ok(roots)
}
```

- `snake_case` items, `PascalCase` types, `SCREAMING_SNAKE_CASE` consts; modules per responsibility.
- Functions small and single-purpose; early returns over deep nesting; no magic literals.
- Errors are typed (`thiserror`) in the library; never silently swallowed. Comment *why*, not *what*.
- `#[serde(rename_all = "camelCase")]` to map JSONL; `#[serde(default)]` + `Option<_>` for optional fields.

## Testing Strategy

- **Unit:** parser (each entry/content/tool variant), DAG threading, aggregation,
  date filtering, diff rendering. Located in `#[cfg(test)]` modules.
- **Snapshot (`insta`):** HTML + Markdown output per fixture session — the regression net.
  Update intentional changes via `cargo insta review` (never blindly accept).
- **CLI integration (`assert_cmd`):** flags produce expected files; `--all-projects`
  builds index; `--from-date/--to-date` filter; exit codes on bad input.
- **Self-containment assertion:** released-mode HTML contains **no** `http(s)://` asset URLs.
- Fixtures ported from `test/test_data/`. Target: meaningful coverage of parser + render paths.

## Boundaries

- **Always:** run `cargo fmt`, `cargo clippy -D warnings`, `cargo test` before commit;
  keep `new/rust-reimpl-analysis.md` + this spec as the source of truth; tolerant parsing
  (unknown entry/tool/field never panics); released HTML fully self-contained.
- **Ask first:** adding dependencies beyond the table above; changing the cache schema;
  diverging the CLI surface from `claude-code-log`; pulling pillar C/D work into v0.1.
- **Never:** ship the Tailwind Play CDN or any external asset URL in a release; invent
  cost figures; commit secrets or real user transcripts; remove failing tests to go green.

## Success Criteria (testable)

1. `claude-code-log-rs path/to/session.jsonl` produces an `.html` that opens **offline**
   with no network requests (verified: zero `http(s)://` in output).
2. Output visually matches `new/` mockups for: header, sidebar (4 panels), user/assistant/
   thinking cards, Bash, Read, Edit (side-by-side diff), Write, images, error state.
3. `--all-projects` produces per-session files, per-project combined pages, and a master
   `index.html` linking them; second run is materially faster (cache hit).
4. `--format md --detail user-only --compact` yields LLM-feedable Markdown.
5. `--from-date/--to-date` accept natural language and filter correctly.
6. Client-side message-type filter toggles visibility in the static HTML.
7. `cargo clippy -D warnings` and `cargo test` are clean; snapshots committed.
8. Installable via `cargo install` and a prebuilt GitHub-release binary.

## Open Questions

1. **Crate/binary name** — keep `claude-code-log` (collides w/ PyPI), or e.g. `cclog`,
   `claude-code-log-rs`, `ccview`? Decide before publishing. (use cclog)
2. **`askama` vs `minijinja`** — start with askama (compile-time safety); fall back to
   minijinja only if template iteration friction is high. OK to defer to Plan phase? ok
3. **`--format json`** — include a passthrough in v0.1 or drop until later? drop
4. **Repo location** — new top-level repo, or a `rust/` subdir of this repo during bring-up? will create separate folder and repo not in this
5. **Font licensing** — confirm Geist/Space Grotesk/JetBrains Mono + Material Symbols are
   OK to embed (all OFL/redistributable — confirm before bundling). ok
```
