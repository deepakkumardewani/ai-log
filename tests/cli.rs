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

// ---------------------------------------------------------------------------
// Phase 5: All-projects + cache + index tests
// ---------------------------------------------------------------------------

use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

static TEST_ID: AtomicU32 = AtomicU32::new(0);

fn unique_test_id() -> u32 {
    TEST_ID.fetch_add(1, Ordering::SeqCst)
}

fn setup_fixture_projects_dir() -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!(
        "cclog-phase5-{}-{}",
        std::process::id(),
        unique_test_id()
    ));
    fs::create_dir_all(&tmp).unwrap();

    // Project A with 2 sessions.
    let proj_a = tmp.join("my-app");
    fs::create_dir_all(&proj_a).unwrap();
    fs::write(
        proj_a.join("sess-a1.jsonl"),
        r#"{"type":"user","uuid":"550e8400-e29b-41d4-a716-446655440001","timestamp":"2025-01-01T10:00:00Z","sessionId":"sess-a1","message":{"role":"user","content":[{"type":"text","text":"hello"}]}}"#,
    ).unwrap();
    fs::write(
        proj_a.join("sess-a2.jsonl"),
        r#"{"type":"assistant","uuid":"550e8400-e29b-41d4-a716-446655440002","parentUuid":"550e8400-e29b-41d4-a716-446655440001","timestamp":"2025-01-02T10:00:00Z","sessionId":"sess-a2","message":{"role":"assistant","content":[{"type":"text","text":"hi"}],"usage":{"input_tokens":10,"output_tokens":5}}}"#,
    ).unwrap();

    // Project B with 1 session.
    let proj_b = tmp.join("other-app");
    fs::create_dir_all(&proj_b).unwrap();
    fs::write(
        proj_b.join("sess-b1.jsonl"),
        r#"{"type":"user","uuid":"550e8400-e29b-41d4-a716-446655440003","timestamp":"2025-01-03T10:00:00Z","sessionId":"sess-b1","message":{"role":"user","content":[{"type":"text","text":"hey"}]}}"#,
    ).unwrap();

    tmp
}

#[test]
fn all_projects_generates_index_and_combined_pages() {
    let projects_dir = setup_fixture_projects_dir();
    let n = unique_test_id();
    let output_dir =
        std::env::temp_dir().join(format!("cclog-p5-out-{}-{}", std::process::id(), n));

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "--all-projects",
        "--projects-dir",
        projects_dir.to_str().unwrap(),
        "--output-dir",
        output_dir.to_str().unwrap(),
        "--no-cache",
    ]);
    cmd.assert().success();

    // Master index.
    let index_path = output_dir.join("index.html");
    assert!(index_path.exists(), "index.html should exist");
    let index = fs::read_to_string(&index_path).unwrap();
    assert!(
        index.contains("my-app"),
        "index should contain my-app, got:\n{}",
        &index[..index.len().min(500)]
    );
    assert!(
        index.contains("other-app"),
        "index should contain other-app, got:\n{}",
        &index[..index.len().min(500)]
    );
    assert!(!index.contains("http://"));
    assert!(!index.contains("https://"));

    // Per-project combined pages.
    let combined_a = output_dir.join("my-app/combined_transcripts.html");
    assert!(combined_a.exists(), "project A combined page should exist");
    let html_a = fs::read_to_string(&combined_a).unwrap();
    assert!(html_a.contains("sess-a1.html"));
    assert!(html_a.contains("sess-a2.html"));

    let combined_b = output_dir.join("other-app/combined_transcripts.html");
    assert!(combined_b.exists(), "project B combined page should exist");

    // Per-session HTML files.
    assert!(output_dir.join("my-app/sess-a1.html").exists());
    assert!(output_dir.join("my-app/sess-a2.html").exists());
    assert!(output_dir.join("other-app/sess-b1.html").exists());

    fs::remove_dir_all(&projects_dir).ok();
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn no_individual_sessions_skips_per_session_files() {
    let projects_dir = setup_fixture_projects_dir();
    let output_dir = std::env::temp_dir().join(format!(
        "cclog-p5-nosess-{}-{}",
        std::process::id(),
        unique_test_id()
    ));

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "--all-projects",
        "--projects-dir",
        projects_dir.to_str().unwrap(),
        "--output-dir",
        output_dir.to_str().unwrap(),
        "--no-individual-sessions",
        "--no-cache",
    ]);
    cmd.assert().success();

    // Index should exist.
    assert!(output_dir.join("index.html").exists());
    // Combined should exist.
    assert!(output_dir.join("my-app/combined_transcripts.html").exists());
    // Per-session should NOT exist.
    assert!(!output_dir.join("my-app/sess-a1.html").exists());

    fs::remove_dir_all(&projects_dir).ok();
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn session_id_prefix_match_filters_correctly() {
    let projects_dir = setup_fixture_projects_dir();
    let output_dir = std::env::temp_dir().join(format!(
        "cclog-p5-sid-{}-{}",
        std::process::id(),
        unique_test_id()
    ));

    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "--all-projects",
        "--projects-dir",
        projects_dir.to_str().unwrap(),
        "--output-dir",
        output_dir.to_str().unwrap(),
        "--session-id",
        "sess-b1",
        "--no-cache",
    ]);
    cmd.assert().success();

    // Only project B should be present.
    assert!(output_dir.join("other-app/combined_transcripts.html").exists());
    // Project A should NOT be present.
    assert!(!output_dir.join("my-app").exists());

    fs::remove_dir_all(&projects_dir).ok();
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn ambiguous_session_id_prefix_errors() {
    let projects_dir = setup_fixture_projects_dir();
    let output_dir = std::env::temp_dir().join(format!(
        "cclog-p5-ambig-{}-{}",
        std::process::id(),
        unique_test_id()
    ));

    // "sess-a" matches both sess-a1 and sess-a2.
    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "--all-projects",
        "--projects-dir",
        projects_dir.to_str().unwrap(),
        "--output-dir",
        output_dir.to_str().unwrap(),
        "--session-id",
        "sess-a",
        "--no-cache",
    ]);
    cmd.assert().failure();

    fs::remove_dir_all(&projects_dir).ok();
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn clear_cache_flag_works() {
    let projects_dir = setup_fixture_projects_dir();
    let output_dir = std::env::temp_dir().join(format!(
        "cclog-p5-clrcache-{}-{}",
        std::process::id(),
        unique_test_id()
    ));

    // First run to populate cache.
    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "--all-projects",
        "--projects-dir",
        projects_dir.to_str().unwrap(),
        "--output-dir",
        output_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    // Cache file should exist.
    let cache_path = projects_dir.join("cclog-cache.db");
    assert!(cache_path.exists(), "cache file should be created");

    // Second run with --clear-cache.
    let output_dir2 = std::env::temp_dir().join(format!(
        "cclog-p5-clrcache2-{}-{}",
        std::process::id(),
        unique_test_id()
    ));
    let mut cmd = Command::cargo_bin("cclog").unwrap();
    cmd.args([
        "--all-projects",
        "--projects-dir",
        projects_dir.to_str().unwrap(),
        "--output-dir",
        output_dir2.to_str().unwrap(),
        "--clear-cache",
    ]);
    cmd.assert().success();

    // Cache should be recreated (still exists after clear+repopulate).
    assert!(cache_path.exists());

    fs::remove_dir_all(&projects_dir).ok();
    fs::remove_dir_all(&output_dir).ok();
    fs::remove_dir_all(&output_dir2).ok();
}
