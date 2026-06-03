//! Tool card and message content renderers.
//!
//! Each renderer produces an HTML string that is embedded in a message card.
//! The card chrome (header, border, dot) is applied by [`wrap_card`].

use crate::model::tool::ToolInput;

// ---------------------------------------------------------------------------
// v3 row primitives — dot class constants
// ---------------------------------------------------------------------------

/// Dot class for assistant text, thinking, and skill rows (gray).
pub const DOT_ASSISTANT: &str = "dot--assistant";

/// Dot class for tool calls and sub-agent rows (green).
pub const DOT_TOOL: &str = "dot--tool";

/// Render a flat v3 timeline row: `● <label> [meta]`.
///
/// This is the shared primitive used by all event row renderers. Use
/// [`DOT_ASSISTANT`] for gray (assistant text / thinking / skill) or
/// [`DOT_TOOL`] for green (tool calls / sub-agents). `meta` is optional
/// secondary annotation (e.g. a file path, arg summary); pass `""` to omit.
pub fn render_row(dot_class: &str, label: &str, meta: &str) -> String {
    let meta_html = if meta.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="row-meta">{}</span>"#, meta)
    };
    format!(
        r#"<div class="timeline-row"><div class="dot {dot_class}"></div><span class="row-label">{label}</span>{meta_html}</div>"#
    )
}

// ---------------------------------------------------------------------------
// Card wrapper
// ---------------------------------------------------------------------------

/// Visible text length (chars) at or above which a card's body becomes
/// expandable with a "Show more" toggle. Measured on stripped text so
/// short content wrapped in heavy markup (e.g. a thinking block) does
/// not falsely trigger the toggle.
const EXPAND_THRESHOLD: usize = 300;

/// Count visible (non-tag) characters in an HTML fragment.
///
/// This gives a far better proxy for "how much will the user actually
/// read" than `body.len()`, which inflates with every wrapping `<div>`
/// or class attribute.
fn visible_text_length(html: &str) -> usize {
    let mut count = 0usize;
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => count += 1,
            _ => {}
        }
    }
    count
}

/// Wrap content in a message card with a colored header.
///
/// Cards are open by default; clicking the header (or its chevron)
/// toggles the body. Long bodies (≥ [`EXPAND_THRESHOLD`] chars) are
/// additionally wrapped in a fade-gradient container with an in-body
/// `Show more / Show less` toggle so the card can be skimmed without
/// fully expanding. `meta` is right-aligned in the header (e.g. a
/// timestamp); pass `""` for inner / nested cards.
pub fn wrap_card(
    role: &str,
    dot_class: &str,
    header_class: &str,
    body: &str,
    is_error: bool,
    meta: &str,
) -> String {
    let error_class = if is_error { " message-card--error" } else { "" };
    let meta_html = if meta.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="message-card-meta">{}</span>"#, meta)
    };

    let body_html = if visible_text_length(body) >= EXPAND_THRESHOLD {
        format!(
            r#"<div class="body-collapsible" data-collapsible>
  <div class="body-collapsible-content">{body}</div>
  <button type="button" class="show-more-btn" data-show-more>Show more</button>
</div>"#
        )
    } else {
        body.to_string()
    };

    format!(
        r#"<div class="message-card{error_class}" data-card-collapse>
  <div class="message-card-header {header_class}" data-card-toggle>
    <div class="message-card-header-left">
      <div class="message-dot {dot_class}"></div>
      <span class="message-card-role">{role}</span>
    </div>
    <div class="message-card-header-right">
      {meta_html}
      <span class="message-card-chevron" aria-hidden="true">&#x25BE;</span>
    </div>
  </div>
  <div class="message-card-body">{body_html}</div>
</div>"#,
    )
}

// ---------------------------------------------------------------------------
// User / assistant text
// ---------------------------------------------------------------------------

/// Render a user message as plain text (escaped HTML).
pub fn render_user_message(msg: &crate::model::content::Message) -> String {
    let text: String = msg
        .content
        .iter()
        .filter_map(|item| {
            if let crate::model::content::ContentItem::Text { text } = item {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    render_user_message_text(&text)
}

/// Render plain text for a user message (escaped, no markdown).
pub fn render_user_message_text(text: &str) -> String {
    let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!("<p>{}</p>", escaped.replace('\n', "<br>"))
}

/// Render a thinking block as a collapsible card.
pub fn render_thinking(thinking: &str) -> String {
    let snippet: String = thinking.chars().take(200).collect();
    let escaped = snippet.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let more = if thinking.len() > 200 { " …" } else { "" };
    format!(
        r#"<details class="thinking-block" open>
  <summary class="thinking-summary">Thinking</summary>
  <div class="thinking-content">{}{}</div>
</details>"#,
        escaped, more
    )
}

// ---------------------------------------------------------------------------
// Tool dispatch
// ---------------------------------------------------------------------------

/// Render a tool_use card (dispatches by name).
pub fn render_tool_use(name: &str, input: &serde_json::Value, _id: &str) -> String {
    let ti = ToolInput::from_name_and_input(name, input.clone());
    match &ti {
        ToolInput::Bash(b) => render_bash(b, false),
        ToolInput::Read(r) => render_read(r),
        ToolInput::Write(w) => render_write(w),
        ToolInput::Edit(e) => render_edit_card(e),
        ToolInput::MultiEdit(me) => render_multiedit(me),
        ToolInput::Glob(g) => render_glob(g),
        ToolInput::Grep(g) => render_grep(g),
        ToolInput::TodoWrite(tw) => render_todo_write(tw),
        ToolInput::AskUserQuestion(aq) => render_ask_user_question(aq),
        ToolInput::WebSearch(ws) => render_web_search(ws),
        ToolInput::WebFetch(wf) => render_web_fetch(wf),
        ToolInput::ScheduleWakeup(sw) => render_schedule_wakeup(sw),
        ToolInput::CronCreate(cc) => render_cron_create(cc),
        ToolInput::CronDelete(cd) => render_cron_delete(cd),
        ToolInput::CronList(_) => render_cron_list(),
        ToolInput::Monitor(m) => render_monitor(m),
        ToolInput::Task(t) => render_task(t),
        ToolInput::Team(t) => render_team(t),
        ToolInput::SendMessage(sm) => render_send_message(sm),
        ToolInput::Skill(s) => render_skill(s),
        ToolInput::ExitPlanMode(ep) => render_exit_plan_mode(ep),
        ToolInput::Generic { name, input } => render_generic(name, input),
    }
}

/// Render a tool_result block.
pub fn render_tool_result(content: &str, is_error: bool) -> String {
    let error_class = if is_error { " tool-result--error" } else { "" };
    let escaped = content.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<div class="tool-result{error_class}">
  <div class="tool-result-label">OUT:</div>
  <div class="tool-result-content">{}</div>
</div>"#,
        escaped.replace('\n', "<br>")
    )
}

/// Render an embedded image as an `<img>` tag with base64 data.
pub fn render_image(source: &crate::model::content::ImageSource) -> String {
    format!(
        r#"<img class="embedded-image" src="data:{};{},{}" alt="Attached image" loading="lazy">"#,
        source.media_type, source.source_type, source.data
    )
}

// ---------------------------------------------------------------------------
// Individual tool renderers
// ---------------------------------------------------------------------------

fn render_bash(b: &crate::model::tool::BashInput, _is_error: bool) -> String {
    let desc = b.description.as_deref().unwrap_or("");
    let bg = if b.run_in_background.unwrap_or(false) {
        r#" <span class="badge badge--bg">background</span>"#
    } else {
        ""
    };
    let title =
        if desc.is_empty() { format!("Bash{}", bg) } else { format!("Bash — {} {}", desc, bg) };
    wrap_card(
        &title,
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">IN:</span><span class="tool-io-value">{}</span></div>"#,
            b.command.replace('&', "&amp;").replace('<', "&lt;")
        ),
        false,
        "",
    )
}

fn render_read(r: &crate::model::tool::ReadInput) -> String {
    let meta = match (r.offset, r.limit) {
        (Some(off), Some(lim)) => format!("lines {}-{}", off, off + lim),
        (Some(off), None) => format!("from line {}", off),
        _ => String::new(),
    };
    let title = format!("Read — {}", r.file_path);
    let body = if meta.is_empty() {
        format!(
            r#"<div class="tool-io"><span class="tool-io-label">FILE:</span><span class="tool-io-value">{}</span></div>"#,
            r.file_path
        )
    } else {
        format!(
            r#"<div class="tool-io"><span class="tool-io-label">FILE:</span><span class="tool-io-value">{}</span></div><div class="tool-io-footer">{}</div>"#,
            r.file_path, meta
        )
    };
    wrap_card(&title, "message-dot--tool", "message-card-header--tool", &body, false, "")
}

fn render_write(w: &crate::model::tool::WriteInput) -> String {
    let diff = crate::render::diff::render_unified_diff("", &w.content);
    let summary = crate::render::diff::render_change_summary(diff.added, diff.removed);
    let body = format!("{}{}", summary, diff.html);
    wrap_card(
        &format!("Write — {}", w.file_path),
        "message-dot--tool",
        "message-card-header--diff",
        &body,
        false,
        "",
    )
}

fn render_edit_card(e: &crate::model::tool::EditInput) -> String {
    let diff = crate::render::diff::render_unified_diff(&e.old_string, &e.new_string);
    let summary = crate::render::diff::render_change_summary(diff.added, diff.removed);
    let body = format!("{}{}", summary, diff.html);
    wrap_card(
        &format!("Edit — {}", e.file_path),
        "message-dot--tool",
        "message-card-header--diff",
        &body,
        false,
        "",
    )
}

fn render_multiedit(me: &crate::model::tool::MultiEditInput) -> String {
    let diffs: Vec<String> = me
        .edits
        .iter()
        .map(|op| {
            let d = crate::render::diff::render_unified_diff(&op.old_string, &op.new_string);
            let s = crate::render::diff::render_change_summary(d.added, d.removed);
            format!(r#"<div class="multiedit-op">{}{}</div>"#, s, d.html)
        })
        .collect();
    wrap_card(
        &format!("MultiEdit — {} ({} edits)", me.file_path, me.edits.len()),
        "message-dot--tool",
        "message-card-header--diff",
        &diffs.join(""),
        false,
        "",
    )
}

fn render_glob(g: &crate::model::tool::GlobInput) -> String {
    let path = g.path.as_deref().unwrap_or(".");
    wrap_card(
        "Glob",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">PATTERN:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">PATH:</span><span class="tool-io-value">{}</span></div>"#,
            g.pattern, path
        ),
        false,
        "",
    )
}

fn render_grep(g: &crate::model::tool::GrepInput) -> String {
    let path = g.path.as_deref().unwrap_or(".");
    let inc = g.include.as_deref().unwrap_or("*");
    wrap_card(
        "Grep",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">PATTERN:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">PATH:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">INCLUDE:</span><span class="tool-io-value">{}</span></div>"#,
            g.pattern, path, inc
        ),
        false,
        "",
    )
}

fn render_todo_write(tw: &crate::model::tool::TodoWriteInput) -> String {
    let items: Vec<String> = tw
        .todos
        .iter()
        .map(|t| {
            let chip = status_chip(&t.status);
            let checked = if t.status == "completed" { " checked" } else { "" };
            format!(
                r#"<div class="todo-item"><input type="checkbox"{} disabled> <span class="todo-content">{}</span> <span class="todo-priority">P:{}</span> {}</div>"#,
                checked, t.content, t.priority, chip
            )
        })
        .collect();
    wrap_card(
        "TodoWrite",
        "message-dot--thinking",
        "message-card-header--thinking",
        &items.join(""),
        false,
        "",
    )
}

fn status_chip(status: &str) -> String {
    let (color, label) = match status {
        "completed" => ("#03DAC6", "done"),
        "in_progress" => ("#F59E0B", "in progress"),
        "pending" => ("#737373", "pending"),
        _ => ("#737373", status),
    };
    format!(
        r#"<span class="status-chip" style="background:{}20;color:{};border:1px solid {}40">{}</span>"#,
        color, color, color, label
    )
}

fn render_ask_user_question(aq: &crate::model::tool::AskUserQuestionInput) -> String {
    let qs: Vec<String> = aq
        .questions
        .iter()
        .map(|q| {
            let opts: Vec<String> = q
                .options
                .iter()
                .map(|o| {
                    format!(
                        r#"<span class="question-option">{}</span>"#,
                        o.label
                    )
                })
                .collect();
            format!(
                r#"<div class="question-block"><div class="question-text">{}</div><div class="question-options">{}</div></div>"#,
                q.question, opts.join("")
            )
        })
        .collect();
    wrap_card(
        "AskUserQuestion",
        "message-dot--thinking",
        "message-card-header--thinking",
        &qs.join(""),
        false,
        "",
    )
}

fn render_web_search(ws: &crate::model::tool::WebSearchInput) -> String {
    wrap_card(
        "WebSearch",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">QUERY:</span><span class="tool-io-value">{}</span></div>"#,
            ws.query
        ),
        false,
        "",
    )
}

fn render_web_fetch(wf: &crate::model::tool::WebFetchInput) -> String {
    let prompt = wf.prompt.as_deref().unwrap_or("—");
    wrap_card(
        "WebFetch",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">URL:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">PROMPT:</span><span class="tool-io-value">{}</span></div>"#,
            wf.url, prompt
        ),
        false,
        "",
    )
}

fn render_schedule_wakeup(sw: &crate::model::tool::ScheduleWakeupInput) -> String {
    let prompt = sw.prompt.as_deref().unwrap_or("—");
    wrap_card(
        "ScheduleWakeup",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">DELAY:</span><span class="tool-io-value">{}s</span></div><div class="tool-io"><span class="tool-io-label">REASON:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">PROMPT:</span><span class="tool-io-value">{}</span></div>"#,
            sw.delay_seconds, sw.reason, prompt
        ),
        false,
        "",
    )
}

fn render_cron_create(cc: &crate::model::tool::CronCreateInput) -> String {
    wrap_card(
        "CronCreate",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">CRON:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">PROMPT:</span><span class="tool-io-value">{}</span></div>"#,
            cc.cron, cc.prompt
        ),
        false,
        "",
    )
}

fn render_cron_delete(cd: &crate::model::tool::CronDeleteInput) -> String {
    wrap_card(
        "CronDelete",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">ID:</span><span class="tool-io-value">{}</span></div>"#,
            cd.id
        ),
        false,
        "",
    )
}

fn render_cron_list() -> String {
    wrap_card(
        "CronList",
        "message-dot--tool",
        "message-card-header--tool",
        "Listing all cron jobs.",
        false,
        "",
    )
}

fn render_monitor(m: &crate::model::tool::MonitorInput) -> String {
    wrap_card(
        "Monitor",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">DESC:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">TIMEOUT:</span><span class="tool-io-value">{}ms</span></div><div class="tool-io"><span class="tool-io-label">PERSISTENT:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">CMD:</span><span class="tool-io-value">{}</span></div>"#,
            m.description, m.timeout_ms, m.persistent, m.command
        ),
        false,
        "",
    )
}

fn render_task(t: &crate::model::tool::TaskInput) -> String {
    let desc = t.description.as_deref().unwrap_or("—");
    let sub = t.subagent_type.as_deref().unwrap_or("—");
    wrap_card(
        "Task / Agent",
        "message-dot--thinking",
        "message-card-header--thinking",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">DESC:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">AGENT:</span><span class="tool-io-value">{}</span></div>"#,
            desc, sub
        ),
        false,
        "",
    )
}

fn render_team(t: &crate::model::tool::TeamInput) -> String {
    let name = t.name.as_deref().unwrap_or("—");
    wrap_card(
        "Team",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">NAME:</span><span class="tool-io-value">{}</span></div>"#,
            name
        ),
        false,
        "",
    )
}

fn render_send_message(sm: &crate::model::tool::SendMessageInput) -> String {
    let agent = sm.agent_id.as_deref().unwrap_or("—");
    wrap_card(
        "SendMessage",
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">TO:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">MSG:</span><span class="tool-io-value">{}</span></div>"#,
            agent, sm.message
        ),
        false,
        "",
    )
}

fn render_skill(s: &crate::model::tool::SkillInput) -> String {
    let args = s.args.as_deref().unwrap_or("—");
    let title = if s.skill.is_empty() { "Skill".to_string() } else { s.skill.clone() };
    wrap_card(
        &title,
        "message-dot--tool",
        "message-card-header--tool",
        &format!(
            r#"<div class="tool-io"><span class="tool-io-label">SKILL:</span><span class="tool-io-value">{}</span></div><div class="tool-io"><span class="tool-io-label">ARGS:</span><span class="tool-io-value">{}</span></div>"#,
            s.skill, args
        ),
        false,
        "",
    )
}

fn render_exit_plan_mode(_ep: &crate::model::tool::ExitPlanModeInput) -> String {
    wrap_card(
        "ExitPlanMode",
        "message-dot--tool",
        "message-card-header--tool",
        "Plan approved — exiting plan mode.",
        false,
        "",
    )
}

fn render_generic(name: &str, input: &serde_json::Value) -> String {
    let rows: Vec<String> = input
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let val = if v.is_string() {
                        v.as_str().unwrap().to_string()
                    } else {
                        v.to_string()
                    };
                    format!(
                        r#"<div class="tool-io"><span class="tool-io-label">{}:</span><span class="tool-io-value">{}</span></div>"#,
                        k.to_uppercase(),
                        val
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let body = if rows.is_empty() { input.to_string() } else { rows.join("") };
    wrap_card(name, "message-dot--file", "message-card-header--diff", &body, false, "")
}

// ---------------------------------------------------------------------------
// Shared HTML helper
// ---------------------------------------------------------------------------

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// v3 event renderers — T5: Thinking row
// ---------------------------------------------------------------------------

/// Render a thinking event as a v3 gray dot-row with inline expand.
///
/// Empty/blank thinking → disabled static row (no body, no toggle).
/// Non-empty thinking → `<details>` row that expands inline on click.
pub fn render_thinking_row(text: &str) -> String {
    if text.trim().is_empty() {
        format!(
            r#"<div class="timeline-row"><div class="dot {DOT_ASSISTANT}"></div><span class="row-label thinking-disabled">Thinking</span></div>"#
        )
    } else {
        let content = html_escape(text);
        format!(
            r#"<details class="thinking-row"><summary class="timeline-row"><div class="dot {DOT_ASSISTANT}"></div><span class="row-label">Thinking &#x203A;</span></summary><div class="thinking-body"><pre class="thinking-pre">{content}</pre></div></details>"#
        )
    }
}

// ---------------------------------------------------------------------------
// v3 event renderers — T6: Unified tool call row + IN/OUT presence
// ---------------------------------------------------------------------------

/// Render a [`ToolCallEvent`] as a v3 timeline row.
///
/// Each tool gets a green dot-row with its primary arg. Clicking expands
/// IN and/or OUT sections; sections are only emitted when data is present.
pub fn render_tool_call_event(tce: &crate::conversation::ToolCallEvent) -> String {
    let ti = ToolInput::from_name_and_input(&tce.name, tce.input.clone());
    let result = tce.result.as_ref();
    match &ti {
        ToolInput::Bash(b) => render_bash_event(b, result),
        ToolInput::Read(r) => render_read_event(r, result, &tce.id),
        ToolInput::Write(w) => render_write_event(w),
        ToolInput::Edit(e) => render_edit_event(e),
        ToolInput::MultiEdit(me) => render_multiedit_event(me),
        ToolInput::Skill(s) => render_skill_event(s, result, &tce.id),
        _ => render_generic_tool_event(&tce.name, &tce.input, result),
    }
}

/// Wrap tool content in a `<details>` dot-row with expandable body.
///
/// Falls back to a plain non-expandable row when `body` is empty.
fn render_tool_details_row(dot_class: &str, label: &str, body: &str) -> String {
    if body.is_empty() {
        format!(
            r#"<div class="timeline-row"><div class="dot {dot_class}"></div><span class="row-label">{label}</span></div>"#
        )
    } else {
        format!(
            r#"<details class="tool-details"><summary class="timeline-row"><div class="dot {dot_class}"></div><span class="row-label">{label}</span></summary><div class="tool-details-body">{body}</div></details>"#
        )
    }
}

/// Render a labeled `tool-section` with a `<pre>` body.
///
/// Only call when content is non-empty; callers gate on data presence.
fn render_tool_section(label: &str, content: &str, is_error: bool) -> String {
    let error_class = if is_error { " tool-section--error" } else { "" };
    format!(
        r#"<div class="tool-section{error_class}"><div class="tool-section-label">{label}</div><pre class="tool-section-body">{}</pre></div>"#,
        html_escape(content)
    )
}

fn render_bash_event(
    b: &crate::model::tool::BashInput,
    result: Option<&crate::conversation::ToolResult>,
) -> String {
    let desc = b.description.as_deref().unwrap_or("").trim();
    let bg_badge = if b.run_in_background.unwrap_or(false) {
        r#" <span class="badge badge--bg">bg</span>"#
    } else {
        ""
    };
    let label = if !desc.is_empty() {
        format!("Bash — {desc}{bg_badge}")
    } else {
        let preview: String = b.command.chars().take(60).collect();
        let ellipsis = if b.command.chars().count() > 60 { "…" } else { "" };
        format!("Bash{bg_badge} — {preview}{ellipsis}")
    };

    let in_section = render_tool_section("IN", &b.command, false);
    let out_section =
        result.map(|r| render_tool_section("OUT", &r.content, r.is_error)).unwrap_or_default();

    render_tool_details_row(DOT_TOOL, &html_escape(&label), &format!("{in_section}{out_section}"))
}

// ---------------------------------------------------------------------------
// v3 event renderers — T7: Read row → modal (file contents)
// ---------------------------------------------------------------------------

fn render_read_event(
    r: &crate::model::tool::ReadInput,
    result: Option<&crate::conversation::ToolResult>,
    id: &str,
) -> String {
    let line_range = match (r.offset, r.limit) {
        (Some(off), Some(lim)) => format!(":{}-{}", off, off + lim),
        (Some(off), None) => format!(":{}", off),
        _ => String::new(),
    };
    let file_escaped = html_escape(&r.file_path);
    let meta_html = if line_range.is_empty() {
        String::new()
    } else {
        format!(r#" <span class="row-meta">{line_range}</span>"#)
    };

    if let Some(res) = result {
        // Has result: filename opens modal with file contents.
        let template_id = format!("read-{id}");
        let contents_html =
            format!(r#"<pre class="file-contents">{}</pre>"#, html_escape(&res.content));
        let label = format!(
            r#"Read — <button type="button" class="file-link" data-modal="{template_id}">{file_escaped}</button>{meta_html}"#
        );
        format!(
            r#"<div class="timeline-row"><div class="dot {DOT_TOOL}"></div><span class="row-label">{label}</span></div><template id="{template_id}">{contents_html}</template>"#
        )
    } else {
        // No result: plain row, filename is not a link.
        let label = format!("Read — {file_escaped}{meta_html}");
        format!(
            r#"<div class="timeline-row"><div class="dot {DOT_TOOL}"></div><span class="row-label">{label}</span></div>"#
        )
    }
}

fn render_write_event(w: &crate::model::tool::WriteInput) -> String {
    let diff = crate::render::diff::render_unified_diff("", &w.content);
    let summary = crate::render::diff::render_change_summary(diff.added, diff.removed);
    let body = format!("{summary}{}", diff.html);
    render_tool_details_row(DOT_TOOL, &format!("Write — {}", html_escape(&w.file_path)), &body)
}

fn render_edit_event(e: &crate::model::tool::EditInput) -> String {
    let diff = crate::render::diff::render_unified_diff(&e.old_string, &e.new_string);
    let summary = crate::render::diff::render_change_summary(diff.added, diff.removed);
    let body = format!("{summary}{}", diff.html);
    render_tool_details_row(DOT_TOOL, &format!("Edit — {}", html_escape(&e.file_path)), &body)
}

fn render_multiedit_event(me: &crate::model::tool::MultiEditInput) -> String {
    let diffs: Vec<String> = me
        .edits
        .iter()
        .map(|op| {
            let d = crate::render::diff::render_unified_diff(&op.old_string, &op.new_string);
            let s = crate::render::diff::render_change_summary(d.added, d.removed);
            format!(r#"<div class="multiedit-op">{s}{}</div>"#, d.html)
        })
        .collect();
    let label = format!("MultiEdit — {} ({} edits)", html_escape(&me.file_path), me.edits.len());
    render_tool_details_row(DOT_TOOL, &label, &diffs.join(""))
}

// ---------------------------------------------------------------------------
// v3 event renderers — T8: Skill row → modal
// ---------------------------------------------------------------------------

fn render_skill_event(
    s: &crate::model::tool::SkillInput,
    result: Option<&crate::conversation::ToolResult>,
    id: &str,
) -> String {
    let skill_name = if s.skill.is_empty() { "Skill".to_string() } else { html_escape(&s.skill) };
    let row_label = format!("{skill_name} skill");

    if let Some(res) = result {
        let template_id = format!("skill-{id}");
        let body_html =
            format!(r#"<div class="skill-body"><pre>{}</pre></div>"#, html_escape(&res.content));
        format!(
            r#"<div class="timeline-row"><div class="dot {DOT_ASSISTANT}"></div><span class="row-label"><button type="button" class="skill-link" data-modal="{template_id}">{row_label}</button></span></div><template id="{template_id}">{body_html}</template>"#
        )
    } else {
        format!(
            r#"<div class="timeline-row"><div class="dot {DOT_ASSISTANT}"></div><span class="row-label">{row_label}</span></div>"#
        )
    }
}

fn render_generic_tool_event(
    name: &str,
    input: &serde_json::Value,
    result: Option<&crate::conversation::ToolResult>,
) -> String {
    let label = html_escape(name);
    let in_rows: Vec<String> = input
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let val = if v.is_string() {
                        v.as_str().unwrap().to_string()
                    } else {
                        v.to_string()
                    };
                    format!(
                        r#"<div class="tool-row-kv"><span class="tool-kv-key">{}</span><span class="tool-kv-val">{}</span></div>"#,
                        k,
                        html_escape(&val)
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let in_section = if in_rows.is_empty() {
        String::new()
    } else {
        render_tool_section("IN", &in_rows.join(""), false)
    };
    let out_section =
        result.map(|r| render_tool_section("OUT", &r.content, r.is_error)).unwrap_or_default();
    render_tool_details_row(DOT_TOOL, &label, &format!("{in_section}{out_section}"))
}

// ---------------------------------------------------------------------------
// v3 event renderers — T10: Sub-agent row + IN prompt
// ---------------------------------------------------------------------------

/// Render a sub-agent spawn event as a green dot-row `Agent: <desc>`.
///
/// Expands inline to show the IN prompt. No nested transcript is rendered.
pub fn render_sub_agent_row(sa: &crate::conversation::SubAgentEvent) -> String {
    let desc = sa.input.get("description").and_then(|v| v.as_str()).unwrap_or(&sa.name);
    let label = format!("Agent: {}", html_escape(desc));

    let prompt = sa.input.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.is_empty() {
        format!(
            r#"<div class="timeline-row"><div class="dot {DOT_TOOL}"></div><span class="row-label">{label}</span></div>"#
        )
    } else {
        let in_section = render_tool_section("IN", prompt, false);
        render_tool_details_row(DOT_TOOL, &label, &in_section)
    }
}

// ---------------------------------------------------------------------------
// v3 event renderers — T11: Images — horizontal thumbnails → modal
// ---------------------------------------------------------------------------

/// Render a group of images as horizontally-stacked thumbnails.
///
/// Clicking a thumbnail opens it full-size in the shared modal.
/// `card_id` is the parent card's anchor and is used to generate unique IDs.
pub fn render_images_thumbnail_row(
    images: &[crate::model::content::ImageSource],
    card_id: &str,
) -> String {
    let thumbs: String = images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            let src =
                format!("data:{};{},{}", img.media_type, img.source_type, img.data);
            let template_id = format!("img-{card_id}-{i}");
            format!(
                r#"<button type="button" class="img-thumb-btn" data-modal="{template_id}"><img class="img-thumb" src="{src}" alt="Image {num}" loading="lazy"></button><template id="{template_id}"><img class="img-modal-full" src="{src}" alt="Image {num}"></template>"#,
                num = i + 1,
            )
        })
        .collect();
    format!(
        r#"<div class="timeline-row images-row"><div class="img-thumbnails">{thumbs}</div></div>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_bash_card() {
        let html = render_bash(
            &crate::model::tool::BashInput {
                command: "cargo build".into(),
                description: Some("Build project".into()),
                run_in_background: None,
                timeout: None,
                dangerously_disable_sandbox: None,
            },
            false,
        );
        assert!(html.contains("Bash — Build project"));
        assert!(html.contains("cargo build"));
        assert!(html.contains("message-card"));
    }

    #[test]
    fn render_read_card() {
        let html = render_read(&crate::model::tool::ReadInput {
            file_path: "src/main.rs".into(),
            offset: Some(10),
            limit: Some(20),
            pages: None,
        });
        assert!(html.contains("src/main.rs"));
        assert!(html.contains("lines 10-30"));
    }

    #[test]
    fn render_tool_result_with_error() {
        let html = render_tool_result("command failed", true);
        assert!(html.contains("tool-result--error"));
        assert!(html.contains("command failed"));
    }

    #[test]
    fn render_generic_unknown_tool() {
        let html = render_generic("FutureTool", &serde_json::json!({"key1": "val1", "key2": 42}));
        assert!(html.contains("FutureTool"));
        assert!(html.contains("KEY1"));
        assert!(html.contains("val1"));
    }

    #[test]
    fn all_tool_names_render_without_panic() {
        let names = [
            "Bash",
            "Read",
            "Write",
            "Edit",
            "MultiEdit",
            "Glob",
            "Grep",
            "TodoWrite",
            "AskUserQuestion",
            "WebSearch",
            "WebFetch",
            "ScheduleWakeup",
            "CronCreate",
            "CronList",
            "CronDelete",
            "Task",
            "Agent",
            "SendMessage",
            "Skill",
            "ExitPlanMode",
            "Monitor",
            "TeamCreate",
            "TeamDelete",
        ];
        for name in &names {
            let input = serde_json::json!({"dummy": "test"});
            let html = render_tool_use(name, &input, "t1");
            assert!(!html.is_empty(), "render_tool_use should produce output for {}", name);
            // Generic fallback should never panic.
            assert!(html.contains("message-card"), "{} should produce a card", name);
        }
    }

    // B2 tests — unified diff wiring

    #[test]
    fn edit_card_renders_unified_diff_with_summary() {
        let html = render_edit_card(&crate::model::tool::EditInput {
            file_path: "src/main.rs".into(),
            old_string: "a\nb\n".into(),
            new_string: "a\nc\n".into(),
            replace_all: false,
        });
        // Contains diff lines.
        assert!(html.contains("diff-line--add"), "should have added line");
        assert!(html.contains("diff-line--del"), "should have deleted line");
        // Contains change summary with correct counts (new format: +X · −Y).
        assert!(html.contains("diff-count--add"), "should have add count class");
        assert!(html.contains("diff-count--del"), "should have del count class");
        // File-path header preserved.
        assert!(html.contains("Edit — src/main.rs"), "file-path header should show");
        // Old plain-text blocks are absent.
        assert!(!html.contains("old_string"), "should not contain raw old_string label");
        assert!(!html.contains("new_string"), "should not contain raw new_string label");
    }

    #[test]
    fn write_card_renders_pure_add_diff() {
        let html = render_write(&crate::model::tool::WriteInput {
            file_path: "new_file.rs".into(),
            content: "line one\nline two\n".into(),
        });
        // Only added lines, no deleted lines.
        assert!(html.contains("diff-line--add"));
        assert!(!html.contains("diff-line--del"));
        // Summary contains diff counts (new format: +X · −Y).
        assert!(html.contains("diff-count--add"), "should have add count class");
        assert!(html.contains("+2 lines"), "should show +2 lines");
        // File-path header preserved.
        assert!(html.contains("Write — new_file.rs"));
    }

    #[test]
    fn multiedit_card_renders_diff_per_edit() {
        let html = render_multiedit(&crate::model::tool::MultiEditInput {
            file_path: "src/lib.rs".into(),
            edits: vec![
                crate::model::tool::EditOp {
                    old_string: "x\n".into(),
                    new_string: "y\n".into(),
                    replace_all: false,
                },
                crate::model::tool::EditOp {
                    old_string: "".into(),
                    new_string: "z\n".into(),
                    replace_all: false,
                },
            ],
        });
        assert!(html.contains("MultiEdit — src/lib.rs"));
        // Both edits rendered.
        assert!(html.contains("multiedit-op"));
        // First edit has a change.
        assert!(html.contains("diff-line--add"));
        assert!(html.contains("diff-line--del"));
    }

    // -----------------------------------------------------------------------
    // D2 tests — skill card header
    // -----------------------------------------------------------------------

    #[test]
    fn skill_card_header_shows_full_skill_name() {
        let html = render_skill(&crate::model::tool::SkillInput {
            skill: "agent-skills:interview-me".into(),
            args: Some("".into()),
        });
        // Card header should contain the full skill name, not generic "Skill".
        assert!(html.contains("agent-skills:interview-me"));
        assert!(!html.contains(r#">Skill</span>"#), "should not use generic Skill label");
    }

    #[test]
    fn skill_card_header_falls_back_when_empty() {
        let html = render_skill(&crate::model::tool::SkillInput {
            skill: String::new(),
            args: None,
        });
        // Falls back to generic "Skill" when the skill name is empty.
        assert!(html.contains(">Skill<"), "should fall back to generic Skill label");
    }

    // -----------------------------------------------------------------------
    // T2 — render_row + dot class constants
    // -----------------------------------------------------------------------

    #[test]
    fn render_row_assistant_dot_class() {
        let html = render_row(DOT_ASSISTANT, "Thinking", "");
        assert!(html.contains("dot--assistant"), "must use DOT_ASSISTANT class");
        assert!(html.contains("timeline-row"), "must use timeline-row wrapper");
        assert!(html.contains("Thinking"), "label must appear in output");
        assert!(!html.contains("row-meta"), "no meta span when meta is empty");
    }

    #[test]
    fn render_row_tool_dot_class() {
        let html = render_row(DOT_TOOL, "Bash", "cargo build");
        assert!(html.contains("dot--tool"), "must use DOT_TOOL class");
        assert!(html.contains("Bash"), "label must appear");
        assert!(html.contains("row-meta"), "meta span must appear when meta is provided");
        assert!(html.contains("cargo build"), "meta value must appear");
    }

    #[test]
    fn render_row_meta_omitted_when_empty() {
        let html = render_row(DOT_ASSISTANT, "Some label", "");
        assert!(!html.contains("row-meta"), "row-meta span must be absent for empty meta");
    }

    #[test]
    fn dot_constants_distinct() {
        assert_ne!(DOT_ASSISTANT, DOT_TOOL, "dot constants must differ");
        assert_eq!(DOT_ASSISTANT, "dot--assistant");
        assert_eq!(DOT_TOOL, "dot--tool");
    }

    // -----------------------------------------------------------------------
    // T5 — thinking row
    // -----------------------------------------------------------------------

    #[test]
    fn thinking_row_non_empty_is_details_with_gray_dot() {
        let html = render_thinking_row("deep thought");
        assert!(html.contains("<details"), "non-empty thinking must use <details>");
        assert!(html.contains("dot--assistant"), "must use gray dot");
        assert!(html.contains("Thinking"), "label must appear");
        assert!(html.contains("deep thought"), "thinking content must appear");
    }

    #[test]
    fn thinking_row_empty_is_disabled_static_row() {
        let html = render_thinking_row("");
        assert!(!html.contains("<details"), "empty thinking must NOT use <details>");
        assert!(html.contains("thinking-disabled"), "must have disabled class");
        assert!(html.contains("Thinking"), "label must appear");
        assert!(html.contains("dot--assistant"), "must use gray dot");
    }

    #[test]
    fn thinking_row_whitespace_treated_as_empty() {
        let html = render_thinking_row("   \n  ");
        assert!(!html.contains("<details"), "whitespace thinking treated as empty");
        assert!(html.contains("thinking-disabled"));
    }

    // -----------------------------------------------------------------------
    // T6 — render_tool_call_event unified row + IN/OUT presence
    // -----------------------------------------------------------------------

    #[test]
    fn tool_call_bash_with_result_shows_in_and_out() {
        let tce = crate::conversation::ToolCallEvent {
            id: "b1".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "cargo build", "description": "build"}),
            result: Some(crate::conversation::ToolResult {
                content: "Compiling...".to_string(),
                is_error: false,
            }),
        };
        let html = render_tool_call_event(&tce);
        assert!(html.contains("dot--tool"), "bash must use green dot");
        assert!(html.contains("Bash"), "Bash label must appear");
        assert!(html.contains("cargo build"), "command must appear in IN section");
        assert!(html.contains("Compiling"), "result must appear in OUT section");
        assert!(html.contains("tool-section"), "must have tool-section divs");
    }

    #[test]
    fn tool_call_bash_without_result_has_no_out_section() {
        let tce = crate::conversation::ToolCallEvent {
            id: "b1".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
            result: None,
        };
        let html = render_tool_call_event(&tce);
        // IN section present (command is always there).
        assert!(html.contains("ls"), "command must appear");
        // OUT section absent when no result.
        assert!(!html.contains(">OUT<"), "no OUT section when result is None");
    }

    #[test]
    fn tool_call_in_only_no_result_no_out_block() {
        let tce = crate::conversation::ToolCallEvent {
            id: "g1".to_string(),
            name: "Glob".to_string(),
            input: serde_json::json!({"pattern": "*.rs", "path": "src/"}),
            result: None,
        };
        let html = render_tool_call_event(&tce);
        assert!(html.contains("Glob"), "tool name must appear");
        assert!(!html.contains(">OUT<"), "no OUT section when result is None");
    }

    // -----------------------------------------------------------------------
    // T7 — Read row → modal
    // -----------------------------------------------------------------------

    #[test]
    fn read_event_with_result_has_clickable_filename_and_template() {
        let tce = crate::conversation::ToolCallEvent {
            id: "r1".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs", "offset": 10, "limit": 20}),
            result: Some(crate::conversation::ToolResult {
                content: "fn main() {}".to_string(),
                is_error: false,
            }),
        };
        let html = render_tool_call_event(&tce);
        assert!(html.contains("file-link"), "filename must have file-link class");
        assert!(html.contains("data-modal="), "filename must trigger modal");
        assert!(html.contains("src/main.rs"), "file path must appear");
        assert!(html.contains("<template"), "template element must be present");
        assert!(html.contains("fn main()"), "file contents must appear in template");
        assert!(html.contains(":10-30"), "line range must appear in row");
    }

    #[test]
    fn read_event_without_result_has_no_link() {
        let tce = crate::conversation::ToolCallEvent {
            id: "r2".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({"file_path": "src/lib.rs"}),
            result: None,
        };
        let html = render_tool_call_event(&tce);
        assert!(html.contains("src/lib.rs"), "file path must appear");
        assert!(!html.contains("file-link"), "no link when result is absent");
        assert!(!html.contains("data-modal"), "no modal trigger when result is absent");
    }

    // -----------------------------------------------------------------------
    // T8 — Skill row → modal
    // -----------------------------------------------------------------------

    #[test]
    fn skill_event_with_result_shows_full_name_and_modal() {
        let tce = crate::conversation::ToolCallEvent {
            id: "s1".to_string(),
            name: "Skill".to_string(),
            input: serde_json::json!({"skill": "agent-skills:interview-me", "args": ""}),
            result: Some(crate::conversation::ToolResult {
                content: "Skill output here".to_string(),
                is_error: false,
            }),
        };
        let html = render_tool_call_event(&tce);
        assert!(html.contains("agent-skills:interview-me"), "full skill name must appear");
        assert!(html.contains("skill"), "row must say 'skill'");
        assert!(html.contains("dot--assistant"), "skill uses gray dot");
        assert!(html.contains("skill-link"), "skill name must be a link");
        assert!(html.contains("data-modal="), "must trigger modal");
        assert!(html.contains("<template"), "template element must be present");
        assert!(html.contains("Skill output here"), "skill body in template");
    }

    #[test]
    fn skill_event_without_result_shows_name_no_link() {
        let tce = crate::conversation::ToolCallEvent {
            id: "s2".to_string(),
            name: "Skill".to_string(),
            input: serde_json::json!({"skill": "frontend-design", "args": null}),
            result: None,
        };
        let html = render_tool_call_event(&tce);
        assert!(html.contains("frontend-design"), "full skill name must appear");
        assert!(!html.contains("skill-link"), "no link when result is absent");
        assert!(html.contains("dot--assistant"), "skill uses gray dot");
    }

    // -----------------------------------------------------------------------
    // T9 — Edit/Write use new diff format
    // -----------------------------------------------------------------------

    #[test]
    fn tool_call_edit_shows_unified_diff() {
        let tce = crate::conversation::ToolCallEvent {
            id: "e1".to_string(),
            name: "Edit".to_string(),
            input: serde_json::json!({"file_path": "src/a.rs", "old_string": "a\n", "new_string": "b\n", "replace_all": false}),
            result: None,
        };
        let html = render_tool_call_event(&tce);
        assert!(html.contains("diff-line--add"), "must have add diff line");
        assert!(html.contains("diff-line--del"), "must have del diff line");
        assert!(html.contains("diff-count--add"), "must have summary add class");
        assert!(html.contains("Edit — src/a.rs"), "file header must show");
    }

    // -----------------------------------------------------------------------
    // T10 — Sub-agent row
    // -----------------------------------------------------------------------

    #[test]
    fn sub_agent_row_shows_agent_description_green_dot() {
        let sa = crate::conversation::SubAgentEvent {
            tool_call_id: "t1".to_string(),
            name: "Task".to_string(),
            input: serde_json::json!({"description": "search codebase", "prompt": "find *.rs"}),
            result: None,
        };
        let html = render_sub_agent_row(&sa);
        assert!(html.contains("Agent:"), "must show Agent: prefix");
        assert!(html.contains("search codebase"), "description must appear");
        assert!(html.contains("dot--tool"), "must use green dot");
        assert!(html.contains("find *.rs"), "IN prompt must appear");
        assert!(html.contains("<details"), "must be expandable for non-empty prompt");
    }

    #[test]
    fn sub_agent_row_no_prompt_is_plain_row() {
        let sa = crate::conversation::SubAgentEvent {
            tool_call_id: "t2".to_string(),
            name: "Agent".to_string(),
            input: serde_json::json!({"description": "do work"}),
            result: None,
        };
        let html = render_sub_agent_row(&sa);
        assert!(html.contains("do work"), "description must appear");
        assert!(!html.contains("<details"), "no prompt → no expandable details");
    }

    // -----------------------------------------------------------------------
    // T11 — Images thumbnails → modal
    // -----------------------------------------------------------------------

    #[test]
    fn images_thumbnail_row_renders_horizontal_layout() {
        let images = vec![
            crate::model::content::ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: "abc123".to_string(),
            },
            crate::model::content::ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: "def456".to_string(),
            },
        ];
        let html = render_images_thumbnail_row(&images, "msg-5");
        assert!(html.contains("img-thumbnails"), "must have thumbnails container");
        assert!(html.contains("img-thumb"), "must have thumbnail class");
        assert_eq!(html.matches("img-thumb-btn").count(), 2, "two thumbnails for two images");
        assert_eq!(html.matches("<template").count(), 2, "two templates for two images");
        assert!(html.contains("data-modal="), "thumbnails must trigger modal");
        assert!(html.contains("img-modal-full"), "full-size img in template");
    }
}
