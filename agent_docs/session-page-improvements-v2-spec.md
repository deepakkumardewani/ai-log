# Spec: Session Page Improvements (v2)

Status: Approved (intent confirmed via interview, 2026-06-02)
Owner: Deepak
Related: builds on `ui-restructure-spec-v0.1.md` and `ui-improvements-tasks-v0.1.md`. Scope is the **per-session transcript view**, which was explicitly out of scope in v0.1.

---

## Objective

Restructure the per-session transcript page from a flat stream of equal-weight cards into a **conversational view** where each assistant turn is a single grouped unit with collapsible thinking + tool sections, and clean up per-card metadata noise so the dialogue reads first and the mechanics read second.

### User

A Claude Code user (Deepak first; other cclog users second) reading their own session transcripts in a browser. Primary need is to **skim the dialogue first**, then drill into how the assistant arrived at its responses only when curious. Static HTML, no runtime/server.

### Why now

The current per-session page:
- Renders every thought, every tool call, and every assistant message as a peer card in a flat stream — breaks the conversational rhythm.
- Repeats date, in/out tokens, and timestamps on every card — visual noise.
- Renders `Edit` / `Write` tool calls as `old_string` + `new_string` literal blocks instead of as diffs — defeats the point of viewing the change.
- Shows empty thinking ("thinking content not stored") as a full card with chrome.
- Stuffs session-inactive + total-token info into a footer where it's redundant with the header.
- Has no aggregate counts (users / assistants / tool calls) at the session level — you can't tell the shape of the session at a glance.

### Success criteria

1. **Skill cards** — header shows the full skill name inline (e.g., `agent-skills:interview-me`), not a generic "Skill" label.
2. **Empty thinking** — rendered as a disabled `Thinking` pill, no card chrome, no timestamp, no placeholder text. Same visual treatment for "thinking happened but wasn't stored" and "no thinking ran" (single state covers both).
3. **Per-card metadata** — only **time** shown on each card. Date and in/out tokens removed from every card. The thinking step has no separate timestamp (the parent assistant card carries it).
4. **Edit / Write diffs** — rendered as a **unified diff** (red `-` lines above green `+` lines), built with **plain HTML/CSS** (no diff library). Line-level only for v2; character-level highlighting deferred.
5. **Change summary** — `Added X lines, removed Y lines` shown as a GitHub-style header above each Edit/Write diff.
6. **Footer cleanup** — remove the "session inactive" and "total tokens" lines from the page footer entirely.
7. **Conversational grouping (the headline change):**
   - The unit of collapse is **one assistant turn** = one assistant message + its thinking + its tool calls, up to the next user message.
   - Thinking + tools are **hidden by default**. Two separate toggles per turn: `▸ Thinking · {in} in · {out} out` and `▸ Tools · N calls`.
   - **Sub-assistants** (spawned via Task/Agent) nest **inside** the parent assistant's Tools toggle, each with its own collapse toggle.
   - **User vs assistant visual differentiation**: both **left-aligned**. No emoji.
     - User turn = soft **bubble** with small rounded corners.
     - Assistant turn = flat structured card.
     - Elegant, not fancy.
   - Session header strip gains aggregate counts (users / assistants / tool calls) alongside the existing `ID · time · date · duration · msgs · tokens` line.

---

## Tech Stack

Unchanged from v0.1:

| Layer | Choice |
|---|---|
| Renderer | Rust (`src/render/`) producing static HTML |
| Templating | Existing template approach (`src/render/session.rs` etc. — exact module per current layout) |
| Styling | Tailwind via `assets/tailwind.config.js` + `assets/tailwind.input.css`. Theme tokens already established in v0.1. |
| Interactivity | Vanilla JS in `assets/` (no framework). Toggles use small `<details>` or click handlers on `data-*` attributes — final mechanism chosen in plan phase. |
| Diff rendering | Plain HTML/CSS via line-level diffing in Rust (e.g., `similar` crate, already a candidate — confirmed in plan phase). No JS diff library, no external CSS framework. |
| Tests | `cargo test` (render smoke tests + asset self-containment) |

---

## Commands

```bash
# Development
cargo build                                       # Debug build
cargo build --release                             # Release build
cargo run -- --input <dir> --output-dir tests/cclog-out --clear-cache   # Regenerate fixtures

# Code quality
cargo fmt --all                                   # Format
cargo clippy --all-targets -- -D warnings         # Lint
cargo test                                        # All tests

# CI gate (pre-merge)
just ci                                           # fmt → clippy → test
```

---

## Project Structure

```
src/
  render/
    session.rs           # Per-session page renderer — primary surface for v2
    index.rs             # Project list — untouched
    project.rs           # Per-project list — untouched
  conversation/          # (likely existing; or new) — grouping logic that turns flat
                         # messages into assistant-turn units
  diff.rs                # (new) — line-level diff generation for Edit/Write tool calls
assets/
  session.js             # (new or extended) — toggle handlers for Thinking / Tools / Sub-agent
  tailwind.input.css     # Theme tokens (extend with diff colors + bubble styles)
agent_docs/
  session-page-improvements-v2-spec.md   # this file
  session-page-improvements-v2-plan.md   # produced next by /plan
  session-page-improvements-v2-tasks.md  # produced next by /plan
tests/
  cclog-out/             # Regenerated fixture output for visual review
```

Final module names (e.g., `conversation.rs` vs nesting under `render/`) are decided in the plan phase against the actual current tree.

---

## Code Style

Follow `.claude/skills/rust-best-practices/SKILL.md` and project `CLAUDE.md`. Key conventions reinforced for this work:

- **Named structs over long parameter lists**:
  ```rust
  // good
  struct AssistantTurn {
      message: AssistantMessage,
      thinking: Option<ThinkingStep>,
      tool_calls: Vec<ToolCall>,
      sub_agents: Vec<SubAgentTurn>,
  }

  fn render_turn(turn: &AssistantTurn, ctx: &RenderContext) -> String { ... }

  // bad
  fn render_turn(msg: &AssistantMessage, thinking: Option<&ThinkingStep>,
                 tools: &[ToolCall], subs: &[SubAgentTurn], theme: &Theme,
                 show_thinking: bool, show_tools: bool) -> String { ... }
  ```
- **Illegal states unrepresentable**: an `AssistantTurn` always owns its thinking + tools by construction; the renderer doesn't have to handle "tools belong to a different turn."
- **No `unwrap()` in production paths** — `Result` / `Option` flow through; render errors surface with context via `anyhow`.
- **Pure render functions**: input → string. Side-effecting bits (file writes, cache) stay in the caller.
- **Functions under ~30 lines**. Extract diff rendering, toggle markup, and turn assembly into dedicated functions.
- **No magic strings** — toggle data attributes (`data-toggle="thinking"`, `data-toggle="tools"`) defined as constants in one place.

---

## Testing Strategy

| Concern | Test type | Location |
|---|---|---|
| Conversation grouping (flat messages → `AssistantTurn` vec) | Unit test against fixture transcripts | `src/conversation/` (alongside module) or `tests/` |
| Diff line-level output for sample old/new strings | Unit test | alongside `diff.rs` |
| Rendered HTML contains expected structural markers (`data-turn`, `data-toggle="thinking"`, `data-toggle="tools"`, diff classes) | Snapshot / contains-assertion test | `tests/render.rs` (or existing render test module) |
| Asset self-containment (no external CDN URLs) | Existing test from `3b12977` | `tests/` — must continue to pass |
| Regression guards for v2 specifically: empty-thinking renders as disabled pill, footer no longer contains "session inactive" / "total tokens", per-card date is absent, session header strip contains aggregate counts | Contains-assertion tests | `tests/render.rs` |
| `just ci` clean | CI gate | local + CI |

No new e2e or browser-driver tests. Visual verification is manual: regenerate fixtures, open in browser, eyeball.

---

## Out of Scope (explicit)

These were considered and explicitly excluded during interview:

1. **Character-level diff highlighting** within changed lines — deferred to a future iteration.
2. **Side-by-side diff layout** — unified is the chosen presentation; side-by-side rejected for narrow-width nested contexts.
3. **Token totals on the Tools toggle** — call count only.
4. **Distinguishing "thinking happened but wasn't stored" from "no thinking ran"** — single disabled state covers both.
5. **Right-alignment / chat-bubble layout for user turns** — both roles stay left-aligned.
6. **Color-only role differentiation** — shape (bubble vs flat card) carries it.
7. **Sticky session header** — regular header block at top of page, scrolls away with content.
8. **Emojis or icons on role labels** — text only, elegant.
9. **External diff or chat libraries** — plain HTML/CSS only.
10. **Re-design of the index or per-project pages** — those landed in v0.1 and stay as-is.
11. **Sub-agent recursion beyond one level of nesting being structurally distinct** — UI supports it (any sub-agent's tools can themselves contain sub-agents via the same toggle pattern), but no special handling for "deeply nested" cases.

---

## Boundaries

### Always do
- Run `just ci` before declaring a task complete.
- Regenerate `tests/cclog-out/` fixtures after rendering changes so visual review is against current output.
- Keep toggle markup data-attribute-driven (`data-toggle="thinking"`, etc.) so the JS stays decoupled from CSS.
- Extract any inline render block over ~30 lines into its own function.
- Use named structs for any function that would otherwise take >3 parameters.

### Ask first
- Adding any new Rust crate (diff library, etc.) — confirm before adding to `Cargo.toml`.
- Changing the toggle interaction mechanism away from `<details>` / data-attributes (e.g., introducing a JS framework).
- Touching the index or per-project page renderers (out of scope here; if a shared component needs changes, surface the ripple).
- Any change to how `AssistantTurn` aggregation is computed if it could affect existing tests for flat-message rendering.

### Never do
- Introduce an external CDN dependency (would break the self-containment test from `3b12977`).
- Skip `cargo fmt`, `cargo clippy`, or `cargo test` before commit.
- Use `unwrap()` / `expect()` in render paths without a documented `// SAFETY:`-style justification.
- Add emojis or icons to role labels in rendered output.
- Re-introduce per-card date or in/out-token metadata anywhere in the per-turn cards.
- Commit `tests/cclog-out/` regenerated bytes as if they were source — they're build output, follow existing convention.

---

## Assumptions

Listed so they can be challenged before the plan lands:

1. The current renderer already has access to ordered `Message` items per session (user / assistant / tool_use / tool_result / thinking). Grouping into `AssistantTurn` is a pure transformation over that stream.
2. Sub-agent invocations are detectable from the existing message stream (e.g., `Task` tool calls whose results contain a nested transcript), or already modeled separately. The plan phase will confirm and, if needed, add detection.
3. The `similar` crate (or equivalent already in the workspace) is acceptable for line-level diffing. If not, a hand-rolled longest-common-subsequence over lines is fine — both are within scope.
4. Existing Tailwind theme tokens (warm neutrals, terracotta accent) from v0.1 are the palette base. Diff red/green pick warm-leaning hues that harmonize with that palette, not generic GitHub green/red.
5. Toggle state is **session-page-local** and resets on reload (no `localStorage` persistence). Keeps JS surface small.
6. "Time only" on per-card headers means `HH:MM:SS` in the session's timezone (same format already shown). No relative time ("2m ago").

→ Flag any assumption now or it stands during planning.
