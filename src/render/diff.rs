//! Side-by-side diff rendering via [`similar`].
//!
//! Computes per-line diffs from `old_string` / `new_string` and emits
//! a two-column HTML table with red/green highlights.

use similar::{ChangeTag, TextDiff};

/// Maximum lines to show before collapsing the diff.
const COLLAPSE_LIMIT: usize = 200;

/// Render a side-by-side diff as an HTML string.
///
/// Returns the HTML for the diff container with old/new sides.
/// If the diff exceeds [`COLLAPSE_LIMIT`] lines, a "truncated" notice
/// is appended instead.
pub fn render_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);

    let total_changes =
        diff.ops().iter().map(|op| op.old_range().len().max(op.new_range().len())).sum::<usize>();

    if total_changes > COLLAPSE_LIMIT {
        return format!(
            r#"<div class="diff-truncated">Diff too large ({} lines changed). Truncated to {COLLAPSE_LIMIT} lines.</div>"#,
            total_changes
        );
    }

    let mut old_side = String::new();
    let mut new_side = String::new();

    for change in diff.iter_all_changes() {
        let line = html_escape(change.value());
        match change.tag() {
            ChangeTag::Equal => {
                old_side.push_str(&format!(
                    r#"<div class="diff-line diff-line--equal"><span class="diff-marker"> </span>{}</div>"#,
                    line
                ));
                new_side.push_str(&format!(
                    r#"<div class="diff-line diff-line--equal"><span class="diff-marker"> </span>{}</div>"#,
                    line
                ));
            }
            ChangeTag::Delete => {
                old_side.push_str(&format!(
                    r#"<div class="diff-line diff-line--removed"><span class="diff-marker">-</span>{}</div>"#,
                    line
                ));
            }
            ChangeTag::Insert => {
                new_side.push_str(&format!(
                    r#"<div class="diff-line diff-line--added"><span class="diff-marker">+</span>{}</div>"#,
                    line
                ));
            }
        }
    }

    format!(
        r#"<div class="diff-container">
<div class="diff-side diff-side--old">{old}</div>
<div class="diff-side diff-side--new">{new}</div>
</div>"#,
        old = old_side,
        new = new_side,
    )
}

fn html_escape(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_diff() {
        let old = "hello\nworld\n";
        let new = "hello\nrust\n";
        let html = render_diff(old, new);
        assert!(html.contains("diff-container"));
        assert!(html.contains("diff-line--removed"));
        assert!(html.contains("diff-line--added"));
        assert!(html.contains("world"));
        assert!(html.contains("rust"));
    }

    #[test]
    fn unicode_safe() {
        let old = "héllo α\nwörld β\n";
        let new = "héllo α\nrüst γ\n";
        let html = render_diff(old, new);
        assert!(html.contains("héllo α"));
        assert!(html.contains("rüst γ"));
        assert!(html.contains("wörld β"));
    }

    #[test]
    fn no_diff() {
        let old = "same\n";
        let new = "same\n";
        let html = render_diff(old, new);
        assert!(!html.contains("diff-line--removed"));
        assert!(!html.contains("diff-line--added"));
    }
}
