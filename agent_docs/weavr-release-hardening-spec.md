# weavr Release-Hardening — Specification

> Status: **Draft for approval** · Owner: Deepak · Created: 2026-06-06
> Supersedes the project name **cclog**. See [[weavr-rename-and-release]].

---

## 1. Objective

Take the project currently named **cclog** from a private tool to a **publicly shippable Rust CLI named `weavr`**: cleaned up, proven fast, well-tested, and installable + self-updatable by strangers.

The work is a single combined **release-hardening** effort with four quality gates, executed in order:

1. **Review / Simplify / DRY** — whole-codebase cleanup (Rust + HTML/CSS/JS).
2. **Coverage** — `cargo-llvm-cov`, 80% CI gate, core logic ~100%.
3. **Performance** — prove `weavr` beats the Python original via a re-runnable benchmark; take cheap wins.
4. **Rename + Deployment** — `cclog → weavr`, then distribution + self-update.

**Target users (MVP):** Claude Code users on **macOS + Linux**.

**Why now:** First public release is being prepared, and the name `cclog` ("cc" = Claude Code) contradicts the multi-tool roadmap.

**Guiding principle:** *Pragmatism over dogma.* Fix real problems; don't rewrite working subsystems for taste, don't write brittle tests to hit 100% on trivial glue, don't build future-tool/Windows support yet.

### Performance baseline

The Python original — [`claude-code-log`](https://github.com/daaain/claude-code-log) — is the comparison baseline. `weavr` is a Rust reimplementation of it.

---

## 2. Scope

### In scope

- Whole-codebase code review, simplification, and DRY pass across `src/**` (Rust) and `templates/**` + `assets/**` (HTML/CSS/JS).
- Coverage measurement + enforcement via `cargo-llvm-cov`.
- A committed, re-runnable `hyperfine` benchmark comparing `weavr` vs `claude-code-log` on a full-projects run, plus cheap performance wins it reveals.
- Rename `cclog → weavr` everywhere it is *functional* (crate, binary, command name, default output dir, branding strings, docs).
- Distribution pipeline: GitHub Releases prebuilt binaries (cargo-dist), Homebrew tap, `cargo-binstall` support, crates.io publish.
- Built-in `self-update` command + passive "new version available" notice.

### Out of scope (explicitly)

- **Windows** builds/support.
- **Other-tool ingestion** (Cursor, Codex, etc.).
- New product **features** or any **UI redesign** beyond cleanup.
- A **landing page / website**.
- Renaming the **content of test fixture JSONL** files (they are captured real-session *data* that legitimately mention "cclog"; leave them as-is).

---

## 3. Functional Requirements & Acceptance Criteria

### Gate 1 — Code Review / Simplification / DRY

- **AC1.1** Every `src/**` module and every file under `templates/**` and `assets/**` has been reviewed; findings are recorded (duplication, dead code, readability, naming).
- **AC1.2** Identified duplication is removed or consolidated into a single source of truth (Rust helpers/types; shared CSS tokens / template partials). Logic appearing 3+ times is mandatory to extract.
- **AC1.3** No dead code, no commented-out code blocks, no unused `pub` surface that isn't part of the intended API.
- **AC1.4** `cargo clippy --all-targets -- -D warnings` passes with zero warnings after cleanup.
- **AC1.5** Behavior is unchanged: the full test suite (and any snapshot/insta fixtures) passes before and after, byte-for-byte where snapshots apply.
- **AC1.6** No working subsystem was rewritten purely for taste (changes are justified by a concrete duplication/clarity/dead-code finding).

### Gate 2 — Coverage

- **AC2.1** `cargo-llvm-cov` is wired into the project (a `just coverage` recipe + CI job).
- **AC2.2** CI **fails** when total line coverage drops below **80%**.
- **AC2.3** Core logic — `parser`, `model` (incl. `model/*` deserialization), `aggregate`, `conversation` — reaches **~100%** coverage, including error/edge paths (malformed JSONL, missing fields, broken parent links, empty input).
- **AC2.4** Pragmatic coverage elsewhere: `main.rs` wiring and pure HTML-glue are not forced to 100%.
- **AC2.5** A coverage summary (overall % + per-core-module %) is reproducible locally via one command.

### Gate 3 — Performance

- **AC3.1** A `hyperfine`-based benchmark is committed (script + docs) that runs both `weavr` and `claude-code-log` against the same full-projects input (`~/.claude/projects/`) and reports wall-clock comparison.
- **AC3.2** The benchmark demonstrates `weavr` is **measurably faster** than `claude-code-log` on a full-projects run.
- **AC3.3** A short results note is recorded (numbers + machine/context) so the comparison is reproducible and re-runnable.
- **AC3.4** Any cheap wins surfaced (redundant IO, needless clones/allocations, accidental O(n²), repeated parsing) are taken; expensive/risky rewrites are deferred and noted.
- **AC3.5** Performance refactors land **after** Gate 2 so test coverage protects them; behavior + snapshots remain unchanged.

### Gate 4a — Rename (`cclog → weavr`)

- **AC4.1** Crate name, binary name, and `clap` command `name` are `weavr`; `--version` reflects the release version (not `0.1.0-dev`).
- **AC4.2** Default output directory is renamed (`cclog-out → weavr-out`); `.gitignore`, README, and tests updated to match.
- **AC4.3** All functional/branding occurrences of "cclog" are updated (source, templates, assets, README, justfile, CI). Test-fixture JSONL *data* is left untouched (see Out of scope).
- **AC4.4** `tests/cli.rs` / `tests/self_containment.rs` use `Command::cargo_bin("weavr")` and pass.
- **AC4.5** Generated HTML/Markdown output branding shows `weavr`, and output remains fully self-contained (no `http://`/`https://`).
- **AC4.6** The rename lands as a discrete, mechanical step (kept out of the earlier review/refactor diffs to keep those reviewable).

### Gate 4b — Deployment & Updates

- **AC4.7** **GitHub Releases** publish prebuilt binaries for macOS (x86_64 + aarch64) and Linux (x86_64, and aarch64 if cheap) via **cargo-dist**, triggered by a version tag.
- **AC4.8** A **Homebrew tap** installs `weavr`; `brew install <tap>/weavr` yields a working binary.
- **AC4.9** **`cargo-binstall`** can fetch the released binary (correct metadata/artifact naming).
- **AC4.10** The crate is **published to crates.io** as `weavr`; `cargo install weavr` works.
- **AC4.11** A built-in **`weavr self-update`** command updates the binary in place from GitHub Releases (skipped/no-op gracefully for package-manager installs).
- **AC4.12** When run on a stale version, `weavr` prints a **passive, non-blocking "new version available" notice** (does not interrupt normal output; respects a quiet/no-network path).
- **AC4.13** Install + update instructions are documented in the README for all channels.

---

## 4. Commands

Existing dev workflow (preserved, renamed where needed):

| Command | Purpose |
| --- | --- |
| `cargo build` / `cargo build --release` | Debug / release build |
| `cargo fmt --all` | Format |
| `cargo clippy --all-targets -- -D warnings` | Lint (warnings = errors) |
| `cargo test` | Run tests |
| `just ci` | fmt → clippy → test |

New recipes introduced by this effort:

| Command | Purpose |
| --- | --- |
| `just coverage` | Run `cargo-llvm-cov`, print overall + per-core-module % |
| `just bench` | Run the `hyperfine` weavr-vs-Python benchmark |
| `weavr self-update` | Update the installed binary from GitHub Releases |

End-user runtime commands (`export`, `--all-projects`, `--detail`, cache flags, etc.) are unchanged in behavior; only the binary name and default output dir change.

---

## 5. Project Structure (relevant)

```
src/
  aggregate.rs        # core — full coverage target
  conversation.rs     # core — full coverage target
  parser.rs           # core — full coverage target
  model/{mod,entry,content,tool}.rs  # core — full coverage target
  cli.rs              # command name + output-dir rename here
  cache.rs  project.rs  session.rs  dates.rs  assets.rs  main.rs  lib.rs
  render/**            # review/DRY + pragmatic coverage
templates/**           # HTML review/DRY + branding rename
assets/**              # JS/CSS review/DRY + branding rename
tests/                 # cli.rs, self_containment.rs → cargo_bin("weavr")
  fixtures/            # JSONL data — DO NOT rename contents
benches/ or scripts/   # NEW: hyperfine benchmark (location TBD in plan)
.github/workflows/     # ci.yml (add coverage gate) + release.yml (NEW, cargo-dist)
agent_docs/            # this spec + plan + tasks
```

---

## 6. Code Style & Constraints

- Follows existing project conventions and `.claude/skills/rust-best-practices` / `rust-testing`.
- Named structs over long parameter lists; lean on the type system; idiomatic `Result`/`Option`; `thiserror` for library errors, `anyhow` for app errors; minimal documented `unsafe`.
- DRY enforced pragmatically (a little duplication beats a wrong abstraction).
- All changes keep `clippy -D warnings` clean and `cargo fmt` stable.
- Pure Rust project — all operations via `cargo` / `just` (no JS toolchain even though `assets/` contains JS/CSS authored by hand).

---

## 7. Testing Strategy

- **Unit tests** for core modules (`parser`, `model`, `aggregate`, `conversation`) covering happy path + error/edge cases → ~100%.
- **Integration tests** (`tests/cli.rs`, `tests/self_containment.rs`) via `assert_cmd`, asserting self-containment (no external URLs) and correct output.
- **Snapshot tests** (`insta`) guard rendering output across the review/perf refactors — they must stay green (behavior-preserving).
- **Coverage** via `cargo-llvm-cov`, enforced at 80% in CI, core ~100%.
- **Benchmark** via `hyperfine` — comparative, not a pass/fail unit test, but committed and documented.
- Order matters: tests/coverage (Gate 2) precede performance refactors (Gate 3) so refactors are protected.

---

## 8. Boundaries

**Always:**
- Keep generated output fully self-contained (no external URLs).
- Keep `clippy -D warnings` clean and snapshots green.
- Preserve runtime behavior across review/perf/rename (it's hardening, not feature work).
- Run the rename as a discrete mechanical step.

**Ask first:**
- Before any change that alters user-facing CLI behavior, flags, or output format.
- Before adding heavyweight dependencies or risky/expensive performance rewrites.
- Before publishing to crates.io / pushing a release tag (irreversible / outward-facing).
- Before creating the Homebrew tap repo or any new public repo.

**Never:**
- Add Windows or other-tool (Cursor/Codex) support in this effort.
- Rename the contents of test-fixture JSONL data.
- Commit (the user commits explicitly) or add Anthropic/Claude attribution to commit messages.
- Rewrite working subsystems purely for taste, or write brittle tests just to hit a coverage number.

---

## 9. Sequence (high level)

```
Gate 1: review / simplify / DRY   ─┐
Gate 2: coverage (80% gate, core ~100%)   (locks behavior)
Gate 3: performance (benchmark + cheap wins, protected by Gate 2)
Gate 4a: rename cclog → weavr (mechanical)
Gate 4b: deployment (cargo-dist, brew tap, binstall, crates.io, self-update, notice)
```

Detailed task breakdown lives in `agent_docs/weavr-release-hardening-plan.md` and `...-tasks.md`.
