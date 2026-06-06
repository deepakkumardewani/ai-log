# Changelog

## [0.2.0] — Unreleased

### Added
- Renamed from `cclog` to `weavr` — binary, crate, output dir, cache, branding
- `weavr self-update` command — update to the latest GitHub Release
- Passive update notice on startup (throttled to once per 24 h, opt-out via `WEAVR_NO_UPDATE_CHECK`)
- Shell installer (`install.sh`) for `curl | sh` quickstart
- Homebrew tap at `deepakkumardewani/homebrew-weavr`
- `cargo-binstall` metadata for prebuilt binary installs
- GitHub Release CI workflow via cargo-dist (macOS ARM/Intel + Linux x86_64)
- Performance optimizations: halved `TranscriptEntry` clones, single-pass HTML escape
- Coverage gate at 80% in CI (`just coverage`)
- `hyperfine` benchmark harness (`just bench`) — 18× faster than Python `claude-code-log`
- Comprehensive test suite: 281 tests, 88% line coverage

### Changed
- Default output directory: `cclog-out` → `weavr-out`
- Cache database: `cclog-cache.db` → `weavr-cache.db`
- Theme/localStorage keys: `cclog-theme` → `weavr-theme`
- CSS class names: `cclog-tooltip` → `weavr-tooltip`
- Modal ID: `cclog-modal` → `weavr-modal`
- Page titles: `— cclog` → `— weavr`

---

## [0.1.0-dev] — 2025

### Initial release (as `cclog`)

- Single-session HTML export with Material 3 dark theme
- Markdown export with detail levels (full/high/low/minimal/user-only) + compact mode
- Project hierarchy with master index + per-project combined pages
- SQLite cache for fast incremental rebuilds
- Rich tool rendering: Bash, Read, Write, Edit, MultiEdit, Glob, Grep, Task/Agent, and more
- Syntax highlighting via syntect
- Side-by-side unified diffs for Edit/MultiEdit
- Client-side interactivity: filter chips, in-page search, sidebar navigation, light/dark theme toggle
- Self-contained output (zero CDN URLs, verified by CI gate)
- Date range filtering with natural language (`today`, `yesterday`, `last week`)
- Pagination for long sessions
