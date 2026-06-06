//! Per-session aggregation.
//!
//! Computes from a [`Session`]:
//! - Token totals (input, output, cache-creation, cache-read)
//! - Message count
//! - First / last timestamp
//! - Derived `is_active` (last_timestamp within 10 minutes)
//! - Tool-usage counts by `tool_use.name`
//! - Virtual file tree from `Read`/`Write`/`Edit`/`MultiEdit`/`Glob` `filePath`s

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Duration, Utc};

use crate::model::content::{ContentItem, Message};
use crate::model::entry::TranscriptEntry;
use crate::session::Session;

/// Aggregated statistics for a session.
#[derive(Debug, Clone, Default)]
pub struct SessionAggregate {
    /// Session identifier.
    pub session_id: String,
    /// Total input / prompt tokens.
    pub total_input_tokens: u64,
    /// Total output / completion tokens.
    pub total_output_tokens: u64,
    /// Total cache-creation tokens.
    pub total_cache_creation_tokens: u64,
    /// Total cache-read tokens.
    pub total_cache_read_tokens: u64,
    /// Number of messages (user + assistant entries only).
    pub message_count: usize,
    /// Earliest timestamp in the session.
    pub first_timestamp: Option<DateTime<Utc>>,
    /// Latest timestamp in the session.
    pub last_timestamp: Option<DateTime<Utc>>,
    /// Whether the session appears active (last message within 10 min of now).
    pub is_active: bool,
    /// Tool usage counts: tool name → number of invocations.
    pub tool_counts: HashMap<String, usize>,
    /// Virtual file tree: directory path → set of file names.
    pub file_tree: BTreeMap<String, Vec<String>>,
    /// Summary titles extracted from SummaryEntry entries.
    pub summaries: Vec<String>,
}

/// Compute aggregated stats for a [`Session`].
pub fn aggregate(session: &Session) -> SessionAggregate {
    let mut agg = SessionAggregate {
        session_id: session.id.clone(),
        ..Default::default()
    };

    let mut timestamps: Vec<DateTime<Utc>> = Vec::new();

    for node in session.messages.values() {
        let entry = &node.entry;

        // Extract timestamp — handle Unknown entries via raw JSON.
        let ts = match entry {
            TranscriptEntry::Unknown { raw, .. } => raw
                .get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            other => Some(other.common().timestamp),
        };
        if let Some(t) = ts {
            timestamps.push(t);
        }

        match entry {
            TranscriptEntry::User(ue) => {
                agg.message_count += 1;
                accumulate_tokens(&ue.message, &mut agg);
                count_tools_and_files(&ue.message.content, &mut agg);
            }
            TranscriptEntry::Assistant(ae) => {
                agg.message_count += 1;
                accumulate_tokens(&ae.message, &mut agg);
                count_tools_and_files(&ae.message.content, &mut agg);
            }
            TranscriptEntry::Summary(se) => {
                if let Some(ref title) = se.title {
                    agg.summaries.push(title.clone());
                }
            }
            _ => {}
        }
    }

    timestamps.sort();

    agg.first_timestamp = timestamps.first().copied();
    agg.last_timestamp = timestamps.last().copied();

    // A session is "active" if its last timestamp is within 10 minutes.
    if let Some(last) = agg.last_timestamp {
        let now = Utc::now();
        agg.is_active = now.signed_duration_since(last) < Duration::minutes(10);
    }

    agg
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Accumulate token usage from a single message into the aggregate.
fn accumulate_tokens(message: &Message, agg: &mut SessionAggregate) {
    if let Some(ref usage) = message.usage {
        agg.total_input_tokens += usage.input_tokens.unwrap_or(0);
        agg.total_output_tokens += usage.output_tokens.unwrap_or(0);
        agg.total_cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
        agg.total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
    }
}

/// Count tool uses in content items and extract file paths into the tree.
fn count_tools_and_files(content: &[ContentItem], agg: &mut SessionAggregate) {
    for item in content {
        if let ContentItem::ToolUse { name, input, .. } = item {
            *agg.tool_counts.entry(name.clone()).or_default() += 1;
            extract_file_paths(name, input, &mut agg.file_tree);
        }
    }
}

/// Extract file paths from tool inputs and add them to the virtual file tree.
fn extract_file_paths(
    tool_name: &str,
    input: &serde_json::Value,
    tree: &mut BTreeMap<String, Vec<String>>,
) {
    let file_path = match tool_name {
        "Read" | "Write" | "Edit" | "MultiEdit" => input.get("file_path"),
        "Glob" => input.get("path"),
        _ => None,
    };

    if let Some(path) = file_path.and_then(|v| v.as_str()) {
        let path = path.trim();
        if path.is_empty() {
            return;
        }

        let (dir, file) = if let Some(last_slash) = path.rfind('/') {
            (path[..last_slash].to_string(), path[last_slash + 1..].to_string())
        } else {
            (".".to_string(), path.to_string())
        };

        tree.entry(dir).or_default().push(file);
    }
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

    fn parse_and_aggregate(jsonl: &str) -> SessionAggregate {
        let result = parse_reader(Cursor::new(jsonl)).unwrap();
        let session = build_session(&result.entries);
        aggregate(&session)
    }

    #[test]
    fn token_totals_match_input() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let a2 = "550e8400-e29b-41d4-a716-446655440003";

        let entries = [
            format!(
                r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}"#
            ),
            format!(
                r#"{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Hello!"}}],"usage":{{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":200,"cache_read_input_tokens":30}}}}}}"#
            ),
            format!(
                r#"{{"type":"assistant","uuid":"{a2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"More"}}],"usage":{{"input_tokens":60,"output_tokens":40}}}}}}"#
            ),
        ];
        let jsonl = entries.join("\n");
        let agg = parse_and_aggregate(&jsonl);

        assert_eq!(agg.message_count, 3);
        assert_eq!(agg.total_input_tokens, 160);
        assert_eq!(agg.total_output_tokens, 90);
        assert_eq!(agg.total_cache_creation_tokens, 200);
        assert_eq!(agg.total_cache_read_tokens, 30);
    }

    fn make_tool_json(uuid: &str, tools: &[serde_json::Value]) -> String {
        let tools_json: Vec<String> = tools.iter().map(|t| t.to_string()).collect();
        format!(
            r#"{{"type":"assistant","uuid":"{}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"assistant","content":[{}]}}}}"#,
            uuid,
            tools_json.join(",")
        )
    }

    #[test]
    fn file_tree_groups_paths_by_directory() {
        let tools = vec![
            serde_json::json!({"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"src/main.rs"}}),
            serde_json::json!({"type":"tool_use","id":"t2","name":"Read","input":{"file_path":"src/lib.rs"}}),
            serde_json::json!({"type":"tool_use","id":"t3","name":"Write","input":{"file_path":"README.md"}}),
            serde_json::json!({"type":"tool_use","id":"t4","name":"Glob","input":{"pattern":"*.rs","path":"tests/"}}),
            serde_json::json!({"type":"tool_use","id":"t5","name":"Edit","input":{"file_path":"src/model/entry.rs","old_string":"a","new_string":"b"}}),
        ];
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = make_tool_json(a1, &tools);
        let agg = parse_and_aggregate(&jsonl);

        let src_files = agg.file_tree.get("src").unwrap();
        assert_eq!(src_files.len(), 2);
        assert!(src_files.contains(&"main.rs".to_string()));
        assert!(src_files.contains(&"lib.rs".to_string()));

        let model_files = agg.file_tree.get("src/model").unwrap();
        assert_eq!(model_files.len(), 1);
        assert!(model_files.contains(&"entry.rs".to_string()));

        let root_files = agg.file_tree.get(".").unwrap();
        assert!(root_files.contains(&"README.md".to_string()));
    }

    #[test]
    fn tool_usage_counts_correct() {
        let tools = vec![
            serde_json::json!({"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}),
            serde_json::json!({"type":"tool_use","id":"t2","name":"Bash","input":{"command":"pwd"}}),
            serde_json::json!({"type":"tool_use","id":"t3","name":"Read","input":{"file_path":"foo.txt"}}),
            serde_json::json!({"type":"tool_use","id":"t4","name":"Write","input":{"file_path":"bar.txt","content":"x"}}),
        ];
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = make_tool_json(a1, &tools);
        let agg = parse_and_aggregate(&jsonl);

        assert_eq!(agg.tool_counts.get("Bash"), Some(&2));
        assert_eq!(agg.tool_counts.get("Read"), Some(&1));
        assert_eq!(agg.tool_counts.get("Write"), Some(&1));
        assert!(!agg.tool_counts.contains_key("Grep"));
    }

    #[test]
    fn first_and_last_timestamps() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let entries = [
            format!(
                r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:00:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"first"}}]}}}}"#
            ),
            format!(
                r#"{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"last"}}]}}}}"#
            ),
        ];
        let jsonl = entries.join("\n");
        let agg = parse_and_aggregate(&jsonl);

        let first = agg.first_timestamp.unwrap();
        let last = agg.last_timestamp.unwrap();
        assert!(first < last);
    }

    #[test]
    fn summaries_are_extracted() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let s1 = "550e8400-e29b-41d4-a716-446655440010";
        let entries = [
            format!(
                r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}"#
            ),
            format!(
                r#"{{"type":"summary","uuid":"{s1}","timestamp":"2025-06-15T11:00:00Z","sessionId":"s1","title":"Quick Chat","summary":"A brief discussion."}}"#
            ),
        ];
        let jsonl = entries.join("\n");
        let agg = parse_and_aggregate(&jsonl);

        assert_eq!(agg.summaries.len(), 1);
        assert_eq!(agg.summaries[0], "Quick Chat");
    }

    #[test]
    fn is_active_true_for_recent_session() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let timestamp = Utc::now() - Duration::minutes(5);
        let ts_str = timestamp.to_rfc3339();

        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"{ts}","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}"#,
            u1 = u1,
            ts = ts_str,
        );

        let agg = parse_and_aggregate(&jsonl);
        assert!(agg.is_active, "recent session should be active");
    }

    #[test]
    fn is_active_false_for_old_session() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let timestamp = Utc::now() - Duration::minutes(15);
        let ts_str = timestamp.to_rfc3339();

        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"{ts}","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}"#,
            u1 = u1,
            ts = ts_str,
        );

        let agg = parse_and_aggregate(&jsonl);
        assert!(!agg.is_active, "old session should not be active");
    }

    #[test]
    fn empty_session_has_default_aggregate() {
        let jsonl = "";
        let agg = parse_and_aggregate(jsonl);
        assert_eq!(agg.message_count, 0);
        assert_eq!(agg.total_input_tokens, 0);
        assert!(agg.first_timestamp.is_none());
        assert!(agg.last_timestamp.is_none());
        assert!(agg.tool_counts.is_empty());
        assert!(agg.file_tree.is_empty());
        assert!(!agg.is_active);
    }

    #[test]
    fn unknown_entry_extracts_timestamp_from_raw_json() {
        let jsonl = r#"{"type":"future-type","uuid":"550e8400-e29b-41d4-a716-446655440001","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","customField":42}"#;
        let agg = parse_and_aggregate(jsonl);
        assert!(agg.first_timestamp.is_some());
        assert_eq!(agg.message_count, 0);
    }

    #[test]
    fn extract_file_paths_empty_path_is_skipped() {
        let tools = vec![
            serde_json::json!({"type":"tool_use","id":"t1","name":"Read","input":{"file_path":""}}),
            serde_json::json!({"type":"tool_use","id":"t2","name":"Write","input":{"file_path":"  "}}),
        ];
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = make_tool_json(a1, &tools);
        let agg = parse_and_aggregate(&jsonl);
        assert!(agg.file_tree.is_empty(), "empty paths should be skipped");
    }

    #[test]
    fn extract_file_paths_unknown_tool_name_is_skipped() {
        let tools = vec![
            serde_json::json!({"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}),
            serde_json::json!({"type":"tool_use","id":"t2","name":"Grep","input":{"pattern":"fn main"}}),
        ];
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = make_tool_json(a1, &tools);
        let agg = parse_and_aggregate(&jsonl);
        assert!(agg.file_tree.is_empty(), "non-file tools should not create file tree entries");
    }

    #[test]
    fn aggregate_session_id_matches_input() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"my-session-42","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}"#
        );
        let agg = parse_and_aggregate(&jsonl);
        assert_eq!(agg.session_id, "my-session-42");
    }

    #[test]
    fn single_entry_session() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}"#
        );
        let agg = parse_and_aggregate(&jsonl);
        assert_eq!(agg.message_count, 1);
        assert_eq!(agg.first_timestamp, agg.last_timestamp);
    }

    #[test]
    fn tool_result_only_message_does_not_increment_count() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let jsonl = format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"t1","content":"result"}}]}}}}"#
        );
        let agg = parse_and_aggregate(&jsonl);
        // tool_result-only user is still a User entry, so it counts.
        assert_eq!(agg.message_count, 1);
    }

    #[test]
    fn file_tree_glob_path_from_tests_dir() {
        let tools = vec![
            serde_json::json!({"type":"tool_use","id":"t1","name":"Glob","input":{"pattern":"*.rs","path":"tests"}}),
            serde_json::json!({"type":"tool_use","id":"t2","name":"Glob","input":{"pattern":"*.md"}}),
        ];
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = make_tool_json(a1, &tools);
        let agg = parse_and_aggregate(&jsonl);

        // Glob with path="tests" → dir=".", file="tests"
        let root_files = agg.file_tree.get(".").unwrap();
        assert!(root_files.contains(&"tests".to_string()));

        // Glob without path → nothing in file_tree (path is None)
        assert!(!agg.file_tree.contains_key("tests"));
    }

    #[test]
    fn multi_edit_file_path_is_extracted() {
        let tools = vec![
            serde_json::json!({"type":"tool_use","id":"t1","name":"MultiEdit","input":{"file_path":"src/lib.rs","edits":[{"old_string":"a","new_string":"b"}]}}),
        ];
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let jsonl = make_tool_json(a1, &tools);
        let agg = parse_and_aggregate(&jsonl);
        let src_files = agg.file_tree.get("src").unwrap();
        assert!(src_files.contains(&"lib.rs".to_string()));
    }

    #[test]
    fn no_entries_produces_empty_timestamps() {
        let jsonl = "";
        let agg = parse_and_aggregate(jsonl);
        assert!(agg.first_timestamp.is_none());
        assert!(agg.last_timestamp.is_none());
        assert!(!agg.is_active);
    }
}
