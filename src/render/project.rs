//! Per-project combined page rendering.
//!
//! Builds an askama context for `templates/project.html` listing all sessions
//! in a project with links to individual session HTML files.

use askama::Template;

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
        .map(|s| SessionCard {
            filename: format!("{}.html", s.id),
            title: s.title.unwrap_or_else(|| s.id.clone()),
            id: s.id,
            message_count: s.message_count,
            token_total: format_token_count(s.total_tokens),
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
            }],
        );
        let html = ctx.render().expect("template should render");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("my-app"));
        assert!(html.contains("sess-1.html"));
        assert!(html.contains("Test Chat"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
    }
}
