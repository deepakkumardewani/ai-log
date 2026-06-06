//! Typed tool-input models.
//!
//! Each variant of [`ToolInput`] corresponds to a known `tool_use.name` value.
//! Unknown tool names fall through to [`ToolInput::Generic`], preserving the raw
//! JSON for key/value rendering.
//!
//! Deserialization uses a custom `TryFrom<ContentItem>` / manual dispatch rather
//! than a serde tag, because the discriminator (`name`) is *inside* the
//! `tool_use` content item, not at the entry level.

use serde::{Deserialize, Serialize};

use super::content::ContentItem;

// ---------------------------------------------------------------------------
// Tool input structs
// ---------------------------------------------------------------------------

/// Bash shell command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct BashInput {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dangerously_disable_sandbox: Option<bool>,
}

/// Read a file from the filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ReadInput {
    pub file_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
}

/// Write content to a file.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct WriteInput {
    pub file_path: String,
    pub content: String,
}

/// Exact string replacement edit in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct EditInput {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

/// Multiple edits applied atomically.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct MultiEditInput {
    pub file_path: String,
    pub edits: Vec<EditOp>,
}

/// A single edit operation within a [`MultiEditInput`].
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct EditOp {
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

/// Glob file pattern search.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct GlobInput {
    pub pattern: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Grep content search.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct GrepInput {
    pub pattern: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,
}

/// A todo item within [`TodoWriteInput`].
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct TodoItem {
    pub content: String,
    pub status: String,
    pub priority: String,
    pub id: String,
}

/// TodoWrite — structured task list.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct TodoWriteInput {
    pub todos: Vec<TodoItem>,
}

/// An option within [`AskUserQuestionInput`].
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

/// A question within [`AskUserQuestionInput`].
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct QuestionItem {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub multi_select: bool,
}

/// AskUserQuestion — interactive user prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct AskUserQuestionInput {
    pub questions: Vec<QuestionItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<serde_json::Value>,
}

/// Web search query.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct WebSearchInput {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
}

/// Web fetch from a URL.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct WebFetchInput {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// Schedule a future wakeup.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ScheduleWakeupInput {
    /// Seconds from now.
    pub delay_seconds: u64,
    /// Human-readable reason.
    pub reason: String,
    /// Prompt to re-invoke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// Cron job creation.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct CronCreateInput {
    /// 5-field cron expression.
    pub cron: String,
    /// Prompt to enqueue.
    pub prompt: String,
    #[serde(default)]
    pub recurring: bool,
    #[serde(default)]
    pub durable: bool,
}

/// Cron job deletion.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct CronDeleteInput {
    pub id: String,
}

/// Cron job listing (no params needed).
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct CronListInput {}

/// Monitor — background task watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct MonitorInput {
    pub description: String,
    pub timeout_ms: u64,
    pub persistent: bool,
    pub command: String,
}

/// Task / Agent invocation (covers `Task`, `Agent`, `TaskCreate`,
/// `TaskUpdate`, `TaskList`, `TaskOutput`, `TaskStop`).
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct TaskInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    // Catch remaining fields.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Team management (TeamCreate, TeamDelete).
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct TeamInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// SendMessage — inter-agent communication.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct SendMessageInput {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

/// Skill invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct SkillInput {
    /// The skill name / command.
    pub skill: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

/// ExitPlanMode — signal plan completion.
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ExitPlanModeInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_prompts: Option<Vec<serde_json::Value>>,
}

// ---------------------------------------------------------------------------
// ToolInput enum — the typed dispatch
// ---------------------------------------------------------------------------

/// Typed tool input, dispatched by `tool_use.name`.
///
/// Unknown / future tool names fall through to [`ToolInput::Generic`],
/// which preserves the name and raw JSON so rendering never panics.
///
/// This enum is **not** directly deserialized — it is constructed via
/// [`ToolInput::from_content_item`] which inspects the tool name to pick
/// the correct variant.
#[derive(Debug, Clone)]
pub enum ToolInput {
    /// Known typed tool inputs.
    Bash(BashInput),
    Read(ReadInput),
    Write(WriteInput),
    Edit(EditInput),
    MultiEdit(MultiEditInput),
    Glob(GlobInput),
    Grep(GrepInput),
    TodoWrite(TodoWriteInput),
    AskUserQuestion(AskUserQuestionInput),
    WebSearch(WebSearchInput),
    WebFetch(WebFetchInput),
    ScheduleWakeup(ScheduleWakeupInput),
    CronCreate(CronCreateInput),
    CronDelete(CronDeleteInput),
    CronList(CronListInput),
    Monitor(MonitorInput),
    Task(TaskInput),
    Team(TeamInput),
    SendMessage(SendMessageInput),
    Skill(SkillInput),
    ExitPlanMode(ExitPlanModeInput),

    /// Generic fallback for unknown tools.
    Generic {
        /// The tool name from `tool_use.name`.
        name: String,
        /// The raw `input` JSON object.
        input: serde_json::Value,
    },
}

impl ToolInput {
    /// Dispatch a [`ContentItem::ToolUse`] into a typed [`ToolInput`].
    ///
    /// Returns `None` if the content item is not a `ToolUse`.
    pub fn from_content_item(item: &ContentItem) -> Option<Self> {
        match item {
            ContentItem::ToolUse { name, input, .. } => {
                Some(Self::from_name_and_input(name, input.clone()))
            }
            _ => None,
        }
    }

    /// Parse the tool input JSON based on the tool name.
    pub fn from_name_and_input(name: &str, input: serde_json::Value) -> Self {
        // Helper macro to keep the match arms readable. `clone` is necessary
        // because serde_json::from_value consumes the Value; on failure we
        // still need the original for the Generic fallback.
        macro_rules! dispatch {
            ($ty:ty, $variant:ident) => {
                serde_json::from_value::<$ty>(input.clone()).map(Self::$variant).unwrap_or_else(
                    |_| Self::Generic {
                        name: name.to_string(),
                        input,
                    },
                )
            };
        }

        match name {
            "Bash" => dispatch!(BashInput, Bash),
            "Read" => dispatch!(ReadInput, Read),
            "Write" => dispatch!(WriteInput, Write),
            "Edit" => dispatch!(EditInput, Edit),
            "MultiEdit" => dispatch!(MultiEditInput, MultiEdit),
            "Glob" => dispatch!(GlobInput, Glob),
            "Grep" => dispatch!(GrepInput, Grep),
            "TodoWrite" => dispatch!(TodoWriteInput, TodoWrite),
            "AskUserQuestion" | "ask_user_question" => {
                dispatch!(AskUserQuestionInput, AskUserQuestion)
            }
            "WebSearch" => dispatch!(WebSearchInput, WebSearch),
            "WebFetch" => dispatch!(WebFetchInput, WebFetch),
            "ScheduleWakeup" => dispatch!(ScheduleWakeupInput, ScheduleWakeup),
            "CronCreate" => dispatch!(CronCreateInput, CronCreate),
            "CronDelete" => dispatch!(CronDeleteInput, CronDelete),
            "CronList" => dispatch!(CronListInput, CronList),
            "Task" | "Agent" | "TaskCreate" | "TaskUpdate" | "TaskOutput" | "TaskList"
            | "TaskStop" => dispatch!(TaskInput, Task),
            "TeamCreate" | "TeamDelete" => dispatch!(TeamInput, Team),
            "SendMessage" => dispatch!(SendMessageInput, SendMessage),
            "Skill" => dispatch!(SkillInput, Skill),
            "ExitPlanMode" => dispatch!(ExitPlanModeInput, ExitPlanMode),
            "Monitor" => dispatch!(MonitorInput, Monitor),
            _ => Self::Generic {
                name: name.to_string(),
                input,
            },
        }
    }

    /// The human-readable tool name for display.
    pub fn display_name(&self) -> &str {
        match self {
            Self::Bash(_) => "Bash",
            Self::Read(_) => "Read",
            Self::Write(_) => "Write",
            Self::Edit(_) => "Edit",
            Self::MultiEdit(_) => "MultiEdit",
            Self::Glob(_) => "Glob",
            Self::Grep(_) => "Grep",
            Self::TodoWrite(_) => "TodoWrite",
            Self::AskUserQuestion(_) => "AskUserQuestion",
            Self::WebSearch(_) => "WebSearch",
            Self::WebFetch(_) => "WebFetch",
            Self::ScheduleWakeup(_) => "ScheduleWakeup",
            Self::CronCreate(_) => "CronCreate",
            Self::CronDelete(_) => "CronDelete",
            Self::CronList(_) => "CronList",
            Self::Monitor(_) => "Monitor",
            Self::Task(_) => "Task",
            Self::Team(_) => "Team",
            Self::SendMessage(_) => "SendMessage",
            Self::Skill(_) => "Skill",
            Self::ExitPlanMode(_) => "ExitPlanMode",
            Self::Generic { name, .. } => name.as_str(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a ContentItem::ToolUse for dispatching.
    fn tool_use(name: &str, input: serde_json::Value) -> ContentItem {
        ContentItem::ToolUse {
            id: "toolu_test".to_string(),
            name: name.to_string(),
            input,
        }
    }

    #[test]
    fn dispatch_bash() {
        let item = tool_use(
            "Bash",
            serde_json::json!({"command": "cargo build", "description": "Build project"}),
        );
        let ti = ToolInput::from_content_item(&item).unwrap();
        match ti {
            ToolInput::Bash(b) => {
                assert_eq!(b.command, "cargo build");
                assert_eq!(b.description.as_deref(), Some("Build project"));
            }
            other => panic!("expected Bash, got {:?}", other),
        }
    }

    #[test]
    fn dispatch_read() {
        let item = tool_use(
            "Read",
            serde_json::json!({"file_path": "src/main.rs", "offset": 10, "limit": 20}),
        );
        let ti = ToolInput::from_content_item(&item).unwrap();
        match ti {
            ToolInput::Read(r) => {
                assert_eq!(r.file_path, "src/main.rs");
                assert_eq!(r.offset, Some(10));
                assert_eq!(r.limit, Some(20));
            }
            other => panic!("expected Read, got {:?}", other),
        }
    }

    #[test]
    fn dispatch_edit() {
        let item = tool_use(
            "Edit",
            serde_json::json!({
                "file_path": "src/lib.rs",
                "old_string": "let x = 1;",
                "new_string": "let x = 2;",
                "replace_all": false
            }),
        );
        let ti = ToolInput::from_content_item(&item).unwrap();
        match ti {
            ToolInput::Edit(e) => {
                assert_eq!(e.old_string, "let x = 1;");
                assert_eq!(e.new_string, "let x = 2;");
                assert!(!e.replace_all);
            }
            other => panic!("expected Edit, got {:?}", other),
        }
    }

    #[test]
    fn dispatch_todo_write() {
        let item = tool_use(
            "TodoWrite",
            serde_json::json!({
                "todos": [
                    {"content": "Add tests", "status": "in_progress", "priority": "high", "id": "1"}
                ]
            }),
        );
        let ti = ToolInput::from_content_item(&item).unwrap();
        match ti {
            ToolInput::TodoWrite(t) => {
                assert_eq!(t.todos.len(), 1);
                assert_eq!(t.todos[0].content, "Add tests");
            }
            other => panic!("expected TodoWrite, got {:?}", other),
        }
    }

    #[test]
    fn dispatch_generic_for_unknown_tool() {
        let item = tool_use("FutureTool", serde_json::json!({"customField": 42, "name": "test"}));
        let ti = ToolInput::from_content_item(&item).unwrap();
        match ti {
            ToolInput::Generic { name, input } => {
                assert_eq!(name, "FutureTool");
                assert_eq!(input["customField"], 42);
            }
            other => panic!("expected Generic, got {:?}", other),
        }
    }

    #[test]
    fn display_name_returns_correct_values() {
        assert_eq!(
            ToolInput::Bash(BashInput {
                command: "ls".into(),
                description: None,
                run_in_background: None,
                timeout: None,
                dangerously_disable_sandbox: None,
            })
            .display_name(),
            "Bash"
        );

        assert_eq!(
            ToolInput::Generic {
                name: "CustomTool".into(),
                input: serde_json::json!({})
            }
            .display_name(),
            "CustomTool"
        );
    }

    #[test]
    fn dispatch_all_tool_names_produce_known_variants() {
        // Pairs of (tool_name, valid_minimal_input).
        let tool_cases: Vec<(&str, serde_json::Value)> = vec![
            ("Bash", serde_json::json!({"command": "ls"})),
            ("Read", serde_json::json!({"file_path": "foo.txt"})),
            ("Write", serde_json::json!({"file_path": "foo.txt", "content": "bar"})),
            (
                "Edit",
                serde_json::json!({"file_path": "f", "old_string": "a", "new_string": "b"}),
            ),
            ("MultiEdit", serde_json::json!({"file_path": "f", "edits": []})),
            ("Glob", serde_json::json!({"pattern": "*.rs"})),
            ("Grep", serde_json::json!({"pattern": "fn"})),
            ("TodoWrite", serde_json::json!({"todos": []})),
            ("AskUserQuestion", serde_json::json!({"questions": []})),
            ("ask_user_question", serde_json::json!({"questions": []})),
            ("WebSearch", serde_json::json!({"query": "rust"})),
            ("WebFetch", serde_json::json!({"url": "https://example.com"})),
            ("ScheduleWakeup", serde_json::json!({"delay_seconds": 60, "reason": "test"})),
            ("CronCreate", serde_json::json!({"cron": "* * * * *", "prompt": "test"})),
            ("CronList", serde_json::json!({})),
            ("CronDelete", serde_json::json!({"id": "abc"})),
            ("Task", serde_json::json!({"description": "test"})),
            ("Agent", serde_json::json!({"description": "test"})),
            ("TaskCreate", serde_json::json!({"description": "test"})),
            ("TaskUpdate", serde_json::json!({"description": "test"})),
            ("TaskList", serde_json::json!({"description": "test"})),
            ("TaskOutput", serde_json::json!({"description": "test"})),
            ("TaskStop", serde_json::json!({"description": "test"})),
            ("TeamCreate", serde_json::json!({"name": "team-a"})),
            ("TeamDelete", serde_json::json!({"name": "team-a"})),
            ("SendMessage", serde_json::json!({"message": "hi"})),
            ("Skill", serde_json::json!({"skill": "build"})),
            ("ExitPlanMode", serde_json::json!({})),
            (
                "Monitor",
                serde_json::json!({"description": "w", "timeout_ms": 1000, "persistent": false, "command": "ls"}),
            ),
        ];

        for (name, input) in &tool_cases {
            let item = tool_use(name, input.clone());
            let result = ToolInput::from_content_item(&item);
            assert!(
                result.is_some(),
                "ToolInput::from_content_item returned None for tool '{}'",
                name
            );

            let ti = result.unwrap();
            assert!(
                !matches!(ti, ToolInput::Generic { .. }),
                "Tool '{}' fell through to Generic with input {:?}",
                name,
                input
            );
        }

        // A genuinely unknown tool should hit Generic.
        let item = tool_use("TotallyUnknownTool", serde_json::json!({"x": 1}));
        let ti = ToolInput::from_content_item(&item).unwrap();
        assert!(
            matches!(ti, ToolInput::Generic { .. }),
            "Unknown tool should fall through to Generic"
        );
    }

    /// All Task* tool names must produce ToolInput::Task.
    #[test]
    fn task_variant_names_map_to_task() {
        for name in
            &["Task", "Agent", "TaskCreate", "TaskUpdate", "TaskList", "TaskOutput", "TaskStop"]
        {
            let item = tool_use(name, serde_json::json!({"description": "do stuff"}));
            let ti = ToolInput::from_content_item(&item).unwrap();
            assert!(
                matches!(ti, ToolInput::Task(_)),
                "Tool '{}' should map to ToolInput::Task",
                name
            );
        }
    }

    /// Team* tool names must produce ToolInput::Team.
    #[test]
    fn team_variant_names_map_to_team() {
        for name in &["TeamCreate", "TeamDelete"] {
            let item = tool_use(name, serde_json::json!({"name": "my-team"}));
            let ti = ToolInput::from_content_item(&item).unwrap();
            assert!(
                matches!(ti, ToolInput::Team(_)),
                "Tool '{}' should map to ToolInput::Team",
                name
            );
        }
    }

    #[test]
    fn from_content_item_with_non_tool_use_returns_none() {
        let text = ContentItem::Text {
            text: "hello".into(),
        };
        assert!(ToolInput::from_content_item(&text).is_none());

        let thinking = ContentItem::Thinking {
            thinking: "hmm".into(),
            signature: None,
        };
        assert!(ToolInput::from_content_item(&thinking).is_none());
    }

    #[test]
    fn malformed_input_falls_through_to_generic() {
        // "Bash" tool with missing required "command" field should fall to Generic.
        let item = tool_use("Bash", serde_json::json!({"not_command": "x"}));
        let ti = ToolInput::from_content_item(&item).unwrap();
        assert!(matches!(ti, ToolInput::Generic { .. }));

        // "Read" tool with missing required "file_path" field.
        let item = tool_use("Read", serde_json::json!({"not_file_path": "x"}));
        let ti = ToolInput::from_content_item(&item).unwrap();
        assert!(matches!(ti, ToolInput::Generic { .. }));
    }

    #[test]
    fn display_name_all_variants() {
        let cases: Vec<(ToolInput, &str)> = vec![
            (
                ToolInput::Bash(BashInput {
                    command: "ls".into(),
                    description: None,
                    run_in_background: None,
                    timeout: None,
                    dangerously_disable_sandbox: None,
                }),
                "Bash",
            ),
            (
                ToolInput::Read(ReadInput {
                    file_path: "f".into(),
                    offset: None,
                    limit: None,
                    pages: None,
                }),
                "Read",
            ),
            (
                ToolInput::Write(WriteInput {
                    file_path: "f".into(),
                    content: "c".into(),
                }),
                "Write",
            ),
            (
                ToolInput::Edit(EditInput {
                    file_path: "f".into(),
                    old_string: "a".into(),
                    new_string: "b".into(),
                    replace_all: false,
                }),
                "Edit",
            ),
            (
                ToolInput::MultiEdit(MultiEditInput {
                    file_path: "f".into(),
                    edits: vec![],
                }),
                "MultiEdit",
            ),
            (
                ToolInput::Glob(GlobInput {
                    pattern: "*.rs".into(),
                    path: None,
                }),
                "Glob",
            ),
            (
                ToolInput::Grep(GrepInput {
                    pattern: "fn".into(),
                    path: None,
                    include: None,
                }),
                "Grep",
            ),
            (ToolInput::TodoWrite(TodoWriteInput { todos: vec![] }), "TodoWrite"),
            (
                ToolInput::AskUserQuestion(AskUserQuestionInput {
                    questions: vec![],
                    answers: None,
                }),
                "AskUserQuestion",
            ),
            (
                ToolInput::WebSearch(WebSearchInput {
                    query: "rust".into(),
                    allowed_domains: None,
                    blocked_domains: None,
                }),
                "WebSearch",
            ),
            (
                ToolInput::WebFetch(WebFetchInput {
                    url: "https://example.com".into(),
                    prompt: None,
                }),
                "WebFetch",
            ),
            (
                ToolInput::ScheduleWakeup(ScheduleWakeupInput {
                    delay_seconds: 60,
                    reason: "test".into(),
                    prompt: None,
                }),
                "ScheduleWakeup",
            ),
            (
                ToolInput::CronCreate(CronCreateInput {
                    cron: "* * * * *".into(),
                    prompt: "test".into(),
                    recurring: false,
                    durable: false,
                }),
                "CronCreate",
            ),
            (ToolInput::CronDelete(CronDeleteInput { id: "abc".into() }), "CronDelete"),
            (ToolInput::CronList(CronListInput {}), "CronList"),
            (
                ToolInput::Monitor(MonitorInput {
                    description: "w".into(),
                    timeout_ms: 1000,
                    persistent: false,
                    command: "ls".into(),
                }),
                "Monitor",
            ),
            (
                ToolInput::Task(TaskInput {
                    description: Some("d".into()),
                    prompt: None,
                    subagent_type: None,
                    model: None,
                    task_id: None,
                    subject: None,
                    status: None,
                    extra: serde_json::Value::Object(Default::default()),
                }),
                "Task",
            ),
            (
                ToolInput::Team(TeamInput {
                    name: Some("t".into()),
                    description: None,
                    extra: serde_json::Value::Object(Default::default()),
                }),
                "Team",
            ),
            (
                ToolInput::SendMessage(SendMessageInput {
                    message: "hi".into(),
                    agent_id: None,
                }),
                "SendMessage",
            ),
            (
                ToolInput::Skill(SkillInput {
                    skill: "build".into(),
                    args: None,
                }),
                "Skill",
            ),
            (
                ToolInput::ExitPlanMode(ExitPlanModeInput {
                    allowed_prompts: None,
                }),
                "ExitPlanMode",
            ),
            (
                ToolInput::Generic {
                    name: "CustomTool".into(),
                    input: serde_json::json!({}),
                },
                "CustomTool",
            ),
        ];

        for (ti, expected_name) in &cases {
            assert_eq!(ti.display_name(), *expected_name, "display_name mismatch for variant");
        }
    }

    #[test]
    fn deserialize_tool_input_structs_bash_all_fields() {
        let json = serde_json::json!({
            "command": "cargo build",
            "description": "Build the project",
            "run_in_background": true,
            "timeout": 30000,
            "dangerously_disable_sandbox": false
        });
        let bi: BashInput = serde_json::from_value(json).unwrap();
        assert_eq!(bi.command, "cargo build");
        assert_eq!(bi.description.as_deref(), Some("Build the project"));
        assert_eq!(bi.run_in_background, Some(true));
        assert_eq!(bi.timeout, Some(30000));
        assert_eq!(bi.dangerously_disable_sandbox, Some(false));
    }

    #[test]
    fn deserialize_tool_input_structs_read_with_pages() {
        let json = serde_json::json!({
            "file_path": "doc.pdf",
            "pages": "1-5"
        });
        let ri: ReadInput = serde_json::from_value(json).unwrap();
        assert_eq!(ri.file_path, "doc.pdf");
        assert_eq!(ri.pages.as_deref(), Some("1-5"));
        assert!(ri.offset.is_none());
    }

    #[test]
    fn deserialize_tool_input_structs_web_search_with_domains() {
        let json = serde_json::json!({
            "query": "rust lang",
            "allowed_domains": ["docs.rs", "crates.io"],
            "blocked_domains": ["example.com"]
        });
        let ws: WebSearchInput = serde_json::from_value(json).unwrap();
        assert_eq!(ws.query, "rust lang");
        assert_eq!(
            ws.allowed_domains.as_deref(),
            Some(vec!["docs.rs".to_string(), "crates.io".to_string()]).as_deref()
        );
        assert_eq!(ws.blocked_domains.as_deref(), Some(vec!["example.com".to_string()]).as_deref());
    }

    #[test]
    fn deserialize_tool_input_structs_ask_user_question() {
        let json = serde_json::json!({
            "questions": [
                {
                    "question": "What is your preference?",
                    "header": "Preference",
                    "options": [
                        {"label": "Option A", "description": "First option"},
                        {"label": "Option B", "description": "Second option"}
                    ],
                    "multi_select": true
                }
            ],
            "answers": {"0": "Option A"}
        });
        let aq: AskUserQuestionInput = serde_json::from_value(json).unwrap();
        assert_eq!(aq.questions.len(), 1);
        assert_eq!(aq.questions[0].header, "Preference");
        assert!(aq.questions[0].multi_select);
        assert_eq!(aq.questions[0].options.len(), 2);
        assert!(aq.answers.is_some());
    }

    #[test]
    fn deserialize_tool_input_structs_task_with_extra_fields() {
        let json = serde_json::json!({
            "description": "search codebase",
            "subagent_type": "Explore",
            "model": "sonnet",
            "run_in_background": true
        });
        let ti: TaskInput = serde_json::from_value(json).unwrap();
        assert_eq!(ti.description.as_deref(), Some("search codebase"));
        assert_eq!(ti.subagent_type.as_deref(), Some("Explore"));
        assert_eq!(ti.model.as_deref(), Some("sonnet"));
        assert_eq!(ti.extra["run_in_background"], true);
    }

    #[test]
    fn deserialize_tool_input_structs_schedule_wakeup() {
        let json = serde_json::json!({
            "delay_seconds": 120,
            "reason": "check back in 2 minutes",
            "prompt": "continue the task"
        });
        let sw: ScheduleWakeupInput = serde_json::from_value(json).unwrap();
        assert_eq!(sw.delay_seconds, 120);
        assert_eq!(sw.reason, "check back in 2 minutes");
        assert_eq!(sw.prompt.as_deref(), Some("continue the task"));
    }

    #[test]
    fn from_name_and_input_direct_call() {
        // Test from_name_and_input directly (not through ContentItem).
        let ti = ToolInput::from_name_and_input("Bash", serde_json::json!({"command": "ls"}));
        assert!(matches!(ti, ToolInput::Bash(_)));

        let ti = ToolInput::from_name_and_input("FutureTool", serde_json::json!({"x": 1}));
        assert!(matches!(ti, ToolInput::Generic { .. }));
    }
}
