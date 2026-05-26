// cclog — Claude Code transcript exporter.
// Library crate: models, parser, renderer, cache.
// See agent_docs/rust-spec-v0.1.md for architecture.

pub mod aggregate;
pub mod assets;
pub mod cli;
pub mod model;
pub mod parser;
pub mod render;
pub mod session;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn embedded_css_contains_material3_tokens() {
        let css = assets::CSS;
        assert!(
            css.contains("--surface"),
            "CSS should contain Material-3 token '--surface'. Got: {css}"
        );
    }

    /// A complete linear session JSONL fixture for integration tests.
    pub(crate) fn fixture_linear_session() -> String {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let u2 = "550e8400-e29b-41d4-a716-446655440003";
        let a2 = "550e8400-e29b-41d4-a716-446655440004";
        format!(
            r#"{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"test-session","message":{{"role":"user","content":[{{"type":"text","text":"Build the project"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"test-session","agentId":"claude-opus-4-7","message":{{"role":"assistant","model":"claude-opus-4-7","stopReason":"tool_use","usage":{{"inputTokens":100,"outputTokens":50}},"content":[{{"type":"text","text":"I will build the project."}},{{"type":"tool_use","id":"t1","name":"Bash","input":{{"command":"cargo build"}}}}]}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:31:00Z","sessionId":"test-session","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"t1","content":"Compiling cclog...\nFinished","is_error":false}}]}}}}
{{"type":"assistant","uuid":"{a2}","parentUuid":"{u2}","timestamp":"2025-06-15T10:31:05Z","sessionId":"test-session","agentId":"claude-opus-4-7","message":{{"role":"assistant","model":"claude-opus-4-7","stopReason":"end_turn","usage":{{"inputTokens":80,"outputTokens":40}},"content":[{{"type":"text","text":"Build completed successfully!"}}]}}}}
"#
        )
    }
}
