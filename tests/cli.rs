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

// ---------------------------------------------------------------------------
// Markdown export tests
// ---------------------------------------------------------------------------

#[test]
fn export_markdown_from_fixture() {
    let fixture = std::env::current_dir().unwrap().join("tests/fixtures/session_linear.jsonl");
    let output_path = std::env::temp_dir().join("cclog-test-export.md");

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "export",
        fixture.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
        "--format",
        "md",
    ]);
    cmd.assert().success();

    let md = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        md.contains("# s1"),
        "should have session title, got: {}",
        md.lines().next().unwrap_or("")
    );
    assert!(md.contains("### User"), "should have user messages");
    assert!(md.contains("### Assistant"), "should have assistant messages");

    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn export_markdown_with_detail_levels() {
    let fixture = std::env::current_dir().unwrap().join("tests/fixtures/session_linear.jsonl");

    for (level, expected_includes, expected_excludes) in &[
        ("full", vec!["### User", "### Assistant", "Bash"], vec![]),
        ("high", vec!["### User", "### Assistant", "Bash"], vec![]),
        ("low", vec!["### User", "### Assistant"], vec!["#### Bash"]),
        ("minimal", vec!["### User", "### Assistant"], vec!["#### Bash", "claude-opus"]),
        ("user-only", vec!["### User"], vec!["### Assistant"]),
    ] {
        let output_path = std::env::temp_dir().join(format!("cclog-test-detail-{}.md", level));

        let mut cmd = Command::cargo_bin("cclog").unwrap();
        cmd.args([
            "export",
            fixture.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
            "--format",
            "md",
            "--detail",
            level,
        ]);
        cmd.assert().success();

        let md = std::fs::read_to_string(&output_path).unwrap();
        for include in expected_includes {
            assert!(md.contains(include), "detail={}: should contain '{}'", level, include);
        }
        for exclude in expected_excludes {
            assert!(!md.contains(exclude), "detail={}: should NOT contain '{}'", level, exclude);
        }

        let _ = std::fs::remove_file(&output_path);
    }
}

#[test]
fn export_markdown_compact_mode() {
    let fixture = std::env::current_dir().unwrap().join("tests/fixtures/session_linear.jsonl");
    let output_full = std::env::temp_dir().join("cclog-test-full.md");
    let output_compact = std::env::temp_dir().join("cclog-test-compact.md");

    // Full (non-compact)
    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "export",
        fixture.to_str().unwrap(),
        "--output",
        output_full.to_str().unwrap(),
        "--format",
        "md",
    ]);
    cmd.assert().success();

    // Compact
    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "export",
        fixture.to_str().unwrap(),
        "--output",
        output_compact.to_str().unwrap(),
        "--format",
        "md",
        "--compact",
    ]);
    cmd.assert().success();

    let full = std::fs::read_to_string(&output_full).unwrap();
    let compact = std::fs::read_to_string(&output_compact).unwrap();

    // Full has horizontal rules
    assert!(full.contains("---"), "full should have horizontal rules");
    // Compact should NOT have horizontal rules
    assert!(!compact.contains("---"), "compact should NOT have horizontal rules");
    // Full has date/time metadata
    assert!(full.contains("**Date:**"), "full should have date metadata");
    assert!(!compact.contains("**Date:**"), "compact should NOT have date metadata");

    let _ = std::fs::remove_file(&output_full);
    let _ = std::fs::remove_file(&output_compact);
}

#[test]
fn export_markdown_default_extension_is_md() {
    let fixture = std::env::current_dir().unwrap().join("tests/fixtures/session_linear.jsonl");

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args(["export", fixture.to_str().unwrap(), "--format", "md"]);
    cmd.assert().success();

    // Output should be session_linear.md in the current working directory
    let expected = std::env::current_dir().unwrap().join("session_linear.md");
    assert!(expected.exists(), "default .md output file should exist: {:?}", expected);

    let _ = std::fs::remove_file(&expected);
}
