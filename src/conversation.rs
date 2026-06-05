//! Turn aggregation — groups flat transcript messages into conversational turns.
//!
//! A "turn" is either:
//! - A **user turn**: a single user text message.
//! - An **assistant turn**: one or more assistant messages (with their thinking,
//!   tool calls, and tool results) bracketed between user text messages.
//!
//! Turn boundary rule: a turn opens on the first assistant content following
//! a user *text* message; closes at the next user *text* message.
//! `tool_result`-only messages (synthetic user wrappers) do **not** close a turn.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::model::content::ContentItem;
use crate::model::entry::TranscriptEntry;
use crate::session::Session;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A thinking step extracted from an assistant message.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingStep {
    pub text: String,
}

/// A single tool call extracted from an assistant message.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// A sub-agent turn spawned via Task / Agent tool calls.
#[derive(Debug, Clone, PartialEq)]
pub struct SubAgentTurn {
    pub tool_call_id: String,
    pub name: String,
    pub thinking: Option<ThinkingStep>,
    pub tool_calls: Vec<ToolCall>,
    pub message_text: String,
}

/// A user turn — a single user text message.
#[derive(Debug, Clone, PartialEq)]
pub struct UserTurn {
    pub message: String,
    pub timestamp: DateTime<Utc>,
    /// Embedded images (base64 data) attached to the user message.
    pub images: Vec<crate::model::content::ImageSource>,
}

/// An assistant turn — groups assistant messages, their thinking,
/// tool calls, and sub-agents up to the next user text message.
#[derive(Debug, Clone, PartialEq)]
pub struct AssistantTurn {
    pub message_text: String,
    pub thinking: Option<ThinkingStep>,
    pub tool_calls: Vec<ToolCall>,
    pub sub_agents: Vec<SubAgentTurn>,
    pub timestamp: DateTime<Utc>,
    pub total_in: u64,
    pub total_out: u64,
    /// Embedded images (base64 data) attached to the assistant messages.
    pub images: Vec<crate::model::content::ImageSource>,
}

/// A grouped conversational turn.
#[derive(Debug, Clone, PartialEq)]
pub enum TurnGroup {
    User(UserTurn),
    Assistant(AssistantTurn),
}

/// A display item for the transcript — either a grouped conversational turn
/// or a standalone non-conversation entry (system, summary, hook, etc.).
#[derive(Debug, Clone)]
pub enum DisplayItem {
    Turn(TurnGroup),
    Entry(Box<TranscriptEntry>),
}

// ---------------------------------------------------------------------------
// v3 flat timeline types
// ---------------------------------------------------------------------------

/// The result of a tool execution.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

/// A single tool-call event in the flat timeline, carrying its optional result.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallEvent {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
    /// Paired result, or `None` if the tool result wasn't received.
    pub result: Option<ToolResult>,
}

/// A sub-agent spawn event (Task / Agent tool) in the flat timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct SubAgentEvent {
    pub tool_call_id: String,
    pub name: String,
    pub input: serde_json::Value,
    /// Result content returned by the sub-agent, if available.
    pub result: Option<String>,
}

/// A flat timeline event — the primary rendering model for v3.
///
/// Produced by [`flatten_to_timeline`]. Events appear in DFS (chronological)
/// order; thinking, tool calls, and assistant text are siblings, never nested.
#[derive(Debug, Clone, PartialEq)]
pub enum TimelineEvent {
    /// A user message (text + optional images).
    UserMessage(UserTurn),
    /// A block of assistant text.
    AssistantText {
        text: String,
        timestamp: DateTime<Utc>,
        images: Vec<crate::model::content::ImageSource>,
    },
    /// A thinking block.
    Thinking(ThinkingStep),
    /// A non-sub-agent tool call with its paired result.
    ToolCall(ToolCallEvent),
    /// A sub-agent invocation (Task / Agent tool).
    SubAgent(SubAgentEvent),
    /// Embedded images from an assistant message.
    Images(Vec<crate::model::content::ImageSource>),
}

// ---------------------------------------------------------------------------
// Tool names that spawn sub-agents
// ---------------------------------------------------------------------------

/// Tool names that indicate a sub-agent spawn.
const SUB_AGENT_TOOL_NAMES: &[&str] = &["Task", "Agent"];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Group flat messages from a session into conversational turns.
///
/// Messages are walked in DFS order from session roots. Each user text
/// message becomes a [`TurnGroup::User`]; each run of assistant messages
/// (plus any intervening `tool_result`-only user messages) becomes a
/// [`TurnGroup::Assistant`].
pub fn group_into_turns(session: &Session) -> Vec<TurnGroup> {
    let ordered = dfs_order(session);
    let mut turns: Vec<TurnGroup> = Vec::new();
    let mut i = 0;

    while i < ordered.len() {
        let node = &ordered[i];
        match &node.entry {
            TranscriptEntry::User(ue) if has_text_content(&ue.message) => {
                let text = extract_text_content(&ue.message);
                let images = extract_images(&ue.message);
                if !text.is_empty() || !images.is_empty() {
                    turns.push(TurnGroup::User(UserTurn {
                        message: text,
                        timestamp: ue.common.timestamp,
                        images,
                    }));
                }
                i += 1;
            }
            TranscriptEntry::User(ue) if is_tool_result_only(&ue.message) => {
                // tool_result-only user message without a preceding assistant —
                // skip (shouldn't happen in practice, but don't panic).
                i += 1;
            }
            TranscriptEntry::Assistant(_) => {
                let (turn, consumed) = build_assistant_turn(&ordered, i);
                turns.push(TurnGroup::Assistant(turn));
                i += consumed;
            }
            _ => {
                i += 1;
            }
        }
    }

    turns
}

/// Group a session into [`DisplayItem`]s — conversational turns for
/// User/Assistant messages plus standalone entries for system, summary,
/// hook, and other metadata entries.
///
/// Like [`group_into_turns`] but preserves non-conversation entries in
/// their original DFS position.
pub fn group_session(session: &Session) -> Vec<DisplayItem> {
    let ordered = dfs_order(session);
    let mut items: Vec<DisplayItem> = Vec::new();
    let mut i = 0;

    while i < ordered.len() {
        let node = &ordered[i];
        match &node.entry {
            TranscriptEntry::User(ue) if has_text_content(&ue.message) => {
                let text = extract_text_content(&ue.message);
                let images = extract_images(&ue.message);
                if !text.is_empty() || !images.is_empty() {
                    items.push(DisplayItem::Turn(TurnGroup::User(UserTurn {
                        message: text,
                        timestamp: ue.common.timestamp,
                        images,
                    })));
                }
                i += 1;
            }
            TranscriptEntry::User(ue) if is_tool_result_only(&ue.message) => {
                i += 1;
            }
            TranscriptEntry::Assistant(_) => {
                let (turn, consumed) = build_assistant_turn(&ordered, i);
                items.push(DisplayItem::Turn(TurnGroup::Assistant(turn)));
                i += consumed;
            }
            _ => {
                // Non-conversation entries (system, summary, hook, etc.).
                items.push(DisplayItem::Entry(Box::new(node.entry.clone())));
                i += 1;
            }
        }
    }

    items
}

/// Flatten a [`Session`] into a chronological `Vec<TimelineEvent>`.
///
/// Each content item (text, thinking, tool_use, image) within an assistant
/// message becomes a separate event, preserving the order they appear in the
/// original content array. Tool results are paired back to their
/// [`ToolCallEvent`] or [`SubAgentEvent`] by `tool_use_id`.
pub fn flatten_to_timeline(session: &Session) -> Vec<TimelineEvent> {
    let ordered = dfs_order(session);
    let mut events: Vec<TimelineEvent> = Vec::new();
    // Maps tool_use_id → index into `events` for result pairing.
    let mut tool_call_index: HashMap<String, usize> = HashMap::new();

    for node in &ordered {
        // Skip sidechain messages (sub-agent internal turns).
        if node.is_sidechain {
            continue;
        }
        match &node.entry {
            TranscriptEntry::User(ue) if ue.common.is_meta => {
                // Skip Claude Code meta/injected context messages.
            }
            TranscriptEntry::User(ue) if has_text_content(&ue.message) => {
                let text = extract_text_content(&ue.message);
                // Skip known meta text patterns even without the isMeta flag.
                if is_meta_text(&text) {
                    continue;
                }
                let images = extract_images(&ue.message);
                events.push(TimelineEvent::UserMessage(UserTurn {
                    message: text,
                    timestamp: ue.common.timestamp,
                    images,
                }));
            }
            TranscriptEntry::User(ue) if is_tool_result_only(&ue.message) => {
                for item in &ue.message.content {
                    if let ContentItem::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } = item
                    {
                        if let Some(&idx) = tool_call_index.get(tool_use_id) {
                            match &mut events[idx] {
                                TimelineEvent::ToolCall(tc) => {
                                    tc.result = Some(ToolResult {
                                        content: content.as_string(),
                                        is_error: is_error.unwrap_or(false),
                                    });
                                }
                                TimelineEvent::SubAgent(sa) => {
                                    sa.result = Some(content.as_string());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            TranscriptEntry::Assistant(ae) => {
                let timestamp = ae.common.timestamp;
                let mut pending_images: Vec<crate::model::content::ImageSource> = Vec::new();

                for item in &ae.message.content {
                    match item {
                        ContentItem::Image { source } => {
                            pending_images.push(source.clone());
                        }
                        other => {
                            // Flush accumulated images before any non-image item.
                            if !pending_images.is_empty() {
                                events.push(TimelineEvent::Images(std::mem::take(
                                    &mut pending_images,
                                )));
                            }
                            match other {
                                ContentItem::Text { text } => {
                                    let trimmed = text.trim();
                                    if !trimmed.is_empty() {
                                        events.push(TimelineEvent::AssistantText {
                                            text: trimmed.to_string(),
                                            timestamp,
                                            images: vec![],
                                        });
                                    }
                                }
                                ContentItem::Thinking { thinking, .. } => {
                                    events.push(TimelineEvent::Thinking(ThinkingStep {
                                        text: thinking.clone(),
                                    }));
                                }
                                ContentItem::ToolUse { id, name, input } => {
                                    let idx = events.len();
                                    if SUB_AGENT_TOOL_NAMES.contains(&name.as_str()) {
                                        events.push(TimelineEvent::SubAgent(SubAgentEvent {
                                            tool_call_id: id.clone(),
                                            name: name.clone(),
                                            input: input.clone(),
                                            result: None,
                                        }));
                                    } else {
                                        events.push(TimelineEvent::ToolCall(ToolCallEvent {
                                            id: id.clone(),
                                            name: name.clone(),
                                            input: input.clone(),
                                            result: None,
                                        }));
                                    }
                                    tool_call_index.insert(id.clone(), idx);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Flush any trailing images with no following text.
                if !pending_images.is_empty() {
                    events.push(TimelineEvent::Images(pending_images));
                }
            }
            _ => {
                // System, summary, hook — not part of the conversation timeline.
            }
        }
    }

    events
}

// ---------------------------------------------------------------------------
// DFS ordering
// ---------------------------------------------------------------------------

fn dfs_order(session: &Session) -> Vec<&crate::session::MessageNode> {
    let mut result: Vec<&crate::session::MessageNode> = Vec::new();
    let mut visited: HashMap<Uuid, bool> = HashMap::new();

    for root_id in &session.root_message_ids {
        dfs_collect(*root_id, session, &mut visited, &mut result);
    }

    result
}

fn dfs_collect<'s>(
    uuid: Uuid,
    session: &'s Session,
    visited: &mut HashMap<Uuid, bool>,
    result: &mut Vec<&'s crate::session::MessageNode>,
) {
    if visited.contains_key(&uuid) {
        return;
    }
    visited.insert(uuid, true);

    if let Some(node) = session.messages.get(&uuid) {
        result.push(node);

        for child_id in &node.children {
            dfs_collect(*child_id, session, visited, result);
        }
    }
}

// ---------------------------------------------------------------------------
// Turn building
// ---------------------------------------------------------------------------

fn build_assistant_turn(
    ordered: &[&crate::session::MessageNode],
    start: usize,
) -> (AssistantTurn, usize) {
    let mut message_parts: Vec<String> = Vec::new();
    let mut thinking: Option<ThinkingStep> = None;
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut sub_agents: Vec<SubAgentTurn> = Vec::new();
    let mut images: Vec<crate::model::content::ImageSource> = Vec::new();
    let mut timestamp: Option<DateTime<Utc>> = None;
    let mut total_in: u64 = 0;
    let mut total_out: u64 = 0;
    let mut consumed: usize = 0;
    let mut pending_tool_calls: Vec<ToolCall> = Vec::new();

    let mut i = start;
    while i < ordered.len() {
        let node = ordered[i];
        match &node.entry {
            TranscriptEntry::Assistant(ae) => {
                if timestamp.is_none() {
                    timestamp = Some(ae.common.timestamp);
                }

                // Accumulate tokens.
                if let Some(ref usage) = ae.message.usage {
                    let in_tok = usage.input_tokens.unwrap_or(0)
                        + usage.cache_read_input_tokens.unwrap_or(0)
                        + usage.cache_creation_input_tokens.unwrap_or(0);
                    total_in += in_tok;
                    total_out += usage.output_tokens.unwrap_or(0);
                }

                // Process content items.
                for item in &ae.message.content {
                    match item {
                        ContentItem::Text { text } => {
                            let t = text.trim();
                            if !t.is_empty() {
                                message_parts.push(t.to_string());
                            }
                        }
                        ContentItem::Thinking { thinking: th, .. } => {
                            // Keep the first thinking block only.
                            if thinking.is_none() {
                                thinking = Some(ThinkingStep { text: th.clone() });
                            }
                        }
                        ContentItem::ToolUse { id, name, input } => {
                            let tc = ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                input: input.clone(),
                            };
                            if SUB_AGENT_TOOL_NAMES.contains(&name.as_str()) {
                                pending_tool_calls.push(tc);
                            } else {
                                tool_calls.push(tc);
                            }
                        }
                        ContentItem::ToolResult { .. } => {
                            // Tool results within assistant messages are
                            // rare; they're handled in user entries below.
                        }
                        ContentItem::Image { source } => {
                            images.push(source.clone());
                        }
                    }
                }

                consumed += 1;
                i += 1;
            }
            TranscriptEntry::User(ue) if is_tool_result_only(&ue.message) => {
                // Tool results belong to the current assistant turn.
                // Try to match them with pending sub-agent tool calls.
                for item in &ue.message.content {
                    if let ContentItem::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } = item
                    {
                        // Check if this result completes a sub-agent tool call.
                        if let Some(pos) =
                            pending_tool_calls.iter().position(|tc| tc.id == *tool_use_id)
                        {
                            let tc = pending_tool_calls.remove(pos);
                            let sub = build_sub_agent(&tc, &content.as_string());
                            sub_agents.push(sub);
                        }
                        // Other tool results are just attached data; we don't
                        // need to store them separately in the turn model.
                    }
                }
                consumed += 1;
                i += 1;
            }
            TranscriptEntry::User(ue) if has_text_content(&ue.message) => {
                // Next user text — close the assistant turn.
                break;
            }
            _ => {
                // Skip non-user, non-assistant entries (system, summary, etc.)
                consumed += 1;
                i += 1;
            }
        }
    }

    // Any remaining pending tool calls without results become sub-agents
    // with minimal info (the result wasn't in the same turn).
    for tc in pending_tool_calls {
        sub_agents.push(SubAgentTurn {
            tool_call_id: tc.id.clone(),
            name: tc.name.clone(),
            thinking: None,
            tool_calls: Vec::new(),
            message_text: String::new(),
        });
    }

    // Ensure we always have a timestamp.
    let timestamp = timestamp.unwrap_or_else(|| {
        // Fallback — shouldn't happen if there's at least one assistant message.
        Utc::now()
    });

    (
        AssistantTurn {
            message_text: message_parts.join("\n"),
            thinking,
            tool_calls,
            sub_agents,
            timestamp,
            total_in,
            total_out,
            images,
        },
        consumed,
    )
}

// ---------------------------------------------------------------------------
// Sub-agent detection
// ---------------------------------------------------------------------------

fn build_sub_agent(tc: &ToolCall, result_content: &str) -> SubAgentTurn {
    // Try to extract sub-agent information from the tool result content.
    // Tool results for Task/Agent typically contain the sub-agent's output
    // as text, potentially with structured content.
    //
    // For now, parse the result content to extract text and look for
    // nested thinking / tool call patterns in the result string.
    let message_text = extract_agent_result_text(result_content);

    SubAgentTurn {
        tool_call_id: tc.id.clone(),
        name: tc.name.clone(),
        thinking: None,
        tool_calls: Vec::new(),
        message_text,
    }
}

/// Extract the meaningful text from a Task/Agent tool result.
///
/// Task tool results often contain structured output that includes the
/// sub-agent's response. We attempt to extract the final text portion.
fn extract_agent_result_text(result: &str) -> String {
    // The result is typically the sub-agent's output. Keep it as-is
    // but trim excessive whitespace.
    let trimmed = result.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // If the result is very long, it likely contains the full sub-agent
    // transcript. Take a reasonable portion.
    if trimmed.len() > 10_000 {
        trimmed.chars().take(10_000).collect::<String>() + "\u{2026}"
    } else {
        trimmed.to_string()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` for known Claude Code meta message text patterns that should
/// be hidden from the timeline even when `isMeta` is absent.
///
/// Patterns:
/// - "Caveat: … local commands" injected context blocks
/// - "/model …" and other slash-command echoes (plain text starting with `/`)
/// - `<command-name>…</command-name>` XML-wrapped slash commands
/// - `<local-command-stdout>…</local-command-stdout>` local command output
/// - "Set model to …" acknowledgements (plain or XML-wrapped)
fn is_meta_text(text: &str) -> bool {
    let trimmed = text.trim();
    // Plain slash-command echoes: "/model gpt-4", "/help", etc.
    if trimmed.starts_with('/') {
        return true;
    }
    // Plain "Set model to …" acknowledgements.
    if trimmed.starts_with("Set model to ") {
        return true;
    }
    // "Caveat: … local commands" injected blocks.
    if trimmed.starts_with("Caveat:") && trimmed.contains("local commands") {
        return true;
    }
    // XML-wrapped slash-command echoes: `<command-name>/model</command-name> …`
    if trimmed.contains("<command-name>") {
        return true;
    }
    // Local command stdout wrapper: `<local-command-stdout>…</local-command-stdout>`
    if trimmed.contains("<local-command-stdout>") {
        return true;
    }
    false
}

fn has_text_content(msg: &crate::model::content::Message) -> bool {
    msg.content.iter().any(|c| match c {
        ContentItem::Text { text } => !text.trim().is_empty(),
        ContentItem::Image { .. } => true,
        _ => false,
    })
}

fn extract_images(msg: &crate::model::content::Message) -> Vec<crate::model::content::ImageSource> {
    msg.content
        .iter()
        .filter_map(
            |c| {
                if let ContentItem::Image { source } = c {
                    Some(source.clone())
                } else {
                    None
                }
            },
        )
        .collect()
}

fn is_tool_result_only(msg: &crate::model::content::Message) -> bool {
    !msg.content.is_empty()
        && msg.content.iter().all(|c| matches!(c, ContentItem::ToolResult { .. }))
}

fn extract_text_content(msg: &crate::model::content::Message) -> String {
    msg.content
        .iter()
        .filter_map(|c| match c {
            ContentItem::Text { text } => {
                let t = text.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_reader;
    use crate::session::build_session;
    use std::io::Cursor;

    fn parse_and_group(jsonl: &str) -> Vec<TurnGroup> {
        let result = parse_reader(Cursor::new(jsonl)).unwrap();
        let session = build_session(&result.entries);
        group_into_turns(&session)
    }

    fn parse_and_flatten(jsonl: &str) -> Vec<TimelineEvent> {
        let result = parse_reader(Cursor::new(jsonl)).unwrap();
        let session = build_session(&result.entries);
        flatten_to_timeline(&session)
    }

    // -----------------------------------------------------------------------
    // T1 tests — flat timeline
    // -----------------------------------------------------------------------

    #[test]
    fn timeline_user_then_assistant_text_in_order() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hello"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"hi there"}}]}}}}"#
        );
        let events = parse_and_flatten(&jsonl);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TimelineEvent::UserMessage(_)));
        assert!(matches!(events[1], TimelineEvent::AssistantText { .. }));
        if let TimelineEvent::UserMessage(ref ut) = events[0] {
            assert_eq!(ut.message, "hello");
        }
        if let TimelineEvent::AssistantText { ref text, .. } = events[1] {
            assert_eq!(text, "hi there");
        }
    }

    #[test]
    fn timeline_thinking_is_sibling_not_nested() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"think"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"thinking","thinking":"deep thought"}},{{"type":"text","text":"result"}}]}}}}"#
        );
        let events = parse_and_flatten(&jsonl);
        // UserMessage, Thinking, AssistantText — 3 siblings
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], TimelineEvent::UserMessage(_)));
        assert!(matches!(events[1], TimelineEvent::Thinking(_)));
        assert!(matches!(events[2], TimelineEvent::AssistantText { .. }));
        if let TimelineEvent::Thinking(ref ts) = events[1] {
            assert_eq!(ts.text, "deep thought");
        }
    }

    #[test]
    fn timeline_tool_call_paired_with_result() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let tr1 = "550e8400-e29b-41d4-a716-446655440010";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"run"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"b1","name":"Bash","input":{{"command":"ls"}}}}]}}}}
{{"type":"user","uuid":"{tr1}","parentUuid":"{a1}","timestamp":"2025-06-15T10:30:06Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"b1","content":"file1\nfile2","is_error":false}}]}}}}"#
        );
        let events = parse_and_flatten(&jsonl);
        // UserMessage + ToolCall (no trailing user text)
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TimelineEvent::UserMessage(_)));
        if let TimelineEvent::ToolCall(ref tc) = events[1] {
            assert_eq!(tc.id, "b1");
            assert_eq!(tc.name, "Bash");
            assert!(tc.result.is_some());
            let result = tc.result.as_ref().unwrap();
            assert_eq!(result.content, "file1\nfile2");
            assert!(!result.is_error);
        } else {
            panic!("expected ToolCall at index 1, got {:?}", events[1]);
        }
    }

    #[test]
    fn timeline_sub_agent_is_separate_event() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let tr1 = "550e8400-e29b-41d4-a716-446655440010";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"delegate"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"task1","name":"Task","input":{{"description":"do work","prompt":"find files"}}}}]}}}}
{{"type":"user","uuid":"{tr1}","parentUuid":"{a1}","timestamp":"2025-06-15T10:30:10Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"task1","content":"Done: found 3 files","is_error":false}}]}}}}"#
        );
        let events = parse_and_flatten(&jsonl);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TimelineEvent::UserMessage(_)));
        if let TimelineEvent::SubAgent(ref sa) = events[1] {
            assert_eq!(sa.tool_call_id, "task1");
            assert_eq!(sa.name, "Task");
            assert_eq!(sa.result.as_deref(), Some("Done: found 3 files"));
        } else {
            panic!("expected SubAgent at index 1, got {:?}", events[1]);
        }
    }

    #[test]
    fn timeline_event_order_thinking_tool_text() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"go"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"thinking","thinking":"plan"}},{{"type":"tool_use","id":"b1","name":"Bash","input":{{"command":"echo hi"}}}},{{"type":"text","text":"done"}}]}}}}"#
        );
        let events = parse_and_flatten(&jsonl);
        // UserMessage, Thinking, ToolCall, AssistantText = 4
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], TimelineEvent::UserMessage(_)));
        assert!(matches!(events[1], TimelineEvent::Thinking(_)));
        assert!(matches!(events[2], TimelineEvent::ToolCall(_)));
        assert!(matches!(events[3], TimelineEvent::AssistantText { .. }));
    }

    #[test]
    fn timeline_tool_without_result_has_none_result() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"go"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"b1","name":"Bash","input":{{"command":"sleep 1"}}}}]}}}}"#
        );
        let events = parse_and_flatten(&jsonl);
        assert_eq!(events.len(), 2);
        if let TimelineEvent::ToolCall(ref tc) = events[1] {
            assert!(tc.result.is_none(), "no tool_result message → result should be None");
        } else {
            panic!("expected ToolCall");
        }
    }

    #[test]
    fn timeline_empty_session_produces_no_events() {
        let events = parse_and_flatten("");
        assert!(events.is_empty());
    }

    // -----------------------------------------------------------------------
    // A1 tests
    // -----------------------------------------------------------------------

    #[test]
    fn single_user_single_assistant_produces_two_turns() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Hello!"}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 2);
        assert!(matches!(turns[0], TurnGroup::User(_)));
        assert!(matches!(turns[1], TurnGroup::Assistant(_)));

        if let TurnGroup::User(ref ut) = turns[0] {
            assert_eq!(ut.message, "hi");
        }
        if let TurnGroup::Assistant(ref at) = turns[1] {
            assert_eq!(at.message_text, "Hello!");
        }
    }

    #[test]
    fn assistant_with_thinking_and_tools_grouped_into_one_turn() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"build"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"I will build."}},{{"type":"thinking","thinking":"Let me run cargo build"}},{{"type":"tool_use","id":"t1","name":"Bash","input":{{"command":"cargo build"}}}},{{"type":"tool_use","id":"t2","name":"Read","input":{{"filePath":"Cargo.toml"}}}},{{"type":"tool_use","id":"t3","name":"Write","input":{{"filePath":"out.txt","content":"done"}}}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 2);
        assert!(matches!(turns[0], TurnGroup::User(_)));
        assert!(matches!(turns[1], TurnGroup::Assistant(_)));

        if let TurnGroup::Assistant(ref at) = turns[1] {
            assert_eq!(at.message_text, "I will build.");
            assert!(at.thinking.is_some());
            assert_eq!(at.thinking.as_ref().unwrap().text, "Let me run cargo build");
            assert_eq!(at.tool_calls.len(), 3);
            assert_eq!(at.tool_calls[0].name, "Bash");
            assert_eq!(at.tool_calls[1].name, "Read");
            assert_eq!(at.tool_calls[2].name, "Write");
        }
    }

    #[test]
    fn assistant_with_tool_results_still_one_turn() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let tr1 = "550e8400-e29b-41d4-a716-446655440010";
        let tr2 = "550e8400-e29b-41d4-a716-446655440011";
        let tr3 = "550e8400-e29b-41d4-a716-446655440012";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"list files"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Running ls."}},{{"type":"tool_use","id":"t1","name":"Bash","input":{{"command":"ls"}}}},{{"type":"tool_use","id":"t2","name":"Read","input":{{"filePath":"a.txt"}}}},{{"type":"tool_use","id":"t3","name":"Write","input":{{"filePath":"b.txt","content":"x"}}}}]}}}}
{{"type":"user","uuid":"{tr1}","parentUuid":"{a1}","timestamp":"2025-06-15T10:30:06Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"t1","content":"file1\nfile2","is_error":false}}]}}}}
{{"type":"user","uuid":"{tr2}","parentUuid":"{tr1}","timestamp":"2025-06-15T10:30:07Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"t2","content":"content a","is_error":false}}]}}}}
{{"type":"user","uuid":"{tr3}","parentUuid":"{tr2}","timestamp":"2025-06-15T10:30:08Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"t3","content":"done","is_error":false}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 2);
        assert!(matches!(turns[0], TurnGroup::User(_)));
        assert!(matches!(turns[1], TurnGroup::Assistant(_)));

        if let TurnGroup::Assistant(ref at) = turns[1] {
            assert_eq!(at.tool_calls.len(), 3, "should have all 3 tool calls in one turn");
        }
    }

    #[test]
    fn user_assistant_user_assistant_produces_four_turns() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let u2 = "550e8400-e29b-41d4-a716-446655440003";
        let a2 = "550e8400-e29b-41d4-a716-446655440004";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Hello!"}}]}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"bye"}}]}}}}
{{"type":"assistant","uuid":"{a2}","parentUuid":"{u2}","timestamp":"2025-06-15T10:31:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Goodbye!"}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 4, "should have User, Assistant, User, Assistant = 4 turns");
        assert!(matches!(turns[0], TurnGroup::User(_)));
        assert!(matches!(turns[1], TurnGroup::Assistant(_)));
        assert!(matches!(turns[2], TurnGroup::User(_)));
        assert!(matches!(turns[3], TurnGroup::Assistant(_)));
    }

    #[test]
    fn assistant_turn_aggregates_token_counts() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let a2 = "550e8400-e29b-41d4-a716-446655440003";
        let u2 = "550e8400-e29b-41d4-a716-446655440004";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Part 1"}}],"usage":{{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":20,"cache_read_input_tokens":10}}}}}}
{{"type":"assistant","uuid":"{a2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:30:06Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Part 2"}}],"usage":{{"input_tokens":60,"output_tokens":30}}}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{a2}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"bye"}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 3, "User, Assistant, User");

        if let TurnGroup::Assistant(ref at) = turns[1] {
            // total_in = (100 + 20 + 10) + (60 + 0 + 0) = 130 + 60 = 190
            assert_eq!(
                at.total_in, 190,
                "total_in should sum input + cache tokens from both messages"
            );
            // total_out = 50 + 30 = 80
            assert_eq!(at.total_out, 80, "total_out should sum output tokens from both messages");
        } else {
            panic!("expected Assistant turn");
        }
    }

    #[test]
    fn empty_session_produces_no_turns() {
        let jsonl = "";
        let turns = parse_and_group(jsonl);
        assert!(turns.is_empty());
    }

    // -----------------------------------------------------------------------
    // A2 tests
    // -----------------------------------------------------------------------

    #[test]
    fn task_tool_use_creates_sub_agent() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let tr1 = "550e8400-e29b-41d4-a716-446655440010";
        let u2 = "550e8400-e29b-41d4-a716-446655440003";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"search the code"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"I will search."}},{{"type":"tool_use","id":"task1","name":"Task","input":{{"description":"search for files","prompt":"find *.rs"}}}}]}}}}
{{"type":"user","uuid":"{tr1}","parentUuid":"{a1}","timestamp":"2025-06-15T10:30:06Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"task1","content":"Found: src/main.rs, src/lib.rs","is_error":false}}]}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{tr1}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"thanks"}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 3);

        if let TurnGroup::Assistant(ref at) = turns[1] {
            assert_eq!(at.sub_agents.len(), 1, "should have one sub-agent from Task tool");
            assert_eq!(
                at.sub_agents[0].tool_call_id, "task1",
                "sub-agent should reference the Task tool call id"
            );
            assert_eq!(at.sub_agents[0].name, "Task");
            assert!(
                at.sub_agents[0].message_text.contains("Found:"),
                "sub-agent message should contain result text"
            );
            // Non-Task tool calls should remain empty.
            assert_eq!(at.tool_calls.len(), 0);
        } else {
            panic!("expected Assistant turn at index 1");
        }
    }

    #[test]
    fn non_task_tool_uses_become_regular_tool_calls() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let u2 = "550e8400-e29b-41d4-a716-446655440003";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"run ls"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Running."}},{{"type":"tool_use","id":"b1","name":"Bash","input":{{"command":"ls"}}}}]}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"ok"}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 3);

        if let TurnGroup::Assistant(ref at) = turns[1] {
            assert_eq!(at.tool_calls.len(), 1);
            assert_eq!(at.tool_calls[0].name, "Bash");
            assert_eq!(at.sub_agents.len(), 0, "non-Task tools should NOT become sub-agents");
        }
    }

    #[test]
    fn sub_agent_without_tool_result_still_tracked() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let u2 = "550e8400-e29b-41d4-a716-446655440003";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"search"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Delegating."}},{{"type":"tool_use","id":"task1","name":"Task","input":{{"description":"search"}}}},{{"type":"tool_use","id":"b1","name":"Bash","input":{{"command":"ls"}}}}]}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"ok"}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 3);

        if let TurnGroup::Assistant(ref at) = turns[1] {
            // Bash tool call is regular.
            assert_eq!(at.tool_calls.len(), 1);
            assert_eq!(at.tool_calls[0].name, "Bash");
            // Task tool call without result → still tracked as sub-agent.
            assert_eq!(at.sub_agents.len(), 1);
            assert_eq!(at.sub_agents[0].name, "Task");
            assert_eq!(at.sub_agents[0].tool_call_id, "task1");
            assert!(at.sub_agents[0].message_text.is_empty());
        }
    }

    #[test]
    fn multiple_assistant_messages_merged_into_one_turn() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let a2 = "550e8400-e29b-41d4-a716-446655440003";
        let u2 = "550e8400-e29b-41d4-a716-446655440004";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"First."}}],"usage":{{"input_tokens":10,"output_tokens":5}}}}}}
{{"type":"assistant","uuid":"{a2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:30:06Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Second."}}],"usage":{{"input_tokens":8,"output_tokens":4}}}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{a2}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"bye"}}]}}}}"#
        );
        let turns = parse_and_group(&jsonl);
        assert_eq!(turns.len(), 3);

        if let TurnGroup::Assistant(ref at) = turns[1] {
            assert_eq!(at.message_text, "First.\nSecond.");
            assert_eq!(at.total_in, 18);
            assert_eq!(at.total_out, 9);
        }
    }

    // -----------------------------------------------------------------------
    // T19 tests — meta-message filtering
    // -----------------------------------------------------------------------

    /// Helper: build a minimal JSONL user message line with optional isMeta flag.
    fn user_line(uuid: &str, parent: Option<&str>, text: &str, is_meta: bool) -> String {
        let parent_field = match parent {
            Some(p) => format!(r#","parentUuid":"{}""#, p),
            None => String::new(),
        };
        let meta_field = if is_meta { r#","isMeta":true"# } else { "" };
        format!(
            r#"{{"type":"user","uuid":"{uuid}","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z"{parent_field}{meta_field},"message":{{"role":"user","content":[{{"type":"text","text":{text_json}}}]}}}}"#,
            uuid = uuid,
            parent_field = parent_field,
            meta_field = meta_field,
            text_json = serde_json::to_string(text).unwrap(),
        )
    }

    fn assistant_line(uuid: &str, parent: &str, text: &str) -> String {
        format!(
            r#"{{"type":"assistant","uuid":"{uuid}","parentUuid":"{parent}","sessionId":"s1","timestamp":"2025-01-01T00:01:00Z","message":{{"role":"assistant","content":[{{"type":"text","text":{text_json}}}]}}}}"#,
            uuid = uuid,
            parent = parent,
            text_json = serde_json::to_string(text).unwrap(),
        )
    }

    #[test]
    fn meta_flag_drops_user_message() {
        let u1 = "550e8400-e29b-41d4-a716-000000000001";
        let a1 = "550e8400-e29b-41d4-a716-000000000002";
        let u2 = "550e8400-e29b-41d4-a716-000000000003";
        let a2 = "550e8400-e29b-41d4-a716-000000000004";
        // First pair: isMeta user + real assistant
        // Second pair: real user + real assistant
        let jsonl = format!(
            "{}\n{}\n{}\n{}",
            user_line(u1, None, "Invoke the skill.", true),
            assistant_line(a1, u1, "Sure, here is the result."),
            user_line(u2, Some(a1), "What is the capital of France?", false),
            assistant_line(a2, u2, "Paris."),
        );
        let events = parse_and_flatten(&jsonl);
        // The isMeta user message must be dropped; the real prompt must survive.
        let user_texts: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let TimelineEvent::UserMessage(ut) = e {
                    Some(ut.message.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            user_texts,
            vec!["What is the capital of France?"],
            "isMeta user must be dropped"
        );
    }

    #[test]
    fn slash_command_echo_dropped() {
        let u1 = "550e8400-e29b-41d4-a716-000000000011";
        let a1 = "550e8400-e29b-41d4-a716-000000000012";
        let u2 = "550e8400-e29b-41d4-a716-000000000013";
        let a2 = "550e8400-e29b-41d4-a716-000000000014";
        let jsonl = format!(
            "{}\n{}\n{}\n{}",
            user_line(u1, None, "/model claude-opus-4-8", false),
            assistant_line(a1, u1, "Set model to claude-opus-4-8"),
            user_line(u2, Some(a1), "Hello, what can you do?", false),
            assistant_line(a2, u2, "I can help with many things."),
        );
        let events = parse_and_flatten(&jsonl);
        let user_texts: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let TimelineEvent::UserMessage(ut) = e {
                    Some(ut.message.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            user_texts,
            vec!["Hello, what can you do?"],
            "/model slash-command echo must be dropped"
        );
    }

    #[test]
    fn caveat_local_commands_dropped() {
        let u1 = "550e8400-e29b-41d4-a716-000000000021";
        let a1 = "550e8400-e29b-41d4-a716-000000000022";
        let u2 = "550e8400-e29b-41d4-a716-000000000023";
        let a2 = "550e8400-e29b-41d4-a716-000000000024";
        let caveat =
            "Caveat: the following are local commands that may be available in this context.";
        let jsonl = format!(
            "{}\n{}\n{}\n{}",
            user_line(u1, None, caveat, false),
            assistant_line(a1, u1, "Understood."),
            user_line(u2, Some(a1), "Run tests please.", false),
            assistant_line(a2, u2, "Running tests."),
        );
        let events = parse_and_flatten(&jsonl);
        let user_texts: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let TimelineEvent::UserMessage(ut) = e {
                    Some(ut.message.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            user_texts,
            vec!["Run tests please."],
            "Caveat/local-commands block must be dropped"
        );
    }

    #[test]
    fn set_model_text_dropped() {
        let u1 = "550e8400-e29b-41d4-a716-000000000031";
        // Stand-alone "Set model to …" as a user message (some transcripts have it this way)
        let jsonl = user_line(u1, None, "Set model to claude-sonnet-4-6", false);
        let events = parse_and_flatten(&jsonl);
        assert!(events.is_empty(), "\"Set model to …\" user message must be filtered out");
    }

    #[test]
    fn xml_command_name_dropped() {
        let u1 = "550e8400-e29b-41d4-a716-000000000051";
        let text = "<command-name>/model</command-name>\n<command-message>model</command-message>\n<command-args>opus</command-args>";
        let jsonl = user_line(u1, None, text, false);
        let events = parse_and_flatten(&jsonl);
        assert!(events.is_empty(), "XML-wrapped slash command must be filtered out");
    }

    #[test]
    fn xml_local_command_stdout_dropped() {
        let u1 = "550e8400-e29b-41d4-a716-000000000061";
        let text = "<local-command-stdout>Set model to claude-opus-4-7</local-command-stdout>";
        let jsonl = user_line(u1, None, text, false);
        let events = parse_and_flatten(&jsonl);
        assert!(events.is_empty(), "local-command-stdout wrapper must be filtered out");
    }

    #[test]
    fn real_prompt_survives_filtering() {
        let u1 = "550e8400-e29b-41d4-a716-000000000041";
        let a1 = "550e8400-e29b-41d4-a716-000000000042";
        let jsonl = format!(
            "{}\n{}",
            user_line(u1, None, "Explain how Rust lifetimes work.", false),
            assistant_line(a1, u1, "Lifetimes ensure references are valid."),
        );
        let events = parse_and_flatten(&jsonl);
        assert_eq!(events.len(), 2, "real user + assistant must both appear");
        assert!(matches!(events[0], TimelineEvent::UserMessage(_)));
        assert!(matches!(events[1], TimelineEvent::AssistantText { .. }));
    }

    #[test]
    fn tool_result_only_user_absorbed_into_assistant_turn() {
        let jsonl = crate::tests::fixture_linear_session();
        let turns = parse_and_group(&jsonl);
        // fixture: User → Assistant+Bash → tool_result user → Assistant
        // tool_result-only and second assistant are both absorbed → 2 turns.
        assert_eq!(turns.len(), 2, "User + merged Assistant = 2 turns");
        assert!(matches!(turns[0], TurnGroup::User(_)));
        assert!(matches!(turns[1], TurnGroup::Assistant(_)));

        if let TurnGroup::Assistant(ref at) = turns[1] {
            assert_eq!(at.tool_calls.len(), 1);
            assert_eq!(at.tool_calls[0].name, "Bash");
            assert!(
                at.message_text.contains("Build completed successfully!"),
                "second assistant text should be merged in"
            );
            assert!(
                at.message_text.contains("I will build the project."),
                "first assistant text should be merged in"
            );
        }
    }
}
