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

### [x] T12 — Remove date/time from rows; footer cleanup
- **Files:** `src/render/turn.rs`, `tools/mod.rs`, `templates/components/*`, `html.rs`.
- **Do:** Remove per-row/per-card date and time. Remove footer "session inactive" / "total
  tokens" lines (if still present). Session-level metadata strip may keep its time.
- **Acceptance:** no date/time on any row/card; footer cleaned.
- **Verify:** grep for removed strings; browser scan.

### [x] T13 — Combined transcripts newest-first sort
- **Files:** `src/project.rs` (line ~114, currently `sort_by(id)`).
- **Do:** Sort sessions **newest-first** by timestamp (use existing session timestamp; else
  derive from first/last message timestamp — note which).
- **Acceptance:** most recent session listed first on the combined/multi-session page.
- **Verify:** unit test on sort ordering; browser check of combined page.
- **Checkpoint E** after T12+T13.

---

## P4 — Verification

### [x] T14 — Full verification
- **Do:** Regenerate HTML for both fixtures; in agent-browser walk every success criterion
  (#1–#12 in spec). Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`.
- **Acceptance:** all success criteria visually confirmed; CI commands clean.
- **Verify:** checklist below, all ticked.

#### Success-criteria checklist (from spec)
- [x] Flat timeline, no grouping/nesting
- [x] User messages = distinct muted block, no dot
- [x] Gray dot (assistant/thinking/skill) · green dot (tool/sub-agent)
- [x] Thinking inline-expand; empty = disabled pill
- [x] Tool rows unified `● <Tool> <arg>`, IN/OUT only when present
- [x] Skill row `<name> skill` → modal
- [x] Read filename → file-contents modal
- [x] Edit/Write unified diff, line bg + word-token highlight, summary header
- [x] Sub-agent `Agent: <desc>` + IN prompt, no nested transcript
- [x] Images horizontal thumbnails → full-size modal
- [x] No date/time on rows/cards
- [x] Combined page newest-first

---

# Round 2 — Refinements (T15–T26)

Polish + bugfixes on shipped v3 (commit `57ebbaa`). Spec: **Round 2 — Refinements**; plan:
**Round 2** section. Order follows the Round 2 dependency graph. Each task ends with an
**agent-browser** check on both fixtures (NOT chrome-devtools MCP). Mark `[x]` when acceptance
**and** verification pass.

## R2-P0 — Shared primitives

### [x] T15 — Custom tooltip component (R4 dep)
- **Files:** `assets/transcript.js`, `assets/tailwind.input.css`, `templates/transcript.html`.
- **Do:** A small reusable tooltip primitive: element with `data-tooltip="<text>"` shows a
  positioned, escaped tooltip on hover; dismiss on mouse-leave. **Not** the native `title`
  attribute. One open at a time; positioned to stay in viewport.
- **Acceptance:** hovering a tagged element shows the custom tooltip; leaving hides it; text
  is HTML-escaped.
- **Verify:** agent-browser — hover a sample element, confirm custom tooltip appears.

## R2-P1 — Layout & theming (CSS-centric, land together)

### [x] T16 — Wider centered layout (R7)
- **Files:** `assets/tailwind.input.css`, `templates/transcript.html`.
- **Do:** Remove the empty left-gutter offset; widen the reading column to ~960–1100px,
  centered with balanced gutters both sides.
- **Acceptance:** timeline is centered (no large left gap); column noticeably wider.
- **Verify:** agent-browser at a normal desktop width on both fixtures.

### [x] T17 — User-block padding + contrast (R6)
- **Files:** `templates/components/user_message.html`, `assets/tailwind.input.css`.
- **Do:** Add inset padding so the user block no longer hugs the edge; give it a background
  with sufficient contrast in **both** light and dark themes while staying muted vs assistant.
- **Acceptance:** user block visibly distinct in both themes; text not flush-left.
- **Verify:** agent-browser, toggle both themes.

### [x] T22 — Left connector line (R2)
- **Files:** `assets/tailwind.input.css`, `templates/components/*`.
- **Do:** Vertical line down the left rail connecting consecutive dots; aligns with dot
  centers across all row types; does not break the dot-less user block.
- **Acceptance:** continuous thread line links dots; alignment correct for every row kind.
- **Verify:** agent-browser visual on both fixtures.

## R2-P2 — Data stage

### [x] T19 — Meta-message filtering (R3)
- **Files:** `src/conversation.rs` (timeline-event build), `src/session.rs`/`model` as needed.
- **Do:** At the event-build stage, drop Claude-Code meta: messages flagged `isMeta` /
  `isSidechain` where present, plus text-pattern fallback (`Caveat: … local commands` block,
  `/model …` & slash-command echoes, local-command stdout, `Set model to …`). Keep real
  prompts/assistant/thinking/tools. Filtering before render so search/filter counts are right.
- **Acceptance:** the 3 known patterns no longer appear; a real user prompt still renders.
- **Verify:** unit test (real prompt survives, 3 patterns dropped); agent-browser scan.

## R2-P3 — Tool-row behavior (share `render/tools/mod.rs` + `transcript.js`)

### [x] T20 — IN/OUT clamp → modal on overflow (R1)
- **Files:** `src/render/tools/mod.rs`, `assets/tailwind.input.css`, `assets/transcript.js`.
- **Do:** Keep IN/OUT visible-by-default with fixed max-height. When overflowing: fade bottom
  + make the block clickable → full IN/OUT text in the **shared modal**. Short content shown
  fully, not clickable. Tag overflowing blocks (CSS `max-height`; a tiny JS load-pass may set
  an `is-clamped` flag for the click affordance).
- **Acceptance:** long OUT clamps + opens full text in modal on click; short OUT not clickable.
- **Verify:** unit test (modal payload = full string); agent-browser on a long Bash OUT.

### [x] T18 — File-name basename + tooltip (R4) [needs T15]
- **Files:** `src/render/tools/mod.rs` (Read/Write/Edit/MultiEdit rows), templates, CSS.
- **Do:** Show **basename only** after the tool name (`Read — html.rs`), left-aligned (fix the
  center-alignment). Attach the **full path** via the T15 `data-tooltip` on hover.
- **Acceptance:** only basename inline; left-aligned; hover shows full path in custom tooltip.
- **Verify:** unit test (row contains basename, tooltip carries full path); agent-browser hover.

### [x] T21 — Modal markdown via comrak (R5)
- **Files:** `src/render/tools/mod.rs` (skill/file-contents modal payloads).
- **Do:** Route skill/markdown modal bodies through existing `render::markdown::render()`
  (comrak — already a dep). **No new crate.** Code-file reads stay code (syntect/`<pre>`);
  non-markdown stdout keeps `<pre>` fallback. Decide by tool kind + content type.
- **Acceptance:** skill body renders formatted (headings/lists/code), not raw; code files
  still render as code.
- **Verify:** unit test (skill payload contains rendered HTML tags, not escaped MD); browser.

## R2-P4 — Regressions + bug

### [x] T23 — User-attached images horizontal (R8)
- **Files:** `src/render/tools/mod.rs` (`render_image` + the user-attached path).
- **Do:** Root-cause why **user-attached** images diverge from the correct horizontal-thumbnail
  renderer; route them through the same small horizontal strip → full-size modal.
- **Acceptance:** user-attached images sit side-by-side small; click → full-size modal.
- **Verify:** agent-browser on a fixture with user-attached images.

### [x] T24 — Skill body → modal (R9) [uses T21]
- **Files:** `src/render/tools/mod.rs` (`render_skill_event`).
- **Do:** Root-cause the inline skill-body regression; ensure the row shows only `<name> skill`
  and the body opens in the modal on click (markdown via T21). No inline dump.
- **Acceptance:** no inline skill content; name click → modal with formatted body.
- **Verify:** unit test (row has no body, modal template present); agent-browser click.

### [x] T25 — Filter-chip disappearing-tool bug (R10)
- **Files:** `assets/transcript.js` (filter logic).
- **Do:** Reproduce on session `08022288-1289-4f52-bdb6-7f9f0902f2a5` (tool
  `mcp__…ctx_batch_execute` vanishes when all chips selected). Root-cause: unmatched tool types
  only survive the show-all-when-none branch. Fix so unmatched tools stay visible under
  all-selected (treat unmatched as always-shown, or make all-selected ≡ none-selected).
- **Acceptance:** the mcp tool row is visible with no chips AND with all chips selected.
- **Verify:** agent-browser on the named session — toggle none → all, row stays.

## R2-P5 — Verification

### [x] T26 — Round 2 full verification
- **Do:** Regenerate both fixtures; in **agent-browser** walk R1–R10. Run `cargo fmt --all`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **Acceptance:** all Round 2 items confirmed; CI clean.

#### Round 2 checklist
- [x] R1 IN/OUT clamp; overflow → modal; short not clickable
- [x] R2 left connector line aligned across row types
- [x] R3 meta messages (caveat / `/model` / `Set model`) hidden; real prompts kept
- [x] R4 basename only + custom hover tooltip with full path; left-aligned
- [x] R5 skill/markdown modal bodies formatted (comrak); code files still code
- [x] R6 user block padded + contrast in both themes
- [x] R7 layout wider + centered, no left gutter
- [x] R8 user-attached images horizontal → modal
- [x] R9 skill body in modal, not inline
- [x] R10 filter chips: mcp tool visible under none AND all selected
