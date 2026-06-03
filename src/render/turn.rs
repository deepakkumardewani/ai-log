//! Turn-based rendering for the conversational transcript view.
//!
//! Replaces the old flat per-message card loop with grouped turn cards:
//! - User turns → soft bubble markup.
//! - Assistant turns → flat card with optional Thinking + Tools pills.

use crate::conversation::{
    AssistantTurn, SubAgentTurn, ThinkingStep, ToolCall, TurnGroup, UserTurn,
};
use crate::render::markdown;
use crate::render::tools;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a single turn group (user or assistant) as an HTML string.
pub fn render_turn_group(turn: &TurnGroup) -> String {
    match turn {
        TurnGroup::User(ut) => render_user_turn(ut),
        TurnGroup::Assistant(at) => render_assistant_turn(at),
    }
}

/// Render a user turn as a soft bubble.
fn render_user_turn(turn: &UserTurn) -> String {
    let time = format_time(&turn.timestamp);
    let body = if turn.message.trim().is_empty() {
        String::new()
    } else {
        markdown::render(&turn.message)
    };
    format!(
        r#"<article class="turn turn--user">
  <header class="turn-header"><time datetime="{iso}">{time}</time></header>
  <div class="turn-body">{body}</div>
</article>"#,
        iso = turn.timestamp.to_rfc3339(),
    )
}

/// Render an assistant turn as a flat structured card.
fn render_assistant_turn(turn: &AssistantTurn) -> String {
    let time = format_time(&turn.timestamp);
    let body = if turn.message_text.trim().is_empty() {
        String::new()
    } else {
        markdown::render(&turn.message_text)
    };

    let thinking_html = render_thinking_pill(&turn.thinking, turn.total_in, turn.total_out);

    let total_calls = turn.tool_calls.len() + turn.sub_agents.len();
    let tools_html = if total_calls > 0 {
        render_tools_pill(&turn.tool_calls, &turn.sub_agents)
    } else {
        String::new()
    };

    format!(
        r#"<article class="turn turn--assistant">
  <header class="turn-header"><time datetime="{iso}">{time}</time></header>
  <div class="turn-body">{body}</div>
  {thinking_html}
  {tools_html}
</article>"#,
        iso = turn.timestamp.to_rfc3339(),
    )
}

// ---------------------------------------------------------------------------
// Thinking pill
// ---------------------------------------------------------------------------

fn render_thinking_pill(thinking: &Option<ThinkingStep>, total_in: u64, total_out: u64) -> String {
    match thinking {
        Some(ts) if ts.text.trim().is_empty() => {
            // Empty thinking → disabled span (no <details>).
            r#"<span class="pill pill--thinking pill--disabled">Thinking</span>"#.to_string()
        }
        Some(ts) => {
            let content_html = html_escape(&ts.text).replace('\n', "<br>");
            format!(
                r#"<details class="pill pill--thinking">
  <summary class="pill-summary">Thinking · {total_in} in · {total_out} out</summary>
  <div class="pill-body thinking-content">{content_html}</div>
</details>"#
            )
        }
        None => {
            // No thinking at all → omit.
            String::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Tools pill
// ---------------------------------------------------------------------------

fn render_tools_pill(tool_calls: &[ToolCall], sub_agents: &[SubAgentTurn]) -> String {
    let total = tool_calls.len() + sub_agents.len();

    let mut tools_html = String::new();

    // Render regular tool calls.
    for tc in tool_calls {
        tools_html.push_str(&tools::render_tool_use(&tc.name, &tc.input, &tc.id));
    }

    // Render sub-agents nested inside.
    for sa in sub_agents {
        tools_html.push_str(&render_sub_agent(sa));
    }

    format!(
        r#"<details class="pill pill--tools">
  <summary class="pill-summary">Tools · {total} calls</summary>
  <div class="pill-body">{tools_html}</div>
</details>"#
    )
}

// ---------------------------------------------------------------------------
// Sub-agent rendering
// ---------------------------------------------------------------------------

fn render_sub_agent(sa: &SubAgentTurn) -> String {
    let mut body = String::new();

    // Sub-agent message text.
    if !sa.message_text.trim().is_empty() {
        body.push_str(&markdown::render(&sa.message_text));
    }

    // Sub-agent thinking pill (reuses same helper).
    let thinking_html = render_thinking_pill(&sa.thinking, 0, 0);

    // Sub-agent tools (recursive).
    let total_sa_calls = sa.tool_calls.len();
    let sa_tools_html =
        if total_sa_calls > 0 { render_tools_pill(&sa.tool_calls, &[]) } else { String::new() };

    let n_calls = total_sa_calls;
    let label = if n_calls > 0 {
        format!("↳ Sub-agent · {} · {} calls", sa.name, n_calls)
    } else {
        format!("↳ Sub-agent · {}", sa.name)
    };

    format!(
        r#"<details class="sub-agent">
  <summary class="sub-agent-summary">{label}</summary>
  <div class="sub-agent-body">{body}{thinking_html}{sa_tools_html}</div>
</details>"#
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_time(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.with_timezone(&chrono::Local).format("%H:%M:%S").to_string()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn test_timestamp() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2025, 6, 15, 10, 30, 5).unwrap()
    }

    // -----------------------------------------------------------------------
    // C1 tests
    // -----------------------------------------------------------------------

    #[test]
    fn user_turn_contains_turn_user_class_and_message() {
        let turn = UserTurn {
            message: "Hello, Claude!".to_string(),
            timestamp: test_timestamp(),
        };
        let html = render_user_turn(&turn);
        assert!(html.contains(r#"class="turn turn--user""#));
        assert!(html.contains("Hello, Claude!"));
    }

    #[test]
    fn assistant_turn_with_thinking_renders_details() {
        let turn = AssistantTurn {
            message_text: "I will help.".to_string(),
            thinking: Some(ThinkingStep {
                text: "Let me think about this...".to_string(),
            }),
            tool_calls: vec![],
            sub_agents: vec![],
            timestamp: test_timestamp(),
            total_in: 150,
            total_out: 80,
        };
        let html = render_assistant_turn(&turn);
        assert!(html.contains(r#"class="pill pill--thinking""#));
        assert!(html.contains("<details"));
        assert!(html.contains("Thinking · 150 in · 80 out"));
        assert!(html.contains("Let me think about this..."));
        assert!(!html.contains("pill--disabled"));
    }

    #[test]
    fn assistant_turn_with_empty_thinking_renders_disabled_span() {
        let turn = AssistantTurn {
            message_text: "Done.".to_string(),
            thinking: Some(ThinkingStep {
                text: String::new(),
            }),
            tool_calls: vec![],
            sub_agents: vec![],
            timestamp: test_timestamp(),
            total_in: 10,
            total_out: 5,
        };
        let html = render_assistant_turn(&turn);
        assert!(html.contains(r#"class="pill pill--thinking pill--disabled""#));
        assert!(html.contains("Thinking"));
        // Must NOT contain a <details> for thinking.
        assert!(!html.contains(r#"<details class="pill pill--thinking""#));
    }

    #[test]
    fn assistant_turn_with_no_thinking_omits_pill() {
        let turn = AssistantTurn {
            message_text: "Quick reply.".to_string(),
            thinking: None,
            tool_calls: vec![],
            sub_agents: vec![],
            timestamp: test_timestamp(),
            total_in: 5,
            total_out: 3,
        };
        let html = render_assistant_turn(&turn);
        assert!(!html.contains("pill--thinking"));
        assert!(!html.contains("Thinking"));
    }

    #[test]
    fn assistant_turn_with_no_tools_omits_tools_pill() {
        let turn = AssistantTurn {
            message_text: "Text only.".to_string(),
            thinking: None,
            tool_calls: vec![],
            sub_agents: vec![],
            timestamp: test_timestamp(),
            total_in: 0,
            total_out: 0,
        };
        let html = render_assistant_turn(&turn);
        assert!(!html.contains("pill--tools"));
        assert!(!html.contains("Tools"));
    }

    #[test]
    fn assistant_turn_header_contains_only_time() {
        let turn = AssistantTurn {
            message_text: "Reply.".to_string(),
            thinking: None,
            tool_calls: vec![],
            sub_agents: vec![],
            timestamp: test_timestamp(),
            total_in: 0,
            total_out: 0,
        };
        let html = render_assistant_turn(&turn);
        // Header should contain time but no date and no token totals.
        assert!(html.contains("<time"));
        assert!(!html.contains("in:"));
        assert!(!html.contains("out:"));
        assert!(!html.contains("Cache"));
        // The visible text of <time> should be HH:MM:SS only, no date.
        let time_start = html.find("<time").unwrap();
        let time_end = html[time_start..].find("</time>").unwrap() + "</time>".len();
        let time_element = &html[time_start..time_start + time_end];
        // Should contain only time, no month abbreviation or year in visible text.
        let inner =
            &time_element[time_element.find('>').unwrap() + 1..time_element.rfind('<').unwrap()];
        assert!(!inner.contains("Jun"), "visible time text should not contain month: {inner}");
        assert!(!inner.contains("2025"), "visible time text should not contain year: {inner}");
    }

    // -----------------------------------------------------------------------
    // C2 tests
    // -----------------------------------------------------------------------

    fn make_sub_agent(name: &str, tool_call_id: &str) -> SubAgentTurn {
        SubAgentTurn {
            tool_call_id: tool_call_id.to_string(),
            name: name.to_string(),
            thinking: None,
            tool_calls: vec![],
            message_text: "Sub-agent output.".to_string(),
        }
    }

    #[test]
    fn sub_agent_renders_nested_details_inside_tools_pill() {
        let sa = make_sub_agent("Task", "task1");
        let turn = AssistantTurn {
            message_text: "Delegating.".to_string(),
            thinking: None,
            tool_calls: vec![],
            sub_agents: vec![sa],
            timestamp: test_timestamp(),
            total_in: 0,
            total_out: 0,
        };
        let html = render_assistant_turn(&turn);
        // Outer Tools pill exists.
        assert!(html.contains(r#"class="pill pill--tools""#));
        assert!(html.contains("Tools · 1 calls"));
        // Sub-agent nested inside.
        assert!(html.contains(r#"class="sub-agent""#));
        assert!(html.contains("↳ Sub-agent · Task"));
        assert!(html.contains("Sub-agent output."));
        // Sub-agent is inside the Tools pill (verified by structural nesting).
        let tools_pos = html.find(r#"class="pill pill--tools""#).unwrap();
        let sub_pos = html.find(r#"class="sub-agent""#).unwrap();
        assert!(sub_pos > tools_pos, "sub-agent should be inside tools pill");
    }

    #[test]
    fn sub_agent_with_thinking_renders_nested_thinking_pill() {
        let sa = SubAgentTurn {
            tool_call_id: "task2".to_string(),
            name: "Agent".to_string(),
            thinking: Some(ThinkingStep {
                text: "Sub-agent reasoning...".to_string(),
            }),
            tool_calls: vec![],
            message_text: "Result.".to_string(),
        };
        let turn = AssistantTurn {
            message_text: "Delegating.".to_string(),
            thinking: None,
            tool_calls: vec![],
            sub_agents: vec![sa],
            timestamp: test_timestamp(),
            total_in: 0,
            total_out: 0,
        };
        let html = render_assistant_turn(&turn);
        // Sub-agent has its own thinking pill.
        let sub_pos = html.find(r#"class="sub-agent""#).unwrap();
        let think_pos = html[sub_pos..].find(r#"class="pill pill--thinking""#);
        assert!(think_pos.is_some(), "sub-agent should contain a nested thinking pill");
        assert!(html.contains("Sub-agent reasoning..."));
    }

    #[test]
    fn sub_agent_nested_inside_tools_details() {
        // Verify that closing the outer Tools <details> hides the sub-agent:
        // the sub-agent <details> is structurally inside the Tools <details>.
        let sa = make_sub_agent("Explore", "exp1");
        let turn = AssistantTurn {
            message_text: "Searching.".to_string(),
            thinking: None,
            tool_calls: vec![ToolCall {
                id: "b1".to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({"command": "ls"}),
            }],
            sub_agents: vec![sa],
            timestamp: test_timestamp(),
            total_in: 0,
            total_out: 0,
        };
        let html = render_assistant_turn(&turn);
        // The entire structure should be: Tools > (Bash card + sub-agent details).
        let tools_open = html.find("<details class=\"pill pill--tools\">").unwrap();
        let tools_close = html[tools_open..].find("</details>").unwrap();
        let tools_end = tools_open + tools_close + "</details>".len();
        let sub_agent_pos = html.find("class=\"sub-agent\"").unwrap();
        assert!(
            sub_agent_pos > tools_open && sub_agent_pos < tools_end,
            "sub-agent must be structurally inside the Tools <details>"
        );
    }

    #[test]
    fn turn_group_dispatches_correctly() {
        let user = TurnGroup::User(UserTurn {
            message: "hi".to_string(),
            timestamp: test_timestamp(),
        });
        let html = render_turn_group(&user);
        assert!(html.contains("turn--user"));

        let assistant = TurnGroup::Assistant(AssistantTurn {
            message_text: "hey".to_string(),
            thinking: None,
            tool_calls: vec![],
            sub_agents: vec![],
            timestamp: test_timestamp(),
            total_in: 0,
            total_out: 0,
        });
        let html = render_turn_group(&assistant);
        assert!(html.contains("turn--assistant"));
    }
}
