//! Per-project combined page rendering.
//!
//! Builds an askama context for `templates/project.html` listing all sessions
//! in a project with links to individual session HTML files.

use askama::Template;
use chrono::{DateTime, Utc};

/// Context for rendering a per-project `combined_transcripts.html`.
#[derive(Template)]
#[template(path = "project.html")]
pub struct ProjectContext {
    pub css: String,
    pub version: String,
    pub project_name: String,
    pub session_count: u32,
    pub message_count: u32,
    pub token_total: String,
    pub sessions: Vec<SessionCard>,
}

pub struct SessionCard {
    pub id: String,
    pub filename: String,
    pub title: String,
    pub message_count: u32,
    pub token_total: String,
    pub first_user_prompt: Option<String>,
    pub started_at: Option<String>,
    /// Human-readable relative timestamp (e.g. "2h ago", "May 24").
    pub relative_time: String,
}

/// Build a [`ProjectContext`] from session metadata.
pub fn build_context(
    css: String,
    project_name: String,
    sessions: Vec<super::ProjectSessionData>,
) -> ProjectContext {
    let session_count = sessions.len() as u32;
    let message_count: u32 = sessions.iter().map(|s| s.message_count).sum();
    let total_tokens: u64 = sessions.iter().map(|s| s.total_tokens).sum();

    let session_cards: Vec<SessionCard> = sessions
        .into_iter()
        .map(|s| {
            let relative_time = s
                .started_at
                .as_deref()
                .map(format_relative_time)
                .unwrap_or_else(|| "—".to_string());
            SessionCard {
                filename: format!("{}.html", s.id),
                title: s.title.unwrap_or_else(|| s.id.clone()),
                id: s.id,
                message_count: s.message_count,
                token_total: format_token_count(s.total_tokens),
                first_user_prompt: s.first_user_prompt,
                started_at: s.started_at,
                relative_time,
            }
        })
        .collect();

    ProjectContext {
        css,
        version: env!("CARGO_PKG_VERSION").to_string(),
        project_name,
        session_count,
        message_count,
        token_total: format_token_count(total_tokens),
        sessions: session_cards,
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

/// Format an ISO-8601 timestamp as a human-readable relative time.
///
/// - `< 1m ago` for under a minute
/// - `Xm ago` for under an hour
/// - `Xh ago` for under 24 hours
/// - `Mon DD` for older dates
pub(crate) fn format_relative_time(iso: &str) -> String {
    let ts = match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return "—".to_string(),
    };

    let now = Utc::now();
    let dur = now.signed_duration_since(ts);

    if dur.num_minutes() < 1 {
        "< 1m ago".to_string()
    } else if dur.num_hours() < 1 {
        format!("{}m ago", dur.num_minutes())
    } else if dur.num_days() < 1 {
        format!("{}h ago", dur.num_hours())
    } else {
        ts.format("%b %d").to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_project_context_is_self_contained() {
        let css = crate::assets::CSS.to_string();
        let ctx = build_context(
            css.clone(),
            "my-app".into(),
            vec![super::super::ProjectSessionData {
                id: "sess-1".into(),
                title: Some("Test Chat".into()),
                message_count: 12,
                total_tokens: 3400,
                first_user_prompt: Some("Help me build an app".into()),
                started_at: Some("2025-06-15T10:30:00Z".into()),
            }],
        );
        let html = ctx.render().expect("template should render");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("my-app"));
        assert!(html.contains("sess-1.html"));
        assert!(html.contains("Help me build an app"));
        // Relative time should be present (the exact value depends on now,
        // but it should contain the expected markers).
        assert!(
            html.contains("h ago")
                || html.contains("m ago")
                || html.contains("Jun")
                || html.contains("—"),
            "relative time should be present"
        );
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
    }

    #[test]
    fn format_relative_time_returns_dash_on_parse_failure() {
        assert_eq!(format_relative_time("not-a-date"), "—");
    }

    #[test]
    fn format_relative_time_handles_recent_timestamp() {
        // A timestamp just a few seconds ago should produce "< 1m ago".
        let recent = chrono::Utc::now() - chrono::Duration::seconds(30);
        let iso = recent.to_rfc3339();
        let result = format_relative_time(&iso);
        assert!(
            result.contains("ago"),
            "expected relative time for recent timestamp, got: {result}"
        );
    }

    #[test]
    fn format_relative_time_handles_old_timestamp() {
        // A timestamp months ago should produce a "Mon DD" format.
        let old = chrono::Utc::now() - chrono::Duration::days(60);
        let iso = old.to_rfc3339();
        let result = format_relative_time(&iso);
        // Should be in "%b %d" format, not "Xh ago" or "—".
        assert!(!result.contains("ago"), "old timestamp should not say 'ago', got: {result}");
        assert_ne!(result, "—", "old timestamp should not be dash");
    }
}
