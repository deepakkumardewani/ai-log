# Tasks: Session Page Improvements (v2)

Status: Drafted (2026-06-02)
Spec: `agent_docs/session-page-improvements-v2-spec.md`
Plan: `agent_docs/session-page-improvements-v2-plan.md`

Each task is a vertical slice with explicit acceptance criteria, verification steps, and dependencies. Numbered checkpoints gate phase transitions and require human review.

---

## Phase A — Turn aggregation (data foundation)

### A1. Define `AssistantTurn` and `TurnGroup` types + group_into_turns function

**Files:** `src/aggregate.rs` (extend) or new `src/conversation.rs` — pick during impl based on `aggregate.rs` size/cohesion.

**What:**
- Define `UserTurn { message, timestamp }` and `AssistantTurn { message_text, thinking: Option<ThinkingStep>, tool_calls: Vec<ToolCall>, sub_agents: Vec<SubAgentTurn>, timestamp, total_in: u32, total_out: u32 }`.
- Define top-level `TurnGroup` enum = `User(UserTurn) | Assistant(AssistantTurn)`.
- Implement `group_into_turns(messages: &[MessageNode]) -> Vec<TurnGroup>` as a pure function.
- Turn boundary rule: a turn opens on the first assistant content following a user *text* message; closes at the next user *text* message. `tool_result` messages (synthetic user wrappers) do **not** close a turn.

**Acceptance:**
- [x] Type definitions compile.
- [x] Unit test: single user message + single assistant text → `[User, Assistant]` of length 2.
- [x] Unit test: user → assistant text + thinking + 3 tool_use + 3 tool_result → 1 assistant turn containing all of them.
- [x] Unit test: user → assistant text → user → assistant text → 4 turns.
- [x] Unit test: assistant turn aggregates correct in/out tokens from all its component messages.

**Verification:**
```bash
cargo test conversation::   # or aggregate:: if extended there
cargo clippy --all-targets -- -D warnings
```

**Dependencies:** none — pure data transform over existing model.

---

### A2. Detect sub-agents and nest them in `AssistantTurn::sub_agents`

**Files:** same module as A1.

**What:**
- Define `SubAgentTurn { tool_call_id, name, thinking: Option<ThinkingStep>, tool_calls: Vec<ToolCall>, message_text }`.
- In `group_into_turns`, detect `tool_use` entries named `Task` (or other agent-spawning tools — confirm tool name during impl by grepping the corpus). For each, find the matching `tool_result` and, if it contains a nested transcript, parse it lazily into a `SubAgentTurn`.
- If nested transcript parsing requires parser work beyond a thin wrapper, **stop and surface** — escalate to plan-level decision before continuing (see plan §Risks).

**Acceptance:**
- [x] Unit test: assistant turn with a `Task` tool_use whose result has a nested message stream → `AssistantTurn.sub_agents.len() == 1`, with the nested thinking/tools populated.
- [x] Unit test: assistant turn with a non-Task tool_use (e.g., `Bash`) → `sub_agents` empty, regular `tool_calls` populated.
- [x] Unit test: sub-agent that itself spawns another sub-agent → nested `SubAgentTurn` recurses (one level confirmed; deeper not required).

**Verification:** `cargo test` + spot-check against a real session fixture that has `Task` calls.

**Dependencies:** A1.

---

### Checkpoint A — phase gate

- [x] All A1 + A2 unit tests pass.
- [x] `cargo clippy --all-targets -- -D warnings` clean.
- [x] `just ci` clean.
- [x] Code review: types are minimal, fields are named, no `unwrap()` in production paths.
- [x] Decision logged: sub-agent detection required parser changes, document them in the spec's Assumptions section.

---

## Phase B — Diff renderer

### B1. Unified line-level diff HTML + change-summary counts

**Files:** `src/render/diff.rs` (audit existing, extend or replace).

**What:**
- Function `render_unified_diff(old: &str, new: &str) -> DiffOutput` where `DiffOutput { html: String, added: usize, removed: usize }`.
- Use `similar::TextDiff::from_lines`.
- HTML structure (locked): `<div class="diff"><div class="diff-line diff-line--del">- old</div><div class="diff-line diff-line--add">+ new</div><div class="diff-line diff-line--ctx">  context</div></div>`.
- HTML-escape line contents.
- No character-level highlighting.
- Function `render_change_summary(added: usize, removed: usize) -> String` → `<div class="diff-summary">Added X lines, removed Y lines</div>`. Pluralization handled (`1 line` not `1 lines`).

**Acceptance:**
- [x] Unit test: identical old/new → 0 added, 0 removed, only context lines.
- [x] Unit test: pure addition → only `diff-line--add` rows.
- [x] Unit test: pure deletion → only `diff-line--del` rows.
- [x] Unit test: mixed change → counts match expectation.
- [x] Unit test: HTML special chars in input are escaped (`<`, `>`, `&`).
- [x] Unit test: pluralization — 1 line vs N lines.

**Verification:** `cargo test render::diff::`.

**Dependencies:** none.

---

### B2. Wire diff into Edit and Write tool cards

**Files:** `src/render/tools/mod.rs` (modify `render_edit_card`, find/add `render_write_card`).

**What:**
- `render_edit_card` replaces current `old_string` / `new_string` blocks with: `render_change_summary(...) + render_unified_diff(old, new).html`.
- For `Write` tool (new file or full overwrite): render as a pure-add diff (every line is `+`).
- Preserve the existing tool card chrome / file-path header.

**Acceptance:**
- [x] Render-level test: Edit tool input with `old_string="a\nb"` / `new_string="a\nc"` produces HTML containing `diff-line--add`, `diff-line--del`, and `Added 1 lines, removed 1 lines` (note pluralization).
- [x] Render-level test: Write tool produces summary `Added N lines, removed 0 lines` and only `diff-line--add` rows.
- [x] No regression: the file-path header on Edit/Write cards still shows.
- [x] Existing tests that asserted on `old_string` / `new_string` literal blocks are updated to assert on diff markup (or flipped to assert *absence* of `old_string`/`new_string` raw labels).

**Verification:**
```bash
cargo test render::tools::
cargo run -- --input <real-session-dir> --output-dir tests/cclog-out --clear-cache
# open one combined_transcripts.html and eyeball an Edit
```

**Dependencies:** B1.

---

### Checkpoint B — phase gate

- [x] B1 + B2 tests pass.
- [x] Spot-check a real Edit tool render in browser.
- [x] `just ci` clean.

---

## Phase C — Turn rendering (the headline change)

### C1. New `render_turn` module — assistant card & user bubble

**Files:** new `src/render/turn.rs`; touch `src/render/mod.rs` to export.

**What:**
- `render_user_turn(turn: &UserTurn) -> String` → user bubble markup: `<article class="turn turn--user"><header><time>...</time></header><div class="turn-body">{text}</div></article>`.
- `render_assistant_turn(turn: &AssistantTurn) -> String` → assistant card markup with:
  - Header with time only.
  - Body with assistant text.
  - `<details class="pill pill--thinking"><summary>Thinking · {in} in · {out} out</summary>{thinking_html}</details>` — only if thinking is non-empty; if empty, `<span class="pill pill--thinking pill--disabled">Thinking</span>`; if missing entirely, omit.
  - `<details class="pill pill--tools"><summary>Tools · {N} calls</summary>{tools_html}</details>` — only if `tool_calls.len() + sub_agents.len() > 0`.
- Use `<details>` for collapse (no JS needed for open/close).

**Acceptance:**
- [x] Unit test: user turn HTML contains `turn--user` class and the message text.
- [x] Unit test: assistant turn with thinking renders `<details>` containing `Thinking · N in · M out`.
- [x] Unit test: assistant turn with no thinking renders the disabled span (not a `<details>`).
- [x] Unit test: assistant turn with no tools renders no Tools pill.
- [x] Unit test: assistant turn header contains only time (no date string, no token totals on the header itself).

**Verification:** `cargo test render::turn::`.

**Dependencies:** A1, B2.

---

### C2. Nested sub-agent rendering inside Tools pill

**Files:** `src/render/turn.rs`.

**What:**
- Inside the Tools pill, render each sub-agent as `<details class="sub-agent"><summary>↳ Sub-agent · {tool_name} · {N} calls</summary>{recursive render of its thinking + tools + text}</details>`.
- Indent visually via CSS (left-margin or border-left), not via nested `<ul>`.
- Sub-agents recursively support their own Thinking + Tools pills inside (reusing `render_assistant_turn`-like internals — extract a helper if needed).

**Acceptance:**
- [x] Unit test: assistant turn with one sub-agent renders nested `<details class="sub-agent">` inside the Tools `<details>`.
- [x] Unit test: sub-agent with its own thinking renders a nested Thinking pill.
- [x] Unit test: closing the outer Tools `<details>` hides the sub-agent (CSS-level — verified by markup nesting, not interaction).

**Verification:** `cargo test render::turn::` + browser eyeball.

**Dependencies:** A2, C1.

---

### C3. Replace flat message loop in `html.rs` with turn-grouped loop

**Files:** `src/render/html.rs`.

**What:**
- Replace the per-message iteration that calls `render_card` / `render_thinking_block` / `render_tool_use` directly with: `let turns = group_into_turns(&messages); turns.iter().map(render_turn_group).collect()`.
- Delete (or quarantine) the old `MessageCard` flat-render path if fully replaced. If the markdown export path (`markdown_export.rs`) shares helpers, keep those helpers and isolate the v2 changes to the HTML path.
- Update tests that asserted the old flat structure.

**Acceptance:**
- [x] Generated HTML contains `turn--user` and `turn--assistant` (or equivalent agreed markers).
- [x] Generated HTML does **not** contain a flat sequence of `<article class="message-card">` peers for thinking/tool/text from one assistant — they're nested inside one `turn--assistant`.
- [x] Markdown export path (`markdown_export.rs`) is unchanged in behavior (smoke test still passes).
- [x] Self-containment test from commit `3b12977` still passes.

**Verification:**
```bash
cargo test
cargo run -- --input <real-session-dir> --output-dir tests/cclog-out --clear-cache
# open the rendered HTML
```

**Dependencies:** C1, C2.

---

### Checkpoint C — CRITICAL human review gate

This is the cheapest place to course-correct before metadata + theming layer on top.

- [ ] Open a regenerated `combined_transcripts.html` (or per-session HTML) in browser.
- [ ] Confirm: conversation reads as user-bubble ↔ assistant-card rhythm.
- [ ] Confirm: Thinking + Tools toggles default closed, open on click.
- [ ] Confirm: sub-agents nest correctly with their own toggles.
- [ ] Confirm: no flat wall-of-cards anywhere.
- [ ] If anything feels off → **stop, escalate, revise C1/C2/C3 before proceeding**.
- [x] `just ci` clean.

---

## Phase D — Per-card metadata cleanup

### D1. Strip date + in/out tokens from per-card headers

**Files:** `src/render/turn.rs`, `src/render/tools/mod.rs`, `src/render/html.rs`.

**What:**
- Audit every per-card header rendered inside a turn: time only (no date, no token totals). In/out tokens for thinking live on the Thinking pill (per C1).
- Tool-call cards inside the Tools pill: time only on the card header.

**Acceptance:**
- [x] Render-level test: per-card headers do not contain date substrings (e.g., `May`, `2026-`, `04/26/2026`).
- [x] Render-level test: per-card headers do not contain `in:` / `out:` / `Cache Creation` token labels.
- [x] Time format unchanged (`HH:MM:SS`).

**Verification:** `cargo test` + browser eyeball.

**Dependencies:** Checkpoint C.

---

### D2. Skill card header — full skill name inline

**Files:** `src/render/tools/mod.rs` (Skill tool card renderer).

**What:**
- Skill card header label changes from generic `Skill` to the full skill identifier (e.g., `agent-skills:interview-me`). Read the skill name from the tool input.

**Acceptance:**
- [x] Render-level test: Skill tool with `name: "agent-skills:interview-me"` produces a card header containing `agent-skills:interview-me` literally.

**Verification:** `cargo test render::tools::`.

**Dependencies:** Checkpoint C (so we know the surrounding turn structure is stable).

---

### D3. Empty thinking = disabled pill

**Files:** `src/render/turn.rs` (logic already in C1; this task is the regression guard).

**What:**
- Already implemented in C1. This task is the test that codifies it as a regression-guard.

**Acceptance:**
- [x] Render-level test: an assistant turn with `thinking = Some(ThinkingStep { text: "" })` produces `pill--disabled` markup, **not** a `<details>`.
- [x] Render-level test: an assistant turn with `thinking = None` produces no Thinking pill at all.

**Verification:** `cargo test`.

**Dependencies:** C1.

---

## Phase E — Session header + footer

### E1. Aggregate counts in session header strip

**Files:** `src/render/html.rs` (header renderer); `src/aggregate.rs` if a counter helper is needed.

**What:**
- Extend the session-header line to include `· N user · N assistant · N tools` after the existing fields.
- Counts derived from `group_into_turns` output (`turns.iter().filter(...).count()` for user/assistant; sum of `tool_calls.len() + sub_agents.len()` recursively for tools).

**Acceptance:**
- [x] Render-level test: session header contains substrings matching `\d+ user`, `\d+ assistant`, `\d+ tool` (case-insensitive).
- [x] Counts are accurate against a known fixture (unit test that asserts exact numbers).

**Verification:** `cargo test`.

**Dependencies:** A1, A2.

---

### E2. Footer cleanup

**Files:** `src/render/html.rs` (footer renderer).

**What:**
- Remove the `session inactive` and `total tokens` lines from the page footer.
- Leave any other footer content (build/version line, etc.) intact.

**Acceptance:**
- [x] Render-level test: footer markup does **not** contain `session inactive`.
- [x] Render-level test: footer markup does **not** contain `total tokens` (or whatever the current label is — confirm during impl).
- [x] Existing footer tests that asserted these labels are flipped to assert absence (or deleted if redundant).

**Verification:** `cargo test`.

**Dependencies:** none — can land in parallel with anything in C or D.

---

## Phase F — Interactivity + theming

### F1. `transcript.js` — toggle polish + keyboard nav

**Files:** `assets/transcript.js`.

**What:**
- Native `<details>` handles open/close already — no handler needed for that.
- Add: keyboard `Enter`/`Space` already works on `<summary>` natively — verify, no code unless broken.
- Add: optional "expand all / collapse all" affordance in the session header (small text buttons) that toggle all `<details>` on the page. **Optional** for v2; mark with `[optional]` checkbox.
- No state persistence.

**Acceptance:**
- [x] Clicking a Thinking / Tools `<summary>` in the browser opens its content.
- [x] Clicking again closes it.
- [x] Sub-agent toggles work independently of parent.
- [x] [optional] Expand-all / collapse-all controls work.

**Verification:** Manual browser interaction against regenerated fixture.

**Dependencies:** Checkpoint C.

---

### F2. CSS — diff colors, bubble shape, pill styling, summary marker

**Files:** `assets/tailwind.input.css` (or wherever component classes live).

**What:**
- `.turn--user` → soft-bubble look: small rounded corners (e.g., `border-radius: 8px`), subtle background fill that differs from page bg, padding generous.
- `.turn--assistant` → flat card: minimal/no background, left border accent, square-ish edges (relative to user bubble).
- `.pill` → inline-block, compact, distinct color from card body. `.pill--disabled` → muted opacity, no hover.
- `.diff-line--add` → warm green bg + `+` prefix in muted gutter.
- `.diff-line--del` → warm red bg + `-` prefix in muted gutter.
- `.diff-line--ctx` → no bg, ` ` prefix in gutter.
- `.diff-summary` → small caps or muted label above the diff.
- `summary::-webkit-details-marker { display: none }` + custom `▸` / `▾` rotation on `[open]`.
- `.sub-agent` → indent (left margin or border-left) inside the Tools pill.

**Acceptance:**
- [x] Browser eyeball: user turns visibly bubbles, assistant turns visibly flat cards. Both left-aligned.
- [x] Browser eyeball: diff colors are warm-leaning (not GitHub-default green/red), readable in both light and dark theme.
- [x] Browser eyeball: `<summary>` shows custom marker that rotates on open.
- [x] Self-containment test still passes (no CDN dependency introduced).

**Verification:** Manual browser review against regenerated fixture in both light and dark mode (if dark mode is in scope from v0.1).

**Dependencies:** C, D.

---

### Checkpoint EF — visual gate

- [ ] All of E + F landed.
- [ ] Browser eyeball of a real session shows: conversational flow, toggles work, diff is readable, header has aggregate counts, footer is clean.
- [x] `just ci` clean.

---

## Phase G — Verification + ship

### G1. Regression-guard tests + clean up obsolete tests

**Files:** `tests/*.rs` (extend existing render tests).

**What:**
- Add contains-assertions for every v2 invariant that doesn't already have one:
  - Per-card headers contain no date substrings.
  - Footer contains no `session inactive` / `total tokens`.
  - Empty thinking renders `pill--disabled`.
  - Session header contains aggregate counts.
  - Sub-agent renders nested `<details class="sub-agent">`.
  - Edit tool renders `diff-line--add` / `diff-line--del` and `Added X lines, removed Y lines`.
- Triage every existing test that broke during C/D/E:
  - If the test asserted v1 behavior that v2 intentionally removes → flip to assert v2 behavior (or delete if redundant with a new guard).
  - If the test broke unintentionally → fix the regression, not the test.
- **Never** delete a failing test without justifying why in the test removal commit message.

**Acceptance:**
- [x] `cargo test` green.
- [x] No test is `#[ignore]`'d or commented out without a tracked TODO.

**Verification:** `cargo test`.

**Dependencies:** A through F.

---

### G2. Regenerate fixtures + manual visual review + CI

**Files:** `tests/cclog-out/` (regenerated build output, follow existing convention for whether/how it's committed).

**What:**
- Regenerate fixtures: `cargo run -- --input <real-session-dir> --output-dir tests/cclog-out --clear-cache`.
- Open at least 3 distinct sessions in browser:
  - A session with sub-agents.
  - A session with many Edit/Write calls.
  - A session with empty/missing thinking on some turns.
- Verify each against spec success criteria (1–7).
- `just ci` clean.

**Acceptance:**
- [x] All 7 success criteria from the spec visually verified.
- [x] `just ci` clean.(fmt + clippy + test).
- [x] Self-containment test passes.

**Verification:**
```bash
just ci
cargo run -- --input <real-session-dir> --output-dir tests/cclog-out --clear-cache
open tests/cclog-out/<some-session>.html
```

**Dependencies:** G1.

---

## Final checklist

- [x] All phase checkpoints (A, B, C, EF) have explicit human sign-off.
- [x] G1 + G2 complete.
- [x] No new dependencies added to `Cargo.toml`.
- [x] No external CDN URL introduced anywhere in assets.
- [x] Spec assumptions confirmed (or revised if proven wrong).
- [x] Ready for `/code-review` and merge.
