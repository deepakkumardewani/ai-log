# cclog

A fast, self-contained Rust CLI that converts Claude Code transcript JSONL files into beautiful HTML and Markdown.

`cclog` is a Rust reimplementation of [claude-code-log](https://github.com/daaain/claude-code-log), focused on speed, zero-dependency output artefacts, and a clean Material 3 dark theme.

## Quickstart

```sh
cclog -i ~/.claude/projects/my-project/session.jsonl
```

Opens (or writes) a fully self-contained `session.html` — no CDN URLs, no external fonts, no JavaScript dependencies.

## Key Features

- **Self-Contained HTML**: Every output file embeds fonts, CSS tokens, and assets inline — nothing phones home
- **Material 3 Dark Theme**: Clean, minimalist dark UI using Material Design 3 colour tokens and typography
- **Markdown Export**: Lightweight portable alternative to HTML, compatible with GitHub, GitLab, and LLM context windows
- **Detail Levels**: `--detail full|high|low|minimal|user-only` — filter verbosity from everything down to user prompts only
- **Compact Mode**: `--compact` strips timestamps and horizontal rules; pairs with `--detail low` for feeding past sessions to an LLM
- **Rich Tool Rendering**: Bash commands, file reads/writes, diffs (unified `+/-`), MultiEdit, Glob, Grep, TodoWrite, and more rendered with full context
- **Thinking Block Support**: Collapsible extended thinking sections
- **Token Usage Tracking**: Per-message and per-session input/output token counts
- **Zero-Config**: Sensible defaults — just point it at a JSONL file
- **Multi-Project Export**: Export all your Claude Code projects at once with `cclog --all-projects`, generating a static navigable site with master index + per-project pages
- **SQLite Cache**: Session metadata cached for fast incremental rebuilds; `--clear-cache` and `--no-cache` flags for control
- **Interactive HTML Output**: Client-side message-type filter chips with URL-hash persistence, in-page search with 150ms debounce, and a light/dark theme toggle persisted to localStorage. Index page includes project search, date range filter with presets, and card/list view toggle with localStorage persistence
- **Fast**: Single-pass JSONL parser with a session DAG built in memory; typical sessions export in milliseconds

## Installation

### From source (current)

```sh
git clone https://github.com/deepakdewani1/cclog.git
cd cclog
cargo build --release
# binary is at target/release/cclog
```

### Requirements

- Rust 1.80+
- Optional: Tailwind CLI (for rebuilding CSS tokens; a pre-built fallback is embedded)

## Usage

### Export a session to HTML (default)

```sh
cclog export session.jsonl
# writes session.html
```

```sh
cclog export session.jsonl -o /tmp/review.html --open-browser
```

### Shorthand with `-i`

```sh
cclog -i session.jsonl
```

### Export to Markdown

```sh
cclog export session.jsonl --format md
# writes session.md
```

### Detail levels

```sh
cclog export session.jsonl --format md --detail full       # everything (default)
cclog export session.jsonl --format md --detail high       # messages + tool calls, no thinking
cclog export session.jsonl --format md --detail low        # messages only, no tool calls
cclog export session.jsonl --format md --detail minimal    # user + assistant text only
cclog export session.jsonl --format md --detail user-only  # user prompts only
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
cclog export session.jsonl --format md --detail low --compact -o context.md
```

`--compact` merges repeated same-type headings so runs of assistant turns share one `### Assistant` instead of repeating it for each message — significantly reduces token count.

### Export all projects

```sh
cclog --all-projects
# walks ~/.claude/projects/ and generates cclog-out/
#   index.html
#   <project>/combined_transcripts.html
#   <project>/<session>.html
```

```sh
cclog --all-projects --projects-dir /path/to/projects --output-dir ./out
```

### Filter by session ID

```sh
cclog --all-projects --session-id 6162c547
# exports only the matching session (prefix match)
```

### Date-filtered export

```sh
cclog export session.jsonl --from-date yesterday --to-date today
cclog --all-projects --from-date 2025-06-01 --to-date 2025-06-30
```

Accepts: `today`, `yesterday`, `last week`, `last month`, and ISO dates (`YYYY-MM-DD`). Sessions whose timestamp range doesn't overlap the filter window are skipped.

### Paginated output

```sh
cclog export session.jsonl --page-size 50
# splits into session-page-1.html, session-page-2.html, ...
```

Long sessions can be split across multiple HTML files with `--page-size N` (messages per page). The first page includes the full chrome; subsequent pages are content-only.

### Cache control

```sh
cclog --all-projects --no-cache      # skip cache entirely
cclog --all-projects --clear-cache   # drop and rebuild cache
```

### Debug logging

```sh
cclog --debug export session.jsonl   # enable tracing output
cclog --debug --all-projects         # verbose logging for all-projects mode
```

### Combined pages only (skip per-session HTML)

```sh
cclog --all-projects --no-individual-sessions
```

### Open in browser after export

```sh
cclog export session.jsonl --open-browser
```

## Output Formats

### HTML

- Dark Material 3 theme with embedded design tokens
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

## Project Status

`cclog` is in active development. Completed phases:

- [x] Phase 0 — Scaffolding (Cargo, CI, Tailwind build.rs)
- [x] Phase 1 — Data layer (models, JSONL parser, session DAG, aggregation)
- [x] Phase 2 — Templates and assets (Material 3 tokens, Askama base)
- [x] Phase 3 — Single-session HTML export with full tool rendering
- [x] Phase 4 — Markdown export with detail levels and compact mode
- [x] Phase 5 — Project hierarchy + master index + SQLite cache
- [x] Phase 6 — CLI parity (date filters, pagination, `--debug`, `--tui` stub)
- [x] Phase 7 — Client-side interactivity (filter chips, in-page search, sidebar navigation, theme toggle)

Coming next:

- [ ] Phase 8 — Release packaging, `cargo install`, crates.io publish

## Development

```sh
just ci          # fmt → clippy (-D warnings) → test
just test        # cargo test only
cargo build      # debug build
cargo build --release
```

All CI gates are enforced via `just ci`. The self-containment gate (`self_contained_output_no_external_urls`) runs as part of the integration test suite.

## Related Projects

- **[claude-code-log](https://github.com/daaain/claude-code-log)** — the original Python implementation; includes TUI, project hierarchy, timeline, and `uvx` quickstart

## License

MIT
