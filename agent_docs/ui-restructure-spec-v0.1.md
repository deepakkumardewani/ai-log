# Spec: UI Restructure & Visual Refresh (v0.1)

Status: Approved (intent confirmed via interview, 2026-05-31)
Owner: Deepak
Related: builds on `ui-improvements-tasks-v0.1.md` (data foundation + interactivity already landed)

---

## Objective

Restructure the cclog static HTML viewer so it reads as a crafted product UI rather than a generic AI-template dashboard. Scope is the **index page** (project list) and the **per-project page** (`combined_transcripts.html`). The per-session transcript view is out of scope.

### User
A Claude Code user who runs `cclog` locally and browses their session history in a browser. Single user per generated output. Static HTML, no runtime/server.

### Why now
The data foundation (filters, search, pagination, cache, session cleanup) just landed. The visual layer has lagged: paths render as encoded filesystem strings, the grid/list toggle is a mystery purple icon button, card and table show inconsistent labels for the same data, the toolbar is stacked across three vertical rows, and the accent color reads as "AI-generated default purple."

### Success criteria
1. Project rows/cards render the project **name** (last path segment) as the primary identifier, not the encoded path.
2. Table project column shows bold name on line 1, muted truncated path on line 2.
3. Grid card shows bold name + single-line truncated path (full path on `title` tooltip) + **labeled** stats including "Last activity" (parity with table).
4. View switcher is **two side-by-side icon buttons** (grid + list), both always visible, active state visually obvious — not a single mystery toggle.
5. Index page toolbar collapses to **one row**: search (left) · date chips (center/middle) · view-switcher (right).
6. Per-project page (`combined_transcripts.html`) gains the same search + date-chip pattern as the index page. Search targets session content/title; date chips filter by session last-activity.
7. Light theme background is a warm neutral (not pure white); accent is warm (terracotta/amber family); no purple anywhere in the rendered output.
8. A `design.md` is produced capturing the new visual system (palette, type scale, spacing, component patterns).
9. The asset-bundling self-containment test (added in `3b12977`) continues to pass — no external CDN dependencies introduced.
10. `just ci` (fmt + clippy + test) passes.

---

## Assumptions

Listed so they can be challenged before code lands:

1. Project name extraction happens in Rust (`src/project.rs`) and is passed to templates as a new field (e.g., `name`), separate from the existing encoded path. Templates do not parse paths.
2. Name = last `-`-separated segment of the encoded directory name (e.g., `-Users-deepakdewani1-Documents-Programs-claude-code-log` → `claude-code-log`). No collision handling in v0.1.
3. Tailwind remains the styling layer (`assets/tailwind.config.js`, `assets/tailwind.input.css`). The accent recolor + warm neutrals are theme-token changes, not a CSS framework swap.
4. Existing client-side filter/search JS (`assets/index.js`) is the right place to extend for the per-project page; we add a parallel module or reuse, rather than introducing a framework.
5. Date-chip behavior on the per-project page mirrors index (All time / Today / Last 7 days / Last 30 days / custom range) but operates on session timestamps, not project timestamps.
6. Existing generated test fixtures under `tests/cclog-out/` will be regenerated after the change. We do **not** maintain backward visual compatibility with previously generated HTML.
7. The `/arrange` and `/impeccable` skills are invoked during the implementation phase (not now). The spec defines *what*; those skills inform *how* during the build.

→ Flag any assumption now or it stands.

---

## Out of Scope (explicit)

- Name-collision handling when two projects share a last segment (e.g., `~/work/repo` and `~/personal/repo`). Deferred until needed.
- Session-specific filters on the per-project page (by model, message count, duration). Only search + date chips in v0.1.
- Any change to the per-session transcript view (`templates/transcript.html`, `templates/components/*`). This spec covers index + project list only.
- New CLI flags, output formats, or backend changes beyond the name-extraction field.
- Migration of previously generated HTML directories. Regeneration produces the new UI.
- Mobile-first redesign. Desktop browser is the primary target; responsive behavior should not regress but is not the design driver.

---

## Tech Stack

- Rust (templates rendered server-side at CLI invocation time)
- Tera (template engine, per `templates/*.html`)
- Tailwind CSS (compiled to `assets/output.css` or equivalent; config in `assets/tailwind.config.js`)
- Vanilla JS for client-side interactivity (`assets/index.js`, `assets/transcript.js`)
- No new dependencies expected. If a dep is added (icon library, etc.), it must be ask-first per Boundaries.

---

## Commands

```
Build (debug):     cargo build
Build (release):   cargo build --release
Format:            cargo fmt --all
Lint:              cargo clippy --all-targets -- -D warnings
Test:              cargo test
Full CI:           just ci
Run CLI:           cargo run -- <args>        (regenerates test output)
Tailwind rebuild:  (per existing project convention — confirm in implementation)
```

---

## Project Structure (relevant subset)

```
src/
  project.rs              → Project model — ADD: name extraction (last segment)
  render/
    index.rs              → Project-list page data shaping
    project.rs            → Per-project page data shaping
    mod.rs                → Render orchestration
templates/
  base.html               → Shared shell (theme tokens, fonts)
  index.html              → Project list page — MAJOR RESTRUCTURE
  project.html            → Per-project session list — ADD search + date chips
  components/
    header.html           → Page header — may consolidate stats strip
    status_bar.html       → Possibly affected by toolbar restructure
assets/
  tailwind.config.js      → ADD warm neutrals + terracotta/amber palette; REMOVE purple
  tailwind.input.css      → Theme tokens for warm light bg
  index.js                → EXTEND: power per-project page filters too (or split file)
agent_docs/
  ui-restructure-spec-v0.1.md   → this file
  ui-restructure-tasks-v0.1.md  → plan + todos (produced by /plan)
  design.md                     → visual system deliverable (produced during build)
```

---

## Visual Direction (summary; full system goes in `design.md`)

- **Light theme bg:** warm neutral (warm off-white, ~#FAF7F2 ballpark — final value chosen during build). Dark theme stays close to current but accent recolors.
- **Accent:** warm — terracotta or amber family. Active toolbar buttons, primary CTAs, focus rings, active filter chip.
- **Typography:** existing monospace identity stays for stats and meta; project names move to a stronger sans-serif weight for readability.
- **Toolbar:** one row — search left, date chips middle, view-switcher right. Aligned, not floating.
- **Cards:** uniform height, calm spacing, labeled stats. No path-as-title.
- **Table:** project column = name (semibold) + path (muted, truncated). Same date label as cards.
- **View-switcher:** segmented two-button control (grid icon + list icon), shared container, active state filled with accent.

`design.md` will lock the exact tokens (hex values, spacing scale, type ramp, component states).

---

## Code Style

Rust per `.claude/skills/rust-best-practices/SKILL.md`. For the name-extraction addition:

```rust
// src/project.rs — illustrative; final form may differ
impl Project {
    /// Human-readable project name derived from the encoded directory.
    /// Example: "-Users-deepak-Documents-Programs-cclog" → "cclog"
    pub fn display_name(&self) -> &str {
        self.encoded_dir
            .rsplit('-')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.encoded_dir)
    }
}
```

Templates reference `project.display_name` as a string field exposed via the render context — no logic in templates.

Tailwind classes prefer semantic theme tokens (`bg-surface`, `text-accent`) over raw color utilities (`bg-amber-500`), defined once in `tailwind.config.js`.

---

## Testing Strategy

- **Unit (Rust):** Add tests in `src/project.rs` covering `display_name` — normal case, leading dash, trailing dash, single segment, empty string.
- **Integration:** Existing render tests regenerate sample output; visually inspect via the test fixture directory after each significant change.
- **Self-containment:** The existing test added in `3b12977` (asset bundling) must continue to pass. No new external CDN refs.
- **Manual verification:** After build, regenerate `tests/cclog-out/` and open both `index.html` and a `combined_transcripts.html` in a browser. Verify all 10 success criteria.
- **No automated visual regression** in v0.1. Manual sign-off against design.md.

---

## Boundaries

**Always do**
- Run `just ci` before declaring a task done
- Update `design.md` when a visual token changes during build
- Keep template logic in Rust (computed fields), not in Tera expressions
- Preserve self-containment (no external CDN/font/icon requests)

**Ask first**
- Adding a new dependency (icon library, etc.)
- Touching `templates/transcript.html` or `templates/components/*` beyond what the toolbar/header restructure forces
- Changing the file structure under `assets/` (splitting `index.js`, etc.)
- Renaming any public field on `Project` (would break the templates and tests)
- Any change that affects generated output structure (file paths, asset names) — those have downstream effects on `cclog open` and the cache

**Never do**
- Reintroduce purple anywhere in the rendered viewer
- Render encoded paths as primary labels
- Add CDN links (breaks self-containment test)
- Skip the self-containment test
- Commit without explicit user request

---

## Open Questions

None at spec time. The interview resolved:
- Accent direction → warm (terracotta/amber); exact hex picked during build
- Layout direction → full restructure; one-row toolbar
- Session page filters → search + date chips only (symmetric with index)
- Project name → last segment; no collision handling in v0.1
- Card path → truncated single-line with hover tooltip

If any of these need to change post-build, update this spec first, then re-plan affected tasks.
