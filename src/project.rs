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

impl Project {
    /// Human-readable display name derived from the encoded directory name.
    ///
    /// Takes the substring after the last `-` in `self.name`. If the last
    /// segment is empty (trailing dash, single dash, or empty string), falls
    /// back to the full encoded string.
    ///
    /// # Examples
    ///
    /// | `self.name` | `display_name()` |
    /// |---|---|
    /// | `-Users-deepak-Documents-Programs-weavr` | `weavr` |
    /// | `my-project` | `my-project` |
    /// | `-leading-dash` | `leading-dash` |
    /// | `trailing-dash-` | `trailing-dash-` (fallback) |
    /// | `` (empty) | `` (fallback) |
    pub fn display_name(&self) -> &str {
        self.name.rsplit('-').next().filter(|s| !s.is_empty()).unwrap_or(&self.name)
    }
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
        let tmp = std::env::temp_dir().join(format!("weavr-proj-test-{}", std::process::id()));
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
        let tmp = std::env::temp_dir().join(format!("weavr-empty-{}", std::process::id()));
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

    // -----------------------------------------------------------------------
    // display_name tests
    // -----------------------------------------------------------------------

    #[test]
    fn display_name_normal_case() {
        let p = Project {
            name: "-Users-deepak-Documents-Programs-weavr".into(),
            path: PathBuf::from("/tmp"),
            sessions: vec![],
        };
        assert_eq!(p.display_name(), "weavr");
    }

    #[test]
    fn display_name_leading_dash() {
        let p = Project {
            name: "-leading-dash".into(),
            path: PathBuf::from("/tmp"),
            sessions: vec![],
        };
        // Last segment after rsplit('-') on "-leading-dash" is "dash".
        assert_eq!(p.display_name(), "dash");
    }

    #[test]
    fn display_name_trailing_dash_falls_back() {
        let p = Project {
            name: "trailing-dash-".into(),
            path: PathBuf::from("/tmp"),
            sessions: vec![],
        };
        // Last segment is empty after trailing dash, fallback to full string.
        assert_eq!(p.display_name(), "trailing-dash-");
    }

    #[test]
    fn display_name_single_segment_no_dash() {
        let p = Project {
            name: "simple".into(),
            path: PathBuf::from("/tmp"),
            sessions: vec![],
        };
        // No dash at all — the whole string is the last (only) segment.
        assert_eq!(p.display_name(), "simple");
    }

    #[test]
    fn display_name_empty_string() {
        let p = Project {
            name: String::new(),
            path: PathBuf::from("/tmp"),
            sessions: vec![],
        };
        assert_eq!(p.display_name(), "");
    }

    #[test]
    fn display_name_multiple_dashes() {
        let p = Project {
            name: "-Users-deepak-some-long-path-name".into(),
            path: PathBuf::from("/tmp"),
            sessions: vec![],
        };
        assert_eq!(p.display_name(), "name");
    }
}
