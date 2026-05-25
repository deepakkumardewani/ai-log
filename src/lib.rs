// cclog — Claude Code transcript exporter.
// Library crate: models, parser, renderer, cache.
// See agent_docs/rust-spec-v0.1.md for architecture.

pub mod assets;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_css_contains_material3_tokens() {
        let css = assets::CSS;
        assert!(
            css.contains("--surface"),
            "CSS should contain Material-3 token '--surface'. Got: {css}"
        );
    }
}
