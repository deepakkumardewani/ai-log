# Implementation Plan: cclog UI Improvements v0.1

## Overview

Make cclog easier to scan and navigate by surfacing what sessions/projects *are
about* (instead of opaque IDs and raw counts), fixing the broken filter chips on
the session page, adding navigation back to the parent project, and giving the
index page real triage tools (search, date filter, list/card toggle). Confirmed
intent and approach are captured in
[`/Users/deepakdewani1/.claude/plans/1-need-to-remove-proud-summit.md`](../../../.claude/plans/1-need-to-remove-proud-summit.md).

## Architecture Decisions

- **Pure client-side filtering on the index page.** Search, date filter, and
  view toggle operate on already-rendered DOM via vanilla JS reading `data-*`
  attributes. No re-fetch, no new build artifacts beyond a sibling to
  `assets/transcript.js`.
- **Chip filter semantics: OR-within-category, AND-across-categories.** No
  chips selected = show everything (avoids the empty-UI footgun).
- **Always-on back link.** No referrer sniffing — the session page always shows
  `← {project name}`, mirroring the existing `← All Projects` pattern on the
  project page.
- **Data extensions live in the existing render structs**, not a parallel cache.
  `first_user_prompt`, `started_at`, `last_activity`, `short_name` are computed
  during the existing render pass.
- **Relative time formatting is a single shared helper** — verify whether
  `chrono`/`time` already provides one before adding a new utility.

## Task List

### Phase 1: Data Foundation

#### Task 1: Expose session preview + start timestamp to templates

**Description:** Extend the per-session struct passed to `project.html` and
`transcript.html` with `first_user_prompt` (truncated to ~120 chars, suffix `…`
if cut) and `started_at` (chrono `DateTime` or string usable by the template).
Compute by reading the session's first message with role=user.

**Acceptance criteria:**
- [x] `first_user_prompt` is non-empty for sessions that contain at least one user message; empty/None otherwise
- [x] Truncation preserves UTF-8 grapheme boundaries and trims trailing whitespace before appending `…`
- [x] `started_at` reflects the timestamp of the session's first event

**Verification:**
- [x] `cargo test` passes (add a unit test for truncation helper if one doesn't exist)
- [x] `cargo run -- <projects dir>` regenerates `tests/cclog-out/` and inspecting the rendered HTML shows the new fields on at least one session

**Dependencies:** None

**Files likely touched:**
- `src/render/html.rs`
- possibly `src/cache.rs` or wherever the session struct is built
- a small unit test file for the truncation helper

**Estimated scope:** S (1–2 files)

---

#### Task 2: Expose project last-activity + short name

**Description:** Add `last_activity` (timestamp of most recent session event) and
`short_name` (trailing path segment, e.g. `react-mockforge`) to the project
struct rendered by `index.html`.

**Acceptance criteria:**
- [x] `short_name` equals the final non-empty segment of the project path
- [x] `last_activity` equals the max `started_at` / event timestamp across the project's sessions
- [x] Both fields are available in the template context

**Verification:**
- [x] `cargo test` passes
- [x] Regenerated `index.html` exposes both values on every project card (inspect DOM)

**Dependencies:** Task 1 (uses the same timestamp source)

**Files likely touched:**
- `src/render/html.rs`
- the project aggregation site (likely `src/cache.rs` or a sibling)

**Estimated scope:** S (1–2 files)

---

#### Checkpoint: Data foundation
- [x] `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test`
- [x] Regenerated output contains the new fields (sanity-grep for a session prompt and a project `short_name`)

---

### Phase 2: Session Page Cleanup

#### Task 3: Remove sidebar from transcript page

**Description:** Stop rendering the session-history sidebar on `transcript.html`.
Delete the include and any sidebar-specific CSS rules that are unused after.
`templates/components/sidebar/toc.html` is already deleted on the working tree —
finish the job for the remaining sidebar partials and their parent layout.

**Acceptance criteria:**
- [x] `transcript.html` renders with main content full-width — no sidebar column, no leftover empty grid track
- [x] No console errors and no orphan references to removed partials
- [x] Page still scrolls correctly on long sessions

**Verification:**
- [x] Manual: open a regenerated session HTML in a browser — no sidebar visible, layout breathes correctly at desktop and ~768px widths
- [x] `cargo test` (update `tests/self_containment.rs` if it asserts on sidebar markup)

**Dependencies:** None

**Files likely touched:**
- `templates/transcript.html`
- `templates/components/sidebar/sidebar.html` (and `session_history.html`)
- `templates/base.html`
- `assets/tailwind.input.css`
- `tests/self_containment.rs`

**Estimated scope:** S (2–4 files)

---

#### Task 4: Add `← {project name}` back link to session header

**Description:** Add a back link in the `transcript.html` header that points to
the parent `combined_transcripts.html`. Mirror the markup/styles of the existing
`← All Projects` link on the project page so it feels native. Always render —
no conditional on referrer.

**Acceptance criteria:**
- [x] Header shows `← {project name}` (using the project's short name from Task 2) above or beside the session title
- [x] Clicking navigates to the project page; works when opening the HTML directly via `file://` (relative href)
- [x] Visual style matches the existing back link on the project page

**Verification:**
- [x] Manual: from index → project → session → click back link → lands on the project page
- [x] Manual: open a session HTML directly (skipping the index) — link still works

**Dependencies:** Task 2 (uses `short_name`); Task 3 (header lives in the cleaned-up template)

**Files likely touched:**
- `templates/transcript.html`
- `templates/components/header.html`
- `assets/tailwind.input.css`

**Estimated scope:** S (1–3 files)

---

#### Task 5: Fix filter chip behavior on the session page

**Description:** Make every chip (`User`, `Assistant`, `Bash`, `Read`, `Write`,
`Edit`, `Thinking`) actually filter turns. Semantics: OR within category
(role chips OR'd together; tool chips OR'd together), AND across categories
(role-set ∩ tool-set when both have selections). No chips selected = show
everything. Filter reads `data-*` attributes on each turn — render those
attributes from the tool classifier in `src/render/tools/mod.rs` to keep the
chip set and data in sync.

**Acceptance criteria:**
- [x] Selecting `Bash` alone hides every turn that doesn't contain a Bash tool call
- [x] `User + Bash` shows all user turns AND all turns containing Bash (set union behavior is correct for the role-vs-tool case where the user intends "either")
- [x] `User + Assistant + Bash` behaves the same — role union ∪ tool union (clarify with a comment in the JS so future-you doesn't second-guess it)
- [x] No chips selected = all turns visible
- [x] Sanity-check `Thinking` and `Edit` chips work the same way

**Verification:**
- [x] Manual: click each chip individually and confirm visible turns match expectation
- [x] Manual: combine pairs across categories
- [x] No console errors

**Dependencies:** Task 3 (template cleanup may relocate the chip bar)

**Files likely touched:**
- `assets/transcript.js`
- `templates/transcript.html` (data attributes on turn elements)
- `src/render/tools/mod.rs` (expose tool kind to template if not already)

**Estimated scope:** M (3 files)

---

#### Checkpoint: Session page
- [x] All three session-page tasks verified end-to-end in a browser
- [x] `cargo fmt && cargo clippy -- -D warnings && cargo test` clean

---

### Phase 3: Project Page Card Content

#### Task 6: Replace duplicated session ID with prompt preview + relative time

**Description:** Restyle the session card in `project.html` so the primary line
is the first user prompt (~120 chars) and a relative timestamp (e.g. `2h ago`,
`May 24`). The session UUID becomes a faded secondary line. Counts (messages,
tokens) stay but become tertiary. Add the relative-time helper if one doesn't
already exist; reuse if it does.

**Acceptance criteria:**
- [x] Card primary text = truncated first user prompt; if missing, fall back to a short placeholder (`(no user message)`)
- [x] A human-readable relative timestamp is visible next to the prompt
- [x] UUID still present but visually de-emphasized (smaller, lower contrast)
- [x] Counts remain visible

**Verification:**
- [x] Manual: project page shows distinguishable, scannable session entries
- [x] Visual check at desktop and mobile widths — prompt doesn't overflow

**Dependencies:** Task 1

**Files likely touched:**
- `templates/project.html`
- `src/render/html.rs` (relative-time helper if added)
- `assets/tailwind.input.css`

**Estimated scope:** S (2–3 files)

---

#### Checkpoint: Project page
- [x] Manual scan of the project page reads cleanly across at least 5 sessions
- [x] Tests pass

---

### Phase 4: Index Page Card Content

#### Task 7: Restyle project cards with short name + last activity

**Description:** Make `short_name` (from Task 2) the primary heading on each
project card; demote the full path to a secondary, faded line. Add the
relative-time-formatted `last_activity` as a primary metadata line beside or
under the title.

**Acceptance criteria:**
- [x] Short name is the visual anchor (larger, higher contrast)
- [x] Last-activity timestamp is visible at a glance
- [x] Full path still present (as a faded subtitle) so disambiguation is possible

**Verification:**
- [x] Manual: index page is readable; you can name a project from across the room
- [x] Tests pass

**Dependencies:** Task 2; reuses the relative-time helper from Task 6

**Files likely touched:**
- `templates/index.html`
- `assets/tailwind.input.css`

**Estimated scope:** S (2 files)

---

### Phase 5: Index Page Interactivity

#### Task 8: View-mode toggle (cards ⇄ list) with localStorage persistence

**Description:** Add a pill toggle in the index page header that switches the
project grid between the existing card layout and a dense list (one row per
project: short name | last activity | sessions | messages | tokens). Default to
cards. Persist the chosen mode in `localStorage` under a namespaced key
(e.g. `cclog:index:viewMode`). The list layout's columns should be sortable on
click (toggle asc/desc).

**Acceptance criteria:**
- [x] Toggle visible in the index header; clicking flips the layout
- [x] Refreshing the page preserves the chosen mode
- [x] List view shows a clear table-like row per project; columns sort on header click
- [x] No layout shift on load (apply persisted mode before/with first paint — inline `<script>` in `<head>` if needed)

**Verification:**
- [x] Manual: toggle, refresh, confirm persistence
- [x] Manual: sort by each column
- [x] Tests pass

**Dependencies:** Task 7 (cards already have the new data); Task 2 (data attributes)

**Files likely touched:**
- `templates/index.html`
- `assets/index.js` (new)
- `assets/tailwind.input.css`
- `src/assets.rs` (register the new JS file)

**Estimated scope:** M (3–4 files)

---

#### Task 9: Search bar (substring on name/path)

**Description:** Add a search input to the index header. As the user types,
filter the visible projects to those whose `short_name` or full path contain the
query (case-insensitive substring, no regex, no fuzzy matching). Works in both
card and list view.

**Acceptance criteria:**
- [x] Typing filters in real time (debounce ~100–150 ms)
- [x] Matching is case-insensitive
- [x] Empty input = all projects visible
- [x] Works alongside the date filter (combine via AND)

**Verification:**
- [x] Manual: type `mock` and confirm only mockforge remains
- [x] Manual: combine with a date preset and confirm intersection

**Dependencies:** Task 8 (shares `assets/index.js`)

**Files likely touched:**
- `templates/index.html`
- `assets/index.js`
- `assets/tailwind.input.css`

**Estimated scope:** S (2–3 files)

---

#### Task 10: Date filter (range picker + presets)

**Description:** Add a date filter to the index header. Provide quick presets
(`Today`, `Last 7 days`, `Last 30 days`, `All time`) and a custom range picker
(two date inputs: from / to). Filter projects whose `last_activity` falls within
the selected range. Selecting a preset updates the picker values to match (so
the user can see what range is active). Default = `All time` (no filter).

**Acceptance criteria:**
- [x] Preset chips and a range picker are both present and synced
- [x] Filtering is by `last_activity`, not creation date
- [x] Combines with the search input via AND
- [x] Clearing the range / selecting `All time` restores full list

**Verification:**
- [x] Manual: each preset narrows correctly
- [x] Manual: custom range narrows correctly
- [x] Manual: combine with search

**Dependencies:** Task 9 (shares index JS module)

**Files likely touched:**
- `templates/index.html`
- `assets/index.js`
- `assets/tailwind.input.css`

**Estimated scope:** M (3 files)

---

#### Checkpoint: Index interactivity
- [x] Toggle + search + date filter all coexist without interfering
- [x] LocalStorage persistence verified across refresh
- [x] No console errors

---

### Phase 6: Tests & Polish

#### Task 11: Update self-containment + golden tests

**Description:** Update `tests/self_containment.rs` (and any other tests that
assert on DOM shape) to reflect: no sidebar markup on the session page; new
fields on session and project cards; presence of toggle/search/date-filter
elements on the index page.

**Acceptance criteria:**
- [x] `cargo test` passes
- [x] Tests fail loudly if the sidebar reappears or the new card fields disappear (regression guards)

**Verification:**
- [x] `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test`
- [x] `just ci` clean

**Dependencies:** All prior tasks

**Files likely touched:**
- `tests/self_containment.rs`
- any other test file currently asserting on the changed templates

**Estimated scope:** S (1–2 files)

---

### Checkpoint: Complete
- [x] All acceptance criteria met
- [x] End-to-end flow verified: index → search/filter/toggle → project page (new cards) → session page (no sidebar, back link works, chips filter correctly)
- [x] `just ci` clean
- [x] Ready for review

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tool kinds emitted in templates don't match chip categories (e.g. `Edit` chip but template uses `MultiEdit`) | Med | Source tool labels from `src/render/tools/mod.rs` so both stay in sync (Task 5) |
| Existing self_containment test snapshots break on every template change | Low | Update tests in a single late task (Task 11) rather than per-task churn |
| `first_user_prompt` truncation splits a multi-byte grapheme and corrupts UTF-8 | Med | Use `chars().take(N).collect()` or a grapheme-aware crate already in deps; add a unit test (Task 1) |
| Date-filter picker UX gets messy across browsers | Low | Use native `<input type="date">` for the range picker — minimal styling, no JS library |
| LocalStorage key collisions across cclog versions | Low | Namespace under `cclog:index:*` (Task 8) |

## Open Questions

- Does the codebase already have a relative-time helper? (Verify before Task 6 to avoid duplicating one.)
- Is there an existing place (e.g. a shared util) where the project-path → short-name function should live? (Decide in Task 2.)

## Parallelization

- **Sequential foundation:** Tasks 1 → 2 (data plumbing) must land before anything in Phases 2–5.
- **Parallelizable after Phase 1:**
  - Phase 2 (Tasks 3, 4, 5) can be worked independently of Phase 3 (Task 6) and Phase 4 (Task 7).
  - Within Phase 2: Task 3 (sidebar removal) is independent of Tasks 4 and 5, which both touch the header.
- **Strictly sequential:** Phase 5 (Tasks 8 → 9 → 10) shares one JS module and one template region — work in order to avoid conflicts.
- **Final:** Task 11 lands last, after the DOM has stopped moving.
