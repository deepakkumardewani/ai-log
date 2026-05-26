//! Markdown-to-HTML rendering via [`comrak`].
//!
//! Code fences are rendered as plain `<pre><code>` blocks with a `data-lang`
//! attribute for optional client-side highlighting.

use comrak::{format_html, parse_document, Arena, ComrakOptions};

/// Render a markdown string to HTML.
pub fn render(input: &str) -> String {
    let arena = Arena::new();
    let options = comrak_options();
    let root = parse_document(&arena, input, &options);

    let mut html = vec![];
    format_html(root, &options, &mut html).unwrap();
    String::from_utf8_lossy(&html).into_owned()
}

/// Render inline markdown (no block elements) to HTML.
pub fn render_inline(input: &str) -> String {
    let arena = Arena::new();
    let mut options = comrak_options();
    options.extension.table = false;
    options.extension.tasklist = false;

    let root = parse_document(&arena, input, &options);

    let mut html = vec![];
    for node in root.children() {
        format_html(node, &options, &mut html).unwrap();
    }
    String::from_utf8_lossy(&html).into_owned()
}

fn comrak_options() -> ComrakOptions {
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tagfilter = true;
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_plain_text() {
        let html = render("Hello, world!");
        assert!(html.contains("<p>Hello, world!</p>"));
    }

    #[test]
    fn render_code_fence() {
        let html = render("```rust\nfn main() {}\n```");
        assert!(html.contains("<code"));
        assert!(html.contains("fn main"));
    }

    #[test]
    fn render_inline_plain() {
        let html = render_inline("simple text");
        assert!(html.contains("simple text"));
    }
}
