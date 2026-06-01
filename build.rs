use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let css_output = out_dir.join("styles.css");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let input_css = manifest_dir.join("assets/tailwind.input.css");
    let _config_js = manifest_dir.join("assets/tailwind.config.js");

    // Try the Tailwind CLI for minification if available; plain CSS is a valid fallback.
    let minified = Command::new("tailwindcss")
        .args(["-i", input_css.to_str().unwrap(), "-o", css_output.to_str().unwrap(), "--minify"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !minified {
        std::fs::copy(&input_css, &css_output).unwrap();
    }

    // Re-run build.rs when these assets change.
    println!("cargo:rerun-if-changed=assets/tailwind.input.css");
    println!("cargo:rerun-if-changed=assets/tailwind.config.js");
    println!("cargo:rerun-if-changed=build.rs");
}
