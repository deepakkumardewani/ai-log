//! Message and content-item model.
//!
//! A [`Message`] is the core payload of user and assistant entries.
//! Its `content` field is a list of [`ContentItem`] variants representing
//! text, thinking blocks, tool calls, tool results, and images.
//!
//! The `content` field in the JSONL can be either a plain string
//! (e.g. `"content": "hello"`) or an array of objects
//! (e.g. `"content": [{"type": "text", "text": "hello"}]`).
//! Both are normalized to `Vec<ContentItem>` on deserialization.

use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// A chat message — the payload of user and assistant transcript entries.
/// Anthropic API uses snake_case fields inside the message object
/// (`input_tokens`, `stop_reason`, etc.) even when the outer transcript
/// uses camelCase (`parentUuid`, `isSidechain`, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// `"user"` or `"assistant"`.
    pub role: String,

    /// Model identifier (assistant messages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Reason the model stopped: `"end_turn"`, `"tool_use"`, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Token usage breakdown (assistant messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,

    /// The list of content blocks making up the message.
    /// Accepts both a plain string or an array in JSON.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_content"
    )]
    pub content: Vec<ContentItem>,
}

/// Custom deserializer: accepts a string OR an array of ContentItem objects.
fn deserialize_content<'de, D>(deserializer: D) -> Result<Vec<ContentItem>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ContentVisitor;

    impl<'de> Visitor<'de> for ContentVisitor {
        type Value = Vec<ContentItem>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or an array of content items")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![ContentItem::Text {
                text: s.to_string(),
            }])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut items = Vec::new();
            while let Some(item) = seq.next_element::<ContentItem>()? {
                items.push(item);
            }
            Ok(items)
        }
    }

    deserializer.deserialize_any(ContentVisitor)
}

/// Token usage information for an assistant message.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageInfo {
    /// Input / prompt tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,

    /// Output / completion tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,

    /// Tokens used to create a new cache entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,

    /// Tokens read from an existing cache entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,

    /// API service tier, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
}

/// A single content block within a [`Message`].
///
/// Tagged by `#[serde(tag = "type")]`. Tool-aware processing happens in
/// [`super::tool`]; this enum focuses on faithful deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentItem {
    /// Plain text (user or assistant).
    #[serde(rename = "text")]
    Text { text: String },

    /// Extended thinking block (assistant only).
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },

    /// A tool-use request emitted by the assistant.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique tool-call identifier.
        id: String,
        /// Tool name (e.g. `"Bash"`, `"Read"`, `"Write"`).
        name: String,
        /// Tool arguments as a JSON object.
        input: serde_json::Value,
    },

    /// The result of a tool execution, associated with a prior `tool_use` by ID.
    #[serde(rename = "tool_result")]
    ToolResult {
        /// The `id` of the matching `tool_use`.
        tool_use_id: String,
        /// Result content — either a plain string or a list of nested content blocks.
        content: ToolResultContent,
        /// Whether the tool reported an error.
        #[serde(default)]
        is_error: Option<bool>,
    },

    /// An embedded image (base64-encoded or referenced).
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

/// The content of a tool result — flat string or structured blocks.
///
/// Deserialized from either a JSON string or an array of nested
/// [`ContentItem`] blocks (used for multi-block results like
/// `Bash` stdout+stderr).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Plain string content (most tool results).
    String(String),
    /// Multi-block content (e.g. Bash stdout + stderr as separate text blocks).
    Blocks(Vec<ContentItem>),
}

impl ToolResultContent {
    /// Return the content as a plain string, joining blocks if necessary.
    pub fn as_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Blocks(items) => items
                .iter()
                .filter_map(|item| match item {
                    ContentItem::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// Source of an embedded image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImageSource {
    /// Source type (e.g. `"base64"`).
    #[serde(rename = "type")]
    pub source_type: String,

    /// MIME type (e.g. `"image/png"`).
    pub media_type: String,

    /// Base64-encoded image data.
    pub data: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_text_content() {
        let json = serde_json::json!({"type": "text", "text": "Hello, world!"});
        let item: ContentItem = serde_json::from_value(json).unwrap();
        assert!(matches!(item, ContentItem::Text { .. }));
    }

    #[test]
    fn deserialize_thinking_content() {
        let json = serde_json::json!({
            "type": "thinking",
            "thinking": "Let me analyze this...",
            "signature": "abc123sig"
        });
        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "Let me analyze this...");
                assert_eq!(signature.unwrap(), "abc123sig");
            }
            other => panic!("expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_thinking_without_signature() {
        let json = serde_json::json!({
            "type": "thinking",
            "thinking": "Just thinking..."
        });
        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::Thinking { signature, .. } => {
                assert!(signature.is_none());
            }
            other => panic!("expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_tool_use_content() {
        let json = serde_json::json!({
            "type": "tool_use",
            "id": "toolu_01ABC",
            "name": "Bash",
            "input": {"command": "ls -la", "description": "List files"}
        });
        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_01ABC");
                assert_eq!(name, "Bash");
                assert_eq!(input["command"], "ls -la");
            }
            other => panic!("expected ToolUse, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_tool_result_string() {
        let json = serde_json::json!({
            "type": "tool_result",
            "tool_use_id": "toolu_01ABC",
            "content": "file1.txt\nfile2.txt",
            "is_error": false
        });
        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "toolu_01ABC");
                assert_eq!(content.as_string(), "file1.txt\nfile2.txt");
                assert_eq!(is_error, Some(false));
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_tool_result_blocks() {
        let json = serde_json::json!({
            "type": "tool_result",
            "tool_use_id": "toolu_02DEF",
            "content": [
                {"type": "text", "text": "stdout line 1"},
                {"type": "text", "text": "stderr line 1"}
            ],
            "is_error": true
        });
        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "toolu_02DEF");
                assert_eq!(content.as_string(), "stdout line 1\nstderr line 1");
                assert_eq!(is_error, Some(true));
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_image_content() {
        let json = serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": "iVBORw0KGgo=="
            }
        });
        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::Image { source } => {
                assert_eq!(source.source_type, "base64");
                assert_eq!(source.media_type, "image/png");
                assert_eq!(source.data, "iVBORw0KGgo==");
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_full_message() {
        let json = serde_json::json!({
            "role": "assistant",
            "model": "claude-opus-4-7",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 1200,
                "output_tokens": 300,
                "cache_creation_input_tokens": 500,
                "cache_read_input_tokens": 200,
                "service_tier": "standard"
            },
            "content": [
                {"type": "text", "text": "Here is the result:"},
                {"type": "thinking", "thinking": "I should explain this clearly."}
            ]
        });

        let msg: Message = serde_json::from_value(json).unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.model.as_deref(), Some("claude-opus-4-7"));
        assert_eq!(msg.stop_reason.as_deref(), Some("end_turn"));

        let usage = msg.usage.unwrap();
        assert_eq!(usage.input_tokens, Some(1200));
        assert_eq!(usage.output_tokens, Some(300));
        assert_eq!(usage.cache_creation_input_tokens, Some(500));
        assert_eq!(usage.cache_read_input_tokens, Some(200));
        assert_eq!(usage.service_tier.as_deref(), Some("standard"));

        assert_eq!(msg.content.len(), 2);
    }

    #[test]
    fn deserialize_message_with_empty_content_and_no_usage() {
        let json = serde_json::json!({
            "role": "user",
            "content": []
        });

        let msg: Message = serde_json::from_value(json).unwrap();
        assert_eq!(msg.role, "user");
        assert!(msg.model.is_none());
        assert!(msg.usage.is_none());
        assert!(msg.content.is_empty());
    }

    #[test]
    fn deserialize_message_with_string_content() {
        let json = serde_json::json!({
            "role": "user",
            "content": "Hello, this is a plain string"
        });

        let msg: Message = serde_json::from_value(json).unwrap();
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            ContentItem::Text { text } => assert_eq!(text, "Hello, this is a plain string"),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_minimal_message() {
        let json = serde_json::json!({
            "role": "assistant",
            "content": [{"type": "text", "text": "minimal"}]
        });

        let msg: Message = serde_json::from_value(json).unwrap();
        assert_eq!(msg.role, "assistant");
        assert!(msg.model.is_none());
        assert!(msg.stop_reason.is_none());
        assert!(msg.usage.is_none());
    }

    #[test]
    fn tool_result_blocks_with_mixed_content_types() {
        let json = serde_json::json!({
            "type": "tool_result",
            "tool_use_id": "toolu_mixed",
            "content": [
                {"type": "text", "text": "stdout output"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "abc"}},
                {"type": "text", "text": "more text"}
            ]
        });

        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::ToolResult { content, .. } => {
                // Non-text items should be filtered out by as_string.
                assert_eq!(content.as_string(), "stdout output\nmore text");
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn tool_result_blocks_empty() {
        let json = serde_json::json!({
            "type": "tool_result",
            "tool_use_id": "toolu_empty",
            "content": []
        });

        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::ToolResult { content, .. } => {
                assert_eq!(content.as_string(), "");
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn tool_result_is_error_defaults_to_none() {
        let json = serde_json::json!({
            "type": "tool_result",
            "tool_use_id": "toolu_noerr",
            "content": "result"
        });

        let item: ContentItem = serde_json::from_value(json).unwrap();
        match item {
            ContentItem::ToolResult { is_error, .. } => {
                assert!(is_error.is_none());
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn usage_info_all_fields_present() {
        let json = serde_json::json!({
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_creation_input_tokens": 200,
            "cache_read_input_tokens": 30,
            "service_tier": "standard"
        });

        let usage: UsageInfo = serde_json::from_value(json).unwrap();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(usage.cache_creation_input_tokens, Some(200));
        assert_eq!(usage.cache_read_input_tokens, Some(30));
        assert_eq!(usage.service_tier.as_deref(), Some("standard"));
    }

    #[test]
    fn usage_info_default_all_none() {
        let json = serde_json::json!({});
        let usage: UsageInfo = serde_json::from_value(json).unwrap();
        assert!(usage.input_tokens.is_none());
        assert!(usage.output_tokens.is_none());
        assert!(usage.cache_creation_input_tokens.is_none());
        assert!(usage.cache_read_input_tokens.is_none());
        assert!(usage.service_tier.is_none());
    }

    #[test]
    fn content_item_unknown_type_deserializes_as_error() {
        // serde(tag = "type") with unknown variant should fail.
        let json = serde_json::json!({"type": "nonexistent_variant", "data": 42});
        let result: Result<ContentItem, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_content_items_all_variants() {
        // Text
        let text: ContentItem =
            serde_json::from_value(serde_json::json!({"type": "text", "text": "hello"})).unwrap();
        assert!(matches!(text, ContentItem::Text { .. }));

        // Thinking
        let think: ContentItem =
            serde_json::from_value(serde_json::json!({"type": "thinking", "thinking": "hmm"}))
                .unwrap();
        assert!(matches!(think, ContentItem::Thinking { .. }));

        // ToolUse
        let tu: ContentItem = serde_json::from_value(
            serde_json::json!({"type": "tool_use", "id": "t1", "name": "Bash", "input": {}}),
        )
        .unwrap();
        assert!(matches!(tu, ContentItem::ToolUse { .. }));

        // ToolResult (string content)
        let tr: ContentItem = serde_json::from_value(
            serde_json::json!({"type": "tool_result", "tool_use_id": "t1", "content": "done"}),
        )
        .unwrap();
        assert!(matches!(tr, ContentItem::ToolResult { .. }));

        // Image
        let img: ContentItem = serde_json::from_value(
            serde_json::json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "aaaa"}}),
        )
        .unwrap();
        assert!(matches!(img, ContentItem::Image { .. }));
    }
}
