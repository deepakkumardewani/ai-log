//! Syntax highlighting via [`syntect`].
//!
//! The syntax set and theme are loaded once and cached behind a `OnceLock`,
//! so cold-start cost is paid only on the first code fence rendered.

use std::sync::OnceLock;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

/// Highlight a code string for the given language.
///
/// Returns an HTML `<pre>` block with inline styles. Falls back to
/// plain `<pre><code>` if the language is unknown or highlighting fails.
pub fn highlight(code: &str, language: &str) -> String {
    let ss = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);

    let syntax = ss
        .find_syntax_by_token(language)
        .or_else(|| ss.find_syntax_by_extension(language))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = &ts.themes["base16-ocean.dark"];

    match highlighted_html_for_string(code, ss, syntax, theme) {
        Ok(html) => html,
        Err(_) => {
            // Fallback: escape and wrap.
            let escaped = html_escape(code);
            format!("<pre style=\"background:#1a1a1a;color:#e0e0e0;padding:12px;border-radius:2px;overflow-x:auto;font-family:var(--font-mono);font-size:12px;line-height:1.5;\"><code>{}</code></pre>", escaped)
        }
    }
}

fn html_escape(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_rust() {
        let html = highlight("fn main() {}", "rust");
        assert!(html.contains("fn"));
        assert!(html.contains("main"));
        assert!(html.contains("<pre"));
    }

    #[test]
    fn highlight_shell() {
        let html = highlight("cargo build", "sh");
        assert!(html.contains("cargo"));
    }

    #[test]
    fn unknown_language_falls_back() {
        let html = highlight("some code", "zzz-unknown-lang");
        assert!(html.contains("<pre"));
        assert!(html.contains("some code"));
    }
}
