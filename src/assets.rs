//! Compile-time asset embedding.
//!
//! CSS, JS, and font files are embedded into the binary at build time,
//! ensuring the generated HTML is fully self-contained with zero CDN references.

/// Compiled CSS, embedded at build time from `assets/tailwind.input.css`.
pub const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/styles.css"));

/// Client-side interactivity script (filter chips, search, scroll-spy, theme).
///
/// Inlined into every transcript HTML page. Under 2 KB gzipped.
pub const TRANSCRIPT_JS: &str = include_str!("../assets/transcript.js");

/// Index page interactivity script (view toggle, search, date filter).
///
/// Inlined into the master index page. Under 2 KB gzipped.
pub const INDEX_JS: &str = include_str!("../assets/index.js");
