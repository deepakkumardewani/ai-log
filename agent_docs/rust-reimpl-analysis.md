# Claude Code Log — Rust Reimplementation Analysis

**Goal**: Build a Rust tool that generates static HTML from Claude Code JSONL transcripts,
targeting the new design (screenshot: dark theme, 4-panel sidebar, typed message cards, filter
buttons, cost display).

---

## 1. Pipeline: JSONL → HTML

```
~/.claude/projects/<project>/<session>.jsonl
          │
          ▼  cli.py → converter.py:convert_jsonl_to()
          │
          ▼  parser.py  (line-by-line JSONL read)
  list[raw dicts]
          │
          ▼  factories/transcript_factory.py:create_transcript_entry()
  list[TranscriptEntry]   ← Pydantic union of UserEntry | AssistantEntry | SummaryEntry | SystemEntry | QueueOpEntry
          │
          ▼  renderer.py:generate_template_messages()
  Tree[TemplateMessage]   ← DAG threading via parentUuid, fork/branch metadata attached
          │
          ▼  html/renderer.py:HtmlRenderer.generate()
          │    ├── html/user_formatters.py
          │    ├── html/assistant_formatters.py
          │    ├── html/system_formatters.py
          │    └── html/tool_formatters.py
          │
          ▼  Jinja2 → templates/transcript.html  (one per session)
          │           templates/index.html        (master project index)
  final .html files
```

Key points for Rust:
- Sessions within a project are linked via a shared index page.
- Messages form a **DAG** (not a flat list) — `parentUuid` enables branching/forking.
- `isSidechain: true` marks messages that are on a sidechain (sub-agent tasks).
- Session metadata (title, timestamps, tokens) is pre-computed and cached in SQLite.

---

## 2. Raw JSONL Schema

Every line in the JSONL is one of these `type` values:

| type | Purpose |
|---|---|
| `user` | Human turn |
| `assistant` | Claude turn |
| `summary` | User-written session summary |
| `system` | Metadata / context entries |
| `queue-operation` | Async task queue events |
| `ai-title` | Claude-generated short session title |
| `away-summary` | Summary generated while away |
| `hook-attachment` | Hook lifecycle events |

### Top-level fields (all entries)

| Field | Type | Notes |
|---|---|---|
| `type` | string | Entry type (see above) |
| `timestamp` | string | ISO 8601 |
| `sessionId` | string | Groups messages into a session |
| `uuid` | string | Unique entry ID |
| `parentUuid` | string? | DAG parent for threading |
| `cwd` | string | Working directory at message time |
| `version` | string | Protocol version |
| `isSidechain` | bool | Sub-agent / sidechain flag |
| `agentId` | string? | Agent identifier (Teammates feature) |
| `requestId` | string? | Assistant entries only |
| `userType` | string | "human" / "assistant" |
| `gitBranch` | string? | Git branch at time of message |
| `teamName` | string? | Teammates team name |
| `message` | object | Claude message payload (see below) |

### `message` object (user/assistant)

| Field | Type | Notes |
|---|---|---|
| `role` | string | "user" / "assistant" |
| `content` | array | List of content items (see below) |
| `model` | string? | Model ID (assistant only, e.g. `claude-sonnet-4-6`) |
| `stop_reason` | string? | "end_turn", "tool_use", etc. |
| `usage` | UsageInfo? | Token counts (assistant only) |

### UsageInfo

| Field | Type |
|---|---|
| `input_tokens` | int? |
| `output_tokens` | int? |
| `cache_creation_input_tokens` | int? |
| `cache_read_input_tokens` | int? |
| `service_tier` | string? |

### Content item types

| type | Key fields |
|---|---|
| `text` | `text: string` |
| `thinking` | `thinking: string`, `signature: string?` |
| `tool_use` | `id: string`, `name: string`, `input: object` |
| `tool_result` | `tool_use_id: string`, `content: string|array`, `is_error: bool?` |
| `image` | `source: {type, media_type, data}` |

---

## 3. Tool Types (28 parsed)

These are the values of `tool_use.name`:

**File operations**: `Bash`, `Read`, `Write`, `Edit`, `MultiEdit`, `Glob`, `Grep`

**AI / agents**: `Task`, `Agent` (alias), `SendMessage`, `TaskCreate`, `TaskUpdate`,
`TaskList`, `TaskOutput`, `TaskStop`, `TeamCreate`, `TeamDelete`

**Planning / UI**: `TodoWrite`, `AskUserQuestion`, `ask_user_question`, `ExitPlanMode`, `Skill`

**Web**: `WebSearch`, `WebFetch`

**Scheduling**: `Monitor`, `ScheduleWakeup`, `CronCreate`, `CronList`, `CronDelete`

### Tool input shapes (Rust structs needed)

| Tool | Key input fields |
|---|---|
| `Bash` | `command`, `description?`, `timeout?`, `run_in_background?` |
| `Read` | `file_path`, `offset?`, `limit?` |
| `Write` | `file_path`, `content` |
| `Edit` | `file_path`, `old_string`, `new_string`, `replace_all?` |
| `MultiEdit` | `file_path`, `edits: [{old_string, new_string}]` |
| `Glob` | `pattern`, `path?` |
| `Grep` | `pattern`, `path?`, `include?` |
| `Task` / `Agent` | `description`, `prompt`, `subagent_type?` |
| `TodoWrite` | `todos: [{content, status, priority, id}]` |
| `WebSearch` | `query`, `allowed_domains?`, `blocked_domains?` |
| `WebFetch` | `url`, `prompt` |
| `ScheduleWakeup` | `delaySeconds`, `reason`, `prompt` |
| `CronCreate` | `schedule`, `prompt`, `label?` |

---

## 4. Variables Available for HTML Templates

### Session-level (transcript page)

| Variable | Source | Notes |
|---|---|---|
| `session_id` | `sessionId` field | Unique session identifier |
| `ai_title` | `ai-title` entry `.title` | Claude-generated title (e.g. "Implement Connect4 Rules Engine") |
| `summary` | `summary` entry `.summary` | User-written summary |
| `first_timestamp` | first entry `.timestamp` | ISO 8601 |
| `last_timestamp` | last entry `.timestamp` | ISO 8601 |
| `message_count` | computed | Total messages |
| `duration_minutes` | `last - first` timestamps | Session length |
| `cwd` | entry `.cwd` | Working directory |
| `git_branch` | entry `.gitBranch` | Git branch |
| `team_name` | entry `.teamName` | Teammates team |
| `total_input_tokens` | sum of all `usage.input_tokens` | |
| `total_output_tokens` | sum of all `usage.output_tokens` | |
| `total_cache_creation_tokens` | sum of `usage.cache_creation_input_tokens` | |
| `total_cache_read_tokens` | sum of `usage.cache_read_input_tokens` | |
| `model` | `message.model` on assistant entries | E.g. "claude-sonnet-4-6" |

### Per-message variables

| Variable | Source | Notes |
|---|---|---|
| `uuid` | entry `.uuid` | Message ID |
| `parent_uuid` | entry `.parentUuid` | For DAG threading |
| `timestamp` | entry `.timestamp` | |
| `role` | `message.role` | "user" / "assistant" |
| `is_sidechain` | entry `.isSidechain` | Sub-agent flag |
| `agent_id` | entry `.agentId` | |
| `input_tokens` | `message.usage.input_tokens` | Per-message tokens |
| `output_tokens` | `message.usage.output_tokens` | |
| `cache_creation_tokens` | `message.usage.cache_creation_input_tokens` | |
| `cache_read_tokens` | `message.usage.cache_read_input_tokens` | |
| `stop_reason` | `message.stop_reason` | "end_turn" / "tool_use" |
| `content_items` | `message.content` | List of text/thinking/tool_use/tool_result/image |
| `pair_duration` | computed | Time between tool_use and tool_result |

### Project/index-level

| Variable | Source |
|---|---|
| `project_name` | directory name under `~/.claude/projects/` |
| `sessions` | list of SessionCacheData per project |
| `total_projects` | count |
| `total_messages` | aggregate |
| `total_tokens` | aggregate |
| `earliest_timestamp` | across all sessions |
| `latest_timestamp` | across all sessions |

---

## 5. Gap Analysis: Screenshot Design vs. Available Data

### Top header bar

| UI element | Available? | Source |
|---|---|---|
| Session name ("Implement Connect4 Rules Engine") | ✅ | `ai_title` from `ai-title` entry |
| Date ("Oct 24") | ✅ | `first_timestamp` |
| Duration ("12m") | ✅ | computed from `first/last_timestamp` |
| Message count ("24 Msgs") | ✅ | `message_count` |
| Token count ("15.4k Tokens") | ✅ | sum of `usage.*_tokens` |
| Dark/light theme toggle | ✅ | CSS + JS toggle, no JSONL data needed |
| Settings / menu icons | ✅ | static UI chrome |

### Left sidebar

| Panel | Available? | Notes |
|---|---|---|
| **Session History** (same project) | ✅ | List of session files in same project dir; use `ai_title`, `first_timestamp`, `message_count` per session |
| **Table of Contents** | ✅ | Generated from message UUIDs + timestamps; anchor links |
| **File Explorer** | ⚠️ Partial | Must be derived by scanning `Read/Write/Edit/Glob` tool inputs for `file_path`; Python tool does NOT pre-compute this tree |
| **Tool Usage** | ⚠️ Partial | Count per tool type is computable from `tool_use.name`; Python tool does not expose this as a pre-built summary |

**Action needed for File Explorer**: Collect all unique `file_path` values from `Read`, `Write`, `Edit`, `MultiEdit`, `Glob` tool_use entries, build a virtual file tree.

**Action needed for Tool Usage panel**: Group and count all `tool_use.name` occurrences per session.

### Filter buttons

| Button | Available? | CSS class / JSONL mapping |
|---|---|---|
| User | ✅ | entries where `message.role = "user"` |
| Assistant | ✅ | entries where `message.role = "assistant"` and content is `text` |
| Bash | ✅ | `tool_use.name = "Bash"` |
| Read | ✅ | `tool_use.name = "Read"` |
| Write | ✅ | `tool_use.name = "Write"` |
| Edit | ✅ | `tool_use.name = "Edit"` or `"MultiEdit"` |
| Thinking | ✅ | content items where `type = "thinking"` |

### Message cards

| Card element | Available? | Source |
|---|---|---|
| User text | ✅ | `content[].type = "text"` on user role |
| Assistant text (markdown) | ✅ | `content[].type = "text"` on assistant role |
| Thinking block (collapsible) | ✅ | `content[].type = "thinking"` |
| Bash IN/OUT | ✅ | `tool_use.input.command` / `tool_result.content` |
| Read file path + content | ✅ | `tool_use.input.file_path` / `tool_result.content` |
| Edit file path + side-by-side diff | ✅ | `tool_use.input.{old_string, new_string, file_path}` — diff must be rendered client-side or server-side |
| Write file path + content | ✅ | `tool_use.input.{file_path, content}` |
| Token display (right-aligned) | ✅ | `message.usage` |
| Pair duration ("12ms") | ✅ | computed from message timestamps |
| Error state (red border) | ✅ | `tool_result.is_error = true` |
| Image content | ✅ | `content[].type = "image"` (base64 embed) |

### Bottom status bar

| Element | Available? | Source |
|---|---|---|
| "Session Active" / timestamp | ⚠️ Partial | No explicit "active" flag in JSONL; infer from recency of `last_timestamp` |
| "Total Tokens: 15.4k" | ✅ | sum of all `usage.*_tokens` |
| "Cost Est: $0.12" | ✅ per-user request | JSONL does not contain cost; **user wants JSONL metadata only** — skip or omit |
| Version ("v1.2.4-active") | ✅ | library version string |

> **Note on cost**: The current Python tool does NOT calculate or store cost. No cost field exists
> in JSONL. The bottom bar cost in the screenshot must come from an external source or be dropped
> for the static export. Per your answer: use JSONL metadata only → **omit cost line**.

---

## 6. JSONL Fields NOT Rendered by Python Tool (Potential New Data)

These fields exist in JSONL but are not currently surfaced in the HTML:

| Field | Location | Potential use |
|---|---|---|
| `gitBranch` | every entry | Show git context per message or in header |
| `model` | assistant `message.model` | Show which Claude model was used |
| `stop_reason` | assistant `message.stop_reason` | Show why Claude stopped (tool_use vs end_turn) |
| `service_tier` | `usage.service_tier` | Show if cache was hit |
| `cwd` | every entry | Show working directory in File Explorer header |
| `agentId` | entries | Show which agent produced a message (sub-agent chains) |
| `teamName` | entries | Show team name for Teammates sessions |
| `requestId` | assistant entries | Correlate to API requests |
| `version` | every entry | Protocol version drift detection |
| `hook-attachment` entries | root level | Show hook events (pre/post hooks, lifecycle) |
| `queue-operation` entries | root level | Show async task queue operations |
| `away-summary` entries | root level | Show summaries generated during away mode |
| Tool `description` field | `BashInput.description` | Bash tool description shown in card header |
| Tool `run_in_background` | `BashInput` | Flag backgrounded commands visually |
| `TodoWrite` todos with status/priority | tool input | Render todo list with checkboxes |
| `AskUserQuestion` options | tool input | Render question with option buttons |
| `ScheduleWakeup` reason/delay | tool input | Render schedule card |

---

## 7. Design Decisions for Rust Tool

### Diff rendering (Edit cards)
The JSONL contains `old_string` and `new_string` raw. The Python tool renders a simple side-by-side colored diff. For Rust:
- Use the `similar` crate for diff computation at export time
- Render as an HTML table with red/green line highlights
- No JS diff library needed — static HTML

### Write cards
Show full file content with syntax highlighting. Use `syntect` crate or embed a lightweight
Highlight.js in the HTML template.

### File Explorer tree
Build at parse time by collecting all `file_path` values from file tool inputs.
Sort and de-duplicate, then build a virtual directory tree structure.

### Session History panel
At build time (when generating a session HTML), scan sibling JSONL files in the same project
directory and include their `session_id`, `ai_title`, `first_timestamp`, and `message_count`
as a JSON blob in the HTML for the sidebar to render.

### "Session Active" heuristic
If `last_timestamp` is within the last 10 minutes of the export time → show "Session Active".
Otherwise show the last timestamp formatted as a relative time.

---

## 8. Files to Study for Rust Port

| File | Purpose |
|---|---|
| `claude_code_log/models.py` | Authoritative Pydantic schema → Rust serde structs |
| `claude_code_log/factories/transcript_factory.py` | Entry type dispatch logic |
| `claude_code_log/factories/tool_factory.py` | Tool input/output parsing |
| `claude_code_log/renderer.py` | DAG threading, session boundary detection |
| `claude_code_log/html/renderer.py` | Template context assembly |
| `claude_code_log/html/tool_formatters.py` | Per-tool HTML formatting |
| `claude_code_log/templates/transcript.html` | Full HTML structure reference |
| `claude_code_log/templates/index.html` | Index page structure reference |
| `claude_code_log/cache.py` | Session/project metadata aggregation |
| `test/test_data/` | Representative JSONL samples for testing |
