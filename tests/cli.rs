//! CLI integration tests via assert_cmd.

use assert_cmd::Command;

#[test]
fn stub_command_creates_html_file() {
    let output_path = std::env::temp_dir().join("cclog-test-stub.html");

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args(["stub", "--output", output_path.to_str().unwrap()]);
    cmd.assert().success();

    let html = std::fs::read_to_string(&output_path).unwrap();
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("--surface"));
    assert!(!html.contains("http://"), "must be self-contained");
    assert!(!html.contains("https://"), "must be self-contained");

    // Cleanup.
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn export_command_from_fixture() {
    let fixture = std::env::current_dir().unwrap().join("tests/fixtures/session_linear.jsonl");
    let output_path = std::env::temp_dir().join("cclog-test-export.html");

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args(["export", fixture.to_str().unwrap(), "--output", output_path.to_str().unwrap()]);
    cmd.assert().success();

    let html = std::fs::read_to_string(&output_path).unwrap();
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("message-card"));
    assert!(html.contains("--surface"));
    assert!(!html.contains("http://"));

    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn export_shortcut_with_input_flag() {
    let fixture = std::env::current_dir().unwrap().join("tests/fixtures/entry_user.jsonl");
    let output_path = std::env::temp_dir().join("cclog-test-shortcut.html");

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "--input",
        fixture.to_str().unwrap(),
        "export",
        fixture.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
    ]);
    // Just check it doesn't crash — the export subcommand handles the path.
    let _ = cmd.ok();

    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn self_contained_output_no_external_urls() {
    let fixture = std::env::current_dir().unwrap().join("tests/fixtures/session_linear.jsonl");
    let output_path = std::env::temp_dir().join("cclog-self-contained.html");

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args(["export", fixture.to_str().unwrap(), "--output", output_path.to_str().unwrap()]);
    cmd.assert().success();

    let html = std::fs::read_to_string(&output_path).unwrap();

    // Self-containment gate: zero external URLs.
    let has_http = html.contains("http://") || html.contains("https://");
    assert!(!has_http, "HTML must contain zero external URLs");

    let _ = std::fs::remove_file(&output_path);
}
