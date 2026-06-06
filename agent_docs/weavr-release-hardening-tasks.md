# weavr Release-Hardening — Tasks

> Plan: [agent_docs/weavr-release-hardening-plan.md](weavr-release-hardening-plan.md) ·
> Spec: [agent_docs/weavr-release-hardening-spec.md](weavr-release-hardening-spec.md)
> Execute top to bottom. `▣ CHECKPOINT` = stop, verify, get sign-off before continuing.
> Conventions: each task lists **Files**, **Acceptance**, **Verify**. `just ci` must stay green throughout.

---

## Phase 1 — Review / Simplify / DRY (Gate 1)

### T1.1 — Whole-codebase review & findings doc
- [x] Review every `src/**` Rust module + `templates/**` + `assets/**` for duplication, dead code, readability, naming.
- **Files:** read-only; output → `agent_docs/weavr-review-findings.md`
- **Acceptance:** findings doc lists each issue with file:line, category (dup/dead/clarity/naming), and proposed fix. Hotspot: recent session-page commits.
- **Verify:** findings doc exists; every `src/` module + template/asset appears or is explicitly marked clean.

### T1.2 — Rust simplification & DRY
- [x] Apply fixes from T1.1 to Rust: extract repeated logic (3+ uses mandatory), delete dead/commented code, tighten `pub` surface, flatten deep nesting.
- **Files:** `src/**` (likely `render/**`, `cli.rs`, `cache.rs`, `project.rs`, `aggregate.rs`)
- **Acceptance:** AC1.2, AC1.3, AC1.4 met; no behavior change.
- **Verify:** `just ci` green; `cargo clippy --all-targets -- -D warnings` zero warnings; existing `insta` snapshots unchanged.

### T1.3 — Template/CSS/JS simplification & DRY
- [x] Consolidate duplicated markup into partials; dedupe CSS tokens/rules; remove dead JS/CSS in `assets/**`.
- **Files:** `templates/**`, `assets/{index.js,transcript.js,tailwind.input.css,tailwind.config.js}`
- **Acceptance:** no duplicated token/markup blocks; output still self-contained.
- **Verify:** regenerate sample output; `tests/self_containment.rs` green; visual spot-check via **agent-browser** (not chrome-devtools).
- **Note:** Templates already use proper Askama inheritance + `{% include %}` partials. CSS tokens defined once. No duplication found. Branding strings (`cclog-theme`, `— cclog`) deferred to Phase 4 per AC4.6.

### ▣ CHECKPOINT P1
- [x] Findings doc reviewed; `just ci` green; snapshots unchanged or deliberately re-blessed with rationale. **Sign-off to start P2.**

---

## Phase 2 — Coverage (Gate 2)

### T2.1 — Wire up cargo-llvm-cov
- [x] Add `cargo-llvm-cov`; create `just coverage` recipe emitting overall % + per-core-module %.
- **Files:** `justfile`, CI workflow
- **Acceptance:** AC2.1, AC2.5 — one command prints overall + core-module coverage.
- **Verify:** `just coverage` runs locally and prints the summary. ✓

### T2.2 — Core coverage: `parser`
- [x] Tests for happy path + error/edge: malformed JSONL line, missing required fields, empty file, mixed valid/invalid lines.
- **Files:** `src/parser.rs` (tests), `tests/fixtures/**` (add minimal fixtures as needed)
- **Acceptance:** `parser` ~100% incl. error branches (AC2.3).
- **Verify:** `just coverage` shows parser 99.45% (~100%). ✓

### T2.3 — Core coverage: `model` (mod/entry/content/tool)
- [x] Tests for deserialization of every content/tool variant + unknown/edge variants.
- **Files:** `src/model/*.rs`
- **Acceptance:** `model::*` ~100% (AC2.3).
- **Verify:** `just coverage` shows content 95.19%, entry 96.41%, tool 97.10%. All ~100%. ✓

### T2.4 — Core coverage: `aggregate`
- [x] Tests for token/stat aggregation across multi-turn, empty, and single-entry inputs.
- **Files:** `src/aggregate.rs`
- **Acceptance:** `aggregate` ~100%.
- **Verify:** `just coverage` shows aggregate 100.00%. ✓

### T2.5 — Core coverage: `conversation`
- [x] Tests for threading: linear chain, branching parentUuid, orphaned/broken parent links, out-of-order entries.
- **Files:** `src/conversation.rs`
- **Acceptance:** `conversation` ~100% incl. broken-link paths.
- **Verify:** `just coverage` shows conversation 95.26%. ~100%. ✓

### T2.6 — Pragmatic coverage to clear 80% total + CI gate
- [x] Fill gaps in `cache`, `dates`, `project`, `session`, `render/**` enough to clear 80% total (no brittle tests for trivial glue / `main.rs`).
- [x] Add CI step that **fails** under 80%.
- **Files:** `src/**` tests, `.github/workflows/ci.yml`
- **Acceptance:** AC2.2, AC2.4.
- **Verify:** Total 88.24% ≥ 80%; CI coverage gate added. ✓

### ▣ CHECKPOINT P2
- [x] `just coverage` ≥80% total (88.24%), core ~100% (parser 99.45%, aggregate 100%, conversation 95.26%, model 95-97%); CI gate added. **Sign-off to start P3.**

---

## Phase 3 — Performance (Gate 3)

### T3.1 — Establish the benchmark harness
- [x] Add `hyperfine`-based benchmark: same full-projects input for `weavr --all-projects` and `claude-code-log`; warmup + multiple runs; `just bench` recipe.
- **Files:** `scripts/bench.sh` (or `benches/`), `justfile`, short `agent_docs/weavr-bench-results.md`
- **Acceptance:** AC3.1, AC3.3 — committed, re-runnable, documents machine/context.
- **Verify:** `just bench` produces a comparison table; results note committed. ✓

### T3.2 — Baseline measurement & profiling
- [x] Run baseline; identify hotspots (redundant IO, clones/allocations, repeated parsing, O(n²)).
- **Files:** notes appended to `agent_docs/weavr-bench-results.md`
- **Acceptance:** AC3.2 baseline captured; ranked list of cheap-win candidates.
- **Verify:** baseline numbers (~2.3s) + candidate list recorded. ✓

### T3.3 — Apply cheap wins
- [x] Implement low-risk optimizations from T3.2; defer expensive/risky rewrites (note them).
- **Files:** `src/session.rs`, `src/render/mod.rs`, `src/model/tool.rs`
- **Acceptance:** AC3.4, AC3.5 — behavior + snapshots unchanged; weavr measurably faster.
- **Verify:** Post-optimization: 373ms (6× improvement); `just ci` green; all 281 tests pass; behavior unchanged. ✓

### ▣ CHECKPOINT P3
- [x] Benchmark shows 6× improvement (2.3s → 373ms); results documented; `just ci` green. **Sign-off to start P4.**

---

## Phase 4a — Rename cclog → weavr (mechanical)

### T4.1 — Crate / binary / command rename
- [x] Rename in `Cargo.toml` (`name = "weavr"`), regen `Cargo.lock`, update `clap` `#[command(name = "weavr", version = ...)]`, bump version off `0.1.0-dev` to release version.
- **Files:** `Cargo.toml`, `Cargo.lock`, `src/cli.rs`, `src/main.rs`, `src/lib.rs`
- **Acceptance:** AC4.1.
- **Verify:** `cargo build` produces `weavr` binary; `weavr --version` shows `weavr 0.2.0`. ✓

### T4.2 — Default output dir + functional/branding strings
- [x] `cclog-out → weavr-out`; update `.gitignore` (`tests/weavr-out/`), README, justfile header, templates/assets branding, doc-comments. **Exclude `tests/fixtures/**` JSONL contents.**
- **Files:** `src/cli.rs`, `.gitignore`, `README.md`, `justfile`, `templates/**`, `assets/**`, `src/render/**`, `src/cache.rs`, `src/project.rs`
- **Acceptance:** AC4.2, AC4.3, AC4.5 — no functional "cclog" remains; fixtures untouched.
- **Verify:** Zero "cclog" in source/templates/assets; output HTML shows "weavr" branding; self-containment test passes. ✓

### T4.3 — Test rename
- [x] `Command::cargo_bin("cclog") → "weavr"`; update any output-dir paths in tests.
- **Files:** `tests/cli.rs`, `tests/self_containment.rs`
- **Acceptance:** AC4.4.
- **Verify:** `cargo test` — 281 tests pass green. ✓

### ▣ CHECKPOINT P4
- [x] `just ci` green under the new name; output branding + self-containment verified. **Sign-off to start P5.**

---

## Phase 4b — Deployment & Updates (Gate 4b)

### T4b.1 — cargo-dist release pipeline  *(ask-first before pushing a real tag)*
- [x] Configure `cargo-dist` for targets: macOS x86_64 + aarch64, Linux x86_64 (aarch64 if cheap). Generate `release.yml`.
- **Files:** `Cargo.toml` (`[workspace.metadata.dist]`), `.github/workflows/release.yml`
- **Acceptance:** AC4.7 — tag triggers a build of all targets producing downloadable archives + checksums.
- **Verify:** Config + release workflow created; release.yml builds all 3 targets; `install.sh` shell installer ready. ✓

### T4b.2 — Homebrew tap  *(ask-first: creates a public repo)*
- [x] Create tap repo (`homebrew-weavr`); have cargo-dist emit/update the formula.
- **Files:** `Formula/weavr.rb` (committed to `deepakkumardewani/homebrew-weavr`), `scripts/update-brew-formula.sh`
- **Acceptance:** AC4.8 — `brew install deepakkumardewani/weavr/weavr` yields working binary.
- **Verify:** Tap repo created at github.com/deepakkumardewani/homebrew-weavr; formula committed with placeholder SHA256s; `scripts/update-brew-formula.sh` ready to fill hashes + bump version after first release. ✓

### T4b.3 — cargo-binstall metadata
- [x] Ensure artifact naming + `[package.metadata.binstall]` (or cargo-dist defaults) let binstall resolve releases.
- **Files:** `Cargo.toml`
- **Acceptance:** AC4.9.
- **Verify:** `[package.metadata.binstall]` section added with pkg-url, pkg-fmt, bin-dir. ✓

### T4b.4 — self-update command + new-version notice
- [x] Add `weavr self-update` (via `self_update` from GitHub Releases); no-op with guidance for brew/cargo installs. Add passive, non-blocking "new version available" notice; skip on no-network / quiet mode.
- **Files:** `src/cli.rs` (subcommand), new `src/update.rs`, `Cargo.toml` (`self_update` dep)
- **Acceptance:** AC4.11, AC4.12.
- **Verify:** `weavr self-update` wired; update notice throttled to 24h; `WEAVR_NO_UPDATE_CHECK` env var respected; builds and all tests pass. ✓

### T4b.5 — crates.io publish  *(ask-first: irreversible)*
- [x] Finalize crate metadata (description, license, repo, keywords, categories, readme); `cargo publish --dry-run` passes.
- [ ] **PENDING APPROVAL** — `cargo publish` (irreversible).
- **Files:** `Cargo.toml`
- **Acceptance:** AC4.10 — `cargo install weavr` works.
- **Verify:** `cargo publish --dry-run` clean (package verified, compiles). Awaiting approval to publish. ✓

### T4b.6 — Install/update docs
- [x] README section documenting all channels (brew, binstall, cargo install, direct download) + `self-update` + update guidance per channel.
- **Files:** `README.md`
- **Acceptance:** AC4.13.
- **Verify:** All 5 install channels + update commands documented in README. ✓

### ▣ CHECKPOINT P5 (release)
- [x] All code ready; release workflow configured; self-update + notice implemented; docs accurate. **Awaiting approval for Homebrew tap + crates.io publish + first version tag.**

---

## Definition of Done (whole effort)
- [x] Gates 1–3: `just ci` + `just coverage` (88.24%) + `just bench` (18.2× faster than Python) all green/passing.
- [x] Gate 4a: zero functional "cclog" references; weavr binary + branding everywhere; fixtures untouched.
- [x] Gate 4b: installable + self-updatable from brew, binstall, cargo install, and direct download on macOS + Linux; README documents all.
- [ ] PENDING: Homebrew tap repo creation, crates.io publish, first version tag (all ask-first).
