//! Session-to-Markdown export renderer.
//!
//! Walks a [`Session`] and produces a Markdown string. Tool calls become
//! fenced JSON code blocks, diffs render as unified `+/-` blocks.
//! Honors [`DetailLevel`] and compact mode.

use std::collections::HashMap;

use uuid::Uuid;

use crate::aggregate::SessionAggregate;
use crate::model::content::ContentItem;
use crate::model::entry::TranscriptEntry;
use crate::model::tool::ToolInput;
use crate::session::Session;

// ---------------------------------------------------------------------------
// Detail level
// ---------------------------------------------------------------------------

/// Controls how much detail appears in the Markdown output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum DetailLevel {
    /// Every message, tool call, thinking block, and system entry.
    Full,
    /// User + assistant messages + tool calls (no thinking, no system metadata).
    High,
    /// User + assistant messages only (no tool calls, no thinking).
    Low,
    /// User messages + assistant text only (no metadata, no tool blocks).
    Minimal,
    /// Only user messages.
    #[clap(name = "user-only")]
    UserOnly,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a full session as a Markdown string.
pub fn render_session(
    session: &Session,
    agg: &SessionAggregate,
    detail: DetailLevel,
    compact: bool,
) -> String {
    let mut buf = String::with_capacity(32_768);

    // Header
    let title = agg.summaries.first().cloned().unwrap_or_else(|| agg.session_id.clone());
    buf.push_str(&format!("# {}\n\n", title));

    if !compact {
        buf.push_str(&format!("**Session:** {}\n\n", agg.session_id));
        if let (Some(first), Some(last)) = (agg.first_timestamp, agg.last_timestamp) {
            let date = first.with_timezone(&chrono::Local).format("%b %d, %Y");
            let start = first.with_timezone(&chrono::Local).format("%H:%M:%S");
            let end = last.with_timezone(&chrono::Local).format("%H:%M:%S");
            buf.push_str(&format!("**Date:** {} | **Time:** {} – {}\n\n", date, start, end));
        }
        let raw_total = agg.total_input_tokens
            + agg.total_output_tokens
            + agg.total_cache_creation_tokens
            + agg.total_cache_read_tokens;
        buf.push_str(&format!(
            "**Messages:** {} | **Tokens:** {}\n\n",
            agg.message_count,
            format_token_count(raw_total)
        ));
        buf.push_str("---\n\n");
    }

    // Walk messages in DFS order (same as HTML path).
    let mut visited: HashMap<Uuid, bool> = HashMap::new();
    for root_id in &session.root_message_ids {
        dfs_render(*root_id, session, &mut visited, detail, compact, &mut buf);
    }

    buf
}

// ---------------------------------------------------------------------------
// DFS walk + per-entry dispatch
// ---------------------------------------------------------------------------

fn dfs_render(
    uuid: Uuid,
    session: &Session,
    visited: &mut HashMap<Uuid, bool>,
    detail: DetailLevel,
    compact: bool,
    buf: &mut String,
) {
    if visited.contains_key(&uuid) {
        return;
    }
    visited.insert(uuid, true);

    if let Some(node) = session.messages.get(&uuid) {
        render_entry(&node.entry, detail, compact, buf);
        for child_id in &node.children {
            dfs_render(*child_id, session, visited, detail, compact, buf);
        }
    }
}

fn render_entry(entry: &TranscriptEntry, detail: DetailLevel, compact: bool, buf: &mut String) {
    match entry {
        TranscriptEntry::User(ue) => {
            if !compact {
                let ts = format_ts_md(&ue.common.timestamp);
                buf.push_str(&format!("### User ({})\n\n", ts));
            } else {
                buf.push_str("### User\n\n");
            }
            let text = extract_user_text(&ue.message);
            if !text.trim().is_empty() {
                buf.push_str(&text);
                buf.push('\n');
            }
            // Tool results in a user entry
            for item in &ue.message.content {
                if let ContentItem::ToolResult {
                    content, is_error, ..
                } = item
                {
                    if detail == DetailLevel::Full || detail == DetailLevel::High {
                        let prefix = if is_error.unwrap_or(false) { "ERROR" } else { "OUT" };
                        buf.push_str(&format!(
                            "\n**{}:**\n\n```\n{}\n```\n",
                            prefix,
                            content.as_string()
                        ));
                    }
                }
            }
            buf.push('\n');
        }

        TranscriptEntry::Assistant(ae) => {
            if detail == DetailLevel::UserOnly {
                return;
            }

            let model = ae.message.model.as_deref().unwrap_or("assistant");
            if detail == DetailLevel::Minimal {
                buf.push_str("### Assistant\n\n");
            } else if !compact {
                let ts = format_ts_md(&ae.common.timestamp);
                let mut meta = vec![ts];
                if let Some(u) = ae.message.usage.as_ref() {
                    if let Some(out) = u.output_tokens {
                        meta.push(format!("{} out", out));
                    }
                    let total_in = u.input_tokens.unwrap_or(0)
                        + u.cache_read_input_tokens.unwrap_or(0)
                        + u.cache_creation_input_tokens.unwrap_or(0);
                    if total_in > 0 {
                        meta.push(format!("{} in", total_in));
                    }
                }
                buf.push_str(&format!("### Assistant ({}) · {}\n\n", model, meta.join(" · ")));
            } else {
                buf.push_str(&format!("### Assistant ({})\n\n", model));
            }

            let include_thinking = detail == DetailLevel::Full;
            let include_tools = detail == DetailLevel::Full || detail == DetailLevel::High;

            for item in &ae.message.content {
                match item {
                    ContentItem::Text { text } => {
                        if detail != DetailLevel::Minimal || !text.trim().is_empty() {
                            buf.push_str(text);
                            buf.push('\n');
                        }
                    }
                    ContentItem::Thinking { thinking, .. } if include_thinking => {
                        buf.push_str("\n<details>\n<summary>Thinking</summary>\n\n");
                        buf.push_str(thinking);
                        buf.push_str("\n</details>\n\n");
                    }
                    ContentItem::ToolUse { name, input, id: _ } if include_tools => {
                        render_tool_use_md(name, input, buf);
                    }
                    _ => {}
                }
            }
            buf.push('\n');
        }

        TranscriptEntry::Summary(se) if detail == DetailLevel::Full => {
            let title = se.title.as_deref().unwrap_or("Summary");
            buf.push_str(&format!("### {}\n\n", title));
            if let Some(ref text) = se.summary {
                buf.push_str(text);
                buf.push('\n');
            }
            buf.push('\n');
        }

        TranscriptEntry::System(se) if detail == DetailLevel::Full => {
            let subtype = se.subtype.as_deref().unwrap_or("system");
            let title = match subtype {
                "stop_hook_summary" => "System · Stop Hook",
                "turn_duration" => "System · Turn Duration",
                "away_summary" => "System · Away Summary",
                "system" => "System",
                other => {
                    buf.push_str(&format!("### System · {}\n\n", other.replace('_', " ")));
                    return;
                }
            };
            buf.push_str(&format!("### {}\n\n", title));

            match subtype {
                "turn_duration" => {
                    let dur = se.duration_ms.unwrap_or(0);
                    let msgs = se.message_count.unwrap_or(0);
                    buf.push_str(&format!("- Duration: {:.1}s\n", dur as f64 / 1000.0));
                    buf.push_str(&format!("- Messages: {}\n", msgs));
                }
                "stop_hook_summary" => {
                    let count = se.hook_count.unwrap_or(0);
                    buf.push_str(&format!("- Hooks: {}\n", count));
                }
                "away_summary" => {
                    if let Some(ref content) = se.content {
                        buf.push_str(content);
                        buf.push('\n');
                    }
                }
                _ => {}
            }
            buf.push('\n');
        }

        TranscriptEntry::HookAttachment(he) if detail == DetailLevel::Full => {
            let att = he.attachment.as_ref();
            let hook_name = att.and_then(|v| v.get("hookName").and_then(|s| s.as_str()));
            buf.push_str(&format!("### Hook · {}\n\n", hook_name.unwrap_or("attachment")));
            if let Some(cmd) = att.and_then(|v| v.get("command").and_then(|s| s.as_str())) {
                buf.push_str(&format!("- CMD: `{}`\n", cmd));
            }
            if let Some(code) = att.and_then(|v| v.get("exitCode").and_then(|n| n.as_i64())) {
                buf.push_str(&format!("- Exit: {}\n", code));
            }
            buf.push('\n');
        }

        TranscriptEntry::AwaySummary(asum) if detail == DetailLevel::Full => {
            buf.push_str("### Away Summary\n\n");
            if let Some(ref text) = asum.summary {
                buf.push_str(text);
                buf.push('\n');
            }
            buf.push('\n');
        }

        TranscriptEntry::QueueOperation(qe) if detail == DetailLevel::Full => {
            buf.push_str("### Queue Operation\n\n");
            if let Some(ref op) = qe.operation {
                buf.push_str(&format!(
                    "```json\n{}\n```\n\n",
                    serde_json::to_string_pretty(op).unwrap_or_default()
                ));
            }
        }

        // Skip unknown / other types.
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tool use → Markdown
// ---------------------------------------------------------------------------

fn render_tool_use_md(name: &str, input: &serde_json::Value, buf: &mut String) {
    let ti = ToolInput::from_name_and_input(name, input.clone());

    match &ti {
        ToolInput::Bash(b) => {
            buf.push_str(&format!("#### Bash — {}\n\n", b.description.as_deref().unwrap_or("")));
            buf.push_str(&format!("```bash\n{}\n```\n\n", b.command));
        }
        ToolInput::Read(r) => {
            buf.push_str(&format!("#### Read — {}\n\n", r.file_path));
            if let (Some(off), Some(lim)) = (r.offset, r.limit) {
                buf.push_str(&format!("*Offset: {} | Limit: {}*\n\n", off, lim));
            }
        }
        ToolInput::Write(w) => {
            buf.push_str(&format!("#### Write — {}\n\n", w.file_path));
            let preview = truncate_for_md(&w.content, 500);
            buf.push_str(&format!("```\n{}\n```\n\n", preview));
        }
        ToolInput::Edit(e) => {
            buf.push_str(&format!("#### Edit — {}\n\n", e.file_path));
            buf.push_str(&render_unified_diff_md(&e.old_string, &e.new_string));
            buf.push('\n');
        }
        ToolInput::MultiEdit(me) => {
            buf.push_str(&format!(
                "#### MultiEdit — {} ({} edits)\n\n",
                me.file_path,
                me.edits.len()
            ));
            for (i, op) in me.edits.iter().enumerate() {
                buf.push_str(&format!("**Edit {}:**\n\n", i + 1));
                buf.push_str(&render_unified_diff_md(&op.old_string, &op.new_string));
            }
            buf.push('\n');
        }
        ToolInput::Glob(g) => {
            buf.push_str("#### Glob\n\n");
            buf.push_str(&format!("- Pattern: `{}`\n", g.pattern));
            buf.push_str(&format!("- Path: `{}`\n\n", g.path.as_deref().unwrap_or(".")));
        }
        ToolInput::Grep(g) => {
            buf.push_str("#### Grep\n\n");
            buf.push_str(&format!("- Pattern: `{}`\n", g.pattern));
            buf.push_str(&format!("- Path: `{}`\n", g.path.as_deref().unwrap_or(".")));
            if let Some(ref inc) = g.include {
                buf.push_str(&format!("- Include: `{}`\n", inc));
            }
            buf.push('\n');
        }
        ToolInput::TodoWrite(tw) => {
            buf.push_str("#### TodoWrite\n\n");
            for t in &tw.todos {
                let check = if t.status == "completed" { "x" } else { " " };
                let prio = priority_label(&t.priority);
                buf.push_str(&format!("- [{}] {} *({})*\n", check, t.content, prio));
            }
            buf.push('\n');
        }
        ToolInput::AskUserQuestion(aq) => {
            buf.push_str("#### AskUserQuestion\n\n");
            for q in &aq.questions {
                buf.push_str(&format!("**{}**\n\n", q.question));
                for opt in &q.options {
                    buf.push_str(&format!("- {} — {}\n", opt.label, opt.description));
                }
                buf.push('\n');
            }
        }
        ToolInput::WebSearch(ws) => {
            buf.push_str(&format!("#### WebSearch — {}\n\n", ws.query));
        }
        ToolInput::WebFetch(wf) => {
            buf.push_str(&format!("#### WebFetch — {}\n\n", wf.url));
        }
        ToolInput::ScheduleWakeup(sw) => {
            buf.push_str(&format!(
                "#### ScheduleWakeup — {}s · {}\n\n",
                sw.delay_seconds, sw.reason
            ));
        }
        ToolInput::CronCreate(cc) => {
            buf.push_str(&format!("#### CronCreate — `{}`\n\n", cc.cron));
            buf.push_str(&format!("```\n{}\n```\n\n", cc.prompt));
        }
        ToolInput::CronDelete(cd) => {
            buf.push_str(&format!("#### CronDelete — {}\n\n", cd.id));
        }
        ToolInput::CronList(_) => {
            buf.push_str("#### CronList\n\n");
        }
        ToolInput::Monitor(m) => {
            buf.push_str(&format!("#### Monitor — {}\n\n", m.description));
            buf.push_str(&format!(
                "- Timeout: {}ms | Persistent: {}\n",
                m.timeout_ms, m.persistent
            ));
            buf.push_str(&format!("```bash\n{}\n```\n\n", m.command));
        }
        ToolInput::Task(t) => {
            let label = t.description.as_deref().unwrap_or(t.subject.as_deref().unwrap_or("Task"));
            buf.push_str(&format!("#### Task — {}\n\n", label));
            if let Some(ref prompt) = t.prompt {
                buf.push_str(&format!("```\n{}\n```\n\n", truncate_for_md(prompt, 300)));
            }
        }
        ToolInput::Team(t) => {
            buf.push_str(&format!("#### Team — {}\n\n", t.name.as_deref().unwrap_or("team")));
        }
        ToolInput::SendMessage(sm) => {
            buf.push_str(&format!("#### SendMessage\n\n```\n{}\n```\n\n", sm.message));
        }
        ToolInput::Skill(s) => {
            buf.push_str(&format!("#### Skill — {}\n\n", s.skill));
            if let Some(ref args) = s.args {
                buf.push_str(&format!("Args: `{}`\n\n", args));
            }
        }
        ToolInput::ExitPlanMode(_) => {
            buf.push_str("#### ExitPlanMode\n\n");
        }
        ToolInput::Generic { name, input } => {
            buf.push_str(&format!("#### {}\n\n", name));
            let json = serde_json::to_string_pretty(input).unwrap_or_else(|_| input.to_string());
            buf.push_str(&format!("```json\n{}\n```\n\n", json));
        }
    }
}

// ---------------------------------------------------------------------------
// Unified diff rendering (for markdown)
// ---------------------------------------------------------------------------

fn render_unified_diff_md(old: &str, new: &str) -> String {
    let diff = similar::TextDiff::from_lines(old, new);
    let mut buf = String::from("```diff\n");

    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Equal => {
                for line in change.value().lines() {
                    buf.push_str(&format!(" {}\n", line));
                }
            }
            similar::ChangeTag::Delete => {
                for line in change.value().lines() {
                    buf.push_str(&format!("-{}\n", line));
                }
            }
            similar::ChangeTag::Insert => {
                for line in change.value().lines() {
                    buf.push_str(&format!("+{}\n", line));
                }
            }
        }
    }
    buf.push_str("```\n");
    buf
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_ts_md(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.with_timezone(&chrono::Local).format("%H:%M:%S · %b %d").to_string()
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

fn extract_user_text(msg: &crate::model::content::Message) -> String {
    msg.content
        .iter()
        .filter_map(|c| match c {
            ContentItem::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_for_md(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

fn priority_label(p: &str) -> &str {
    match p {
        "high" => "high",
        "medium" => "medium",
        "low" => "low",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::aggregate;
    use crate::parser::parse_reader;
    use crate::session::build_session;
    use std::io::Cursor;

    fn render_fixture(detail: DetailLevel, compact: bool) -> String {
        let jsonl = crate::tests::fixture_linear_session();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        let agg = aggregate(&session);
        render_session(&session, &agg, detail, compact)
    }

    // -----------------------------------------------------------------------
    // Detail level tests
    // -----------------------------------------------------------------------

    #[test]
    fn full_detail_includes_everything() {
        let md = render_fixture(DetailLevel::Full, false);
        // Session header
        assert!(md.contains("# test-session"));
        // User messages
        assert!(md.contains("### User"));
        assert!(md.contains("Build the project"));
        // Assistant messages
        assert!(md.contains("### Assistant"));
        assert!(md.contains("I will build the project"));
        // Tool calls
        assert!(md.contains("Bash"));
        assert!(md.contains("cargo build"));
        // Tool results
        assert!(md.contains("Compiling weavr"));
    }

    #[test]
    fn high_detail_excludes_thinking_but_includes_tools() {
        let md = render_fixture(DetailLevel::High, false);
        assert!(md.contains("### User"));
        assert!(md.contains("### Assistant"));
        assert!(md.contains("Bash"));
        assert!(md.contains("cargo build"));
        // No thinking blocks
        assert!(!md.contains("<details>"));
    }

    #[test]
    fn low_detail_excludes_tools() {
        let md = render_fixture(DetailLevel::Low, false);
        assert!(md.contains("### User"));
        assert!(md.contains("### Assistant"));
        assert!(md.contains("I will build the project"));
        // No tool calls
        assert!(!md.contains("#### Bash"));
        assert!(!md.contains("cargo build"));
    }

    #[test]
    fn minimal_detail_assistant_text_only() {
        let md = render_fixture(DetailLevel::Minimal, false);
        assert!(md.contains("### User"));
        assert!(md.contains("### Assistant"));
        assert!(md.contains("I will build the project"));
        assert!(md.contains("Build completed successfully"));
        // No tool blocks
        assert!(!md.contains("#### Bash"));
    }

    #[test]
    fn user_only_detail_excludes_assistant() {
        let md = render_fixture(DetailLevel::UserOnly, false);
        assert!(md.contains("### User"));
        assert!(md.contains("Build the project"));
        // No assistant
        assert!(!md.contains("### Assistant"));
        assert!(!md.contains("I will build the project"));
    }

    // -----------------------------------------------------------------------
    // Compact mode tests
    // -----------------------------------------------------------------------

    #[test]
    fn compact_mode_strips_horizontal_rules_and_timestamps() {
        let full = render_fixture(DetailLevel::Full, false);
        let compact = render_fixture(DetailLevel::Full, true);

        // Full has horizontal rules
        assert!(full.contains("---"));
        // Compact should NOT have horizontal rules
        assert!(!compact.contains("---"));

        // Full has timestamps in header
        assert!(full.contains("**Date:**"));
        assert!(full.contains("**Time:**"));
        // Compact should NOT have timestamps
        assert!(!compact.contains("**Date:**"));
        assert!(!compact.contains("**Time:**"));

        // Full user entries have timestamps
        assert!(full.contains("("));
        // Compact user entries should NOT have timestamps
        // (compact mode omits the timestamp from headers)
        let compact_user_header = compact.lines().find(|l| l.starts_with("### User")).unwrap();
        assert!(
            !compact_user_header.contains('('),
            "compact user header should not have timestamp"
        );
    }

    #[test]
    fn ten_combinations_all_produce_output() {
        let levels = [
            DetailLevel::Full,
            DetailLevel::High,
            DetailLevel::Low,
            DetailLevel::Minimal,
            DetailLevel::UserOnly,
        ];
        for &detail in &levels {
            for &compact in &[false, true] {
                let md = render_fixture(detail, compact);
                assert!(!md.is_empty(), "empty output for {:?} compact={}", detail, compact);
                // Every output should at least have the session title
                assert!(
                    md.contains("# test-session"),
                    "missing title for {:?} compact={}",
                    detail,
                    compact
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Distinctness
    // -----------------------------------------------------------------------

    #[test]
    fn each_detail_level_produces_distinct_output() {
        // Use a richer fixture with thinking blocks so Full differs from High.
        let jsonl = fixture_with_thinking();
        let result = parse_reader(Cursor::new(&jsonl)).unwrap();
        let session = build_session(&result.entries);
        let agg = aggregate(&session);

        let full = render_session(&session, &agg, DetailLevel::Full, false);
        let high = render_session(&session, &agg, DetailLevel::High, false);
        let low = render_session(&session, &agg, DetailLevel::Low, false);
        let minimal = render_session(&session, &agg, DetailLevel::Minimal, false);
        let user_only = render_session(&session, &agg, DetailLevel::UserOnly, false);

        assert_ne!(full, high, "full and high should differ (thinking blocks)");
        assert_ne!(high, low, "high and low should differ (tool calls)");
        assert_ne!(low, minimal, "low and minimal should differ");
        assert_ne!(minimal, user_only, "minimal and user_only should differ (assistant text)");
    }

    fn fixture_with_thinking() -> String {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"test-session","message":{{"role":"user","content":[{{"type":"text","text":"Build the project"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"test-session","agentId":"claude-opus-4-7","message":{{"role":"assistant","model":"claude-opus-4-7","usage":{{"input_tokens":100,"output_tokens":50}},"content":[{{"type":"thinking","thinking":"Let me think about this carefully."}},{{"type":"text","text":"I will build the project."}},{{"type":"tool_use","id":"t1","name":"Bash","input":{{"command":"cargo build"}}}}]}}}}
"#
        )
    }

    // -----------------------------------------------------------------------
    // Tool rendering
    // -----------------------------------------------------------------------

    #[test]
    fn unified_diff_is_rendered() {
        let diff = render_unified_diff_md("old line\n", "new line\n");
        assert!(diff.contains("```diff"));
        assert!(diff.contains("-old line"));
        assert!(diff.contains("+new line"));
    }

    #[test]
    fn tool_use_in_markdown() {
        let mut buf = String::new();
        render_tool_use_md(
            "Bash",
            &serde_json::json!({"command": "ls -la", "description": "List files"}),
            &mut buf,
        );
        assert!(buf.contains("#### Bash — List files"));
        assert!(buf.contains("```bash"));
        assert!(buf.contains("ls -la"));
    }
}
