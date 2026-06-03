//! Unified line-level diff rendering via [`similar`].
//!
//! Produces a single-column unified diff (red `-` / green `+` / context)
//! and a change-summary header with counts.

use similar::{ChangeTag, TextDiff};

/// Output of a unified diff render.
pub struct DiffOutput {
    /// The rendered diff HTML.
    pub html: String,
    /// Number of added lines.
    pub added: usize,
    /// Number of removed lines.
    pub removed: usize,
}

/// Render a unified line-level diff as HTML.
///
/// HTML structure (locked per spec):
/// ```html
/// <div class="diff">
///   <div class="diff-line diff-line--del">- old</div>
///   <div class="diff-line diff-line--add">+ new</div>
///   <div class="diff-line diff-line--ctx">  context</div>
/// </div>
/// ```
///
/// No character-level highlighting. Line contents are HTML-escaped.
pub fn render_unified_diff(old: &str, new: &str) -> DiffOutput {
    let diff = TextDiff::from_lines(old, new);
    let mut html = String::from(r#"<div class="diff">"#);
    let mut added = 0usize;
    let mut removed = 0usize;

    for change in diff.iter_all_changes() {
        let escaped = html_escape(change.value());
        match change.tag() {
            ChangeTag::Equal => {
                html.push_str(&format!(
                    r#"<div class="diff-line diff-line--ctx"> {}</div>"#,
                    escaped
                ));
            }
            ChangeTag::Delete => {
                removed += 1;
                html.push_str(&format!(
                    r#"<div class="diff-line diff-line--del">-{}</div>"#,
                    escaped
                ));
            }
            ChangeTag::Insert => {
                added += 1;
                html.push_str(&format!(
                    r#"<div class="diff-line diff-line--add">+{}</div>"#,
                    escaped
                ));
            }
        }
    }

    html.push_str("</div>");

    DiffOutput {
        html,
        added,
        removed,
    }
}

/// Render a change-summary header: `Added X lines, removed Y lines`.
///
/// Pluralization is handled: `1 line` vs `N lines`.
pub fn render_change_summary(added: usize, removed: usize) -> String {
    let added_label = if added == 1 { "1 line".to_string() } else { format!("{} lines", added) };
    let removed_label =
        if removed == 1 { "1 line".to_string() } else { format!("{} lines", removed) };
    format!(
        r#"<div class="diff-summary">Added {}, removed {}</div>"#,
        added_label, removed_label
    )
}

fn html_escape(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // B1 tests

    #[test]
    fn identical_old_new_produces_zero_changes() {
        let old = "hello\nworld\n";
        let new = "hello\nworld\n";
        let out = render_unified_diff(old, new);
        assert_eq!(out.added, 0);
        assert_eq!(out.removed, 0);
        assert!(!out.html.contains("diff-line--add"));
        assert!(!out.html.contains("diff-line--del"));
        assert!(out.html.contains("diff-line--ctx"));
    }

    #[test]
    fn pure_addition_produces_only_add_rows() {
        let old = "";
        let new = "line one\nline two\n";
        let out = render_unified_diff(old, new);
        assert_eq!(out.added, 2);
        assert_eq!(out.removed, 0);
        assert!(out.html.contains("diff-line--add"));
        assert!(!out.html.contains("diff-line--del"));
        assert!(out.html.contains("+line one"));
        assert!(out.html.contains("+line two"));
    }

    #[test]
    fn pure_deletion_produces_only_del_rows() {
        let old = "remove me\nand me\n";
        let new = "";
        let out = render_unified_diff(old, new);
        assert_eq!(out.added, 0);
        assert_eq!(out.removed, 2);
        assert!(!out.html.contains("diff-line--add"));
        assert!(out.html.contains("diff-line--del"));
        assert!(out.html.contains("-remove me"));
        assert!(out.html.contains("-and me"));
    }

    #[test]
    fn mixed_change_counts_match() {
        let old = "keep\nremove\nkeep\n";
        let new = "keep\nkeep\nadded\n";
        let out = render_unified_diff(old, new);
        // 'remove' → 1 deletion, 'added' → 1 insertion
        assert_eq!(out.added, 1);
        assert_eq!(out.removed, 1);
        assert!(out.html.contains("diff-line--add"));
        assert!(out.html.contains("diff-line--del"));
        assert!(out.html.contains("diff-line--ctx"));
    }

    #[test]
    fn html_special_chars_are_escaped() {
        let old = "<script>alert('xss')</script>\n";
        let new = "<div>safe</div>\n";
        let out = render_unified_diff(old, new);
        assert!(!out.html.contains("<script>"));
        assert!(out.html.contains("&lt;script&gt;"));
        assert!(!out.html.contains("<div>"));
        assert!(out.html.contains("&lt;div&gt;"));
        // ampersand should be escaped too
        let out2 = render_unified_diff("a & b\n", "a && b\n");
        assert!(out2.html.contains("&amp;"));
    }

    #[test]
    fn pluralization_one_line() {
        let s = render_change_summary(1, 0);
        assert_eq!(s, r#"<div class="diff-summary">Added 1 line, removed 0 lines</div>"#);

        let s = render_change_summary(0, 1);
        assert_eq!(s, r#"<div class="diff-summary">Added 0 lines, removed 1 line</div>"#);
    }

    #[test]
    fn pluralization_n_lines() {
        let s = render_change_summary(3, 2);
        assert_eq!(s, r#"<div class="diff-summary">Added 3 lines, removed 2 lines</div>"#);
    }

    #[test]
    fn diff_output_html_contains_expected_structure() {
        let out = render_unified_diff("a\n", "b\n");
        assert!(out.html.starts_with(r#"<div class="diff">"#));
        assert!(out.html.ends_with("</div>"));
    }
}
