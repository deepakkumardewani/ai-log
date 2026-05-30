//! Tool card and message content renderers.
//!
//! Each renderer produces an HTML string that is embedded in a message card.
//! The card chrome (header, border, dot) is applied by [`wrap_card`].

use crate::model::tool::ToolInput;

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

/// Placeholder for embedded images.
pub fn render_image_placeholder(media_type: &str) -> String {
    format!(r#"<div class="image-placeholder">[Image: {}]</div>"#, media_type)
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
    let preview: String = w.content.chars().take(300).collect();
    let more = if w.content.len() > 300 { " …" } else { "" };
    let escaped = preview.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    wrap_card(
        &format!("Write — {}", w.file_path),
        "message-dot--tool",
        "message-card-header--tool",
        &format!(r#"<pre class="tool-write-preview">{}{}</pre>"#, escaped, more),
        false,
        "",
    )
}

fn render_edit_card(e: &crate::model::tool::EditInput) -> String {
    let diff_html = crate::render::diff::render_diff(&e.old_string, &e.new_string);
    wrap_card(
        &format!("Edit — {}", e.file_path),
        "message-dot--tool",
        "message-card-header--diff",
        &diff_html,
        false,
        "",
    )
}

fn render_multiedit(me: &crate::model::tool::MultiEditInput) -> String {
    let diffs: Vec<String> = me
        .edits
        .iter()
        .map(|op| {
            let d = crate::render::diff::render_diff(&op.old_string, &op.new_string);
            format!(r#"<div class="multiedit-op">{}</div>"#, d)
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
    wrap_card(
        "Skill",
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
}
