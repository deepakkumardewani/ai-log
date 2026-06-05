# Spec: Session Page Improvements (v3) — Flat Dot-Timeline

> **Supersedes v2.** The v2 spec made *assistant-turn grouping* the headline feature
> (turns that collapse thinking + tools behind toggles, user-bubble vs assistant-card).
> In practice that felt cluttered and confusing. v3 **removes grouping** and replaces it
> with a flat, compact, chronological **dot-timeline** modeled on the Claude Code VSCode
> extension transcript. Confirmed via interview on 2026-06-03.

---

> **Round 2 (2026-06-03).** v3 shipped (commit `57ebbaa`, T1–T11). After viewing it in
> the browser, a refinement + bugfix round was confirmed via interview. The flat
> dot-timeline stays; Round 2 polishes IN/OUT overflow, the left connector line, meta-message
> filtering, file-name tooltips, modal markdown, user-block styling, layout width, and fixes
> two regressions (vertical images, inline skill body) plus a filter-chip bug. **See
> [Round 2 — Refinements](#round-2--refinements) below; tasks T15+ in the tasks file.**

## Objective

Rewrite the per-session page as a **flat vertical timeline**: every assistant-side event
(assistant text, thinking, skill, each tool call, each sub-agent) is its own top-level row,
prefixed by a small colored dot. Heavy content (thinking, skill bodies, file reads, images)
collapses behind the row and reveals on click — inline for thinking, **modal** for skills /
reads / images. Edit/Write render as inline unified-diff cards with highlighted changes.

### User

A Claude Code user (Deepak first; other cclog users second) reading their own session
transcripts in a browser. Primary need: **skim the dialogue fast**, drill into *how* the
assistant got there only on demand. Static HTML, no runtime/server.

### Why now

The v2 grouping (nested turns, `▸ Thinking` / `▸ Tools` toggles, large cards, user-bubble
vs assistant-card) is too busy. A dot-timeline is denser, calmer, and easier to scan — it
reads like the VSCode extension transcript the user already likes.

### Success criteria

1. **Flat timeline** — no turn-grouping, no nesting. Assistant text, thinking, skill, each
   tool call, and each sub-agent are sibling rows at one indentation level, in chronological
   order.
2. **User messages** — rendered as a distinct **muted block** (soft bubble / faded prompt
   look), *not* a dot-row, so the human's turns stand out while scrolling.
3. **Dots** — gray dot for assistant text / thinking / skill; green dot for tool calls and
   sub-agents.
4. **Thinking** — collapsed `Thinking ›` row; expands **inline** on click. Empty/unstored
   thinking renders as a disabled `Thinking` pill (no body, no chrome).
5. **Tool rows (unified format)** — one shared pattern: `● <Tool> <primary arg>`. **a section
   is shown only when that data exists** in the log (no empty IN/OUT). Dot is green.
   *(Round 2 R1: IN/OUT now render visible-by-default in a clamped box; overflow → modal.)*
6. **Skill** — row shows `<full-skill-name> skill` (e.g. `agent-skills:interview-me skill`).
   Clicking the name opens a **modal** with the full skill body. *(Round 2 R9 fixes a
   regression where the body rendered inline; R5 renders the body as markdown.)*
7. **Read** — row shows file name + line range. Clicking the filename opens the **file
   contents (the tool OUT) in a modal**. *(Round 2 R4: show basename only + custom hover
   tooltip for the full path.)*
8. **Edit / Write / MultiEdit** — inline **unified** diff card (red `−` lines above green `+`
   lines). Line-level red/green background **plus** brighter **word/token-level** inline
   highlight on the changed span. **Not side-by-side.** GitHub-style `Added X · Removed Y`
   summary header above each diff. File-path header preserved.
9. **Sub-agent** (Task/Agent) — green dot-row `Agent: <description>`, with its **IN prompt**
   expandable. **No nested internal transcript** (the sub-agent's own messages/tools are not
   rendered inline).
10. **Images** — small **horizontally-stacked thumbnails**. Clicking a thumbnail opens it
    **full-size in a modal** with a close button. *(Round 2 R8 fixes a regression where
    user-attached images still stacked vertically at full size.)*
11. **No date/time** on any row or card. (Session-level metadata strip may keep its time.)
12. **Combined transcripts page** — sessions sorted **newest-first** by timestamp.

---

## Round 2 — Refinements

Polish + bugfixes on the shipped v3 (commit `57ebbaa`). Each item is a vertical slice that
ends in a browser verification via the **agent-browser** CLI on the two named fixtures.

### R1 — IN/OUT clamp + click-to-modal on overflow
Tool IN/OUT already render visible-by-default and clip when long, but the full text is
unreachable. Keep the fixed **max-height** box. When content overflows: fade the bottom edge
and make the IN/OUT block **clickable to open the full text in the shared modal**. Short
content stays fully visible and is **not** clickable (no pointless modal). Overflow detection
is presentational (CSS max-height); the modal payload is the full IN or OUT string.

### R2 — Left connector line
Draw a **vertical line down the left rail** connecting consecutive dots, so the timeline reads
as one continuous thread (matches the reference rendering). Pure CSS on the rail; must align
with dot centers across all row types (assistant, thinking, tool, skill, sub-agent) and not
break the user muted-block (which has no dot).

### R3 — Filter Claude-Code meta messages
Do **not** render Claude-Code meta/system noise as dialogue. Filter both ways:
- **Flag-driven** where present: messages marked `isMeta` / `isSidechain` in the JSONL.
- **Pattern-driven** fallback for the known text: the `Caveat: The messages below were
  generated by the user while running local commands…` block, slash-command echoes
  (`/model …`, `command-name`/`command-args`), local-command stdout, and `Set model to …`.
Real user prompts, assistant text, thinking, and tool calls are kept. Filtering happens at the
**timeline-event build** stage (so search/filter counts are also correct), not just hidden in CSS.

### R4 — File-name basename + custom hover tooltip
For **Read / Write / Edit / MultiEdit** rows, show only the **basename** after the tool name
(`Read — html.rs`), never the full absolute path inline, and fix the center-alignment so the
label is left-aligned next to the dot. The **full path** appears in a **custom tooltip
component on hover** (not the native `title` attribute). Tooltip is a small reusable
JS/CSS primitive (positioned, escaped, dismiss on mouse-leave).

### R5 — Markdown in modal bodies (reuse existing comrak)
Skill bodies and other markdown payloads in the shared modal currently render as raw
`<pre>` escaped text. Route them through the **existing `src/render/markdown.rs::render()`**
(comrak — already a dependency; powers assistant-text rendering). **No new crate.** Nuance:
**code-file** reads stay rendered as code (syntect, already present) — only markdown / skill
`.md` bodies go through comrak; non-markdown stdout keeps the `<pre>` fallback.

### R6 — User message block padding + contrast
The user muted-block currently hugs the left edge and its background is barely visible. Add
proper **padding** (inset from the rail/edge) and a background with **sufficient contrast in
both light and dark themes** while staying "muted" relative to assistant rows.

### R7 — Wider centered layout
The timeline sits in a narrow column pushed far right with a large empty left gutter. Remove
the offset and **widen the reading column to ~960–1100px, centered** with balanced gutters on
both sides.

### R8 — User-attached images horizontal (regression)
v3 criterion 10 specified horizontal thumbnails, but **user-attached** images still stack
vertically at full size. Make user-attached images render as the same **small horizontal
thumbnail strip → full-size modal** as other images. Root-cause why the user-attached path
diverges from the already-correct image renderer.

### R9 — Skill body gated behind click (regression)
v3 criterion 6 specified a modal, but the skill body currently renders **inline** (full
content dumped under the row). Ensure the row shows only `<name> skill` and the body opens in
the **modal on click** (rendered via R5 markdown). Root-cause the inline regression.

### R10 — Filter-chip disappearing-tool bug
On session `08022288-1289-4f52-bdb6-7f9f0902f2a5`, a tool whose type matches **no filter
chip** (e.g. `mcp__plugin_context-mode_context-mode__ctx_batch_execute`) is **visible when no
chips are selected but vanishes when all chips are selected**. Root-cause the filter logic
(the "show-all when none selected" branch is the only one that includes unmatched tools) and
fix so selecting all chips never hides content that no chip represents — e.g. treat unmatched
tool types as always-shown, or make "all selected" equivalent to "none selected".

---

## Commands

Pure Rust project (see `CLAUDE.md`). No `package.json`/`bun`.

```bash
cargo build                                  # debug build
cargo fmt --all                              # format
cargo clippy --all-targets -- -D warnings    # lint
cargo test                                   # run tests
just ci                                       # fmt → clippy → test
```

**Verification (manual, in browser):** regenerate HTML for the two fixtures and open via
agent-browser:

```bash
cargo run -- <args> tests/fixtures/08022288-1289-4f52-bdb6-7f9f0902f2a5.jsonl
cargo run -- <args> tests/fixtures/b4d9d192-fecb-4675-8076-634a63192f60.jsonl
```

---

## Project Structure (current tree, confirmed)

```
src/
  conversation.rs        # parse_and_group → TurnGroup { User | Assistant(AssistantTurn) }.
                         #   v3: simplify/replace grouping with a flat ordered event stream.
  session.rs             # MessageNode ordering used by conversation.rs.
  project.rs:114         # sessions.sort_by(id) → change to newest-first by timestamp.
  model/
    content.rs           # ContentItem (Text|Thinking|ToolUse|ToolResult|Image), ImageSource.
  render/
    turn.rs              # render_turn_group → v3: render flat timeline rows.
    tools/mod.rs         # wrap_card + per-tool renderers + render_image, render_thinking.
    diff.rs              # line-level diff → v3: add word/token-level intra-line highlight.
    html.rs              # page assembly (head, body, scripts).
    project.rs           # combined_transcripts.html (per-project list).
    index.rs             # project list — untouched except via shared sort if applicable.
templates/
  transcript.html        # page shell.
  components/*.html       # thinking/tool_card/user_message/diff/header — restyled for rows.
assets/
  transcript.js          # toggle handlers → v3: add shared modal (skill/read/image).
  tailwind.input.css     # tokens → v3: add dot styles, row styles, diff word-highlight,
                         #   thumbnail strip, modal overlay.
agent_docs/
  session-page-improvements-v3-spec.md   # this file
  session-page-improvements-v3-plan.md   # produced by /plan
  session-page-improvements-v3-tasks.md  # produced by /plan
```

Final module decomposition (e.g. whether to keep `AssistantTurn` or flatten to an event
enum) is decided in the plan phase against the actual code.

---

## Code Style

Follow `.claude/skills/rust-best-practices/SKILL.md` and project `CLAUDE.md`:

- **Named structs over long parameter lists**; prefer an `enum` of timeline events
  (`TimelineEvent::{AssistantText, Thinking, ToolCall, SubAgent, ...}`) to make illegal
  states unrepresentable rather than passing many bool flags.
- **No `unwrap()`** in production paths; `Result`/`Option` flow through, `anyhow` for context.
- **Pure render functions**: input → `String`; side effects (file writes) stay in callers.
- **Functions under ~30 lines**; extract diff rendering, modal markup, row assembly.
- **No magic strings** — modal/toggle data attributes (`data-modal`, `data-toggle`) defined
  as constants in one place; dot color classes named (`dot--assistant`, `dot--tool`).

---

## Testing Strategy

Follow `.claude/skills/rust-testing/SKILL.md`.

- **Unit tests** (Rust): row rendering per event type (assistant text, thinking pill vs
  filled, skill row name, tool IN/OUT presence logic, sub-agent IN-only, image thumbnail
  markup); diff generation (line counts + word-token highlight spans); newest-first sort.
- **No-empty-section assertion**: tool row with no OUT must not emit an OUT block, and vice
  versa.
- **Regression**: existing flat-message/diff tests updated to the new markup; v2 grouping
  tests for `AssistantTurn` nesting removed or rewritten.
- **Manual visual verification** in browser via agent-browser using the two named fixtures:
  thumbnails horizontal + click-to-modal, skill modal, read modal, inline diff highlight,
  newest-first combined page.

---

## Boundaries

### Always do
- Keep output **static, self-contained HTML/CSS + minimal vanilla JS**.
- One shared modal component reused for skill / read / image (DRY).
- Run `cargo fmt`, `clippy -D warnings`, `cargo test` before declaring done.
- Verify in a browser with both named fixtures before sign-off.

### Ask first
- Adding any new Rust crate.
- Introducing a JS framework or build step beyond the existing vanilla JS.
- Touching index/project renderers beyond the newest-first sort change.
- Changing the JSONL parsing/data model in a way that affects existing parse tests.

### Never do
- No turn-grouping / nesting of assistant, sub-agent, or tools (the thing being removed).
- No side-by-side diffs (v3 is unified only).
- No `localStorage` persistence of toggle/modal state (resets on reload).
- No date/time on rows/cards.
- No emoji as role markers.

---

## Out of Scope

- Assistant-turn grouping (being removed) and its `▸ Thinking` / `▸ Tools` toggles.
- Side-by-side diff view.
- Sub-agent nested internal transcript rendering.
- Index/project page redesign beyond the newest-first sort.
- localStorage persistence; relative timestamps.

---

## Assumptions (flag before plan stands)

1. `parse_and_group` already yields an **ordered** message/event stream per session;
   flattening to a timeline is a pure transform over it.
2. Tool OUT is recoverable: `tool_result` is paired to `tool_use` by `tool_use_id`
   (confirmed in `model/content.rs`). Read "file contents" = that tool result's content.
3. Sub-agent IN (prompt) is the `Task`/`Agent` tool_use input; its internal transcript is
   intentionally **not** rendered.
4. Images arrive as `ContentItem::Image { source: ImageSource }` (base64 data URL); thumbnails
   are CSS-constrained versions of the same `<img>`, modal shows full size.
5. Session timestamp for newest-first sort is available on the session struct used in
   `project.rs` (else derive from first/last message timestamp).
6. Existing warm-neutral / terracotta theme tokens are the palette base; diff red/green and
   dot colors harmonize with it.
