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
