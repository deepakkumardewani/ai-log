# Plan: Session Page Improvements (v3) — Flat Dot-Timeline

Companion to `session-page-improvements-v3-spec.md`. Task checklist lives in
`session-page-improvements-v3-tasks.md`.

---

## Strategy

The v2 grouping (`AssistantTurn` nesting + Thinking/Tools toggles) is the thing being
removed. The spine of v3 is a **flat ordered `TimelineEvent` stream** per session; every
renderer consumes that stream. So we build the spine first, then deliver one event type at a
time as a complete vertical slice (data → render → CSS → visible in browser). Shared UI
primitives (dot/row markup, the one reusable modal) are built once, early, because three
slices depend on the modal.

**Vertical, not horizontal:** each task after the foundation produces something visibly
correct in the rendered HTML for the two fixtures — not "all the Rust, then all the CSS."

---

## Dependency Graph

```
P0 Foundation
  T1 TimelineEvent model + flatten transform (conversation.rs)
        │
        ├─────────────► everything below consumes the event stream
        ▼
P1 Shared primitives
  T2 Dot/row markup + CSS (gray/green dots, row layout)
  T3 Shared modal (JS + CSS + open/close)   ──► used by T7, T8, T11
        │
        ▼
P2 Event slices (each independent once P0+P1 land; ordered by visibility)
  T4 Assistant text rows + user muted block      (skeleton timeline renders)
  T5 Thinking row (inline expand) + empty pill
  T6 Tool row unified format + IN/OUT presence    ──► pattern reused by T7,T9,T10
  T7 Read row → modal (file contents)             [needs T3, T6]
  T8 Skill row → modal (skill body)               [needs T3, T6]
  T9 Edit/Write/MultiEdit unified diff inline      [needs T6, diff.rs]
  T10 Sub-agent row + IN prompt                    [needs T6]
  T11 Images: horizontal thumbnails → modal       [needs T3]
        │
        ▼
P3 Cleanup + cross-cutting
  T12 Remove date/time from rows; footer cleanup
  T13 Combined transcripts newest-first sort (project.rs)
        │
        ▼
P4 Verification
  T14 Full browser verification (both fixtures) + cargo fmt/clippy/test
```

---

## Phases & Checkpoints

### P0 — Foundation (spine)
- **T1** Define `TimelineEvent` enum and flatten the existing grouped structure into a flat
  ordered `Vec<TimelineEvent>` per session.
- **Checkpoint A:** `cargo test` green; a temporary debug render shows events in correct
  chronological order for both fixtures. **No visual styling yet.**

### P1 — Shared primitives
- **T2** dot/row HTML + CSS. **T3** one reusable modal (open via `data-modal`, close button,
  backdrop click, Esc).
- **Checkpoint B:** a hard-coded sample row renders with correct dot color; clicking a test
  trigger opens/closes the modal in the browser.

### P2 — Event slices (deliver one at a time, verify each in browser)
- T4 → T5 → T6 → (T7, T8, T9, T10, T11). T6 establishes the tool-row pattern T7/T9/T10 reuse.
- **Checkpoint C** (after T6): a Bash tool row shows `● Bash <cmd>` and reveals only the
  sections (IN/OUT) present in the log.
- **Checkpoint D** (after T11): all event types render correctly in both fixtures.

### P3 — Cleanup & cross-cutting
- T12 date/time removal + footer; T13 newest-first sort.
- **Checkpoint E:** no date/time on rows; combined page lists newest session first.

### P4 — Verification
- **T14** Regenerate both fixtures, walk the page in agent-browser against every success
  criterion; `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| v2 grouping is deeply wired into `turn.rs`/tests | T1 keeps a thin adapter; delete dead `AssistantTurn` nesting only after T4 renders. Rewrite/remove grouping tests in the slice that replaces them. |
| Read "file contents" not present in some logs | OUT section is presence-gated (T6); if no tool_result, the filename simply isn't clickable. |
| Word-token diff highlight complexity | Line-level lands first (already exists in diff.rs); intra-line token highlight added as a sub-step of T9 with its own test. |
| Modal reused 3 ways | Single generic modal (T3) takes arbitrary HTML payload; slices only supply content. |
| Session timestamp for sort | Prefer existing field; else derive from first/last message timestamp (note in T13). |

---

## Out of Scope (reaffirmed)
Turn-grouping/nesting, side-by-side diffs, sub-agent internal transcript, localStorage
persistence, index/project redesign beyond sort, relative timestamps.

---

## Review Gate
Present this plan + task list for human review before T1. Each phase checkpoint is a stop
point: do not start the next phase until the checkpoint passes.

---

# Round 2 — Refinements (post-ship)

v3 (T1–T11) shipped in commit `57ebbaa`. Round 2 is a polish + bugfix pass confirmed by
interview on 2026-06-03. Spec section: **Round 2 — Refinements**; tasks **T15–T25**.

## Strategy

These are mostly **independent** edits on a working page, not a dependent build-up. Two
exceptions create ordering: **R4** needs a small **custom tooltip primitive** and **R1** reuses
the **existing shared modal**, so build/confirm those primitives are sound first, then fan out.
Group the work by surface to minimize re-touching the same files:

1. **Shared primitives first** — tooltip component (R4) and confirm the modal accepts the
   overflow payload (R1). These unblock the file-name rows and IN/OUT.
2. **Layout & theming pass** (cheap, high visual impact, low risk): R7 width/centering,
   R6 user-block padding/contrast, R2 connector line. All CSS-centric in `tailwind.input.css`
   + templates; land them together and eyeball once.
3. **Data-stage fix**: R3 meta-message filtering at the timeline-event build (affects search
   counts too) — do before the regressions so the fixtures render clean.
4. **Tool-row behavior**: R1 IN/OUT clamp→modal, R4 basename+tooltip, R5 modal markdown.
   These share `render/tools/mod.rs` + `transcript.js`; sequence to avoid conflicts.
5. **Regressions**: R8 (vertical images), R9 (inline skill body) — each starts with a
   root-cause read of the current renderer, then the fix.
6. **Bug**: R10 filter-chip logic in `transcript.js` — isolated; root-cause against the named
   session, add a regression note.

**Vertical slices still apply:** every task ends with an agent-browser check on both fixtures.

## Round 2 Dependency Graph

```
R2-P0 Primitives
  T15 Custom tooltip component (JS+CSS)      ──► T18 (file-name rows)
  (shared modal already exists — R1 reuses it)
        │
        ▼
R2-P1 Layout & theming (parallel, CSS-centric)
  T16 Wider centered layout (R7)
  T17 User-block padding + contrast (R6)
  T22 Left connector line (R2)
        │
        ▼
R2-P2 Data stage
  T19 Meta-message filtering (R3)  [timeline-event build]
        │
        ▼
R2-P3 Tool-row behavior (share tools/mod.rs + transcript.js)
  T20 IN/OUT clamp → modal on overflow (R1)   [needs modal]
  T18 File-name basename + tooltip (R4)        [needs T15]
  T21 Modal markdown via comrak (R5)
        │
        ▼
R2-P4 Regressions + bug
  T23 Images horizontal (R8)      [root-cause first]
  T24 Skill body → modal (R9)     [root-cause first; uses T21 markdown]
  T25 Filter-chip bug (R10)       [root-cause vs session 08022288…]
        │
        ▼
R2-P5 Verification
  T26 agent-browser walk of R1–R10 on both fixtures + cargo fmt/clippy/test
```

## Round 2 Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| R5 markdown-rendering code-file reads (e.g. `.ts`) would mangle them | Gate: only skill/`.md` bodies through comrak; code files keep syntect/`<pre>`. Decide by tool kind + content type, not blindly. |
| R3 filtering hides real content (over-filtering) | Combine flag-driven (`isMeta`/`isSidechain`) with tightly-scoped text patterns; add a unit test asserting a real prompt survives and the 3 known patterns drop. |
| R1 overflow detection in pure CSS may not know when to show the click affordance | Make the whole clamped block clickable but only when a `is-clamped` marker is set; if exact overflow detection is needed, a tiny JS pass on load can tag overflowing blocks. |
| R10 fix could regress the existing chip filtering | Reproduce on session `08022288…` first; add a JS/unit-level assertion that an unmatched tool stays visible under all-selected. |
| R8/R9 are regressions of "done" tasks | Start each with a root-cause read of the current renderer path; document why it diverged before patching. |
| R2 connector line misaligns on dot-less user blocks | Rail line is independent of the user block; verify alignment across every row type in-browser. |

## Round 2 Review Gate
Land primitives + layout (R2-P0/P1) and eyeball once before the behavior/regression tasks.
Each task ends with an **agent-browser** verification on both fixtures (per the user's
standing preference: agent-browser CLI, not chrome-devtools MCP).
