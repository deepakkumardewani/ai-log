//! HTML rendering via askama templates.
//!
//! [`build_context`] walks a [`Session`] and [`SessionAggregate`] to produce
//! a [`TranscriptContext`] ready for the askama `transcript.html` template.
//! Message content is rendered to HTML strings in Rust (comrak, syntect, similar)
//! and the template only applies layout chrome.

use std::collections::HashMap;

use askama::Template;
use uuid::Uuid;

use crate::aggregate::SessionAggregate;
use crate::model::content::ContentItem;
use crate::model::entry::TranscriptEntry;
use crate::session::Session;

use super::markdown;
use super::tools;

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
}

// ---------------------------------------------------------------------------
// Context builder
// ---------------------------------------------------------------------------

/// Build a [`TranscriptContext`] from a parsed session and its aggregate.
pub fn build_context(session: &Session, agg: &SessionAggregate, css: String) -> TranscriptContext {
    let message_cards = build_message_cards(session);
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

    TranscriptContext {
        css,
        transcript_js: crate::assets::TRANSCRIPT_JS.to_string(),
        page_title: format!("{} — cclog", agg.session_id),
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
    }
}

/// Build a paginated context: only messages within `page.message_range` are
/// included. The chrome (header, sidebar) is only rendered on the first page.
pub fn build_context_paginated(
    session: &Session,
    agg: &SessionAggregate,
    css: String,
    page: &super::pagination::Page,
) -> TranscriptContext {
    let all_cards = build_message_cards(session);
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

    TranscriptContext {
        css,
        transcript_js: crate::assets::TRANSCRIPT_JS.to_string(),
        page_title: format!("{} (page {}/{}) — cclog", agg.session_id, page.number, page.total),
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
// Message card builder
// ---------------------------------------------------------------------------

fn build_message_cards(session: &Session) -> Vec<MessageCard> {
    // Walk messages in DFS order from roots.
    let mut cards: Vec<MessageCard> = Vec::new();
    let mut visited: HashMap<Uuid, bool> = HashMap::new();
    let mut idx = 0usize;

    for root_id in &session.root_message_ids {
        dfs_messages(*root_id, session, &mut visited, &mut idx, &mut cards);
    }

    cards
}

fn dfs_messages(
    uuid: Uuid,
    session: &Session,
    visited: &mut HashMap<Uuid, bool>,
    idx: &mut usize,
    cards: &mut Vec<MessageCard>,
) {
    if visited.contains_key(&uuid) {
        return;
    }
    visited.insert(uuid, true);

    if let Some(node) = session.messages.get(&uuid) {
        if let Some((kind_class, html, snippet)) = render_entry(&node.entry) {
            *idx += 1;
            let anchor = format!("msg-{}", idx);
            cards.push(MessageCard {
                kind_class,
                html,
                anchor,
                snippet,
            });
        }

        for child_id in &node.children {
            dfs_messages(*child_id, session, visited, idx, cards);
        }
    }
}

fn render_entry(entry: &TranscriptEntry) -> Option<(String, String, Option<String>)> {
    match entry {
        TranscriptEntry::User(ue) => {
            let ts = format_ts(&ue.common.timestamp);
            // A "user" turn whose content is only tool_result blocks is
            // really a tool's response coming back to Claude — render it
            // as a Tool Result card rather than an empty USER card.
            if is_tool_result_only(&ue.message) {
                let body = render_tool_results(&ue.message);
                return Some((
                    "message-tool-result".to_string(),
                    tools::wrap_card(
                        "Tool Result",
                        "message-dot--tool",
                        "message-card-header--tool",
                        &body,
                        false,
                        &ts,
                    ),
                    None,
                ));
            }
            let body = render_user_body(&ue.message);
            let snippet = first_text_snippet(&ue.message);
            Some((
                "message-user".to_string(),
                tools::wrap_card(
                    "User",
                    "message-dot--user",
                    "message-card-header--user",
                    &body,
                    false,
                    &ts,
                ),
                snippet,
            ))
        }
        TranscriptEntry::Assistant(ae) => {
            let mut meta_bits = vec![format_ts(&ae.common.timestamp)];
            if let Some(u) = ae.message.usage.as_ref() {
                if let Some(out) = u.output_tokens {
                    meta_bits.push(format!("{} out", out));
                }
                // "in" should reflect everything the model actually
                // processed: fresh input + cache reads + cache writes.
                // `input_tokens` alone undercounts when caching is active.
                let total_in = u.input_tokens.unwrap_or(0)
                    + u.cache_read_input_tokens.unwrap_or(0)
                    + u.cache_creation_input_tokens.unwrap_or(0);
                if total_in > 0 {
                    meta_bits.push(format!("{} in", total_in));
                }
            }
            let meta = meta_bits.join(" · ");
            let msg_str = ae
                .message
                .content
                .iter()
                .map(|item| render_content_item(item, &ae.message.model, &ae.common.timestamp))
                .collect::<Vec<_>>()
                .join("\n");
            let snippet = first_text_snippet(&ae.message);
            Some((
                "message-assistant".to_string(),
                tools::wrap_card(
                    "Assistant",
                    "message-dot--assistant",
                    "message-card-header--assistant",
                    &msg_str,
                    false,
                    &meta,
                ),
                snippet,
            ))
        }
        TranscriptEntry::Summary(se) => {
            let ts = format_ts(&se.common.timestamp);
            let body = se.summary.clone().unwrap_or_default();
            Some((
                "message-summary".to_string(),
                tools::wrap_card(
                    "Summary",
                    "message-dot--assistant",
                    "message-card-header--assistant",
                    &body,
                    false,
                    &ts,
                ),
                None,
            ))
        }
        TranscriptEntry::System(se) => {
            let (k, h) = render_system_entry(se);
            Some((k, h, None))
        }
        TranscriptEntry::HookAttachment(he) => {
            let (k, h) = render_hook_attachment(he);
            Some((k, h, None))
        }
        TranscriptEntry::AwaySummary(asum) => {
            let ts = format_ts(&asum.common.timestamp);
            let body = asum.summary.clone().unwrap_or_else(|| "(no summary text)".to_string());
            Some((
                "message-away".to_string(),
                tools::wrap_card(
                    "Away Summary",
                    "message-dot--file",
                    "message-card-header--system",
                    &html_escape_text(&body),
                    false,
                    &ts,
                ),
                None,
            ))
        }
        TranscriptEntry::QueueOperation(qe) => {
            let ts = format_ts(&qe.common.timestamp);
            let body = qe
                .operation
                .as_ref()
                .map(|v| format!(r#"<pre class="raw-json">{}</pre>"#, pretty_json(v)))
                .unwrap_or_else(|| "(no payload)".to_string());
            Some((
                "message-system".to_string(),
                tools::wrap_card(
                    "Queue Operation",
                    "message-dot--file",
                    "message-card-header--system",
                    &body,
                    false,
                    &ts,
                ),
                None,
            ))
        }
        // file-history-snapshot, last-prompt, permission-mode etc. are session
        // metadata, not messages — drop them rather than showing empty cards.
        TranscriptEntry::Unknown { .. } => None,
    }
}

/// Extract a short, single-line preview from the first text block in a message.
fn first_text_snippet(msg: &crate::model::content::Message) -> Option<String> {
    let text = msg.content.iter().find_map(|c| match c {
        ContentItem::Text { text } => {
            let t = text.trim();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        }
        _ => None,
    })?;
    Some(truncate_snippet(text, 60))
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

fn format_ts(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.with_timezone(&chrono::Local).format("%H:%M:%S · %b %d").to_string()
}

fn is_tool_result_only(msg: &crate::model::content::Message) -> bool {
    !msg.content.is_empty()
        && msg.content.iter().all(|c| matches!(c, ContentItem::ToolResult { .. }))
}

fn render_tool_results(msg: &crate::model::content::Message) -> String {
    msg.content
        .iter()
        .filter_map(|c| match c {
            ContentItem::ToolResult {
                content, is_error, ..
            } => Some(tools::render_tool_result(&content.as_string(), is_error.unwrap_or(false))),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_user_body(msg: &crate::model::content::Message) -> String {
    // Concatenate all Text blocks; render as markdown so lists, code,
    // fenced blocks, etc. show up correctly (real user input is often
    // markdown).
    let text: String = msg
        .content
        .iter()
        .filter_map(|c| match c {
            ContentItem::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        return String::new();
    }
    markdown::render(&text)
}

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

fn html_escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn pretty_json(v: &serde_json::Value) -> String {
    html_escape_text(&serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()))
}

fn render_content_item(
    item: &ContentItem,
    model: &Option<String>,
    parent_ts: &chrono::DateTime<chrono::Utc>,
) -> String {
    match item {
        ContentItem::Text { text } => {
            if let Some(model_name) = model {
                if model_name.to_lowercase().contains("claude") {
                    // Assistant markdown
                    markdown::render(text)
                } else {
                    tools::render_user_message_text(text)
                }
            } else {
                tools::render_user_message_text(text)
            }
        }
        ContentItem::Thinking { thinking, .. } => render_thinking_block(thinking, parent_ts),
        ContentItem::ToolUse {
            name, input, id, ..
        } => tools::render_tool_use(name, input, id),
        ContentItem::ToolResult {
            content, is_error, ..
        } => tools::render_tool_result(&content.as_string(), is_error.unwrap_or(false)),
        ContentItem::Image { source } => tools::render_image_placeholder(&source.media_type),
    }
}

fn render_thinking_block(thinking: &str, parent_ts: &chrono::DateTime<chrono::Utc>) -> String {
    let ts = parent_ts.with_timezone(&chrono::Local).format("%H:%M:%S").to_string();
    let content_html = if thinking.trim().is_empty() {
        r#"<span class="thinking-empty">(thinking content not stored)</span>"#.to_string()
    } else {
        let escaped = thinking.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        escaped.replace('\n', "<br>")
    };
    format!(
        r#"<details class="thinking-block" open>
  <summary class="thinking-summary">
    <span class="thinking-summary-label">Thinking</span>
    <span class="thinking-summary-meta">{ts}</span>
  </summary>
  <div class="thinking-content">{content_html}</div>
</details>"#,
    )
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
            ("message-user" | "message-assistant", 1, Some(s)) if !s.is_empty() => s.to_string(),
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
    match kind {
        "message-user" => ("User", "&#x1F464;", "sidebar-nav-dot--user"),
        "message-assistant" => ("Assistant", "&#x1F916;", "sidebar-nav-dot--assistant"),
        "message-tool-Bash" => ("Bash", "&#x2328;", "sidebar-nav-dot--tool"),
        "message-tool-Read" => ("Read", "&#x1F4D6;", "sidebar-nav-dot--tool"),
        "message-tool-Write" => ("Write", "&#x1F4DD;", "sidebar-nav-dot--tool"),
        "message-tool-Edit" => ("Edit", "&#x1F4DD;", "sidebar-nav-dot--tool"),
        "message-tool-MultiEdit" => ("MultiEdit", "&#x1F4DD;", "sidebar-nav-dot--tool"),
        "message-thinking" => ("Thinking", "&#x1F9E0;", "sidebar-nav-dot--thinking"),
        "message-tool-result" => ("Tool Result", "&#x2328;", "sidebar-nav-dot--tool"),
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
    counts.sort_by(|a, b| b.count.cmp(&a.count));
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
    let cards = vec![
        MessageCard {
            kind_class: "message-user".into(),
            html: tools::wrap_card(
                "User",
                "message-dot--user",
                "message-card-header--user",
                &tools::render_user_message_text("use the /build command to compile"),
                false,
                "",
            ),
            anchor: "msg-1".into(),
            snippet: Some("use the /build command to compile".into()),
        },
        MessageCard {
            kind_class: "message-assistant".into(),
            html: tools::wrap_card(
                "Assistant",
                "message-dot--assistant",
                "message-card-header--assistant",
                &markdown::render("I will run the build command."),
                false,
                "",
            ),
            anchor: "msg-2".into(),
            snippet: Some("I will run the build command.".into()),
        },
        MessageCard {
            kind_class: "message-tool-Bash".into(),
            html: tools::render_tool_use(
                "Bash",
                &serde_json::json!({"command": "cargo build"}),
                "t1",
            ),
            anchor: "msg-3".into(),
            snippet: None,
        },
    ];
    let sidebar_nav = build_sidebar_nav(&cards);
    TranscriptContext {
        css: crate::assets::CSS.to_string(),
        transcript_js: crate::assets::TRANSCRIPT_JS.to_string(),
        page_title: "Claude Code Session — cclog".to_string(),
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
        let ctx = build_context(&session, &agg, crate::assets::CSS.to_string());

        assert!(ctx.message_count > 0);
        assert!(!ctx.message_cards.is_empty());
        // Should have user, assistant, and tool cards.
        let kinds: Vec<&str> = ctx.message_cards.iter().map(|c| c.kind_class.as_str()).collect();
        assert!(kinds.contains(&"message-user"));
        assert!(kinds.contains(&"message-assistant"));
    }
}
