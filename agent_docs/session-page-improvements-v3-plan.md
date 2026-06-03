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
