use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let input_css = manifest_dir.join("assets/main.css");
    let css_output = out_dir.join("styles.css");

    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::copy(&input_css, &css_output).unwrap();

    println!("cargo:rerun-if-changed=assets/main.css");
    println!("cargo:rerun-if-changed=build.rs");
}
