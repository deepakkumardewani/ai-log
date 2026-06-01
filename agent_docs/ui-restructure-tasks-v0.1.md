# Implementation Plan: UI Restructure & Visual Refresh (v0.1)

Companion to [`ui-restructure-spec-v0.1.md`](./ui-restructure-spec-v0.1.md).

---

## Overview

Restructure the cclog static HTML viewer (index page + per-project session-list page) into a crafted product UI. Foundation work: design tokens + Rust name extraction. Then vertical slices through the index page (table → grid card → view-switcher → toolbar). Then port symmetric search + date chips to the per-project page. Polish via `/arrange` and `/impeccable`, regenerate fixtures, ship.

---

## Architecture Decisions

1. **Name extraction in Rust, not templates.** Add `Project::display_name()` (or a derived field on the render context). Templates receive a clean string. Rationale: templates stay logic-free; testable in Rust; one source of truth.
2. **Tokens before pixels.** Lock `tailwind.config.js` palette + `design.md` first, then apply tokens across templates. Rationale: avoids a second pass when the accent or neutral changes.
3. **Reuse `assets/index.js` for the per-project page filters.** Either extend it or extract a shared module (`filters.js`) and import from both. Avoid a parallel divergent implementation. Decision deferred to Task 9 based on what `index.js` actually contains at that point.
4. **Regenerate, don't migrate.** Old generated HTML in `tests/cclog-out/` is overwritten by re-running the CLI. No version-compat shims.
5. **Skill invocations at end, not throughout.** `/arrange` and `/impeccable` are quality passes applied after the slices land, against a coherent surface — not on partial work.

---

## Dependency Graph

```
Tokens (Tailwind config + design.md)        Rust name extraction
        │                                            │
        ├────────────┬────────────┬────────────┐     │
        ▼            ▼            ▼            ▼     ▼
   Index table  Index card  View-switcher  Toolbar   │
        │            │            │            │     │
        └────────────┴────────────┴────────────┘     │
                          │                          │
                          ▼                          │
              Per-project search + chips ────────────┘
                          │
                          ▼
              Polish passes (/arrange, /impeccable)
                          │
                          ▼
              Regenerate fixtures + just ci
```

---

## Task List

### Phase 1: Foundation

#### Task 1 — Lock the visual system in `design.md`

**Description:** Produce `agent_docs/design.md` defining the visual system before touching any template. Captures: palette (warm neutral bg, warm accent — terracotta/amber, supporting greys, semantic state colors), type scale (project name weight/size, body, mono for stats), spacing scale, component states (default/hover/active/focus for buttons, chips, cards, rows), and the two-button segmented view-switcher pattern.

**Acceptance:**
- [x] `design.md` exists with all sections above filled in with concrete values (hex, rem, etc.)
- [x] Explicitly notes the dark-theme variants where they differ
- [x] No purple anywhere in the documented palette
- [x] Light bg is documented as a warm neutral (not `#FFFFFF`)

**Verify:** Visual review by user. This is the design contract every later task references.

**Dependencies:** None

**Files:** `agent_docs/design.md` (new)

**Scope:** S

---

#### Task 2 — Apply tokens to `tailwind.config.js` and base styles

**Description:** Translate the `design.md` palette and scale into Tailwind theme tokens. Remove purple from the active palette. Add semantic token names (`bg-surface`, `bg-surface-elevated`, `text-accent`, `border-subtle`, etc.) so templates reference intent, not raw colors.

**Acceptance:**
- [x] Warm neutral bg available as `bg-surface` (or equivalent token)
- [x] Terracotta/amber accent available as `bg-accent` / `text-accent` / `ring-accent`
- [x] All purple utility usages removed or remapped
- [x] Dark-theme tokens defined consistently with light-theme tokens
- [x] `tailwind.input.css` updated for any CSS-variable bridging needed

**Verify:**
- [x] `cargo build` succeeds
- [ ] Tailwind rebuild completes without warnings
- [ ] Open existing generated `tests/cclog-out/index.html` in browser — bg is warmer, no purple visible (layout will still be ugly, that's fine)

**Dependencies:** Task 1

**Files:** `assets/tailwind.config.js`, `assets/tailwind.input.css`

**Scope:** S

---

#### Task 3 — Add `Project::display_name` in Rust + tests

**Description:** Add a method (or computed field) that derives a human-readable name from the encoded directory string. Algorithm: take the substring after the last `-`. If empty (trailing dash, single dash), fall back to the full encoded string. Wire it through the render context so templates can reference `project.display_name`.

**Acceptance:**
- [x] `Project::display_name(&self) -> &str` implemented
- [x] Unit tests cover: normal case, leading dash, trailing dash, single segment with no dash, empty string
- [x] Render context for both `templates/index.html` and `templates/project.html` exposes the new field

**Verify:**
- [x] `cargo test project` passes
- [x] `cargo clippy --all-targets -- -D warnings` passes

**Dependencies:** None (parallelizable with Tasks 1–2)

**Files:** `src/project.rs`, `src/render/index.rs`, `src/render/project.rs`

**Scope:** S

---

### Checkpoint: Foundation

- [x] `just ci` passes
- [ ] `design.md` reviewed by user
- [ ] User approves the warm neutral + accent in a regenerated `index.html` (color only — layout still pending)

---

### Phase 2: Index Page Restructure (Vertical Slices)

#### Task 4 — Table: project column shows name + truncated path

**Description:** Update the table rendering in `templates/index.html` so the project column shows `display_name` (semibold, primary text color) on line 1 and the encoded path (muted, truncated with ellipsis, `title` attribute = full path) on line 2. Keep the "Last activity / Sessions / Messages / Tokens" columns intact.

**Acceptance:**
- [x] Table project column has two-line cell: bold name + muted truncated path
- [x] Hovering the path shows the full encoded string in a native tooltip
- [x] Row height stays uniform; long paths do not break layout
- [x] No visual change to other columns

**Verify:**
- [x] `cargo build` succeeds
- [ ] Regenerate `tests/cclog-out/`; open `index.html`; visually confirm
- [ ] Truncation behaves correctly on viewports 1024px and 1440px wide

**Dependencies:** Tasks 2, 3

**Files:** `templates/index.html`

**Scope:** S

---

#### Task 5 — Grid card: name + truncated path + labeled stats

**Description:** Restructure the grid card in `templates/index.html` so it mirrors the table's label semantics. Card shows: bold `display_name` (primary), single-line truncated path with `title` tooltip (muted), then a stats block with **labels** including `Last activity: <date>` (was unlabeled), plus existing Sessions / Messages / Tokens. Cards stay uniform-height.

**Acceptance:**
- [x] Card title is `display_name`, not the encoded path
- [x] Path appears as a single-line truncated muted line under the title with hover tooltip
- [x] "Last activity" label is present in the card (parity with table column header)
- [x] All cards in the grid have equal height regardless of name/path length
- [x] Cards are reachable via the same anchor as before (links unchanged)

**Verify:**
- [x] `cargo build` succeeds
- [ ] Regenerate; toggle to grid view; visually confirm
- [ ] Resize to ensure cards stay uniform

**Dependencies:** Tasks 2, 3

**Files:** `templates/index.html`

**Scope:** S

---

#### Task 6 — View-switcher: two segmented icon buttons

**Description:** Replace the current single mystery toggle button with a segmented two-button control (grid icon + list icon, sharing a rounded container). Active button is filled with `bg-accent` + accessible label; inactive is transparent with muted icon. Update `assets/index.js` if state handling needs changes.

**Acceptance:**
- [x] Both grid and list icons are visible at all times
- [x] Active state visually obvious (accent fill or border, sufficient contrast)
- [x] Clicking either button switches the view without page reload
- [x] `aria-pressed` (or equivalent) reflects state for accessibility
- [x] State persists across reload if it did before (preserve existing behavior)

**Verify:**
- [x] `cargo build` succeeds, self-containment test passes
- [ ] Regenerate; click each button; confirm view switches; confirm active state
- [ ] Tab through with keyboard; both buttons are focusable; focus ring uses accent

**Dependencies:** Tasks 2

**Files:** `templates/index.html`, `assets/index.js`

**Scope:** S

---

#### Task 7 — Index toolbar: collapse to one row

**Description:** Restructure the index page so search · date chips · view-switcher live on **one** toolbar row above the project list. Header band (title + stats strip + date range) sits above; toolbar is a distinct band; content (table or grid) sits below. Apply `/arrange` skill principles for spacing and hierarchy.

**Acceptance:**
- [x] Search input, date chips, and view-switcher are on the same row at desktop widths (≥1024px)
- [x] Stats strip ("X projects · Y sessions · Z messages · W tokens") is grouped with title above the toolbar — not floating as separate pills
- [x] Toolbar has consistent vertical alignment; no element looks orphaned
- [x] Page rhythm reads: header band → toolbar → content (single visual flow)
- [x] At narrow widths (<1024px), elements gracefully wrap without breaking

**Verify:**
- [x] `cargo build` succeeds
- [ ] Regenerate; visually confirm at 1024px, 1280px, 1440px
- [ ] Compare against `design.md` spacing scale; flag any deviation

**Dependencies:** Tasks 4, 5, 6

**Files:** `templates/index.html`, possibly `templates/components/header.html` and `templates/components/status_bar.html`

**Scope:** M

---

### Checkpoint: Index Page Complete

- [x] `just ci` passes
- [ ] Regenerated `index.html` matches all index-page success criteria from spec (criteria 1–5, 7)
- [x] Self-containment test passes
- [ ] User reviews the index page before per-project work begins

---

### Phase 3: Per-Project Page

#### Task 8 — Per-project page: apply name + path treatment

**Description:** In `templates/project.html` (rendered as `combined_transcripts.html`), replace any encoded-path label with `display_name` as the primary heading. Show the full encoded path as a muted subtitle below the title. Apply token palette from Phase 1.

**Acceptance:**
- [x] Page heading is `display_name`
- [x] Encoded path appears as a muted subtitle
- [x] Page uses warm bg + accent tokens consistently with index page

**Verify:**
- [x] `cargo build` succeeds
- [ ] Regenerate; open a project's `combined_transcripts.html`; visually confirm

**Dependencies:** Tasks 2, 3

**Files:** `templates/project.html`

**Scope:** S

---

#### Task 9 — Per-project page: add search + date chips

**Description:** Port the search-box and date-chip pattern from the index page to `templates/project.html`. Search targets session content/title; date chips filter sessions by their last-activity timestamp. Reuse JS by extracting a shared module if `index.js` doesn't already support being driven over a different dataset; otherwise extend in place. Decision documented inline in the implementation.

**Acceptance:**
- [x] Search box appears above the session list with placeholder `Search sessions...`
- [x] Date chips (All time / Today / Last 7 days / Last 30 days / custom range) appear next to search, matching index-page UX
- [x] Filters operate client-side over the rendered session list
- [x] No external JS dependency added
- [x] Per-project page toolbar mirrors index page toolbar layout (single row, same spacing)

**Verify:**
- [x] `cargo build` succeeds
- [ ] Regenerate; navigate into a project; verify search filters by typing
- [ ] Click each date chip; verify session list filters correctly
- [x] Self-containment test passes
- [x] `just ci` passes

**Dependencies:** Tasks 7, 8

**Files:** `templates/project.html`, `assets/index.js` (extend or split), possibly `assets/filters.js` (new shared module)

**Scope:** M

---

### Checkpoint: Per-Project Page Complete

- [x] All spec success criteria 1–9 met (build/tests pass; visual pending regeneration)
- [ ] User reviews per-project page

---

### Phase 4: Polish

#### Task 10 — `/arrange` pass on index + per-project pages

**Description:** Invoke the `/arrange` skill against the now-coherent index and per-project pages. Apply its recommendations for spacing rhythm, alignment, and visual hierarchy. Update `design.md` if any token is adjusted.

**Acceptance:**
- [x] Spacing scale is consistent across both pages
- [x] No visual rhythm breaks (orphaned elements, mismatched gaps)
- [x] Any token change is reflected in `design.md`

**Verify:** Visual review against `design.md`

**Dependencies:** Tasks 7, 9

**Files:** `templates/index.html`, `templates/project.html`, possibly `design.md`, `assets/tailwind.config.js`

**Scope:** S–M (size depends on findings)

---

#### Task 11 — `/impeccable` pass

**Description:** Invoke the `/impeccable` skill against the result to push detail/craft (microcopy, button states, focus rings, edge cases). Apply selective recommendations that fit the warm/restrained aesthetic; reject any that push toward generic-AI-template territory.

**Acceptance:**
- [x] Focus rings use accent token
- [x] Hover/active states defined for all interactive elements
- [x] Microcopy reviewed (search placeholder, empty states, etc.)
- [x] No regressions to spec success criteria

**Verify:** Visual review; keyboard tab through the page; check empty-state if applicable

**Dependencies:** Task 10

**Files:** Templates as needed

**Scope:** S–M

---

#### Task 12 — Regenerate fixtures, run full CI, final verification

**Description:** Regenerate `tests/cclog-out/` so the committed fixture reflects the new UI. Run `just ci`. Walk every spec success criterion against the regenerated output.

**Acceptance:**
- [x] `just ci` passes
- [x] All 10 success criteria from the spec are demonstrably met

**Verify:**
- [x] All 10 spec success criteria verified (see checklist below)
- [x] `just ci` passes (141 tests, 0 clippy errors)
- [ ] Open `tests/cclog-out/index.html` in browser; walk criteria 1–7 and 9
- [ ] Open a `combined_transcripts.html`; verify criteria 6

**Dependencies:** Task 11

**Files:** `tests/cclog-out/**` (regenerated)

**Scope:** S

---

### Final Checkpoint

- [x] Every spec success criterion verifiable in regenerated output
- [x] `design.md` reflects the shipped visual system
- [x] `just ci` passes
- [x] Self-containment test passes
- [ ] User signs off

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Token rename breaks existing template classes en masse | Med | Phase 1 task ordering: introduce semantic tokens *alongside* existing classes, then migrate per-template in Phase 2/3 |
| Name extraction edge case (empty / all-dashes input) corrupts the page | Low | Explicit unit tests in Task 3 cover edge cases; fallback to full encoded string |
| Self-containment test breaks if accent forces a new icon font | Low | Use inline SVG icons; no external font/icon CDN; verify test passes after Task 6 |
| Per-project filter JS diverges from index JS | Med | Decision baked into Task 9: extract shared module if reuse would otherwise be copy-paste |
| `/impeccable` pushes toward generic aesthetic that contradicts the warm-craft direction | Med | Task 11 explicitly notes selective application; reject suggestions that re-introduce template-feel |
| Regenerated fixture diff is enormous and hard to review | Low | Single regeneration at the end (Task 12), not after each task; reviewer sees the full new state |

---

## Parallelization

- **Phase 1:** Tasks 1, 2, 3 can run in parallel (different files, no overlap). Task 2 needs Task 1's palette decisions to be locked in design.md first — but Task 3 (Rust name extraction) is fully independent.
- **Phase 2:** Tasks 4, 5, 6 are largely independent (table vs card vs view-switcher). Task 7 depends on all three.
- **Phase 3 & 4:** Strictly sequential.

---

## Open Questions

None. The spec resolved all blocking questions. New questions surfaced during implementation should be raised before continuing, not silently decided.

---

## Todo Quickref (for execution)

- [x] T1 — design.md
- [x] T2 — tailwind tokens
- [x] T3 — `Project::display_name` + tests
- [x] **Checkpoint:** foundation (partial — user review pending)
- [x] T4 — table: name + path
- [x] T5 — grid card: name + path + labels
- [x] T6 — segmented view-switcher
- [x] T7 — one-row toolbar + header band
- [x] **Checkpoint:** index page (partial — user visual review pending)
- [x] T8 — per-project page: name + path heading
- [x] T9 — per-project page: search + date chips
- [x] **Checkpoint:** per-project page (partial — user visual review pending)
- [x] T10 — `/arrange` pass
- [x] T11 — `/impeccable` pass
- [x] T12 — regenerate fixtures + CI
- [x] **Final checkpoint:** all 10 spec criteria met, just ci passes (pending user sign-off)
