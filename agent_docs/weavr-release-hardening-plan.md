# weavr Release-Hardening — Plan

> Companion to [agent_docs/weavr-release-hardening-spec.md](weavr-release-hardening-spec.md).
> Task checklist: [agent_docs/weavr-release-hardening-tasks.md](weavr-release-hardening-tasks.md).
> Status: **Draft for approval** · Created: 2026-06-06

---

## 1. Strategy

Five gates, executed in strict order. Each gate ends in a **checkpoint** (human review + green `just ci`) before the next begins. The ordering is not arbitrary — it is dictated by the dependency graph below.

The central safety mechanism is **behavior preservation**: Gate 2 raises coverage *before* Gate 3 refactors for speed and *before* Gate 4a renames, so every later change is caught by tests and `insta` snapshots if it alters output.

Why this order:

- **Review before coverage** — don't write tests for code you're about to delete or consolidate. Cleaning first means coverage targets stable code.
- **Coverage before performance** — perf refactors are the riskiest behavior-altering changes; they must land under a test net.
- **Performance before rename** — keep benchmark + perf diffs in terms of the current name; one mechanical rename afterward.
- **Rename before deployment** — every release artifact (crate, binary, brew formula, release tag) must carry the final name.

---

## 2. Dependency graph

```
                ┌─────────────────────────┐
                │ G1  Review / Simplify    │  (no deps — start here)
                │     / DRY                │
                └────────────┬────────────┘
                             │ stable code surface
                             ▼
                ┌─────────────────────────┐
                │ G2  Coverage             │  needs clean modules to test
                │  llvm-cov + 80% CI gate  │
                │  core ~100%              │
                └────────────┬────────────┘
                             │ behavior locked by tests + snapshots
                             ▼
                ┌─────────────────────────┐
                │ G3  Performance          │  refactors protected by G2
                │  hyperfine vs Python     │
                │  + cheap wins            │
                └────────────┬────────────┘
                             │ final code shape settled
                             ▼
                ┌─────────────────────────┐
                │ G4a Rename cclog→weavr   │  mechanical; touches everything
                └────────────┬────────────┘
                             │ final name in place
                             ▼
                ┌─────────────────────────┐
                │ G4b Deployment           │
                │  cargo-dist → Releases   │
                │  ├ Homebrew tap          │  (parallel-able once cargo-dist done)
                │  ├ cargo-binstall meta   │
                │  ├ crates.io publish     │
                │  ├ self-update command   │  (code change — could precede G4a,
                │  └ new-version notice    │   but kept here to ride final name)
                └─────────────────────────┘
```

**Intra-G4b parallelism:** once `cargo-dist` produces release artifacts (T4b.1), the Homebrew tap (T4b.2), binstall metadata (T4b.3), and crates.io publish (T4b.5) are independent of each other. The `self-update` command + notice (T4b.4) are pure code and depend only on the release-URL scheme existing.

---

## 3. Vertical slicing principle

Each task is **one complete path**, not a horizontal layer. Examples:

- Gate 2 is sliced **per core module** (parser → model → aggregate → conversation), each taken to ~100% end-to-end, rather than "write all happy-path tests, then all error tests."
- Gate 4b is sliced **per channel** (one channel = artifact + docs + verification), not "all CI config, then all docs."

This keeps every task independently verifiable and shippable.

---

## 4. Phases & checkpoints

| Phase | Gate | Exit checkpoint |
| --- | --- | --- |
| P1 | G1 Review/Simplify/DRY | Findings doc reviewed; `just ci` green; snapshots unchanged or intentionally re-blessed |
| P2 | G2 Coverage | `just coverage` ≥80% total, core ~100%; CI gate fails below 80% (proven) |
| P3 | G3 Performance | `just bench` shows weavr < Python; results noted; `just ci` green; snapshots unchanged |
| P4 | G4a Rename | `cargo_bin("weavr")` tests pass; no functional "cclog" left; output branding = weavr |
| P5 | G4b Deployment | Dry-run release builds all targets; install verified from each channel on a clean machine; self-update + notice demonstrated |

**Checkpoint protocol:** at each `▣ CHECKPOINT` in the tasks file, stop, run the stated verification, and get explicit human sign-off before starting the next phase. Publishing steps (crates.io, release tag, brew tap repo) are **ask-first** per spec §8.

---

## 5. Key risks & mitigations

| Risk | Mitigation |
| --- | --- |
| Refactor/rename silently changes rendered output | `insta` snapshots + self-containment tests must stay green; re-bless only deliberately |
| Coverage chase produces brittle tests | Core ~100% via real fixtures + error paths; glue stays pragmatic (spec AC2.4) |
| Benchmark not reproducible / unfair | Same input dir, warmup runs, documented machine; commit script not just numbers |
| Rename corrupts fixture data | Rename excludes `tests/fixtures/**` JSONL contents (spec out-of-scope) |
| crates.io / tag pushes are irreversible | All publish steps gated as ask-first; dry-run first |
| `self_update` crate fights brew/cargo installs | `self-update` detects non-Release installs and no-ops with guidance (AC4.11) |
| CI currently Linux-only | Add macOS runner only where needed (release matrix); keep quality job lean |

---

## 6. Out of scope (reaffirmed)

Windows, other-tool ingestion (Cursor/Codex), new features, UI redesign, landing page, renaming fixture JSONL data. See spec §2.

---

## 7. Estimated shape

~5 phases, ~22 tasks. Gates 1–3 are code-quality work on a stable surface; Gate 4 is mechanical + infra. The long pole is G4b (release infra + first publish), which is mostly config + verification rather than logic.
