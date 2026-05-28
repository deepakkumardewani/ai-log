# Implementation Plan: `cclog` (claude-code-log Rust v0.1)

> Source specs: [rust-spec-v0.1.md](rust-spec-v0.1.md), [rust-reimpl-analysis.md](rust-reimpl-analysis.md)
> Status: **Phase 2 (Plan / Task breakdown)** — pending human approval.

---

## Overview

Rust reimplementation of `claude-code-log` covering **Pillar A (single-file/session export)** and **Pillar B (project hierarchy + SQLite cache + master index)**. Output is dark, Material-3, fully self-contained HTML + optional Markdown. Timeline (D) and TUI (C) are deferred to later releases.

The plan is sliced vertically: each milestone leaves the binary in a usable, demoable state with a passing snapshot regression net. The order is **strict bottom-up** for data layers, then **vertical slices** for rendering features.

---

## Architecture Decisions (locked)

- **Crate/binary name:** `cclog` (separate repo, not a subdir of this one).
- **Templating:** `askama` (compile-time safety); revisit `minijinja` only if friction emerges.
- **Models:** `serde` + `#[serde(rename_all = "camelCase")]`, `#[serde(default)]` everywhere optional; tolerant unknown-field handling via untagged fallback enums.
- **Errors:** `thiserror` for the library, `anyhow` at the CLI boundary; never silently swallow.
- **Markdown:** `comrak`. **Syntax highlight:** `syntect`. **Diff:** `similar`. **Cache:** `rusqlite` (bundled). **Dates:** custom NL parser or a small crate (e.g. `chrono-english`). **CLI:** `clap` derive.
- **Build:** `build.rs` runs Tailwind CLI → `OUT_DIR/styles.css`; `include_str!` / `include_bytes!` embed CSS + woff2 fonts + JS filter shim. **Zero CDN URLs in any shipped artefact.**
- **Self-containment gate:** a CI test greps released HTML for `http(s)://` and fails on any hit.
- **Drop from v0.1:** `--format json`, `--tui` (errors with "coming in a later release"), Timeline, cost estimation.

---

## Dependency Graph

```
Phase 0: Scaffolding (Cargo, CI, Tailwind build.rs)
        │
Phase 1: Data layer (models → parser → session DAG → aggregate)
        │
Phase 2: Templates + assets (Tailwind tokens, fonts, askama base)
        │
Phase 3: Pillar A — single-session HTML export (slice through to rendered file)
        │   ├── Content renderers (user, assistant, thinking, system)
        │   └── Tool renderers (Bash/Read/Write/Edit/diff/MultiEdit/Glob/Grep/TodoWrite/AskUserQuestion/…/generic)
        │
Phase 4: Pillar A — Markdown export + detail levels + compact mode
        │
Phase 5: Pillar B — project hierarchy + master index + SQLite cache
        │
Phase 6: CLI parity (date filters, image modes, page-size, --open-browser, …)
        │
Phase 7: Filter JS shim + self-containment gate + polish
        │
Phase 8: Release-readiness (docs, install instructions, version bump)
```

---

## Phase 0 — Scaffolding

### Task 0.1: Cargo workspace + repo skeleton

**Description:** Create the `cclog` repo with `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `.gitignore`, `LICENSE`, baseline `README.md`. Pin Rust edition 2021, MSRV.
**Acceptance criteria:**

- [x] `cargo build` succeeds on a stub binary that prints `cclog v0.1.0-dev`.
- [x] `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` are clean.
      **Verification:** `cargo build && cargo clippy --all-targets -- -D warnings`.
      **Files touched:** `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `.gitignore`, `README.md`.
      **Scope:** XS.

### Task 0.2: Dev tooling + CI quality gates

**Description:** Add `rustfmt.toml`, `clippy.toml`, a `justfile` mirroring the Python project (`just test`, `just ci`), and a GitHub Actions workflow running fmt/clippy/test on push.
**Acceptance criteria:**

- [x] `just ci` runs fmt, clippy with `-D warnings`, and `cargo test`.
- [x] CI workflow passes on a stub commit.
      **Verification:** Push branch, observe green CI.
      **Dependencies:** 0.1
      **Files touched:** `justfile`, `.github/workflows/ci.yml`, `rustfmt.toml`, `clippy.toml`.
      **Scope:** S.

### Task 0.3: `build.rs` for Tailwind + asset embedding

**Description:** Wire `build.rs` to invoke the Tailwind CLI on `assets/tailwind.input.css` (with `assets/tailwind.config.js` lifted from the mockups in `new/*.html`) and emit `OUT_DIR/styles.css`. Add `assets/fonts/` with placeholder woff2 files and an `assets.rs` module exposing the embedded bytes via `include_bytes!`.
**Acceptance criteria:**

- [x] `cargo build` regenerates `styles.css` when `tailwind.input.css` changes.
- [x] Embedded CSS string is reachable from `assets::CSS` at runtime.
- [x] Tailwind config defines Material-3 tokens (`surface`, `surface-container*`, `on-surface*`, `primary`, `border: #262626`, `background: #0A0A0A`) matching `new/overall_layout.html`.
      **Verification:** Write a smoke test that asserts `assets::CSS.contains("--surface")`.
      **Dependencies:** 0.1
      **Files touched:** `build.rs`, `assets/tailwind.input.css`, `assets/tailwind.config.js`, `assets/fonts/.gitkeep`, `src/assets.rs`.
      **Scope:** M.

### Checkpoint: Phase 0

- [x] Repo builds; CI is green; Tailwind pipeline produces a non-empty CSS string.
- [ ] Confirm font licenses (Geist, Space Grotesk, JetBrains Mono, Material Symbols) and commit woff2 files. (Deferred: woff2 files not yet committed — fonts directory has placeholder .gitkeep)

---

## Phase 1 — Data layer

### Task 1.1: Top-level entry model (`model/entry.rs`)

**Description:** Define `TranscriptEntry` as `#[serde(tag = "type")]` enum: `user`, `assistant`, `summary`, `system`, `queue-operation`, `hook-attachment`, `away-summary`. Cover top-level fields: `uuid`, `parentUuid`, `timestamp`, `sessionId`, `isSidechain`, `agentId`, `cwd`, `gitBranch`, `version`, `teamName`, `requestId`, `userType`, `message`. Unknown types deserialize into an `Unknown { raw: serde_json::Value }` variant.
**Acceptance criteria:**

- [x] Unit tests round-trip one fixture per known type from `test/test_data/`.
- [x] An unknown `type` value parses into `Unknown` without erroring.
      **Verification:** `cargo test model::entry`.
      **Dependencies:** 0.1
      **Files touched:** `src/model/mod.rs`, `src/model/entry.rs`, `tests/fixtures/entry_*.jsonl`.
      **Scope:** M.

### Task 1.2: Message + content model (`model/content.rs`)

**Description:** Define `Message { role, model?, stop_reason?, usage, content[] }`, `UsageInfo { input_tokens?, output_tokens?, cache_creation_input_tokens?, cache_read_input_tokens?, service_tier? }`, and `ContentItem` enum (`text`, `thinking`, `tool_use`, `tool_result`, `image`).
**Acceptance criteria:**

- [x] Fixtures covering each ContentItem variant deserialize correctly.
- [x] `tool_result.is_error` and `image` (base64 / referenced) variants are preserved.
      **Verification:** `cargo test model::content`.
      **Dependencies:** 1.1
      **Files touched:** `src/model/content.rs`, fixtures.
      **Scope:** M.

### Task 1.3: Typed tool inputs (`model/tool.rs`)

**Description:** Add a `ToolInput` enum tagged by tool `name`: `Bash { command, description?, run_in_background? }`, `Read { file_path, offset?, limit? }`, `Write { file_path, content }`, `Edit { file_path, old_string, new_string, replace_all? }`, `MultiEdit`, `Glob`, `Grep`, `Task/Agent`, `TodoWrite { todos: [{ content, status, priority, id }] }`, `AskUserQuestion { question, options[] }`, `WebSearch`, `WebFetch`, `ScheduleWakeup`, `CronCreate/List/Delete`, `Team*`, `SendMessage`, `Skill`, `ExitPlanMode`, `Monitor`. Unknown tools → `Generic { name, input: serde_json::Value }`.
**Acceptance criteria:**

- [x] All 28 tool variants documented in the spec have a typed input or fall through to `Generic` cleanly.
- [x] A round-trip test for at least Bash/Read/Edit/TodoWrite/Generic passes.
      **Verification:** `cargo test model::tool`.
      **Dependencies:** 1.2
      **Files touched:** `src/model/tool.rs`, fixtures.
      **Scope:** M.

### Task 1.4: Parser (`parser.rs`)

**Description:** Line-by-line JSONL reader producing `Vec<TranscriptEntry>`. Tolerant of blank lines, BOMs, and unknown fields. Returns a structured error for malformed lines but does not abort the whole file (collects errors).
**Acceptance criteria:**

- [x] Parses every JSONL fixture in `test/test_data/` (ported) without panicking.
- [x] A malformed line yields a recoverable `ParseWarning` and parsing continues.
      **Verification:** `cargo test parser`.
      **Dependencies:** 1.3
      **Files touched:** `src/parser.rs`, `tests/parser.rs`.
      **Scope:** M.

### Task 1.5: Session DAG threading (`session.rs`)

**Description:** Build a session DAG from `Vec<TranscriptEntry>` via `parentUuid`. Detect forks (multiple children sharing a parent), sidechains (`isSidechain == true`), and session boundaries. Output a `Session { id, root_message_ids, threaded_messages, sidechains, forks }`.
**Acceptance criteria:**

- [x] Linear, forked, and sidechain fixtures produce expected tree shapes (unit-tested).
- [x] Orphan messages (parent missing) attach to a synthetic root.
      **Verification:** `cargo test session`.
      **Dependencies:** 1.4
      **Files touched:** `src/session.rs`.
      **Scope:** M.

### Task 1.6: Aggregation (`aggregate.rs`)

**Description:** Compute per-session: total tokens (in/out/cache-creation/cache-read), message count, first/last timestamp, derived `is_active` (last_timestamp within 10m), tool-usage counts (by `tool_use.name`), and a virtual file tree from `Read/Write/Edit/MultiEdit/Glob` `file_path`s. Match `summary` entries to their session via `summary.leafUuid`.
**Acceptance criteria:**

- [x] Token totals match a hand-computed fixture.
- [x] File tree groups paths by directory; counts are correct.
- [x] Tool usage counts a representative fixture correctly.
      **Verification:** `cargo test aggregate`.
      **Dependencies:** 1.5
      **Files touched:** `src/aggregate.rs`.
      **Scope:** M.

### Checkpoint: Phase 1

- [x] All JSONL fixtures from `test/test_data/` parse, thread, and aggregate without errors.
- [x] No panics; warnings surface via a `Vec<ParseWarning>` collected per file.

---

## Phase 2 — Templates + base assets

### Task 2.1: askama base templates (`templates/base.html`, `transcript.html`)

**Description:** Port the dark Material-3 chrome from `new/overall_layout.html` into askama: fixed top header (project/session meta + token summary, **no cost line**), left sidebar (Session History, Table of Contents, File Explorer, Tool Usage), main message column, status bar. Embed `assets::CSS` in `<style>` and woff2 fonts via `@font-face url(data:font/woff2;base64,…)`.
**Acceptance criteria:**

- [x] Rendering `transcript.html` with a stub context yields a self-contained HTML file (no external URLs).
- [x] Visual diff against `new/overall_layout.html` is within rounding (manual review).
      **Verification:** Snapshot the rendered stub via `insta`.
      **Dependencies:** 0.3, 1.6
      **Files touched:** `src/templates/base.html`, `src/templates/transcript.html`, `src/render/html.rs`.
      **Scope:** M.

### Task 2.2: Component partials skeleton

**Description:** Carve out component partials matching the mockups: `components/user_message.html`, `components/assistant_message.html`, `components/thinking.html`, `components/tool_card.html`, `components/diff.html`, `components/sidebar/*`, `components/header.html`, `components/status_bar.html`. All initially render with placeholder content.
**Acceptance criteria:**

- [x] Each partial compiles and is included from `transcript.html`.
- [x] Snapshot of a stub session shows all chrome regions populated with placeholders.
      **Verification:** `cargo insta review` confirms baseline snapshot.
      **Dependencies:** 2.1
      **Files touched:** `src/templates/components/*`.
      **Scope:** M.

### Checkpoint: Phase 2

- [x] A `cclog stub` debug subcommand emits a single HTML file that opens correctly and is fully offline (no network requests in DevTools).

---

## Phase 3 — Pillar A: single-session HTML export

### Task 3.1: Render context assembly (`render/html.rs`)

**Description:** Walk the threaded `Session` and build an askama context: header meta, sidebar data (sessions list, TOC, file explorer, tool usage), and an ordered list of `RenderedMessage` items keyed by uuid for the main column.
**Acceptance criteria:**

- [x] Context-build is a pure function; covered by a unit test against a fixture session.
      **Verification:** `cargo test render::html`.
      **Dependencies:** 2.2, 1.6
      **Files touched:** `src/render/html.rs`, `src/render/mod.rs`.
      **Scope:** M.

### Task 3.2: User + assistant + thinking renderers

**Description:** Render user `text` content as plain markdown-escaped HTML; assistant `text` through comrak; collapsible `thinking` block. Token-display + pair-duration hooks rendered in card chrome.
**Acceptance criteria:**

- [x] Snapshot test of a small fixture matches the mockup `user_message_card_variants.html` and `thinking_block_variants.html`.
      **Verification:** `cargo insta test`.
      **Dependencies:** 3.1
      **Files touched:** `src/render/html.rs`, partials, `src/render/markdown_inline.rs`.
      **Scope:** M.

### Task 3.3: Syntax highlighting (`render/highlight.rs`)

**Description:** Wrap `syntect` with a HtmlGenerator preconfigured for dark Material-3 palette. Cache the syntax/theme set behind `OnceLock`. Hook into comrak's code-fence renderer.
**Acceptance criteria:**

- [x] Rust/TS/Python/Shell fences render with correct colors in a snapshot.
- [x] Cold-start cost is amortized (build once per process).
      **Verification:** Snapshot + manual visual.
      **Dependencies:** 3.2
      **Files touched:** `src/render/highlight.rs`.
      **Scope:** S.

### Task 3.4: Tool card renderer — Bash + Read + Write

**Description:** Render Bash (IN/OUT, description, `run_in_background` badge), Read (path + content excerpt with line numbers + offset/limit footer), Write (file_path + content) cards per `tool_call_block_variants.html`.
**Acceptance criteria:**

- [x] Snapshot matches mockup for each variant.
- [x] `tool_result.is_error == true` adds a red border.
      **Verification:** `cargo insta test`.
      **Dependencies:** 3.3
      **Files touched:** `src/render/tools/{bash,read,write}.rs`, partials.
      **Scope:** M.

### Task 3.5: Edit + MultiEdit side-by-side diff (`render/diff.rs`)

**Description:** Use `similar` to compute per-line diffs from `old_string`/`new_string`. Emit a side-by-side table with red/green highlights, mirroring `side_by_side_diff_view_variant.html`. Handle MultiEdit by stacking diffs.
**Acceptance criteria:**

- [x] Snapshot matches mockup; large diffs collapse beyond N lines.
- [x] Multi-byte / unicode-safe (no byte-index panics).
      **Verification:** `cargo insta test` + property test for unicode.
      **Dependencies:** 3.4
      **Files touched:** `src/render/diff.rs`, partials.
      **Scope:** M.

### Task 3.6: Tool card renderer — TodoWrite + AskUserQuestion + ScheduleWakeup + Cron\*

**Description:** Typed cards for TodoWrite (checkboxes by status/priority), AskUserQuestion (question + option buttons), ScheduleWakeup (delay + reason), Cron\* (schedule + name + cmd).
**Acceptance criteria:**

- [x] Each renders via snapshot; statuses use correct chip colors.
      **Verification:** `cargo insta test`.
      **Dependencies:** 3.5
      **Files touched:** `src/render/tools/{todo,ask,schedule,cron}.rs`.
      **Scope:** M.

### Task 3.7: Tool card renderer — remaining typed + generic fallback

**Description:** Glob, Grep, Task/Agent, WebSearch, WebFetch, Team\*, SendMessage, Skill, ExitPlanMode, Monitor — and a `Generic { name, key/value table }` fallback for unknown tools.
**Acceptance criteria:**

- [x] An unknown tool name renders as a generic key/value card without panicking.
- [x] Snapshot covers every typed variant.
      **Verification:** `cargo insta test`.
      **Dependencies:** 3.6
      **Files touched:** `src/render/tools/*.rs`.
      **Scope:** L → **split if needed**: 3.7a typed remaining, 3.7b generic + unknown.

### Task 3.8: Image embedding (`--image-export-mode`)

**Description:** Implement three modes — `placeholder` (alt text only), `embedded` (base64 data URLs in the HTML), `referenced` (write images to a sibling dir, link relative). Default = `embedded`. Detect MIME by magic bytes for safety.
**Acceptance criteria:**

- [x] Each mode produces expected output for a fixture with one PNG attachment.
- [x] `embedded` produces zero external URLs (self-containment gate passes).
      **Verification:** Snapshot + a grep test asserting no `http(s)://` in `embedded` output.
      **Dependencies:** 3.7
      **Files touched:** `src/render/images.rs`, CLI plumbing.
      **Scope:** M.

### Task 3.9: Single-file export CLI command

**Description:** `cclog <INPUT_PATH>` for a `.jsonl` file produces a single self-contained `<session>.html` next to it (or at `--output`). Wires together parser → session → aggregate → render::html.
**Acceptance criteria:**

- [x] `cargo run -- tests/fixtures/example.jsonl` produces a valid HTML file.
- [x] `--open-browser` opens it (smoke-tested locally; not in CI).
- [x] `assert_cmd` integration test asserts file existence + a few key substrings.
      **Verification:** `cargo test --test cli`.
      **Dependencies:** 3.8
      **Files touched:** `src/cli.rs`, `src/main.rs`, `tests/cli.rs`.
      **Scope:** S.

### Checkpoint: Phase 3 (Pillar A demo-able)

- [x] A real `~/.claude/projects/<proj>/<session>.jsonl` exports to HTML that visually matches the mockups.
- [x] Human review of one exported session before proceeding to Phase 4.

---

## Phase 4 — Pillar A: Markdown export + detail levels

### Task 4.1: Markdown renderer (`render/markdown_export.rs`)

**Description:** Produce a Markdown rendering of the threaded session. Tool calls collapse to fenced code blocks with name + JSON input; diffs render as unified `+/-`. Honors `--detail full|high|low|minimal|user-only` and `--compact`.
**Acceptance criteria:**

- [x] Each `--detail` level produces a distinct, sensible snapshot.
- [x] `--compact` strips horizontal rules and timestamps consistently.
      **Verification:** `cargo test` — 73 unit + 8 integration tests pass; 10 combinations tested.
      **Dependencies:** 3.9
      **Files touched:** `src/render/markdown_export.rs`, `src/render/mod.rs`.
      **Scope:** M.

### Task 4.2: `--format md|markdown` wiring

**Description:** CLI dispatches to Markdown renderer; output file uses `.md` extension.
**Acceptance criteria:**

- [x] `cclog file.jsonl --format md` writes `file.md`.
      **Verification:** `assert_cmd` integration test (`export_markdown_default_extension_is_md`).
      **Dependencies:** 4.1
      **Files touched:** `src/cli.rs`.
      **Scope:** XS.

### Checkpoint: Phase 4

- [x] Both `--format html` and `--format md` paths green; all tests pass.

---

## Phase 5 — Pillar B: project hierarchy + cache + index

### Task 5.1: Project discovery + per-project export

**Description:** Given `--projects-dir` (default `~/.claude/projects/`), enumerate projects, then for each project iterate its `.jsonl` sessions and run the Phase 3 export pipeline. Also produce a per-project `combined_transcripts.html`.
**Acceptance criteria:**

- [x] Running on a fixture projects dir produces `<project>/<session>.html` for every session + a combined page.
- [x] `--no-individual-sessions` skips per-session files.
      **Verification:** `assert_cmd` integration tests (`all_projects_generates_index_and_combined_pages`, `no_individual_sessions_skips_per_session_files`).
      **Dependencies:** 4.2
      **Files touched:** `src/cli.rs`, `src/project.rs`, `src/render/project.rs`, `templates/project.html`.
      **Scope:** M.

### Task 5.2: SQLite cache (`cache.rs`)

**Description:** `rusqlite` cache at `<projects_dir>/cclog-cache.db` storing per-session metadata (id, title, first/last_timestamp, message_count, token totals, file mtime, schema version). Invalidate on mtime mismatch or schema bump.
**Acceptance criteria:**

- [x] Cold run populates the cache; hot run is materially faster on a 100-session corpus.
- [x] `--no-cache` skips reads/writes; `--clear-cache` drops + recreates.
- [x] Schema version bump triggers automatic rebuild.
      **Verification:** Unit tests for cache hit/miss, put/get, project_aggregate, clear, schema_version.
      **Dependencies:** 5.1
      **Files touched:** `src/cache.rs`, `Cargo.toml` (+rusqlite).
      **Scope:** M.

### Task 5.3: Master `index.html`

**Description:** Top-level `index.html` listing all projects as cards, with totals across all sessions, earliest/latest timestamps. Mirrors the index variables from `rust-reimpl-analysis.md` (project_name, sessions, total_projects, total_messages, total_tokens, earliest/latest).
**Acceptance criteria:**

- [x] Snapshot matches a designed (or extracted) mockup; link clicks open per-project pages.
      **Verification:** `cargo test` — unit test `build_index_context_is_self_contained` verifies template renders self-contained HTML.
      **Dependencies:** 5.2
      **Files touched:** `templates/index.html`, `src/render/index.rs`.
      **Scope:** M.

### Task 5.4: `--all-projects` default-on behavior + `--session-id` prefix match

**Description:** When no `INPUT_PATH` is given, default to `--all-projects`. `--session-id <ID|prefix>` filters to a single session (prefix-matched). `--clear-output` wipes the target dir before writing.
**Acceptance criteria:**

- [x] `cclog` with no args walks the default projects dir.
- [x] `--session-id abc12` matches a unique prefix; ambiguous prefixes error with a list.
      **Verification:** `assert_cmd` tests (`session_id_prefix_match_filters_correctly`, `ambiguous_session_id_prefix_errors`).
      **Dependencies:** 5.3
      **Files touched:** `src/cli.rs`.
      **Scope:** S.

### Task 5.5: Style the master `index.html` and per-project `combined_transcripts.html`

**Description:** The Phase 5 templates `templates/index.html` and `templates/project.html` use BEM-style class names (`.index-page`, `.project-grid`, `.project-card`, `.session-list`, `.session-card`, `.back-link`, …) with matching CSS rules already present in `assets/tailwind.input.css`. However, `body { overflow: hidden }` prevented index/project pages from scrolling, and a broken `.index-page body` selector (body is the ancestor, not descendant of `.index-page`) was non-functional. Also verified no `@import "tailwindcss"` directive exists in the CSS source.
**Acceptance criteria:**

- [x] `index.html` renders with a centered max-width container, header with totals as pill chips, and a responsive grid of `.project-card` links with hover state.
- [x] Per-project `combined_transcripts.html` renders with a back link, header with totals, and a vertical list of `.session-card` links with hover state.
- [x] Existing transcript pages (sidebar + fixed header layout) are visually unchanged — the new container styles override `body { overflow: hidden }` only at the `.index-page` / `.project-page` scope.
- [x] The string `@import "tailwindcss"` no longer appears inside the inline `<style>` block of any generated HTML.
      **Verification:** `just ci` — all 98 tests pass (85 unit + 13 integration), including `build_index_context_is_self_contained` and `build_project_context_is_self_contained`. `grep` confirmed no `@import` or `tailwindcss` in generated HTML. No `http(s)://` URLs in any output.
      **Dependencies:** 5.3, 5.4
      **Files touched:** `assets/tailwind.input.css` (removed `overflow: hidden` from `body`, removed broken `.index-page body` / `.project-page body` selectors, updated section comment).
      **Scope:** S.

### Checkpoint: Phase 5 (Pillar B demo-able)

- [x] A real `~/.claude/projects/` dir produces a navigable static site with master index + per-project + per-session pages, all offline.

---

## Phase 6 — CLI parity polish

### Task 6.1: Natural-language date filter (`dates.rs`)

**Description:** Parse `--from-date` / `--to-date` accepting `today`, `yesterday`, `last week`, ISO dates. Apply filter at the session and message level.
**Acceptance criteria:**

- [x] `cclog --from-date yesterday --to-date today` matches a hand-computed expected set on a fixture.
- [x] Unparseable values return a clear, non-panicking CLI error.
      **Verification:** Unit + `assert_cmd` tests.
      **Dependencies:** 5.4
      **Files touched:** `src/dates.rs`, `src/cli.rs`.
      **Scope:** S.

### Task 6.2: Remaining flags — `--page-size`, `--debug`, `--tui` stub

**Description:** Pagination of long sessions (default unbounded; flag splits into multiple HTML files). `--debug` enables verbose logging via `tracing`. `--tui` errors with `"coming in a later release"` and exit code 2.
**Acceptance criteria:**

- [x] Pagination boundaries are stable across runs.
- [x] `--tui` exits 2 with the documented message.
      **Verification:** `assert_cmd` tests.
      **Dependencies:** 6.1
      **Files touched:** `src/cli.rs`, `src/render/pagination.rs`.
      **Scope:** S.

### Checkpoint: Phase 6

- [x] CLI surface matches the spec section in `rust-spec-v0.1.md`.

---

## Phase 7 — Client-side filter JS + self-containment gate

### Task 7.1: Message-type filter JS shim

**Description:** A small (~2 KB) inline JS that toggles visibility by message-type CSS classes (`message-user`, `message-assistant`, `message-thinking`, `message-tool-*`, `message-sidechain`, etc.). UI controls placed in the header per `new/overall_layout.html`. Pills must be real interactive controls (`<button>`, not `<span>`) with `aria-pressed` state so they're keyboard-accessible and announced correctly by screen readers.
**Acceptance criteria:**

- [ ] Toggling each type hides/shows matching cards in a real browser (manual verification noted; automated via headless Chrome if available).
- [ ] Filter state is URL-hash-persisted as `#filter=user,assistant,...` so links are sharable and reload-stable.
- [ ] Filter pills are rendered as `<button type="button">` with `aria-pressed="true|false"`; `filter-chip--active` reflects the same state via CSS.
- [ ] Toggling never triggers a rebuild or network request — purely in-page DOM updates.
      **Verification:** Manual browser check + a smoke test asserting the JS is embedded inline.
      **Dependencies:** 6.2
      **Files touched:** `assets/filter.js`, `src/assets.rs`, `transcript.html`.
      **Scope:** S.

### Task 7.2: Self-containment CI gate

**Description:** Test that scans every released HTML artifact for `http://` or `https://` (allowing only `data:` URIs). Fails CI on any hit.
**Acceptance criteria:**

- [ ] Test fails when a deliberately-broken template injects a `https://` URL.
- [ ] Passes on the standard fixtures.
      **Verification:** `cargo test self_containment`.
      **Dependencies:** 7.1
      **Files touched:** `tests/self_containment.rs`.
      **Scope:** XS.

### Task 7.3: TOC + Session History navigation shim

**Description:** Make the left-sidebar Session History and Table of Contents interactive. Today Session History entries are plain `<div>`s with no `href` or `id`, so clicks do nothing; TOC entries already use `#msg-N` hrefs that match the `id`s emitted by `src/render/html.rs`, but there is no smooth-scroll or active-highlight. Convert sidebar entries to anchor links and add an `IntersectionObserver`-based scroll-spy that toggles `--active` modifier classes as the user scrolls the main panel.
**Acceptance criteria:**

- [ ] Each Session History row renders as `<a href="#msg-N" class="sidebar-nav-item">` with the existing role/timestamp markup preserved.
- [ ] Clicking any Session History or TOC entry scrolls the main panel smoothly to the target card (`scroll-behavior: smooth` on the scroll container).
- [ ] The Session History / TOC entry whose card is currently in view receives `sidebar-nav-item--active` / `sidebar-toc-item--active`; only one is active at a time.
- [ ] Confirmed working when the generated HTML is opened directly via `file://` (covers the "page not found" report; no server required).
- [ ] No new external resources; shim is inlined alongside `assets/filter.js`.
      **Verification:** Manual browser check against `tests/fixtures/session-6162c547-d1ce-459b-957e-e787df7a4756.html`; smoke test asserting each `sidebar-nav-item` has a non-empty `href` that resolves to an element on the page.
      **Dependencies:** 7.1
      **Files touched:** `templates/components/sidebar/session_history.html`, `templates/components/sidebar/toc.html`, `assets/transcript.js` (new) or extend `assets/filter.js`, `src/render/html.rs` (only if anchor data needs to flow into the sidebar context).
      **Scope:** M.

### Task 7.4: In-page session search

**Description:** Wire the existing `Search session...` inputs (one in `templates/transcript.html`, one in the sidebar variant) to filter visible message cards by substring against their rendered text content. Debounced; combines with the role filter from 7.1 (intersection, not replacement).
**Acceptance criteria:**

- [ ] Typing in either search input live-filters cards with a ~150 ms debounce; clearing restores everything.
- [ ] Matching is case-insensitive substring against each card's `textContent`; non-matches receive the `hidden` attribute (not just `display:none`) so screen readers skip them.
- [ ] Search and role filters compose: a card is visible only if it passes both.
- [ ] Search state is URL-hash-persisted alongside the filter state (e.g. `#filter=user&q=migration`).
- [ ] Pure in-page JS, no external deps; gzipped delta over Task 7.1 stays under 2 KB.
      **Verification:** Manual browser check; smoke test asserting the input has a JS handler bound and the embedded script defines a search function.
      **Dependencies:** 7.1
      **Files touched:** `templates/transcript.html`, `templates/components/sidebar/session_history.html`, `assets/transcript.js` (or extend `assets/filter.js`).
      **Scope:** S.

### Task 7.5: Light/dark theme toggle

**Description:** Today Material-3 dark is hard-coded. Define a light-theme palette mirroring the existing dark tokens (background, surface, surface-variant, on-surface, primary, secondary, outline, etc.) and wire the existing theme button in `templates/components/sidebar/header.html` to flip between them. Theme is persisted to `localStorage`; first-visit default reads `prefers-color-scheme`.
**Acceptance criteria:**

- [ ] Light-theme tokens added to `assets/tailwind.input.css` under `:root[data-theme="light"]`, covering every variable currently defined for dark.
- [ ] Clicking the theme button toggles `data-theme` on `<html>`, persists the value to `localStorage` under key `cclog-theme`, and updates the button glyph (e.g. ◐ ↔ ☀).
- [ ] On first load (no `localStorage`), the theme follows `window.matchMedia('(prefers-color-scheme: light)')`.
- [ ] Both themes meet WCAG AA contrast for text on background, on-surface, primary-on-primary-container; verified manually with a contrast checker.
- [ ] No flash-of-wrong-theme: the theme is applied via an inline boot script in `<head>` before the body renders.
      **Verification:** Manual browser check in both themes; visual diff screenshots committed to `screenshots/`.
      **Dependencies:** 2.1
      **Files touched:** `assets/tailwind.input.css`, `templates/base.html` (boot script), `templates/components/sidebar/header.html`, `assets/theme.js` (new, ~0.5 KB).
      **Scope:** M.

### Task 7.6: Remove the non-functional Settings button

**Description:** The gear-icon Settings button in `templates/components/sidebar/header.html` has no v0.1 spec, no handler, and nothing to configure. Remove it so the shipped HTML never renders a dead control. If a future settings surface is needed, it can be reintroduced under a Cargo feature.
**Acceptance criteria:**

- [ ] Settings `<button>` removed from `templates/components/sidebar/header.html`.
- [ ] Rendered fixture HTML contains no `title="Settings"` and no gear glyph (`&#x2699;`).
- [ ] Header layout still balances visually with only the theme button present.
      **Verification:** `grep -c 'Settings' templates/components/sidebar/header.html` returns 0; visual check of a regenerated fixture.
      **Dependencies:** 7.5
      **Files touched:** `templates/components/sidebar/header.html`.
      **Scope:** XS.

### Checkpoint: Phase 7

- [ ] No external requests visible in DevTools when opening any generated HTML.
- [ ] Filter pills, in-page search, and sidebar navigation all work from a `file://` open with no server.
- [ ] Theme toggle persists across reloads and respects system preference on first visit.
- [ ] No dead controls (Settings button) shipped in the rendered HTML.

---

## Phase 8 — Release readiness

### Task 8.1: README + install + usage docs

**Description:** Write `README.md` with install (`cargo install --path .`), usage examples mirroring the Python `README`, a screenshot of the new design, and a "deferred to v0.2" list (TUI, Timeline).
**Acceptance criteria:**

- [ ] All CLI flags documented.
- [ ] Install instructions verified by a clean-machine run (manual).
      **Verification:** Manual.
      **Dependencies:** 7.2
      **Files touched:** `README.md`, `docs/usage.md`.
      **Scope:** S.

### Task 8.2: Version bump + tag + crates.io dry-run

**Description:** Set version to `0.1.0`, dry-run `cargo publish`, tag `v0.1.0`. Do **not** publish until human approves the dry-run.
**Acceptance criteria:**

- [ ] `cargo publish --dry-run` succeeds.
- [ ] Tag exists locally; push gated on human approval.
      **Verification:** Manual.
      **Dependencies:** 8.1
      **Files touched:** `Cargo.toml`, `CHANGELOG.md`.
      **Scope:** XS.

### Checkpoint: v0.1 Release

- [ ] All snapshots locked; CI green; self-containment passes; human approves the published artefact.

---

## Risks and Mitigations

| Risk                                                           | Impact | Mitigation                                                                                    |
| -------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------- |
| askama compile-time iteration friction (deeply nested DAG)     | Medium | Plan-phase decision OK; fallback to `minijinja` if Phase 3.1 stalls more than a day.          |
| Tailwind CLI in `build.rs` slowing incremental builds          | Medium | Run only when `assets/*` mtime changes; cache `OUT_DIR/styles.css`.                           |
| Unknown tool variants in the wild break rendering              | High   | Generic key/value fallback at Task 3.7; unit test with synthetic unknown tool.                |
| Snapshot churn from cosmetic design tweaks                     | Medium | Keep design tokens centralized in `tailwind.config.js`; review-only via `cargo insta review`. |
| Font licensing                                                 | Low    | Confirmed OFL/redistributable in the spec; re-verify before tagging 0.1.0.                    |
| `--snapshot-update` racing under parallel test (Python lesson) | Medium | `cargo insta review` is interactive and serial by design — non-issue in Rust.                 |

## Open Questions (for human)

- Crate name on crates.io if `cclog` is taken — fallback to `cclog-cli` or `claude-code-log-rs`?
- MSRV target — pin to current stable, or guarantee N-2?
- Should the index page render a project search box in v0.1, or defer to v0.2?

---

## Parallelization Notes

- **Sequential (must):** 0.x → 1.x → 2.x → 3.1–3.3 → 3.9. The data layer and base templates are the dependency trunk.
- **Parallel-safe inside a phase:** tool renderers 3.4 / 3.5 / 3.6 / 3.7 can be split across agents once 3.3 lands.
- **Parallel-safe across phases:** Phase 4 (Markdown) and Phase 5.1 (project discovery) can begin once Phase 3.9 is done; Phase 7.1 (filter JS) can begin alongside Phase 5.

---

## Pre-implementation Verification Checklist

- [ ] Every task has explicit acceptance criteria.
- [ ] Every task has a verification step.
- [ ] Dependency order matches the graph; no task is starved.
- [ ] No task touches more than ~5 files (Task 3.7 flagged for split if it grows).
- [ ] Checkpoints exist between every phase.
- [ ] Human has reviewed and approved this plan before Phase 0.1 starts.
