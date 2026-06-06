//! HTML rendering via askama templates.
//!
//! [`build_context`] walks a [`Session`] and [`SessionAggregate`] to produce
//! a [`TranscriptContext`] ready for the askama `transcript.html` template.
//! Message content is rendered to HTML strings in Rust (comrak, syntect, similar)
//! and the template only applies layout chrome.
//!
//! As of v2, messages are grouped into conversational turns (user bubbles,
//! assistant cards with Thinking/Tools pills) via [`crate::conversation::group_session`].

use askama::Template;

use crate::aggregate::SessionAggregate;
use crate::conversation::{flatten_to_timeline, TimelineEvent};
use crate::session::Session;

use super::tools;
use super::turn;

/// Find the first user prompt visible on the session page.
///
/// Uses the same flat timeline + visibility filter as `build_flat_timeline_cards`
/// so that messages that render to nothing (e.g. `<local-command-caveat>` XML
/// blocks that comrak silently drops) are skipped, matching what the user sees.
pub fn find_first_user_prompt(session: &Session) -> Option<String> {
    for event in &flatten_to_timeline(session) {
        if let TimelineEvent::UserMessage(ut) = event {
            if ut.message.trim().is_empty() && ut.images.is_empty() {
                continue;
            }
            // Apply the same visibility guard used when building timeline cards.
            let html = turn::render_user_block(ut, "preview");
            if ut.images.is_empty() && !rendered_has_visible_text(&html) {
                continue;
            }
            let text = ut.message.trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Askama context
// ---------------------------------------------------------------------------

/// Context for rendering `transcript.html`.
#[derive(Template)]
#[template(path = "transcript.html")]
pub struct TranscriptContext {
    pub css: String,
    pub transcript_js: String,
    pub page_title: String,
    pub session_title: String,
    pub session_date: String,
    pub session_time: String,
    pub message_count: usize,
    /// Pretty-printed token total (e.g. "68.8k") summing input, output,
    /// cache creation and cache read tokens.
    pub token_total: String,
    pub duration: String,
    pub is_active: bool,
    pub sidebar_nav_items: Vec<SidebarNavItem>,
    pub file_tree: Vec<FileTreeGroup>,
    pub tool_counts: Vec<ToolCount>,
    /// Pre-rendered message card HTML blocks, in display order.
    pub message_cards: Vec<MessageCard>,
    /// First user prompt, truncated to ~120 chars. Useful as a session
    /// description when no summary title exists.
    pub first_user_prompt: Option<String>,
    /// Project directory name for the back-link in the header.
    /// `None` when the session is exported standalone (no project context).
    pub project_name: Option<String>,
    /// Turn counts for the session header strip.
    pub turn_user_count: usize,
    pub turn_assistant_count: usize,
    pub turn_tool_count: usize,
}

pub struct SidebarNavItem {
    pub label: String,
    pub icon: String,
    pub dot_class: String,
    pub time: String,
    pub anchor: String,
}

pub struct FileTreeGroup {
    pub directory: String,
    pub files: Vec<String>,
}

pub struct ToolCount {
    pub name: String,
    pub count: usize,
}

/// A single rendered message card.
pub struct MessageCard {
    /// CSS class for filter targeting (e.g. "message-user", "message-tool-Bash").
    pub kind_class: String,
    /// Pre-rendered inner HTML (safe).
    pub html: String,
    /// Anchor ID for sidebar linking.
    pub anchor: String,
    /// Optional plain-text preview of the message body, used by the
    /// sidebar to label user/assistant turns by content instead of role.
    pub snippet: Option<String>,
    /// Filter category: "user", "assistant", or "" for tool/system cards.
    pub filter_role: String,
    /// Filter tools: space-separated tool names (e.g. "Bash Read").
    pub filter_tools: String,
}

// ---------------------------------------------------------------------------
// Context builder
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Turn-count helper
// ---------------------------------------------------------------------------

/// Count user turns, assistant turns, and tool calls (recursively including
/// sub-agents) from the session's turn groups. Used in the session header.
fn compute_turn_counts(session: &Session) -> (usize, usize, usize) {
    let turns = crate::conversation::group_into_turns(session);
    let mut user_count = 0usize;
    let mut assistant_count = 0usize;
    let mut tool_count = 0usize;

    for turn in &turns {
        match turn {
            crate::conversation::TurnGroup::User(_) => user_count += 1,
            crate::conversation::TurnGroup::Assistant(at) => {
                assistant_count += 1;
                tool_count += at.tool_calls.len();
                for sa in &at.sub_agents {
                    tool_count += sa.tool_calls.len();
                }
            }
        }
    }

    (user_count, assistant_count, tool_count)
}

// ---------------------------------------------------------------------------
// Context builder
// ---------------------------------------------------------------------------

/// Build a [`TranscriptContext`] from a parsed session and its aggregate.
///
/// `project_name` enables the `← {project}` back-link in the header.
/// Pass `None` for standalone single-session exports.
pub fn build_context(
    session: &Session,
    agg: &SessionAggregate,
    css: String,
    project_name: Option<&str>,
) -> TranscriptContext {
    let message_cards = build_flat_timeline_cards(session);
    let sidebar_nav_items = build_sidebar_nav(&message_cards);
    let file_tree = build_file_tree(agg);
    let tool_counts = build_tool_counts(agg);
    let duration = format_duration(agg);
    let session_date = format_date(agg);
    let session_time = format_time(agg);
    // Include cache tokens so the headline number reflects everything
    // the model processed, not just the un-cached input.
    let raw_total = agg.total_input_tokens
        + agg.total_output_tokens
        + agg.total_cache_creation_tokens
        + agg.total_cache_read_tokens;
    let token_total = format_token_count(raw_total);

    let first_user_prompt = find_first_user_prompt(session);
    let (turn_user_count, turn_assistant_count, turn_tool_count) = compute_turn_counts(session);

    TranscriptContext {
        css,
        transcript_js: crate::assets::TRANSCRIPT_JS.to_string(),
        page_title: format!("{} — weavr", agg.session_id),
        session_title: agg.summaries.first().cloned().unwrap_or_else(|| agg.session_id.clone()),
        session_date,
        session_time,
        message_count: agg.message_count,
        token_total,
        duration,
        is_active: agg.is_active,
        sidebar_nav_items,
        file_tree,
        tool_counts,
        message_cards,
        first_user_prompt,
        project_name: project_name.map(|s| s.to_string()),
        turn_user_count,
        turn_assistant_count,
        turn_tool_count,
    }
}

/// Build a paginated context: only messages within `page.message_range` are
/// included. The chrome (header, sidebar) is only rendered on the first page.
pub fn build_context_paginated(
    session: &Session,
    agg: &SessionAggregate,
    css: String,
    page: &super::pagination::Page,
    project_name: Option<&str>,
) -> TranscriptContext {
    let all_cards = build_flat_timeline_cards(session);
    let page_cards: Vec<MessageCard> = all_cards
        .into_iter()
        .skip(page.message_range.start)
        .take(page.message_range.len())
        .collect();

    let file_tree = if page.is_first { build_file_tree(agg) } else { Vec::new() };
    let tool_counts = if page.is_first { build_tool_counts(agg) } else { Vec::new() };
    let duration = format_duration(agg);
    let session_date = format_date(agg);
    let session_time = format_time(agg);
    let raw_total = agg.total_input_tokens
        + agg.total_output_tokens
        + agg.total_cache_creation_tokens
        + agg.total_cache_read_tokens;
    let token_total = format_token_count(raw_total);

    let first_user_prompt = if page.is_first { find_first_user_prompt(session) } else { None };
    let (turn_user_count, turn_assistant_count, turn_tool_count) = compute_turn_counts(session);

    TranscriptContext {
        css,
        transcript_js: crate::assets::TRANSCRIPT_JS.to_string(),
        page_title: format!("{} (page {}/{}) — weavr", agg.session_id, page.number, page.total),
        session_title: agg.summaries.first().cloned().unwrap_or_else(|| agg.session_id.clone()),
        session_date,
        session_time,
        message_count: agg.message_count,
        token_total,
        duration,
        is_active: agg.is_active,
        sidebar_nav_items: if page.is_first { build_sidebar_nav(&page_cards) } else { Vec::new() },
        file_tree,
        tool_counts,
        message_cards: page_cards,
        first_user_prompt,
        project_name: project_name.map(|s| s.to_string()),
        turn_user_count,
        turn_assistant_count,
        turn_tool_count,
    }
}

fn format_token_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

// ---------------------------------------------------------------------------
// Flat timeline card builder (v3 — individual timeline events)
// ---------------------------------------------------------------------------

fn build_flat_timeline_cards(session: &Session) -> Vec<MessageCard> {
    let events = flatten_to_timeline(session);
    let mut cards: Vec<MessageCard> = Vec::new();

    for (idx, event) in events.iter().enumerate() {
        let anchor = format!("msg-{}", idx + 1);

        match event {
            TimelineEvent::UserMessage(ut) => {
                // Render first; skip if the result has no visible text and no images.
                // Some messages contain only XML/HTML tags (e.g. <task-notification>)
                // that comrak replaces entirely with <!-- raw HTML omitted -->, leaving
                // a visually empty block.
                let html = turn::render_user_block(ut, &anchor);
                if ut.images.is_empty() && !rendered_has_visible_text(&html) {
                    continue;
                }
                let snippet = if ut.message.is_empty() {
                    None
                } else {
                    Some(truncate_snippet(&ut.message, 60))
                };
                cards.push(MessageCard {
                    kind_class: "timeline-user".to_string(),
                    html,
                    anchor,
                    snippet,
                    filter_role: "user".to_string(),
                    filter_tools: String::new(),
                });
            }
            TimelineEvent::AssistantText { text, images, .. } => {
                let snippet = if text.is_empty() { None } else { Some(truncate_snippet(text, 60)) };
                cards.push(MessageCard {
                    kind_class: "timeline-assistant-text".to_string(),
                    html: turn::render_assistant_text_row(text, images),
                    anchor,
                    snippet,
                    filter_role: "assistant".to_string(),
                    filter_tools: String::new(),
                });
            }
            TimelineEvent::Thinking(ts) => {
                cards.push(MessageCard {
                    kind_class: "timeline-thinking".to_string(),
                    html: tools::render_thinking_row(&ts.text),
                    anchor,
                    snippet: None,
                    filter_role: "assistant".to_string(),
                    filter_tools: String::new(),
                });
            }
            TimelineEvent::ToolCall(tc) => {
                let kind_class = format!("timeline-tool-{}", tc.name);
                let html = tools::render_tool_call_event(tc);
                let (filter_role, filter_tools) = filter_attrs(&kind_class);
                cards.push(MessageCard {
                    kind_class,
                    html,
                    anchor,
                    snippet: None,
                    filter_role,
                    filter_tools,
                });
            }
            TimelineEvent::SubAgent(sa) => {
                cards.push(MessageCard {
                    kind_class: "timeline-agent".to_string(),
                    html: tools::render_sub_agent_row(sa),
                    anchor,
                    snippet: None,
                    filter_role: String::new(),
                    filter_tools: String::new(),
                });
            }
            TimelineEvent::Images(images) => {
                cards.push(MessageCard {
                    kind_class: "timeline-images".to_string(),
                    html: tools::render_images_thumbnail_row(images, &anchor),
                    anchor,
                    snippet: None,
                    filter_role: String::new(),
                    filter_tools: String::new(),
                });
            }
        }
    }

    cards
}

/// Returns `true` if `html` contains any non-whitespace text outside of HTML
/// tags and comments. Used to detect user blocks that rendered to nothing
/// (e.g. when comrak replaces an entire XML block with `<!-- raw HTML omitted -->`).
fn rendered_has_visible_text(html: &str) -> bool {
    // Strip the known comrak omission comment first so it doesn't count as text.
    let stripped = html.replace("<!-- raw HTML omitted -->", "");
    let mut in_tag = false;
    for ch in stripped.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag && !c.is_whitespace() => return true,
            _ => {}
        }
    }
    false
}

/// Collapse whitespace and truncate to `max_chars` graphemes-ish (chars).
fn truncate_snippet(s: &str, max_chars: usize) -> String {
    let one_line: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max_chars {
        one_line
    } else {
        let truncated: String = one_line.chars().take(max_chars).collect();
        format!("{}…", truncated.trim_end())
    }
}

/// Extract role and tool names from a `kind_class` for filter data attributes.
fn filter_attrs(kind_class: &str) -> (String, String) {
    // v3 flat timeline kind classes.
    if let Some(tool) = kind_class.strip_prefix("timeline-tool-") {
        let tools = match tool {
            "MultiEdit" => "Edit".to_string(),
            other => other.to_string(),
        };
        return (String::new(), tools);
    }
    match kind_class {
        "timeline-user" => ("user".to_string(), String::new()),
        "timeline-assistant-text" | "timeline-thinking" => ("assistant".to_string(), String::new()),
        "timeline-agent" | "timeline-images" => (String::new(), String::new()),
        // Legacy v2 classes kept for backward compat with stub and tests.
        "turn-user" | "message-user" => ("user".to_string(), String::new()),
        "turn-assistant" | "message-assistant" => ("assistant".to_string(), String::new()),
        "message-tool-Bash" => (String::new(), "Bash".to_string()),
        "message-tool-Read" => (String::new(), "Read".to_string()),
        "message-tool-Write" => (String::new(), "Write".to_string()),
        "message-tool-Edit" => (String::new(), "Edit".to_string()),
        "message-tool-MultiEdit" => (String::new(), "Edit".to_string()),
        _ => (String::new(), String::new()),
    }
}

#[cfg(test)]
fn format_ts(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.with_timezone(&chrono::Local).format("%H:%M:%S").to_string()
}

#[cfg(test)]
fn render_system_entry(se: &crate::model::entry::SystemEntry) -> (String, String) {
    let ts = format_ts(&se.common.timestamp);
    let subtype = se.subtype.as_deref().unwrap_or("system");
    let title = match subtype {
        "stop_hook_summary" => "System · Stop Hook".to_string(),
        "turn_duration" => "System · Turn Duration".to_string(),
        "away_summary" => "System · Away Summary".to_string(),
        "system" => "System".to_string(),
        other => format!("System · {}", other.replace('_', " ")),
    };

    let body = match subtype {
        "turn_duration" => {
            let dur = se.duration_ms.unwrap_or(0);
            let msgs = se.message_count.unwrap_or(0);
            format!(
                r#"<div class="tool-io"><span class="tool-io-label">DURATION:</span><span class="tool-io-value">{:.1}s</span></div><div class="tool-io"><span class="tool-io-label">MESSAGES:</span><span class="tool-io-value">{}</span></div>"#,
                dur as f64 / 1000.0,
                msgs
            )
        }
        "stop_hook_summary" => {
            let count = se.hook_count.unwrap_or(0);
            let infos = se
                .hook_infos
                .as_ref()
                .map(|v| format!(r#"<pre class="raw-json">{}</pre>"#, pretty_json(v)))
                .unwrap_or_default();
            format!(
                r#"<div class="tool-io"><span class="tool-io-label">HOOKS:</span><span class="tool-io-value">{}</span></div>{}"#,
                count, infos
            )
        }
        "away_summary" => {
            let content = se.content.as_deref().unwrap_or("(no content)");
            html_escape_text(content)
        }
        _ => se
            .content
            .as_deref()
            .map(html_escape_text)
            .or_else(|| {
                se.system
                    .as_ref()
                    .map(|v| format!(r#"<pre class="raw-json">{}</pre>"#, pretty_json(v)))
            })
            .unwrap_or_else(|| "(system metadata)".to_string()),
    };

    (
        "message-system".to_string(),
        tools::wrap_card(
            &title,
            "message-dot--file",
            "message-card-header--system",
            &body,
            false,
            &ts,
        ),
    )
}

#[cfg(test)]
fn render_hook_attachment(he: &crate::model::entry::HookAttachmentEntry) -> (String, String) {
    let ts = format_ts(&he.common.timestamp);
    let att = he.attachment.as_ref();
    let hook_name_opt = att.and_then(|v| v.get("hookName").and_then(|s| s.as_str()));
    let hook_event_opt = att.and_then(|v| v.get("hookEvent").and_then(|s| s.as_str()));
    // `hookName` (e.g. "SessionStart:startup") already contains the
    // event, so we don't repeat it. If only `hookEvent` is set, use it.
    let title = match (hook_name_opt, hook_event_opt) {
        (Some(name), _) => format!("Hook · {}", name),
        (None, Some(ev)) => format!("Hook · {}", ev),
        (None, None) => "Hook".to_string(),
    };

    let content = att.and_then(|v| v.get("content").and_then(|s| s.as_str())).unwrap_or("");
    let stdout = att.and_then(|v| v.get("stdout").and_then(|s| s.as_str())).unwrap_or("");
    let stderr = att.and_then(|v| v.get("stderr").and_then(|s| s.as_str())).unwrap_or("");
    let command = att.and_then(|v| v.get("command").and_then(|s| s.as_str())).unwrap_or("");
    let exit_code = att.and_then(|v| v.get("exitCode").and_then(|n| n.as_i64()));

    let mut parts: Vec<String> = Vec::new();
    if !command.is_empty() {
        parts.push(format!(
            r#"<div class="tool-io"><span class="tool-io-label">CMD:</span><span class="tool-io-value">{}</span></div>"#,
            html_escape_text(command)
        ));
    }
    if let Some(code) = exit_code {
        parts.push(format!(
            r#"<div class="tool-io"><span class="tool-io-label">EXIT:</span><span class="tool-io-value">{}</span></div>"#,
            code
        ));
    }
    if !content.is_empty() {
        parts.push(format!(
            r#"<div class="hook-section"><div class="hook-section-label">CONTENT</div><pre class="hook-section-body">{}</pre></div>"#,
            html_escape_text(content)
        ));
    }
    if !stdout.is_empty() {
        parts.push(format!(
            r#"<div class="hook-section"><div class="hook-section-label">STDOUT</div><pre class="hook-section-body">{}</pre></div>"#,
            html_escape_text(stdout)
        ));
    }
    if !stderr.is_empty() {
        parts.push(format!(
            r#"<div class="hook-section"><div class="hook-section-label">STDERR</div><pre class="hook-section-body">{}</pre></div>"#,
            html_escape_text(stderr)
        ));
    }
    let body = if parts.is_empty() {
        att.map(|v| format!(r#"<pre class="raw-json">{}</pre>"#, pretty_json(v)))
            .unwrap_or_else(|| "(empty hook attachment)".to_string())
    } else {
        parts.join("")
    };

    let is_error = exit_code.map(|c| c != 0).unwrap_or(false);
    (
        "message-hook".to_string(),
        tools::wrap_card(
            &title,
            "message-dot--file",
            "message-card-header--hook",
            &body,
            is_error,
            &ts,
        ),
    )
}

#[cfg(test)]
fn html_escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
fn pretty_json(v: &serde_json::Value) -> String {
    html_escape_text(&serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()))
}

// ---------------------------------------------------------------------------
// Sidebar helpers
// ---------------------------------------------------------------------------

/// Build the sidebar "Turns" list.
///
/// The session-history sidebar is grouped, not 1:1 with message cards:
/// consecutive entries with the same `kind_class` (e.g. five Hook
/// attachments in a row, or a long tool-result tail) collapse into a
/// single row with a `(×N)` count. User and assistant turns get a short
/// content snippet as the label when one is available, so the sidebar
/// reads as a real conversation outline instead of a wall of role names.
fn build_sidebar_nav(cards: &[MessageCard]) -> Vec<SidebarNavItem> {
    let mut items: Vec<SidebarNavItem> = Vec::new();
    let mut i = 0;
    while i < cards.len() {
        let kind = cards[i].kind_class.as_str();
        // Find run of consecutive cards with the same kind.
        let mut j = i + 1;
        while j < cards.len() && cards[j].kind_class == kind {
            j += 1;
        }
        let run_len = j - i;
        let (role_label, icon, dot_class) = kind_visuals(kind);

        // Prefer a content snippet for user/assistant single-card rows so
        // the sidebar actually distinguishes turns by what was said.
        let label = match (kind, run_len, cards[i].snippet.as_deref()) {
            ("turn-user" | "turn-assistant" | "message-user" | "message-assistant", 1, Some(s))
                if !s.is_empty() =>
            {
                s.to_string()
            }
            (_, 1, _) => role_label.to_string(),
            (_, n, _) => format!("{role_label} ×{n}"),
        };

        items.push(SidebarNavItem {
            label,
            icon: icon.to_string(),
            dot_class: dot_class.to_string(),
            time: String::new(),
            anchor: cards[i].anchor.clone(),
        });
        i = j;
    }
    items
}

fn kind_visuals(kind: &str) -> (&'static str, &'static str, &'static str) {
    // v3 flat timeline kind classes.
    if kind.starts_with("timeline-tool-") {
        return ("Tool", "&#x2328;", "sidebar-nav-dot--tool");
    }
    match kind {
        "timeline-user" => ("User", "&#x1F464;", "sidebar-nav-dot--user"),
        "timeline-assistant-text" => ("Assistant", "&#x1F916;", "sidebar-nav-dot--assistant"),
        "timeline-thinking" => ("Thinking", "&#x1F9E0;", "sidebar-nav-dot--thinking"),
        "timeline-agent" => ("Agent", "&#x2328;", "sidebar-nav-dot--tool"),
        "timeline-images" => ("Images", "&#x1F4F7;", "sidebar-nav-dot--file"),
        // Legacy v2 classes kept for backward compat.
        "turn-user" | "message-user" => ("User", "&#x1F464;", "sidebar-nav-dot--user"),
        "turn-assistant" | "message-assistant" => {
            ("Assistant", "&#x1F916;", "sidebar-nav-dot--assistant")
        }
        "message-thinking" => ("Thinking", "&#x1F9E0;", "sidebar-nav-dot--thinking"),
        "message-summary" => ("Summary", "&#x1F4CB;", "sidebar-nav-dot--assistant"),
        "message-system" => ("System", "&#x2699;", "sidebar-nav-dot--file"),
        "message-hook" => ("Hook", "&#x1F50C;", "sidebar-nav-dot--file"),
        "message-away" => ("Away", "&#x1F4A4;", "sidebar-nav-dot--file"),
        _ => ("Tool", "&#x2328;", "sidebar-nav-dot--tool"),
    }
}

fn build_file_tree(agg: &SessionAggregate) -> Vec<FileTreeGroup> {
    agg.file_tree
        .iter()
        .map(|(dir, files)| FileTreeGroup {
            directory: dir.clone(),
            files: files.clone(),
        })
        .collect()
}

fn build_tool_counts(agg: &SessionAggregate) -> Vec<ToolCount> {
    let mut counts: Vec<ToolCount> = agg
        .tool_counts
        .iter()
        .map(|(name, count)| ToolCount {
            name: name.clone(),
            count: *count,
        })
        .collect();
    counts.sort_by_key(|b| std::cmp::Reverse(b.count));
    counts
}

fn format_duration(agg: &SessionAggregate) -> String {
    match (agg.first_timestamp, agg.last_timestamp) {
        (Some(first), Some(last)) => {
            let d = last - first;
            if d.num_minutes() > 0 {
                format!("{}m", d.num_minutes())
            } else {
                format!("{}s", d.num_seconds())
            }
        }
        _ => "—".to_string(),
    }
}

fn format_date(agg: &SessionAggregate) -> String {
    agg.first_timestamp
        .map(|t| t.with_timezone(&chrono::Local).format("%b %d").to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn format_time(agg: &SessionAggregate) -> String {
    agg.first_timestamp
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_string())
}

// ---------------------------------------------------------------------------
// Stub (for snapshot / design review)
// ---------------------------------------------------------------------------

pub fn stub_context() -> TranscriptContext {
    use crate::conversation::UserTurn;
    use chrono::TimeZone;
    let ts = chrono::Utc.with_ymd_and_hms(2025, 10, 24, 10, 30, 0).unwrap();

    let bash_tc = crate::conversation::ToolCallEvent {
        id: "t1".to_string(),
        name: "Bash".to_string(),
        input: serde_json::json!({"command": "cargo build"}),
        result: None,
    };

    let cards = vec![
        MessageCard {
            kind_class: "timeline-user".into(),
            html: turn::render_user_block(
                &UserTurn {
                    message: "use the /build command to compile".to_string(),
                    timestamp: ts,
                    images: vec![],
                },
                "msg-1",
            ),
            anchor: "msg-1".into(),
            snippet: Some("use the /build command to compile".into()),
            filter_role: "user".into(),
            filter_tools: String::new(),
        },
        MessageCard {
            kind_class: "timeline-assistant-text".into(),
            html: turn::render_assistant_text_row("I will run the build command.", &[]),
            anchor: "msg-2".into(),
            snippet: Some("I will run the build command.".into()),
            filter_role: "assistant".into(),
            filter_tools: String::new(),
        },
        MessageCard {
            kind_class: "timeline-tool-Bash".into(),
            html: tools::render_tool_call_event(&bash_tc),
            anchor: "msg-3".into(),
            snippet: None,
            filter_role: String::new(),
            filter_tools: "Bash".into(),
        },
    ];
    let sidebar_nav = build_sidebar_nav(&cards);
    TranscriptContext {
        css: crate::assets::CSS.to_string(),
        transcript_js: crate::assets::TRANSCRIPT_JS.to_string(),
        page_title: "Claude Code Session — weavr".to_string(),
        session_title: "Implement Connect4 Rules Engine".to_string(),
        session_date: "Oct 24".to_string(),
        session_time: "10:30:00".to_string(),
        message_count: 24,
        token_total: "15.4k".to_string(),
        duration: "12m".to_string(),
        is_active: true,
        sidebar_nav_items: sidebar_nav,
        file_tree: vec![FileTreeGroup {
            directory: "src/".into(),
            files: vec!["main.rs".into(), "lib.rs".into()],
        }],
        tool_counts: vec![
            ToolCount {
                name: "Bash".into(),
                count: 3,
            },
            ToolCount {
                name: "Read".into(),
                count: 2,
            },
        ],
        message_cards: cards,
        first_user_prompt: Some("use the /build command to compile".to_string()),
        project_name: Some("demo-project".to_string()),
        turn_user_count: 1,
        turn_assistant_count: 1,
        turn_tool_count: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_stub_transcript_is_self_contained() {
        let ctx = stub_context();
        let html = ctx.render().expect("template should render");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Claude Code Session"));
        assert!(html.contains("--surface"), "CSS tokens should be embedded");
        assert!(!html.contains("http://"), "must not contain http://");
        assert!(!html.contains("https://"), "must not contain https://");
    }

    #[test]
    fn build_context_from_fixture() {
        use crate::aggregate::aggregate;
        use crate::parser::parse_reader;
        use crate::session::build_session;
        use std::io::Cursor;

        let jsonl = crate::tests::fixture_linear_session();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        let agg = aggregate(&session);
        let ctx = build_context(&session, &agg, crate::assets::CSS.to_string(), None);

        assert!(ctx.message_count > 0);
        assert!(!ctx.message_cards.is_empty());
        // Should have user and assistant timeline cards.
        let kinds: Vec<&str> = ctx.message_cards.iter().map(|c| c.kind_class.as_str()).collect();
        assert!(kinds.contains(&"timeline-user"), "should contain timeline-user, got: {kinds:?}");
        assert!(
            kinds
                .iter()
                .any(|k| *k == "timeline-assistant-text" || k.starts_with("timeline-tool-")),
            "should contain assistant or tool timeline cards, got: {kinds:?}"
        );
    }

    #[test]
    fn find_first_user_prompt_linear_session() {
        use crate::parser::parse_reader;
        use crate::session::build_session;
        use std::io::Cursor;

        let jsonl = crate::tests::fixture_linear_session();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        let prompt = find_first_user_prompt(&session);
        assert!(prompt.is_some());
        assert_eq!(prompt.unwrap(), "Build the project");
    }

    #[test]
    fn find_first_user_prompt_no_user_returns_none() {
        use std::io::Cursor;
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = format!(
            r#"{{"type":"assistant","uuid":"{a1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Hello!"}}]}}}}"#
        );
        let result = crate::parser::parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = crate::session::build_session(&result.entries);
        let prompt = find_first_user_prompt(&session);
        assert!(prompt.is_none());
    }

    #[test]
    fn filter_attrs_extracts_role_and_tools() {
        // v3 kind classes.
        assert_eq!(filter_attrs("timeline-user"), ("user".into(), "".into()));
        assert_eq!(filter_attrs("timeline-assistant-text"), ("assistant".into(), "".into()));
        assert_eq!(filter_attrs("timeline-thinking"), ("assistant".into(), "".into()));
        assert_eq!(filter_attrs("timeline-tool-Bash"), ("".into(), "Bash".into()));
        assert_eq!(filter_attrs("timeline-tool-Read"), ("".into(), "Read".into()));
        assert_eq!(filter_attrs("timeline-tool-Write"), ("".into(), "Write".into()));
        assert_eq!(filter_attrs("timeline-tool-Edit"), ("".into(), "Edit".into()));
        assert_eq!(filter_attrs("timeline-tool-MultiEdit"), ("".into(), "Edit".into()));
        // Legacy v2 classes still work.
        assert_eq!(filter_attrs("message-user"), ("user".into(), "".into()));
        assert_eq!(filter_attrs("message-tool-Bash"), ("".into(), "Bash".into()));
        // Unknown kinds return empty strings.
        assert_eq!(filter_attrs("message-system"), ("".into(), "".into()));
    }

    #[test]
    fn stub_includes_project_name_and_filter_attrs() {
        let ctx = stub_context();
        let html = ctx.render().expect("stub template should render");
        // Back link.
        assert!(html.contains("demo-project"), "should contain project name");
        assert!(html.contains("&larr;"), "should contain back arrow");
        // Filter data attributes on message cards.
        assert!(html.contains(r#"data-role="user""#), "should have data-role user");
        assert!(html.contains(r#"data-tools="Bash""#), "should have data-tools Bash");
    }

    // -----------------------------------------------------------------------
    // D1 tests — per-card header metadata cleanup
    // -----------------------------------------------------------------------

    #[test]
    fn format_ts_produces_time_only_no_date() {
        use chrono::TimeZone;
        let ts = chrono::Utc.with_ymd_and_hms(2025, 6, 15, 10, 30, 5).unwrap();
        let formatted = format_ts(&ts);
        // Should contain HH:MM:SS time.
        assert!(formatted.contains(':'), "should contain time colon");
        // Should not contain date substrings.
        assert!(!formatted.contains("Jun"), "ts should not contain month: {formatted}");
        assert!(!formatted.contains("2025"), "ts should not contain year: {formatted}");
        assert!(!formatted.contains("15"), "ts should not contain day: {formatted}");
    }

    #[test]
    fn system_entry_card_header_has_no_date_or_token_labels() {
        use crate::model::entry::SystemEntry;
        use chrono::TimeZone;
        use uuid::Uuid;

        let ts = chrono::Utc.with_ymd_and_hms(2025, 10, 20, 14, 5, 30).unwrap();
        let se = SystemEntry {
            common: crate::model::entry::CommonFields {
                uuid: Uuid::nil(),
                parent_uuid: None,
                timestamp: ts,
                session_id: "test-session".to_string(),
                is_sidechain: false,
                is_meta: false,
                agent_id: None,
                cwd: None,
                git_branch: None,
                version: None,
            },
            system: None,
            subtype: Some("system".to_string()),
            content: Some("System message body".to_string()),
            duration_ms: None,
            message_count: None,
            hook_count: None,
            hook_infos: None,
            level: None,
        };
        let (_kind, html) = render_system_entry(&se);
        // Extract the meta text (visible header time).
        let meta_start = html.find("message-card-meta").unwrap();
        let meta_end = html[meta_start..].find("</span>").unwrap();
        let meta_text = &html[meta_start..meta_start + meta_end];
        // Visible header text should only contain HH:MM:SS, no date.
        assert!(!meta_text.contains("Oct"), "meta should not contain month: {meta_text}");
        assert!(!meta_text.contains("2025"), "meta should not contain year: {meta_text}");
        assert!(!meta_text.contains(" · "), "meta should not contain date separator");
        // Card header should not contain token labels.
        assert!(!html.contains("in:"), "card header should not contain in: {html}");
        assert!(!html.contains("out:"), "card header should not contain out: {html}");
        assert!(!html.contains("Cache"), "card header should not contain Cache: {html}");
        // Time format: HH:MM:SS (digits separated by colons).
        let has_time = meta_text.chars().filter(|&c| c == ':').count() == 2;
        assert!(has_time, "meta should contain HH:MM:SS time: {meta_text}");
    }

    #[test]
    fn hook_attachment_card_header_has_no_date_or_token_labels() {
        use crate::model::entry::HookAttachmentEntry;
        use chrono::TimeZone;
        use uuid::Uuid;

        let ts = chrono::Utc.with_ymd_and_hms(2025, 8, 10, 9, 0, 0).unwrap();
        let he = HookAttachmentEntry {
            common: crate::model::entry::CommonFields {
                uuid: Uuid::nil(),
                parent_uuid: None,
                timestamp: ts,
                session_id: "test-session".to_string(),
                is_sidechain: false,
                is_meta: false,
                agent_id: None,
                cwd: None,
                git_branch: None,
                version: None,
            },
            attachment: Some(serde_json::json!({
                "hookName": "SessionStart",
                "content": "hook output here"
            })),
        };
        let (_kind, html) = render_hook_attachment(&he);
        // Extract visible meta text.
        let meta_start = html.find("message-card-meta").unwrap();
        let meta_end = html[meta_start..].find("</span>").unwrap();
        let meta_text = &html[meta_start..meta_start + meta_end];
        assert!(!meta_text.contains("Aug"), "meta should not contain month: {meta_text}");
        assert!(!meta_text.contains("2025"), "meta should not contain year: {meta_text}");
        assert!(!html.contains("in:"), "card header should not contain in:");
        assert!(!html.contains("out:"), "card header should not contain out:");
        let has_time = meta_text.chars().filter(|&c| c == ':').count() == 2;
        assert!(has_time, "meta should contain HH:MM:SS time: {meta_text}");
    }

    #[test]
    fn full_render_from_fixture_has_no_date_or_token_labels_in_turn_headers() {
        use crate::aggregate::aggregate;
        use crate::parser::parse_reader;
        use crate::session::build_session;
        use std::io::Cursor;

        let jsonl = crate::tests::fixture_linear_session();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        let agg = aggregate(&session);
        let ctx = build_context(&session, &agg, crate::assets::CSS.to_string(), None);
        let html = ctx.render().expect("template should render");

        // Turn headers should show only time. Extract visible text from <time> elements.
        // Pattern: <time datetime="...">VISIBLE</time>
        let mut pos = 0usize;
        while let Some(tag_start) = html[pos..].find("<time ") {
            let abs = pos + tag_start;
            if let Some(content_start) = html[abs..].find('>') {
                let c_start = abs + content_start + 1;
                if let Some(time_end) = html[c_start..].find("</time>") {
                    let visible = &html[c_start..c_start + time_end];
                    assert!(
                        !visible.contains("2025"),
                        "visible time should not contain year: '{visible}'"
                    );
                    assert!(
                        !visible.contains("Jun"),
                        "visible time should not contain month: '{visible}'"
                    );
                    assert_eq!(
                        visible.chars().filter(|&c| c == ':').count(),
                        2,
                        "visible time should be HH:MM:SS: '{visible}'"
                    );
                    pos = c_start + time_end + "</time>".len();
                    continue;
                }
            }
            pos += 1;
        }
        // No token labels in the entire output.
        assert!(!html.contains("in:</span>"), "should not contain in: label");
        assert!(!html.contains("out:</span>"), "should not contain out: label");
        assert!(!html.contains("Cache Creation"), "should not contain Cache Creation");
    }

    // -----------------------------------------------------------------------
    // E1 tests — aggregate counts in session header
    // -----------------------------------------------------------------------

    #[test]
    fn session_header_contains_turn_counts() {
        use crate::aggregate::aggregate;
        use crate::parser::parse_reader;
        use crate::session::build_session;
        use std::io::Cursor;

        let jsonl = crate::tests::fixture_linear_session();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        let agg = aggregate(&session);
        let ctx = build_context(&session, &agg, crate::assets::CSS.to_string(), None);
        let html = ctx.render().expect("template should render");

        // Session header should contain user, assistant, and tools counts.
        assert!(html.contains("user"), "header should contain 'user'");
        assert!(html.contains("assistant"), "header should contain 'assistant'");
        assert!(html.contains("tools"), "header should contain 'tools'");
    }

    #[test]
    fn turn_counts_are_accurate_against_fixture() {
        use crate::parser::parse_reader;
        use crate::session::build_session;
        use std::io::Cursor;

        let jsonl = crate::tests::fixture_linear_session();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        // Verify counts from compute_turn_counts match expected.
        let (user_count, assistant_count, tool_count) = compute_turn_counts(&session);
        // Linear session fixture: 1 user prompt, 1 assistant turn with 1 Bash tool.
        assert_eq!(user_count, 1, "expected 1 user turn");
        assert_eq!(assistant_count, 1, "expected 1 assistant turn");
        assert_eq!(tool_count, 1, "expected 1 tool call");
    }

    #[test]
    fn stub_context_header_contains_key_stats() {
        let ctx = stub_context();
        let html = ctx.render().expect("stub template should render");
        // Header shows date, duration, message count, and token total.
        assert!(html.contains("msgs"), "header should contain message count label");
        assert!(
            html.contains("tokens") || html.contains("k") || html.contains("M"),
            "header should contain token total"
        );
    }

    // -----------------------------------------------------------------------
    // E2 tests — footer cleanup
    // -----------------------------------------------------------------------

    #[test]
    fn footer_does_not_contain_session_inactive_or_active() {
        let ctx = stub_context();
        let html = ctx.render().expect("stub template should render");
        assert!(!html.contains("Session Inactive"), "footer should not contain Session Inactive");
        assert!(!html.contains("Session Active"), "footer should not contain Session Active");
    }

    #[test]
    fn footer_does_not_contain_total_tokens() {
        let ctx = stub_context();
        let html = ctx.render().expect("stub template should render");
        // The footer should not have a "Total Tokens" label.
        assert!(!html.contains("Total Tokens"), "footer should not contain Total Tokens");
    }

    #[test]
    fn full_render_footer_does_not_contain_session_inactive_or_total_tokens() {
        use crate::aggregate::aggregate;
        use crate::parser::parse_reader;
        use crate::session::build_session;
        use std::io::Cursor;

        let jsonl = crate::tests::fixture_linear_session();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        let agg = aggregate(&session);
        let ctx = build_context(&session, &agg, crate::assets::CSS.to_string(), None);
        let html = ctx.render().expect("template should render");

        assert!(!html.contains("Session Inactive"), "footer should not contain Session Inactive");
        assert!(!html.contains("Session Active"), "footer should not contain Session Active");
        assert!(!html.contains("Total Tokens"), "footer should not contain Total Tokens");
    }
}
