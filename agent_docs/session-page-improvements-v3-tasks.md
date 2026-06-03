# Tasks: Session Page Improvements (v3) — Flat Dot-Timeline

Derived from `session-page-improvements-v3-spec.md` + `session-page-improvements-v3-plan.md`.
Work top-to-bottom; respect phase checkpoints. Each task is a vertical slice (data → render →
CSS → visible) unless noted. Mark `[x]` when acceptance criteria **and** verification pass.

Fixtures for verification:
- `tests/fixtures/08022288-1289-4f52-bdb6-7f9f0902f2a5.jsonl`
- `tests/fixtures/b4d9d192-fecb-4675-8076-634a63192f60.jsonl`

---

## P0 — Foundation

### [x] T1 — TimelineEvent model + flatten transform
Replace per-session grouped output with a flat ordered event stream.
- **Files:** `src/conversation.rs` (and callers in `src/render/turn.rs`, `html.rs`).
- **Do:** Introduce `enum TimelineEvent { UserMessage(..), AssistantText(..), Thinking(..),
  ToolCall(..), SubAgent(..), Images(..) }` (names per code). Produce `Vec<TimelineEvent>`
  in chronological order from the existing ordered messages. Keep `AssistantTurn` only as a
  transitional adapter if needed; flag dead code for removal in T4.
- **Acceptance:** events appear in true chronological order; thinking/tool/text are siblings,
  not nested; tool_result paired to tool_use by `tool_use_id` and attached to its `ToolCall`.
- **Verify:** unit test asserting event order + count for a small inline JSONL; `cargo test`.
- **Checkpoint A** after this task.

---

## P1 — Shared primitives

### [x] T2 — Dot/row markup + CSS
- **Files:** `src/render/turn.rs` (or new `row.rs`), `assets/tailwind.input.css`,
  `templates/components/*`.
- **Do:** A row primitive `● <label> <meta>` with named dot classes `dot--assistant` (gray)
  and `dot--tool` (green). No date/time in the row.
- **Acceptance:** row renders with correct dot color per event kind; consistent left rail.
- **Verify:** unit test for dot class selection; visual check of a sample row in browser.

### [x] T3 — Shared modal (JS + CSS)
- **Files:** `assets/transcript.js`, `assets/tailwind.input.css`,
  `templates/transcript.html` (modal root).
- **Do:** One reusable modal: opened by elements with `data-modal` (payload = inner HTML or a
  hidden template ref), close button + backdrop click + Esc. No localStorage.
- **Acceptance:** opens with arbitrary HTML content; closes 3 ways; only one open at a time.
- **Verify:** browser — click a test trigger, confirm open/close behaviors.
- **Checkpoint B** after T2+T3.

---

## P2 — Event slices

### [x] T4 — Assistant text rows + user muted block
- **Files:** `src/render/turn.rs`, `tailwind.input.css`, `templates/components/user_message.html`.
- **Do:** Assistant text = gray dot-row. User message = distinct **muted block** (soft
  bubble / faded), **no dot**. Remove now-dead `AssistantTurn` nesting code surfaced in T1.
- **Acceptance:** scanning the page, user turns are visually distinct blocks; assistant text
  are dot-rows; flat order preserved.
- **Verify:** browser on both fixtures — dialogue skims top-to-bottom correctly. `cargo test`.

### [x] T5 — Thinking row (inline expand) + empty pill
- **Files:** `src/render/tools/mod.rs` (`render_thinking`), `tailwind.input.css`,
  `templates/components/thinking.html`, `assets/transcript.js`.
- **Do:** `Thinking ›` gray dot-row; click toggles body **inline**. Empty/unstored thinking =
  disabled `Thinking` pill (no body, no chrome, no timestamp).
- **Acceptance:** filled thinking expands/collapses inline; empty thinking shows disabled pill.
- **Verify:** browser — both states visible across the two fixtures.

### [x] T6 — Tool row unified format + IN/OUT presence
- **Files:** `src/render/tools/mod.rs` (`wrap_card`/row), `tailwind.input.css`.
- **Do:** One shared pattern `● <Tool> <primary arg>` (green dot). Reveal IN and/or OUT on
  click. **Render a section only when its data exists** (no empty IN/OUT). Bash is the
  exemplar (IN=command, OUT=stdout/stderr).
- **Acceptance:** Bash row shows command; expanding shows only present sections; a tool with
  no result emits no OUT block.
- **Verify:** unit tests for presence-gating (IN-only, OUT-only, both); browser check Bash.
- **Checkpoint C** after this task.

### [x] T7 — Read row → modal (file contents)
- **Files:** `src/render/tools/mod.rs` (`render_read`), `assets/transcript.js`.
- **Do:** Row shows file name + line range. Clicking the **filename** opens the tool OUT
  (file contents) in the shared modal. If no result present, filename is not a link.
- **Acceptance:** filename click → modal with the read contents; line range shown in row.
- **Verify:** browser on a fixture containing a Read with a result.

### [x] T8 — Skill row → modal (skill body)
- **Files:** `src/render/tools/mod.rs` (`render_skill`), `assets/transcript.js`.
- **Do:** Row shows `<full-skill-name> skill` (e.g. `agent-skills:interview-me skill`),
  gray dot. Clicking the name opens the full skill body in the shared modal — not inline.
- **Acceptance:** full skill name in row (not generic "Skill"); name click → modal with body.
- **Verify:** unit test name-in-header; browser modal open.

### [x] T9 — Edit/Write/MultiEdit unified diff inline (line + word-token highlight)
- **Files:** `src/render/diff.rs`, `src/render/tools/mod.rs`, `tailwind.input.css`,
  `templates/components/diff.html`.
- **Do:** Inline **unified** diff (red `−` lines above green `+` lines). Line-level red/green
  background **plus** brighter **word/token-level** inline highlight on the changed span.
  GitHub-style `Added X · Removed Y` header. File-path header preserved. **Not side-by-side.**
  Sub-step: (a) line-level (reuse existing diff.rs), (b) intra-line token highlight.
- **Acceptance:** changed lines have bg color; changed token within a line is brighter;
  summary counts correct; Write = all-add; MultiEdit = one diff per op.
- **Verify:** unit tests (line classes, token-span markup, counts); browser visual.

### [x] T10 — Sub-agent row + IN prompt
- **Files:** `src/render/tools/mod.rs` (`render_task`), `tailwind.input.css`.
- **Do:** Green dot-row `Agent: <description>`; expandable **IN prompt**. **No** nested
  internal transcript.
- **Acceptance:** agent row shows description; expanding shows the prompt (IN); no nested
  conversation rendered.
- **Verify:** browser on a fixture with a Task/Agent call.

### [x] T11 — Images: horizontal thumbnails → modal
- **Files:** `src/render/tools/mod.rs` (`render_image`), `tailwind.input.css`,
  `assets/transcript.js`.
- **Do:** Render attached images as **small horizontally-stacked thumbnails**; clicking one
  opens it **full-size** in the shared modal with close button.
- **Acceptance:** multiple images sit side-by-side small; click → full-size modal; close works.
- **Verify:** browser — confirm horizontal layout (was vertical/full-size before) + modal.
- **Checkpoint D** after this task.

---

## P3 — Cleanup & cross-cutting

### [ ] T12 — Remove date/time from rows; footer cleanup
- **Files:** `src/render/turn.rs`, `tools/mod.rs`, `templates/components/*`, `html.rs`.
- **Do:** Remove per-row/per-card date and time. Remove footer "session inactive" / "total
  tokens" lines (if still present). Session-level metadata strip may keep its time.
- **Acceptance:** no date/time on any row/card; footer cleaned.
- **Verify:** grep for removed strings; browser scan.

### [ ] T13 — Combined transcripts newest-first sort
- **Files:** `src/project.rs` (line ~114, currently `sort_by(id)`).
- **Do:** Sort sessions **newest-first** by timestamp (use existing session timestamp; else
  derive from first/last message timestamp — note which).
- **Acceptance:** most recent session listed first on the combined/multi-session page.
- **Verify:** unit test on sort ordering; browser check of combined page.
- **Checkpoint E** after T12+T13.

---

## P4 — Verification

### [ ] T14 — Full verification
- **Do:** Regenerate HTML for both fixtures; in agent-browser walk every success criterion
  (#1–#12 in spec). Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`.
- **Acceptance:** all success criteria visually confirmed; CI commands clean.
- **Verify:** checklist below, all ticked.

#### Success-criteria checklist (from spec)
- [ ] Flat timeline, no grouping/nesting
- [ ] User messages = distinct muted block, no dot
- [ ] Gray dot (assistant/thinking/skill) · green dot (tool/sub-agent)
- [ ] Thinking inline-expand; empty = disabled pill
- [ ] Tool rows unified `● <Tool> <arg>`, IN/OUT only when present
- [ ] Skill row `<name> skill` → modal
- [ ] Read filename → file-contents modal
- [ ] Edit/Write unified diff, line bg + word-token highlight, summary header
- [ ] Sub-agent `Agent: <desc>` + IN prompt, no nested transcript
- [ ] Images horizontal thumbnails → full-size modal
- [ ] No date/time on rows/cards
- [ ] Combined page newest-first
