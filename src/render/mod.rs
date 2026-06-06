pub mod diff;
pub mod highlight;
pub mod html;
pub mod index;
pub mod markdown;
pub mod markdown_export;
pub mod pagination;
pub mod project;
pub mod tools;
pub mod turn;

/// Escape `&`, `<`, `>`, and `"` for safe HTML embedding.
///
/// Single-pass: allocates once, scans the input, appends escaped sequences
/// directly. This is measurably faster than chained `.replace()` calls when
/// the input contains many special characters.
pub(crate) fn html_escape(s: &str) -> String {
    // Fast path: most strings have no special characters.
    if !s.contains(['&', '<', '>', '"']) {
        return s.to_string();
    }

    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Data shared between index/project renderers and the CLI all-projects pipeline.
#[derive(Debug, Clone)]
pub struct IndexProjectData {
    pub name: String,
    pub session_count: u32,
    pub message_count: u32,
    pub total_tokens: u64,
    pub short_name: String,
    pub display_name: String,
    pub last_activity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectSessionData {
    pub id: String,
    pub title: Option<String>,
    pub message_count: u32,
    pub total_tokens: u64,
    pub first_user_prompt: Option<String>,
    pub started_at: Option<String>,
}
