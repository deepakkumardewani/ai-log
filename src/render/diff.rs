//! Unified line-level diff rendering with intra-line word/token highlighting.
//!
//! Produces a single-column unified diff (red `-` / green `+` / context)
//! with line numbers, intra-line token highlights on paired del+add lines,
//! and a GitHub-style change-summary header.

use similar::{ChangeTag, TextDiff};

use super::html_escape;

/// Output of a unified diff render.
pub struct DiffOutput {
    /// The rendered diff HTML.
    pub html: String,
    /// Number of added lines.
    pub added: usize,
    /// Number of removed lines.
    pub removed: usize,
}

/// Render a unified diff as HTML.
///
/// Each pair of adjacent (del, add) lines gets intra-line word/token
/// highlights — changed words are wrapped in `diff-tok--del` / `diff-tok--add`
/// spans, making the changed token brighter than the line background.
/// Unpaired del or add lines use plain HTML-escaped content.
///
/// HTML structure:
/// ```html
/// <div class="diff">
///   <div class="diff-line diff-line--del">
///     <span class="ln ln-old">1</span><span class="ln ln-new"></span>
///     -old <span class="diff-tok--del">changed</span> word
///   </div>
///   <div class="diff-line diff-line--add">
///     <span class="ln ln-old"></span><span class="ln ln-new">1</span>
///     +new <span class="diff-tok--add">changed</span> word
///   </div>
///   <div class="diff-line diff-line--ctx">...</div>
/// </div>
/// ```
pub fn render_unified_diff(old: &str, new: &str) -> DiffOutput {
    let diff = TextDiff::from_lines(old, new);
    let changes: Vec<_> = diff.iter_all_changes().collect();

    let mut html = String::from(r#"<div class="diff">"#);
    let mut added = 0usize;
    let mut removed = 0usize;
    let mut i = 0;

    while i < changes.len() {
        let change = &changes[i];
        match change.tag() {
            ChangeTag::Equal => {
                let escaped = html_escape(change.value());
                let old_num = fmt_num(change.old_index());
                let new_num = fmt_num(change.new_index());
                html.push_str(&format!(
                    r#"<div class="diff-line diff-line--ctx"><span class="ln ln-old">{old_num}</span><span class="ln ln-new">{new_num}</span> {escaped}</div>"#
                ));
                i += 1;
            }
            ChangeTag::Delete => {
                removed += 1;
                if i + 1 < changes.len() && changes[i + 1].tag() == ChangeTag::Insert {
                    // Paired del + add → intra-line word/token highlight.
                    added += 1;
                    let add_change = &changes[i + 1];
                    let old_num = fmt_num(change.old_index());
                    let new_num = fmt_num(add_change.new_index());
                    let del_val = strip_newline(change.value());
                    let add_val = strip_newline(add_change.value());
                    let (del_body, add_body) = intra_line_diff(del_val, add_val);
                    html.push_str(&format!(
                        r#"<div class="diff-line diff-line--del"><span class="ln ln-old">{old_num}</span><span class="ln ln-new"></span>-{del_body}</div>"#
                    ));
                    html.push_str(&format!(
                        r#"<div class="diff-line diff-line--add"><span class="ln ln-old"></span><span class="ln ln-new">{new_num}</span>+{add_body}</div>"#
                    ));
                    i += 2;
                } else {
                    let escaped = html_escape(change.value());
                    let old_num = fmt_num(change.old_index());
                    html.push_str(&format!(
                        r#"<div class="diff-line diff-line--del"><span class="ln ln-old">{old_num}</span><span class="ln ln-new"></span>-{escaped}</div>"#
                    ));
                    i += 1;
                }
            }
            ChangeTag::Insert => {
                added += 1;
                let escaped = html_escape(change.value());
                let new_num = fmt_num(change.new_index());
                html.push_str(&format!(
                    r#"<div class="diff-line diff-line--add"><span class="ln ln-old"></span><span class="ln ln-new">{new_num}</span>+{escaped}</div>"#
                ));
                i += 1;
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

/// Render a GitHub-style change-summary header: `+X line(s) · −Y line(s)`.
pub fn render_change_summary(added: usize, removed: usize) -> String {
    let a_word = if added == 1 { "line" } else { "lines" };
    let r_word = if removed == 1 { "line" } else { "lines" };
    format!(
        r#"<div class="diff-summary"><span class="diff-count--add">+{added} {a_word}</span> &middot; <span class="diff-count--del">&minus;{removed} {r_word}</span></div>"#
    )
}

// ---------------------------------------------------------------------------
// Intra-line helpers
// ---------------------------------------------------------------------------

/// Compute word-level intra-line diff spans for a paired (del, add) line.
///
/// Returns `(del_html, add_html)` where changed words are wrapped in
/// `diff-tok--del` / `diff-tok--add` spans for brighter inline highlight.
fn intra_line_diff(del_line: &str, add_line: &str) -> (String, String) {
    let word_diff = TextDiff::from_words(del_line, add_line);
    let mut del_html = String::new();
    let mut add_html = String::new();

    for change in word_diff.iter_all_changes() {
        let escaped = html_escape(change.value());
        match change.tag() {
            ChangeTag::Equal => {
                del_html.push_str(&escaped);
                add_html.push_str(&escaped);
            }
            ChangeTag::Delete => {
                del_html.push_str(&format!(r#"<span class="diff-tok--del">{escaped}</span>"#));
            }
            ChangeTag::Insert => {
                add_html.push_str(&format!(r#"<span class="diff-tok--add">{escaped}</span>"#));
            }
        }
    }

    (del_html, add_html)
}

fn strip_newline(s: &str) -> &str {
    s.trim_end_matches('\n').trim_end_matches('\r')
}

fn fmt_num(idx: Option<usize>) -> String {
    idx.map_or(String::new(), |n| (n + 1).to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // B1 tests — line-level correctness

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
        // Line numbers present on context lines.
        assert!(out.html.contains(r#"class="ln ln-old">1</span>"#));
        assert!(out.html.contains(r#"class="ln ln-new">1</span>"#));
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
        // New line numbers present, old empty.
        assert!(out.html.contains(r#"class="ln ln-old"></span>"#));
        assert!(out.html.contains(r#"class="ln ln-new">1</span>"#));
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
        // Old line numbers present, new empty.
        assert!(out.html.contains(r#"class="ln ln-old">1</span>"#));
        assert!(out.html.contains(r#"class="ln ln-old">2</span>"#));
    }

    #[test]
    fn mixed_change_counts_match() {
        let old = "keep\nremove\nkeep\n";
        let new = "keep\nkeep\nadded\n";
        let out = render_unified_diff(old, new);
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
        let out2 = render_unified_diff("a & b\n", "a && b\n");
        assert!(out2.html.contains("&amp;"));
    }

    #[test]
    fn diff_output_html_contains_expected_structure() {
        let out = render_unified_diff("a\n", "b\n");
        assert!(out.html.starts_with(r#"<div class="diff">"#));
        assert!(out.html.ends_with("</div>"));
        // Paired del + add: old num on del line, new num on add line.
        assert!(out.html.contains(r#"class="ln ln-old">1</span>"#));
        assert!(out.html.contains(r#"class="ln ln-new">1</span>"#));
    }

    // T9 tests — intra-line word/token highlight

    #[test]
    fn paired_del_add_produces_token_spans() {
        let old = "hello world\n";
        let new = "hello earth\n";
        let out = render_unified_diff(old, new);
        // Both del and add lines should contain token highlight spans.
        assert!(out.html.contains("diff-tok--del"), "del line should have token highlight span");
        assert!(out.html.contains("diff-tok--add"), "add line should have token highlight span");
        // Counts are still correct.
        assert_eq!(out.added, 1);
        assert_eq!(out.removed, 1);
    }

    #[test]
    fn unpaired_del_has_no_token_spans() {
        // Two deletions followed by one insertion — second del is not paired.
        let old = "line1\nline2\n";
        let new = "line3\n";
        let out = render_unified_diff(old, new);
        // At least one del line must exist.
        assert!(out.html.contains("diff-line--del"));
        assert_eq!(out.removed, 2);
        assert_eq!(out.added, 1);
    }

    #[test]
    fn change_summary_new_format() {
        let s = render_change_summary(1, 0);
        assert!(s.contains("+1 line"), "added count should appear: {s}");
        assert!(s.contains("0 lines"), "removed count should appear: {s}");
        assert!(s.contains("diff-count--add"), "should have add class: {s}");
        assert!(s.contains("diff-count--del"), "should have del class: {s}");
    }

    #[test]
    fn change_summary_pluralization() {
        let s = render_change_summary(3, 2);
        assert!(s.contains("+3 lines"), "added plural: {s}");
        assert!(s.contains("2 lines"), "removed plural: {s}");

        let s1 = render_change_summary(0, 1);
        assert!(s1.contains("1 line"), "removed singular: {s1}");
    }
}
