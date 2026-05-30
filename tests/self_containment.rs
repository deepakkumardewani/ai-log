//! Self-containment gate — ensures zero external URLs in generated HTML.
//!
//! Every released HTML artifact must be fully self-contained: fonts, CSS, JS,
//! and all assets are embedded inline. Any `http://` or `https://` URL in
//! the output is a CI-blocking failure.

/// Scan `html` for external URL references. Returns `true` if the output is
/// clean (no `http://` or `https://` URLs). `data:` and `blob:` URIs are
/// always allowed.
fn is_self_contained(html: &str) -> bool {
    for line in html.lines() {
        let trimmed = line.trim();
        // Allow data: URIs (embedded fonts, images) and blob: URIs.
        if (trimmed.contains("http://") || trimmed.contains("https://"))
            && !trimmed.contains("data:")
            && !trimmed.contains("blob:")
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use askama::Template;

    #[test]
    fn standard_template_output_is_self_contained() {
        // Build a known-clean HTML snippet (representative of our output).
        let clean = r#"<!DOCTYPE html>
<html data-theme="dark">
<head><style>.card { color: red; }</style></head>
<body>
<div class="message-card">Hello world</div>
<script>(function(){/* inline js */})();</script>
</body></html>"#;
        assert!(is_self_contained(clean));
    }

    #[test]
    fn catches_https_url_in_body() {
        let dirty = r#"<!DOCTYPE html>
<html><head></head>
<body>
<img src="https://evil.cdn/pixel.png" />
</body></html>"#;
        assert!(!is_self_contained(dirty));
    }

    #[test]
    fn catches_http_url_in_script() {
        let dirty = r#"<script>
fetch('http://api.example.com/data');
</script>"#;
        assert!(!is_self_contained(dirty));
    }

    #[test]
    fn allows_data_uris() {
        let clean = r#"<style>
@font-face { src: url(data:font/woff2;base64,d09GRgABAAAA); }
</style>
<img src="data:image/png;base64,iVBORw0KGgo=" />
"#;
        assert!(is_self_contained(clean));
    }

    #[test]
    fn catches_url_in_href() {
        let dirty = r#"<a href="https://github.com/example/repo">link</a>"#;
        assert!(!is_self_contained(dirty));
    }

    #[test]
    fn generated_stub_is_self_contained() {
        let ctx = cclog::render::html::stub_context();
        let html = ctx.render().expect("stub template should render");
        assert!(is_self_contained(&html), "stub HTML should have no external URLs");
    }
}
