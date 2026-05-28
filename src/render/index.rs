//! Master index page rendering.
//!
//! Builds an askama context for `templates/index.html` listing all projects
//! as cards with aggregate totals.

use askama::Template;

/// Context for rendering the master `index.html`.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexContext {
    pub css: String,
    pub version: String,
    pub total_projects: usize,
    pub total_sessions: u32,
    pub total_messages: u32,
    pub total_tokens_display: String,
    pub date_range: String,
    pub projects: Vec<ProjectCard>,
}

pub struct ProjectCard {
    pub name: String,
    pub session_count: u32,
    pub message_count: u32,
    pub token_total: String,
}

/// Build an [`IndexContext`] from cached project metadata.
pub fn build_context(
    css: String,
    projects: Vec<super::IndexProjectData>,
    total_messages: u32,
    total_tokens: u64,
    earliest: Option<String>,
    latest: Option<String>,
) -> IndexContext {
    let date_range = match (earliest.as_deref(), latest.as_deref()) {
        (Some(e), Some(l)) => format!("{} – {}", format_short_date(e), format_short_date(l)),
        (Some(e), None) => format_short_date(e),
        _ => "—".to_string(),
    };

    let project_cards: Vec<ProjectCard> = projects
        .into_iter()
        .map(|p| ProjectCard {
            name: p.name,
            session_count: p.session_count,
            message_count: p.message_count,
            token_total: format_token_count(p.total_tokens),
        })
        .collect();

    let total_projects = project_cards.len();

    IndexContext {
        css,
        version: env!("CARGO_PKG_VERSION").to_string(),
        total_projects,
        total_sessions: project_cards.iter().map(|p| p.session_count).sum(),
        total_messages,
        total_tokens_display: format_token_count(total_tokens),
        date_range,
        projects: project_cards,
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

fn format_short_date(s: &str) -> String {
    // Take just the date part (YYYY-MM-DD) and show as "Mon DD".
    if s.len() >= 10 {
        let date_part = &s[..10];
        if let Ok(d) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            return d.format("%b %d").to_string();
        }
    }
    s.to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_index_context_is_self_contained() {
        let css = crate::assets::CSS.to_string();
        let ctx = build_context(
            css.clone(),
            vec![super::super::IndexProjectData {
                name: "test-proj".into(),
                session_count: 3,
                message_count: 45,
                total_tokens: 15000,
            }],
            45,
            15000,
            Some("2025-06-15T10:00:00Z".into()),
            Some("2025-06-15T12:00:00Z".into()),
        );
        let html = ctx.render().expect("template should render");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("test-proj"));
        assert!(html.contains("15.0k"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
    }
}
