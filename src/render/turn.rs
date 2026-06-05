//! v3 flat-timeline renderers for user and assistant text events.
//!
//! The v2 grouped-turn renderers (`render_turn_group`, `render_assistant_turn`,
//! `render_thinking_pill`, `render_tools_pill`, `render_sub_agent`) have been
//! removed. The primary rendering path now uses [`flatten_to_timeline`] and
//! dispatches each [`TimelineEvent`] to the appropriate renderer.
//!
//! [`flatten_to_timeline`]: crate::conversation::flatten_to_timeline
//! [`TimelineEvent`]: crate::conversation::TimelineEvent

use crate::conversation::UserTurn;
use crate::model::content::ImageSource;
use crate::render::markdown;
use crate::render::tools;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a user message as a muted block (soft faded bubble, no dot).
///
/// `card_id` is used to generate unique IDs for image thumbnail templates.
/// Used in the v3 flat timeline. User messages are visually distinct from
/// the dot-row pattern — they get a block/bubble style with no leading dot.
pub fn render_user_block(ut: &UserTurn, card_id: &str) -> String {
    let body =
        if ut.message.trim().is_empty() { String::new() } else { markdown::render(&ut.message) };
    let images_html = if ut.images.is_empty() {
        String::new()
    } else {
        tools::render_images_thumbnail_row(&ut.images, card_id)
    };
    format!(r#"<div class="user-block">{body}{images_html}</div>"#)
}

/// Render an assistant text block as a gray dot-row.
///
/// Used in the v3 flat timeline. The leading dot is gray (`dot--assistant`),
/// and the text is rendered as prose via markdown.
pub fn render_assistant_text_row(text: &str, images: &[ImageSource]) -> String {
    let body = markdown::render(text);
    let images_html: String = images.iter().map(tools::render_image).collect::<Vec<_>>().join("\n");
    format!(
        r#"<div class="timeline-row timeline-row--text"><div class="dot {dot}"></div><div class="row-body">{body}{images_html}</div></div>"#,
        dot = tools::DOT_ASSISTANT,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::UserTurn;
    use chrono::TimeZone;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2025, 6, 15, 10, 30, 5).unwrap()
    }

    // T4 tests ---------------------------------------------------------------

    #[test]
    fn user_block_has_user_block_class_and_message() {
        let ut = UserTurn {
            message: "Hello, Claude!".to_string(),
            timestamp: ts(),
            images: vec![],
        };
        let html = render_user_block(&ut, "test-card");
        assert!(html.contains(r#"class="user-block""#), "must have user-block class");
        assert!(html.contains("Hello, Claude!"), "message text must appear");
    }

    #[test]
    fn user_block_has_no_dot() {
        let ut = UserTurn {
            message: "hi".to_string(),
            timestamp: ts(),
            images: vec![],
        };
        let html = render_user_block(&ut, "test-card");
        assert!(!html.contains("dot--"), "user block must not have a dot");
        assert!(!html.contains("timeline-row"), "user block is not a dot-row");
    }

    #[test]
    fn user_block_whitespace_only_message_produces_no_body() {
        let ut = UserTurn {
            message: "   ".to_string(),
            timestamp: ts(),
            images: vec![],
        };
        let html = render_user_block(&ut, "test-card");
        assert!(html.contains(r#"class="user-block""#));
        // Empty body → no paragraph tag.
        assert!(!html.contains("<p>"), "whitespace-only message should not produce a paragraph");
    }

    #[test]
    fn assistant_text_row_uses_gray_dot_and_timeline_row() {
        let html = render_assistant_text_row("Hi there", &[]);
        assert!(html.contains("dot--assistant"), "must use gray dot");
        assert!(html.contains("timeline-row"), "must be a timeline-row");
        assert!(html.contains("Hi there"), "text must appear in output");
        assert!(!html.contains("dot--tool"), "must not use green dot");
    }

    #[test]
    fn assistant_text_row_renders_markdown() {
        let html = render_assistant_text_row("**bold** text", &[]);
        assert!(html.contains("<strong>"), "markdown should be rendered to HTML");
    }

    // T23 — user-attached images use thumbnail row + modal ----------------

    #[test]
    fn user_block_images_use_thumbnail_row_with_modal() {
        let ut = UserTurn {
            message: "see attached".to_string(),
            timestamp: ts(),
            images: vec![crate::model::content::ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: "abc123".to_string(),
            }],
        };
        let html = render_user_block(&ut, "msg-7");
        // Must use thumbnail container (horizontal strip), not plain <img>.
        assert!(html.contains("img-thumbnails"), "must use thumbnail container");
        assert!(html.contains("img-thumb-btn"), "must use thumbnail buttons");
        assert!(html.contains("data-modal="), "thumbnails must trigger modal");
        assert!(html.contains("<template"), "must include modal template");
        // Full-size modal image must be present.
        assert!(html.contains("img-modal-full"), "modal must have full-size image");
    }

    #[test]
    fn user_block_no_images_has_no_thumbnail_container() {
        let ut = UserTurn {
            message: "no image".to_string(),
            timestamp: ts(),
            images: vec![],
        };
        let html = render_user_block(&ut, "msg-8");
        assert!(!html.contains("img-thumbnails"), "no thumbnail container without images");
    }

    #[test]
    fn assistant_text_row_uses_row_body_not_row_label() {
        let html = render_assistant_text_row("some text", &[]);
        // Prose goes in row-body, not the monospace row-label.
        assert!(html.contains("row-body"), "prose goes in row-body");
        assert!(!html.contains(r#"class="row-label""#), "must not use monospace row-label");
    }
}
