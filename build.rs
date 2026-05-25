use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let css_output = out_dir.join("styles.css");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let input_css = manifest_dir.join("assets/tailwind.input.css");
    let _config_js = manifest_dir.join("assets/tailwind.config.js");

    // Try the standalone Tailwind CLI first.
    let tw_result = Command::new("tailwindcss")
        .args(["-i", input_css.to_str().unwrap(), "-o", css_output.to_str().unwrap(), "--minify"])
        .output();

    match tw_result {
        Ok(output) if output.status.success() => {
            println!("cargo:warning=Tailwind CLI: CSS compiled successfully");
        }
        _ => {
            // Fallback: write Material-3 tokens directly as CSS custom properties.
            // The full Tailwind pipeline will be used in CI/release builds.
            println!("cargo:warning=Tailwind CLI not found — using fallback CSS tokens");
            let fallback_css = include_str!("assets/tailwind.input.css");
            std::fs::write(&css_output, fallback_css).unwrap();
        }
    }

    // Re-run build.rs when these assets change.
    println!("cargo:rerun-if-changed=assets/tailwind.input.css");
    println!("cargo:rerun-if-changed=assets/tailwind.config.js");
    println!("cargo:rerun-if-changed=build.rs");
}
