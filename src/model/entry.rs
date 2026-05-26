//! Top-level transcript entry model.
//!
//! Each line in a Claude Code JSONL session file deserializes into one
//! variant of [`TranscriptEntry`], discriminated by the `"type"` field.
//!
//! Unknown entry types are preserved in [`TranscriptEntry::Unknown`] with
//! their raw JSON intact, so rendering never panics on future formats.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::content::Message;

// ---------------------------------------------------------------------------
// Common fields — embedded in every entry struct via `#[serde(flatten)]`
// ---------------------------------------------------------------------------

/// Fields common to all transcript entry types.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonFields {
    /// Unique identifier for this entry.
    pub uuid: Uuid,
    /// UUID of the parent message (for DAG threading).
    #[serde(default)]
    pub parent_uuid: Option<Uuid>,
    /// ISO-8601 timestamp.
    pub timestamp: DateTime<Utc>,
    /// Claude Code session identifier.
    pub session_id: String,
    /// Whether this is a sub-agent sidechain message.
    #[serde(default)]
    pub is_sidechain: bool,
    /// The agent / model identifier.
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Working directory at the time of the entry.
    #[serde(default)]
    pub cwd: Option<String>,
    /// Git branch at the time of the entry.
    #[serde(default)]
    pub git_branch: Option<String>,
    /// Claude Code version string.
    #[serde(default)]
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// TranscriptEntry — top-level discriminated union
// ---------------------------------------------------------------------------

/// A single line/entry in a Claude Code session JSONL file.
///
/// Tagged by the `"type"` field. Deserialization is manual so that unknown
/// types are captured as [`TranscriptEntry::Unknown`] with their full JSON.
#[derive(Debug, Clone)]
pub enum TranscriptEntry {
    /// A human user turn.
    User(UserEntry),
    /// A Claude assistant turn.
    Assistant(AssistantEntry),
    /// A user-written session summary.
    Summary(SummaryEntry),
    /// System / metadata entries (e.g. session init, config).
    System(SystemEntry),
    /// Async task queue operation events.
    QueueOperation(QueueOperationEntry),
    /// Hook lifecycle attachment events.
    HookAttachment(HookAttachmentEntry),
    /// Summary generated while the user was away.
    AwaySummary(AwaySummaryEntry),
    /// Catch-all for unknown or future entry types.
    Unknown {
        /// The original `"type"` value.
        entry_type: String,
        /// The full raw JSON object.
        raw: serde_json::Value,
    },
}

impl<'de> Deserialize<'de> for TranscriptEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let type_str = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

        let entry = match type_str {
            "user" => TranscriptEntry::User(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            ),
            "assistant" => TranscriptEntry::Assistant(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            ),
            "summary" => TranscriptEntry::Summary(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            ),
            "system" => TranscriptEntry::System(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            ),
            "queue-operation" => TranscriptEntry::QueueOperation(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            ),
            "hook-attachment" => TranscriptEntry::HookAttachment(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            ),
            "away-summary" => TranscriptEntry::AwaySummary(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            ),
            other => TranscriptEntry::Unknown {
                entry_type: other.to_string(),
                raw: value,
            },
        };

        Ok(entry)
    }
}

// ---------------------------------------------------------------------------
// Concrete entry structs
// ---------------------------------------------------------------------------

/// A human user turn.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEntry {
    #[serde(flatten)]
    pub common: CommonFields,

    /// Team / account name.
    #[serde(default)]
    pub team_name: Option<String>,
    /// Request identifier.
    #[serde(default)]
    pub request_id: Option<String>,
    /// User type classification.
    #[serde(default)]
    pub user_type: Option<String>,

    /// The actual message content.
    pub message: Message,
}

/// A Claude assistant turn.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantEntry {
    #[serde(flatten)]
    pub common: CommonFields,

    /// Team / account name.
    #[serde(default)]
    pub team_name: Option<String>,
    /// Request identifier.
    #[serde(default)]
    pub request_id: Option<String>,

    /// The model response with content and usage.
    pub message: Message,
}

/// A user-authored session summary.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryEntry {
    #[serde(flatten)]
    pub common: CommonFields,

    /// UUID of the leaf / last message covered by this summary.
    #[serde(default)]
    pub leaf_uuid: Option<Uuid>,
    /// The summary text.
    #[serde(default)]
    pub summary: Option<String>,
    /// Optional title assigned to the summary.
    #[serde(default)]
    pub title: Option<String>,
}

/// System-level metadata entries.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEntry {
    #[serde(flatten)]
    pub common: CommonFields,

    /// System-level metadata (tool-allow lists, model config, etc.).
    #[serde(default)]
    pub system: Option<serde_json::Value>,
}

/// Async task queue operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueOperationEntry {
    #[serde(flatten)]
    pub common: CommonFields,

    /// Operation payload.
    #[serde(default)]
    pub operation: Option<serde_json::Value>,
}

/// Hook lifecycle attachment.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookAttachmentEntry {
    #[serde(flatten)]
    pub common: CommonFields,

    /// Attachment payload.
    #[serde(default)]
    pub attachment: Option<serde_json::Value>,
}

/// Away-mode summary.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaySummaryEntry {
    #[serde(flatten)]
    pub common: CommonFields,

    /// The auto-generated summary text.
    #[serde(default)]
    pub summary: Option<String>,
}

// ---------------------------------------------------------------------------
// Convenience accessors
// ---------------------------------------------------------------------------

impl TranscriptEntry {
    /// Return a reference to the common fields for any entry variant.
    pub fn common(&self) -> &CommonFields {
        match self {
            TranscriptEntry::User(e) => &e.common,
            TranscriptEntry::Assistant(e) => &e.common,
            TranscriptEntry::Summary(e) => &e.common,
            TranscriptEntry::System(e) => &e.common,
            TranscriptEntry::QueueOperation(e) => &e.common,
            TranscriptEntry::HookAttachment(e) => &e.common,
            TranscriptEntry::AwaySummary(e) => &e.common,
            TranscriptEntry::Unknown { .. } => {
                // We can't provide common fields for unknown entries.
                // Callers should handle this gracefully.
                panic!("Unknown entry has no common fields")
            }
        }
    }

    /// The `"type"` discriminator string.
    pub fn entry_type(&self) -> &str {
        match self {
            TranscriptEntry::User(_) => "user",
            TranscriptEntry::Assistant(_) => "assistant",
            TranscriptEntry::Summary(_) => "summary",
            TranscriptEntry::System(_) => "system",
            TranscriptEntry::QueueOperation(_) => "queue-operation",
            TranscriptEntry::HookAttachment(_) => "hook-attachment",
            TranscriptEntry::AwaySummary(_) => "away-summary",
            TranscriptEntry::Unknown { entry_type, .. } => entry_type.as_str(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_user_entry() {
        let json = serde_json::json!({
            "type": "user",
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "parentUuid": null,
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-abc123",
            "isSidechain": false,
            "agentId": null,
            "cwd": "/home/user/project",
            "gitBranch": "main",
            "version": "1.0.0",
            "teamName": "my-team",
            "requestId": "req-001",
            "userType": "human",
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Hello, Claude!"}
                ]
            }
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::User(_)));
        assert_eq!(entry.entry_type(), "user");
        assert_eq!(entry.common().session_id, "session-abc123");
    }

    #[test]
    fn round_trip_assistant_entry() {
        let json = serde_json::json!({
            "type": "assistant",
            "uuid": "660e8400-e29b-41d4-a716-446655440000",
            "parentUuid": "550e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:05Z",
            "sessionId": "session-abc123",
            "isSidechain": false,
            "agentId": "claude-opus-4-7",
            "cwd": "/home/user/project",
            "gitBranch": "main",
            "version": "1.0.0",
            "message": {
                "role": "assistant",
                "model": "claude-opus-4-7",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 150,
                    "output_tokens": 80
                },
                "content": [
                    {"type": "text", "text": "Hello! How can I help?"}
                ]
            }
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::Assistant(_)));
    }

    #[test]
    fn round_trip_summary_entry() {
        let json = serde_json::json!({
            "type": "summary",
            "uuid": "770e8400-e29b-41d4-a716-446655440000",
            "parentUuid": null,
            "timestamp": "2025-06-15T11:00:00Z",
            "sessionId": "session-abc123",
            "isSidechain": false,
            "leafUuid": "660e8400-e29b-41d4-a716-446655440000",
            "summary": "Discussed project architecture.",
            "title": "Architecture Discussion"
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::Summary(_)));
    }

    #[test]
    fn round_trip_system_entry() {
        let json = serde_json::json!({
            "type": "system",
            "uuid": "880e8400-e29b-41d4-a716-446655440000",
            "parentUuid": null,
            "timestamp": "2025-06-15T10:29:00Z",
            "sessionId": "session-abc123",
            "isSidechain": false,
            "system": {"tools": ["bash", "read", "write"]}
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::System(_)));
    }

    #[test]
    fn round_trip_queue_operation_entry() {
        let json = serde_json::json!({
            "type": "queue-operation",
            "uuid": "990e8400-e29b-41d4-a716-446655440000",
            "parentUuid": null,
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-abc123",
            "operation": {"action": "create", "taskId": "task-1"}
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::QueueOperation(_)));
    }

    #[test]
    fn round_trip_hook_attachment_entry() {
        let json = serde_json::json!({
            "type": "hook-attachment",
            "uuid": "aa0e8400-e29b-41d4-a716-446655440000",
            "parentUuid": null,
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-abc123",
            "attachment": {"hook": "pre-commit", "data": {}}
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::HookAttachment(_)));
    }

    #[test]
    fn round_trip_away_summary_entry() {
        let json = serde_json::json!({
            "type": "away-summary",
            "uuid": "bb0e8400-e29b-41d4-a716-446655440000",
            "parentUuid": null,
            "timestamp": "2025-06-15T12:00:00Z",
            "sessionId": "session-abc123",
            "summary": "User was away for 30 minutes."
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::AwaySummary(_)));
    }

    #[test]
    fn unknown_type_falls_through_to_unknown_variant() {
        let json = serde_json::json!({
            "type": "future-feature",
            "uuid": "cc0e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-abc123",
            "customField": 42
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        match entry {
            TranscriptEntry::Unknown { entry_type, raw } => {
                assert_eq!(entry_type, "future-feature");
                assert_eq!(raw["customField"], 42);
            }
            other => panic!("expected Unknown variant, got {:?}", other),
        }
    }

    #[test]
    fn missing_optional_fields_default_correctly() {
        let json = serde_json::json!({
            "type": "user",
            "uuid": "dd0e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-abc123",
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "minimal message"}
                ]
            }
        });

        let entry: TranscriptEntry = serde_json::from_value(json).unwrap();
        match entry {
            TranscriptEntry::User(u) => {
                assert!(u.common.parent_uuid.is_none());
                assert!(!u.common.is_sidechain);
                assert!(u.common.agent_id.is_none());
                assert!(u.common.cwd.is_none());
            }
            other => panic!("expected User variant, got {:?}", other),
        }
    }
}
