//! Project discovery and session enumeration.
//!
//! Walks a `--projects-dir` tree (default `~/.claude/projects/`) and yields
//! [`Project`] structs, each containing its [`SessionFile`]s.

use std::path::{Path, PathBuf};

/// A discovered project with its sessions.
#[derive(Debug, Clone)]
pub struct Project {
    /// Project name (the directory name).
    pub name: String,
    /// Absolute path to the project directory.
    pub path: PathBuf,
    /// Session JSONL files found in this project.
    pub sessions: Vec<SessionFile>,
}

/// A single session JSONL file within a project.
#[derive(Debug, Clone)]
pub struct SessionFile {
    /// The file stem (UUID), used as the session identifier.
    pub id: String,
    /// Absolute path to the `.jsonl` file.
    pub path: PathBuf,
}

/// Discover all projects under `projects_dir`.
///
/// Each immediate subdirectory that contains at least one `.jsonl` file is
/// treated as a project. Hidden directories (starting with `.`) are skipped.
pub fn discover_projects(projects_dir: &Path) -> Vec<Project> {
    let mut projects: Vec<Project> = Vec::new();

    let dir_iter = match std::fs::read_dir(projects_dir) {
        Ok(it) => it,
        Err(_) => return projects,
    };

    for entry in dir_iter.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let sessions = discover_sessions(&path);
        if !sessions.is_empty() {
            projects.push(Project {
                name,
                path,
                sessions,
            });
        }
    }

    // Deterministic order.
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects
}

/// Discover all `.jsonl` session files in a project directory.
fn discover_sessions(project_dir: &Path) -> Vec<SessionFile> {
    let mut sessions: Vec<SessionFile> = Vec::new();

    let dir_iter = match std::fs::read_dir(project_dir) {
        Ok(it) => it,
        Err(_) => return sessions,
    };

    for entry in dir_iter.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().map(|e| e != "jsonl").unwrap_or(true) {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // Skip combined files and other non-UUID-like files.
        if stem.is_empty() || stem.contains("combined") {
            continue;
        }
        sessions.push(SessionFile {
            id: stem.to_string(),
            path,
        });
    }

    sessions.sort_by(|a, b| a.id.cmp(&b.id));
    sessions
}

/// Resolve the default projects directory (`~/.claude/projects/`).
pub fn default_projects_dir() -> PathBuf {
    dirs_home().join(".claude").join("projects")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discover_projects_from_fixture_dir() {
        let tmp = std::env::temp_dir().join(format!("cclog-proj-test-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();

        // Create a project with sessions.
        let proj_a = tmp.join("my-app");
        fs::create_dir_all(&proj_a).unwrap();
        fs::write(proj_a.join("sess-1.jsonl"), r#"{"type":"user","uuid":"a","timestamp":"2025-01-01T00:00:00Z","sessionId":"s1","message":{"role":"user","content":[{"type":"text","text":"hi"}]}}"#).unwrap();
        fs::write(proj_a.join("sess-2.jsonl"), r#"{"type":"user","uuid":"b","timestamp":"2025-01-02T00:00:00Z","sessionId":"s2","message":{"role":"user","content":[{"type":"text","text":"bye"}]}}"#).unwrap();

        // Create another project.
        let proj_b = tmp.join("other-app");
        fs::create_dir_all(&proj_b).unwrap();
        fs::write(proj_b.join("sess-3.jsonl"), r#"{"type":"user","uuid":"c","timestamp":"2025-01-03T00:00:00Z","sessionId":"s3","message":{"role":"user","content":[{"type":"text","text":"hey"}]}}"#).unwrap();

        // Hidden directory — should be skipped.
        let hidden = tmp.join(".hidden-proj");
        fs::create_dir_all(&hidden).unwrap();
        fs::write(hidden.join("sess-4.jsonl"), "{}").unwrap();

        let projects = discover_projects(&tmp);
        assert_eq!(projects.len(), 2);

        let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"my-app"));
        assert!(names.contains(&"other-app"));

        let app = projects.iter().find(|p| p.name == "my-app").unwrap();
        assert_eq!(app.sessions.len(), 2);

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn empty_dir_returns_no_projects() {
        let tmp = std::env::temp_dir().join(format!("cclog-empty-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let projects = discover_projects(&tmp);
        assert!(projects.is_empty());
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn non_existent_dir_returns_empty() {
        let projects = discover_projects(Path::new("/nonexistent/path/xyz"));
        assert!(projects.is_empty());
    }

    #[test]
    fn default_projects_dir_ends_with_claude_projects() {
        let dir = default_projects_dir();
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.contains(".claude"), "expected .claude in path, got: {dir_str}");
        assert!(dir_str.contains("projects"), "expected projects in path, got: {dir_str}");
    }
}
