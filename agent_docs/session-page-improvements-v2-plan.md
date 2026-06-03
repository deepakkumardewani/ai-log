# Plan: Session Page Improvements (v2)

Status: Drafted (2026-06-02) — for human review before tasks start
Spec: `agent_docs/session-page-improvements-v2-spec.md`
Tasks: `agent_docs/session-page-improvements-v2-tasks.md`

---

## Current State (verified against tree, 2026-06-02)

| File | Role today |
|---|---|
| `src/render/html.rs` (~38 KB, 786+ lines) | Per-session/per-transcript renderer. Owns `MessageCard`, content-item dispatch (`render_thinking_block`, `render_tool_use`, `render_tool_result`), session header, footer. |
| `src/render/tools/mod.rs` | Tool-specific card renderers (`render_edit_card`, `render_thinking`, etc). Currently uses `<details class="thinking-block" open>` — thinking is already a `<details>` but defaults *open* and has heavy card chrome. |
| `src/render/diff.rs` (~3.5 KB) | Existing diff helper. Already present — may or may not produce unified HTML; **task B1 audits and extends**. |
| `src/model/content.rs` | `Message`, `ContentItem::{Thinking, ToolUse, ToolResult}` enum. **Untouched by this work.** |
| `src/session.rs` (~323 lines) | `MessageNode` parent-child threading. **Source of truth for the flat message stream.** |
| `src/aggregate.rs` | Tool / token aggregation. Good home for new turn-grouping or session-count aggregation. |
| `assets/transcript.js` (~10 KB) | Existing session-page JS. Home for new toggle handlers. |
| `assets/tailwind.input.css` (~35 KB) | Theme tokens. Add diff colors + bubble shapes. |
| `Cargo.toml` | `similar = "2"` already present — no new deps needed. ✓ |
| `tests/` | Uses `insta`, `assert_cmd`, `predicates`. Render contains-assertions live here. |

**Self-containment test (commit `3b12977`)** is the binding non-regression — no external CDN.

---

## Dependency Graph

```
            ┌────────────────────────────────────────────┐
            │  A. Turn aggregation (data layer)          │
            │     Vec<MessageNode> → Vec<AssistantTurn>  │
            │     + SubAgentTurn detection               │
            └─────────────────┬──────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
   ┌──────────────────────┐       ┌──────────────────────────┐
   │ B. Diff renderer     │       │ E1. Session-header        │
   │    Edit/Write diff   │       │     aggregate counts      │
   │    + change summary  │       │     (uses A's totals)     │
   └──────────┬───────────┘       └──────────────────────────┘
              │
              ▼
   ┌────────────────────────────────────────────────────────┐
   │ C. Turn rendering (consumes A + B)                      │
   │    C1: assistant card / user bubble                     │
   │    C2: sub-agent nested + own toggle                    │
   │    C3: replace flat loop in html.rs                     │
   └──────────────┬─────────────────────────────────────────┘
                  │
                  ▼
   ┌────────────────────────────────────────┐
   │ D. Per-card metadata cleanup            │
   │    D1: time-only headers                │
   │    D2: skill name in header             │
   │    D3: empty thinking = disabled pill   │
   └──────────────┬─────────────────────────┘
                  │
                  ▼
   ┌────────────────────────────────────────┐
   │ E2. Footer cleanup                      │
   │     (independent — can land anytime     │
   │      after E1, but grouped here)        │
   └──────────────┬─────────────────────────┘
                  │
                  ▼
   ┌────────────────────────────────────────┐
   │ F. Interactivity + theming              │
   │    F1: transcript.js toggle handlers    │
   │    F2: CSS for diff + bubble + pills    │
   └──────────────┬─────────────────────────┘
                  │
                  ▼
   ┌────────────────────────────────────────┐
   │ G. Verification                         │
   │    G1: tests pass + new regression      │
   │         guards                          │
   │    G2: fixtures regenerated +           │
   │         manual visual review            │
   └────────────────────────────────────────┘
```

**Critical path:** A → C → F → G. Phases B, D, E can land in parallel with C once A is in.

---

## Vertical Slicing Philosophy

Each task delivers an **end-to-end visible change** or a **fully tested isolated unit**. No "wire the data layer now, render later" splits — that's horizontal slicing and produces dead code in tree.

The one exception is **Phase A**: the turn-aggregation type is genuinely a prerequisite that's worth testing in isolation before consuming it, because getting the grouping wrong invalidates everything downstream. It ships behind a unit test, not user-visible until C.

---

## Phases & Checkpoints

### Phase A — Turn aggregation (data foundation)
**Goal:** A pure function `group_into_turns(messages: &[MessageNode]) → Vec<TurnGroup>` plus types, fully unit-tested.
**Checkpoint A:** Unit tests green; the type compiles cleanly into the existing tree without yet being consumed by the renderer.

### Phase B — Diff renderer
**Goal:** Edit/Write tool calls render as unified HTML diff with a `Added X · Removed Y` header.
**Checkpoint B:** Unit tests for diff output green; renders correctly in an isolated test fixture.

### Phase C — Turn rendering (the headline change)
**Goal:** Per-session page now renders as grouped turns with collapsible thinking + tools, user bubble vs assistant card, sub-agents nested.
**Checkpoint C (CRITICAL — human review):** Open a regenerated fixture in browser. Confirm conversational rhythm, toggle behavior, sub-agent nesting. If anything feels off, this is the cheapest place to course-correct before metadata + theming layer on top.

### Phase D — Per-card metadata cleanup
**Goal:** Time-only headers, skill name in header, empty thinking as disabled pill.

### Phase E — Session header + footer
**Goal:** Header strip shows aggregate counts; footer drops session-inactive + total-tokens.

### Phase F — Interactivity + theming
**Goal:** Toggles work in browser; diff colors + bubble shape land in CSS.

### Phase G — Verification + ship
**Goal:** Regression-guard tests added; `just ci` clean; fixtures regenerated; visual review passes.

---

## Key Decisions (locked in plan; flag now to revisit)

1. **Toggle mechanism**: native `<details>`/`<summary>` (semantic, zero JS for open/close). The existing `transcript.js` only needs to add: (a) sub-agent nested-details progressive enhancement if any (b) keyboard nav. The Python version's "▼ 16 thoughts" chip becomes `<summary>Thinking · 3 in · 728 out</summary>`.

2. **Disabled "Thinking" pill** for empty thinking: rendered as a non-interactive `<span class="thinking-pill thinking-pill--empty">Thinking</span>` (not `<details>` at all). Cleaner than a disabled `<details>`.

3. **Diff library**: `similar` crate (already in `Cargo.toml`). Use `TextDiff::from_lines`.

4. **Diff color palette**: warm-leaning, harmonized with v0.1's terracotta accent. Concrete values picked in F2 against `design.md`; rough target — diff-red `#c0392b`-adjacent, diff-green `#7a8c43`-adjacent (warm sage instead of GitHub green). Final values land via the design tokens already in `tailwind.input.css`.

5. **Sub-agent detection**: scan tool_use entries for `Task` / `Agent` tool names and pair them with their corresponding `tool_result` (which contains the spawned agent's nested message stream). If the existing parser doesn't already expose the nested stream as parsed messages, we add a thin lazy-parse step in Phase A.

6. **Turn boundary rule**: a turn opens on the first assistant content following a user message, and closes at the next user message (a `tool_result` is a synthetic "user" wrapper — it does **not** close the turn; only a real user text message does). This matches how the conversation actually flows.

7. **Existing tests**: contains-assertions that look for old card markers (e.g., `thinking-block`, per-card date strings, footer "session inactive") will break. Treat each break as either (a) a v2 regression-guard *flip* (assert absence) or (b) genuine breakage to triage. Decided per-test during G1.

8. **Toggle state persistence**: none (session-page-local, resets on reload). Native `<details>` default state is closed; reopening a page = collapsed view, which matches "skim first" intent.

9. **Tool-name in collapsed toggle**: `▸ Tools · 51 calls` shows count only, not per-tool breakdown. If a user wants to know "which tools", they expand.

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| `html.rs` is 786+ lines, single file — risk of merge friction during turn-rendering rewrite | Extract turn-rendering helpers into a new `src/render/turn.rs` rather than growing `html.rs`. Keeps the rewrite localized. |
| Existing snapshot tests (`insta`) may have many redlines | Accept the diff in a single pass at G1 after manual review — don't bypass review. |
| Sub-agent nested transcripts might require parser work (out of original scope estimate) | If true, surface at A2 and decide: ship without sub-agent nesting in v2.0, then sub-agent in v2.1. Document the decision in the spec's "Out of scope" if deferred. |
| Native `<details>` styling varies across browsers (default `▶` marker) | Override with `summary::-webkit-details-marker { display: none }` + custom CSS triangle. Standard pattern. |
| Diff for very long `old_string` (e.g., 500-line file edit) bloats the page | Cap rendered diff lines (e.g., 200 visible, "show all" toggle) — track as v2.1 if it becomes a real problem. Not blocking v2.0. |

---

## Out-of-Plan (intentionally)

- Index page and per-project page (`combined_transcripts.html` listing) are untouched.
- No new dependencies.
- No JS framework. Vanilla + `<details>` only.
- No persistence of toggle state.
- No char-level diff highlighting.
- `markdown_export.rs` parallel rendering for markdown exports is **not** updated in v2 — markdown stays as-is. (Flag if the user wants it.)

---

## Human review gate

Before tasks start, confirm:

1. The dependency graph above matches your mental model — particularly that A is a hard prerequisite for C, but B / D / E can land in parallel.
2. The decision to use native `<details>` (vs custom JS toggle) is acceptable.
3. The decision to extract turn-rendering into a new `src/render/turn.rs` rather than growing `html.rs` is acceptable.
4. The deferral list in "Out-of-Plan" is correct — nothing should move back into scope.
5. The Phase C checkpoint (browser review before metadata/theming layers on) is honored, not skipped.

→ Proceed to tasks once confirmed.
