# weavr

A fast, self-contained Rust CLI that converts Claude Code transcript JSONL files into beautiful HTML and Markdown.

`weavr` is a Rust reimplementation of [claude-code-log](https://github.com/daaain/claude-code-log), focused on speed, zero-dependency output artefacts, and a clean Material 3 design.

## Why weavr?

The excellent [claude-code-log](https://github.com/daaain/claude-code-log) proved how useful it is to turn raw Claude Code transcripts into something you can actually read. weavr started as a personal itch with that workflow and grew into a ground-up rewrite around two goals:

- **Speed.** Transcripts get large fast. A Python tool re-parses and re-renders the whole archive on every run; weavr is a single native binary with a single-pass JSONL parser and an in-memory session DAG, plus a SQLite cache for incremental rebuilds. In practice it exports **18–46× faster** (see [Performance](#performance)) — fast enough to regenerate your entire history in the time it takes to alt-tab.
- **Design.** weavr ships a deliberate Material 3 light/dark theme with a flat dot-timeline that reads like a chronological event stream, rich inline tool rendering (Bash IN/OUT, Read/Edit/Write diffs, modals for Skill/Agent), and **fully self-contained output** — every HTML file embeds its fonts, CSS, and JS so nothing ever phones home. A CI gate rejects any `http(s)://` URL in the output.

It's also trivially installable as a single binary (`brew`, `cargo binstall`, `cargo install`, shell installer) with built-in `self-update` — no Python runtime, no virtualenv.

### weavr vs. claude-code-log

| | **weavr** (Rust) | **claude-code-log** (Python) |
|---|---|---|
| Speed (all projects) | **~1.3 s** | ~24 s |
| Speed (single session) | **~28 ms** | ~1.3 s |
| Distribution | single static binary (brew / binstall / cargo / shell) | `uvx` / `pip` (needs Python) |
| Self-contained output | ✅ zero external URLs (CI-enforced) | partial |
| Incremental rebuilds | ✅ SQLite cache | ⚠️ re-renders each run |
| Theme | Material 3 light/dark, dot-timeline | minimalist HTML |
| HTML export | ✅ | ✅ |
| Markdown export + detail levels + `--compact` | ✅ | ✅ |
| Token usage tracking | ✅ | ✅ |
| Date-range filtering (natural language) | ✅ | ✅ |
| Client-side filter chips + in-page search | ✅ | ✅ (runtime filtering) |
| Multi-project hierarchy + master index | ✅ | ✅ |
| Self-update command | ✅ | — |
| Interactive TUI | ⏳ planned | ✅ |
| Interactive zoomable timeline | ⏳ planned | ✅ |
| Image rendering | ⏳ planned | ✅ |
| Windows | ⏳ planned | ✅ |

Both tools share the same JSONL input format and the `--detail` / `--compact` philosophy — weavr trades a couple of claude-code-log's richer browsing features (TUI, zoomable timeline) for raw speed, a self-contained artefact, and a single-binary install. If you live in the terminal and want an interactive TUI today, claude-code-log is great. If you want the fastest possible export and offline-portable HTML, reach for weavr.

## Quickstart

```sh
weavr -i ~/.claude/projects/my-project/session.jsonl
```

Opens (or writes) a fully self-contained `session.html` — no CDN URLs, no external fonts, no JavaScript dependencies.

## Key Features

- **Self-Contained HTML**: Every output file embeds fonts, CSS tokens, and assets inline — nothing phones home
- **Light/Dark Themes**: Clean, minimalist UI using warm-neutral design tokens; dark by default with a toggleable light theme persisted to localStorage
- **Markdown Export**: Lightweight portable alternative to HTML, compatible with GitHub, GitLab, and LLM context windows
- **Detail Levels**: `--detail full|high|low|minimal|user-only` — filter verbosity from everything down to user prompts only
- **Compact Mode**: `--compact` strips timestamps and horizontal rules; pairs with `--detail low` for feeding past sessions to an LLM
- **Flat Dot-Timeline**: Conversation rendered as a chronological event stream — user messages as muted blocks, assistant text and thinking as gray dot-rows, tool calls as green dot-rows
- **Rich Tool Rendering**: Bash IN/OUT sections, Read filename → file-contents modal, Edit/Write/MultiEdit unified diff with intra-line word highlights, Skill → modal, Agent rows with IN prompt
- **Thinking Block Support**: Inline-expand thinking rows; empty thinking shown as a disabled pill
- **Token Usage Tracking**: Per-message and per-session input/output token counts
- **Zero-Config**: Sensible defaults — just point it at a JSONL file
- **Multi-Project Export**: Export all your Claude Code projects at once with `weavr --all-projects`, generating a static navigable site with master index + per-project pages
- **SQLite Cache**: Session metadata cached for fast incremental rebuilds; `--clear-cache` and `--no-cache` flags for control
- **Interactive HTML Output**: Client-side message-type filter chips with URL-hash persistence, in-page search with 150ms debounce, and a light/dark theme toggle persisted to localStorage. Index page includes project search, date range filter with presets, and card/list view toggle with localStorage persistence
- **Fast**: Single-pass JSONL parser with a session DAG built in memory; typical sessions export in milliseconds

## Installation

### Via shell installer (macOS, Linux)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/deepakkumardewani/weavr/main/install.sh | sh
```

Downloads the latest prebuilt binary for your platform from GitHub Releases.

### Via Homebrew (macOS)

```sh
brew install deepakkumardewani/weavr/weavr
```

### Via cargo-binstall

```sh
cargo binstall weavr
```

Fetches the latest prebuilt binary from GitHub Releases — no compilation needed.

### Via cargo install

```sh
cargo install weavr
```

Builds from source on crates.io. Requires Rust 1.80+.

### Direct download

Prebuilt binaries for each platform are published on the [GitHub Releases](https://github.com/deepakkumardewani/weavr/releases) page. Download the `.tar.gz` for your platform, extract, and place the `weavr` binary on your `PATH`.

| Platform | Target triple |
|----------|--------------|
| macOS (Apple Silicon) | `aarch64-apple-darwin` |
| Linux (x86_64) | `x86_64-unknown-linux-gnu` |

> Intel macOS has no prebuilt binary — install via `cargo install weavr`, or run the Apple Silicon build under Rosetta 2.

### From source

```sh
git clone https://github.com/deepakkumardewani/weavr.git
cd weavr
cargo build --release
# binary is at target/release/weavr
```

### Updating

```sh
weavr self-update       # update to the latest GitHub Release
```

A passive notice is printed when a newer version is available (throttled to once per 24 h). Set `WEAVR_NO_UPDATE_CHECK=1` to disable.

Package-manager installs (brew, cargo install) should use their native update commands:

```sh
brew upgrade weavr                  # Homebrew
cargo install --force weavr         # cargo install
cargo binstall --force weavr        # cargo-binstall
```

### Requirements

- Rust 1.80+ (for source builds only; prebuilt binaries have no dependencies)
- Optional: Tailwind CLI (for rebuilding CSS tokens; a pre-built fallback is embedded)

## Usage

### Export a session to HTML (default)

```sh
weavr export session.jsonl
# writes session.html
```

```sh
weavr export session.jsonl -o /tmp/review.html --open-browser
```

### Shorthand with `-i`

```sh
weavr -i session.jsonl
```

### Export to Markdown

```sh
weavr export session.jsonl --format md
# writes session.md
```

### Detail levels

```sh
weavr export session.jsonl --format md --detail full       # everything (default)
weavr export session.jsonl --format md --detail high       # messages + tool calls, no thinking
weavr export session.jsonl --format md --detail low        # messages only, no tool calls
weavr export session.jsonl --format md --detail minimal    # user + assistant text only
weavr export session.jsonl --format md --detail user-only  # user prompts only
```

`--detail` levels at a glance (smallest → largest output):

| Level | Includes |
|-------|----------|
| `user-only` | User prompts only — good input for an agent building a requirements doc |
| `minimal` | User + assistant text |
| `low` | + Key tool signals (WebSearch, WebFetch, Task delegations) |
| `high` | + All tool calls; drops thinking blocks and system metadata |
| `full` | Everything — thinking, system entries, all tool calls (default) |

### Feeding a past session to an LLM

```sh
weavr export session.jsonl --format md --detail low --compact -o context.md
```

`--compact` merges repeated same-type headings so runs of assistant turns share one `### Assistant` instead of repeating it for each message — significantly reduces token count.

### Export all projects

```sh
weavr --all-projects
# walks ~/.claude/projects/ and generates weavr-out/
#   index.html
#   <project>/combined_transcripts.html
#   <project>/<session>.html
```

```sh
weavr --all-projects --projects-dir /path/to/projects --output-dir ./out
```

### Filter by session ID

```sh
weavr --all-projects --session-id 6162c547
# exports only the matching session (prefix match)
```

### Date-filtered export

```sh
weavr export session.jsonl --from-date yesterday --to-date today
weavr --all-projects --from-date 2025-06-01 --to-date 2025-06-30
```

Accepts: `today`, `yesterday`, `last week`, `last month`, and ISO dates (`YYYY-MM-DD`). Sessions whose timestamp range doesn't overlap the filter window are skipped.

### Paginated output

```sh
weavr export session.jsonl --page-size 50
# splits into session-page-1.html, session-page-2.html, ...
```

Long sessions can be split across multiple HTML files with `--page-size N` (messages per page). The first page includes the full chrome; subsequent pages are content-only.

### Wipe output directory

```sh
weavr --all-projects --clear-output   # delete output dir before writing
```

### Cache control

```sh
weavr --all-projects --no-cache      # skip cache entirely
weavr --all-projects --clear-cache   # drop and rebuild cache
```

### Design verification stub

```sh
weavr stub -o design-review.html    # emit a stub transcript for design review
```

### Debug logging

```sh
weavr --debug export session.jsonl   # enable tracing output
weavr --debug --all-projects         # verbose logging for all-projects mode
```

### Combined pages only (skip per-session HTML)

```sh
weavr --all-projects --no-individual-sessions
```

### Open in browser after export

```sh
weavr export session.jsonl --open-browser
```

## Output Formats

### HTML

- Warm-neutral light/dark theme with embedded design tokens
- Tool calls rendered as collapsible cards with syntax highlighting
- Unified diff view for `Edit` / `MultiEdit` tool calls
- Token usage displayed per message and as a session total
- Thinking blocks in expandable sections
- 100% self-contained — verified by a CI test that rejects any `http(s)://` URL in the output

### Markdown

- GitHub-Flavored Markdown compatible
- Tool calls collapse to fenced code blocks with the tool name and key parameters
- Diffs rendered as `\`\`\`diff` blocks with unified `+/-` hunks
- `--compact` and `--detail` work orthogonally

## Performance

Benchmarked with [`hyperfine`](https://github.com/sharkdp/hyperfine) (warmup + multiple runs, no cache) on Apple Silicon against `claude-code-log` over the same input. Re-run any time with `just bench`.

| Mode | weavr (Rust) | claude-code-log (Python) | Speedup |
|------|--------------|--------------------------|---------|
| All projects (160 sessions, 97 MB) | **1.32 s** | 24.10 s | **18.2×** |
| Single project (42 sessions) | **466 ms** | 9.68 s | **20.8×** |
| Single session (19 MB, ~500 msgs) | **27.5 ms** | 1.28 s | **46.5×** |

The gap is widest on small inputs, where Python's startup and import overhead dominate; even on large I/O-bound runs weavr stays ~18× ahead. Full methodology and the optimizations behind these numbers live in [agent_docs/weavr-bench-results.md](agent_docs/weavr-bench-results.md).

## Roadmap

### Done (v1.0)

- [x] Single-session HTML export with full, rich tool rendering
- [x] Markdown export with detail levels (`full`/`high`/`low`/`minimal`/`user-only`) + `--compact`
- [x] Project hierarchy + master index + per-project combined pages
- [x] SQLite cache for fast incremental rebuilds
- [x] Material 3 light/dark theme with flat dot-timeline
- [x] Client-side interactivity — filter chips, in-page search, theme toggle, sidebar nav
- [x] Date-range filtering with natural language (`today`, `yesterday`, `last week`)
- [x] Pagination for long sessions
- [x] Self-contained output (zero external URLs, CI-enforced)
- [x] Multi-channel install (brew, binstall, cargo, shell) + `self-update`

### Planned

- [ ] **Interactive TUI** — `--tui` currently exits with "coming in a later release"
- [ ] **Interactive timeline** — zoomable, time-grouped view of message activity
- [ ] **Image rendering** — inline display of images embedded in transcripts
- [ ] **JSON export** — `--format json` for programmatic consumption
- [ ] **Windows support** — currently macOS and Linux only
- [ ] **Cost estimation** — per-session API cost breakdown

## Development

```sh
just ci          # fmt → clippy (-D warnings) → test
just coverage    # run cargo-llvm-cov and print summary (CI gate: >= 80%)
just bench       # run hyperfine benchmark vs claude-code-log
just test        # cargo test only
cargo build      # debug build
cargo build --release
```

All CI gates are enforced via `just ci` plus the coverage gate in `.github/workflows/ci.yml`. The self-containment gate (`self_contained_output_no_external_urls`) runs as part of the integration test suite.

## Related Projects

- **[claude-code-log](https://github.com/daaain/claude-code-log)** — the original Python implementation; includes TUI, project hierarchy, timeline, and `uvx` quickstart

## License

MIT
