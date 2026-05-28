//! CLI argument parsing and command dispatch.

use std::fs;
use std::path::{Path, PathBuf};

use askama::Template;
use clap::{Parser, Subcommand, ValueEnum};

use crate::cache::{Cache, CachedSessionMeta};
use crate::render::markdown_export::DetailLevel;
use crate::render::{IndexProjectData, ProjectSessionData};

/// cclog — Claude Code transcript exporter.
#[derive(Parser)]
#[command(name = "cclog", version = "0.1.0-dev")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Input JSONL file path (shorthand for `export <INPUT>`).
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Projects directory (default: ~/.claude/projects/).
    #[arg(long)]
    pub projects_dir: Option<PathBuf>,

    /// Export all projects (default when no INPUT_PATH is given).
    #[arg(long)]
    pub all_projects: bool,

    /// Skip per-session HTML files; only produce combined + index pages.
    #[arg(long)]
    pub no_individual_sessions: bool,

    /// Disable SQLite cache reads and writes.
    #[arg(long)]
    pub no_cache: bool,

    /// Clear the SQLite cache and rebuild from scratch.
    #[arg(long)]
    pub clear_cache: bool,

    /// Filter to a single session by ID or unique prefix.
    #[arg(long)]
    pub session_id: Option<String>,

    /// Wipe the output directory before writing.
    #[arg(long)]
    pub clear_output: bool,

    /// Output directory (default: ./cclog-out/).
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Emit a stub transcript HTML file for design verification.
    Stub {
        /// Output file path (default: cclog-stub.html in current dir).
        #[arg(short, long, default_value = "cclog-stub.html")]
        output: String,
    },

    /// Export a JSONL session file to HTML or Markdown.
    Export {
        /// Input JSONL file path.
        input: PathBuf,

        /// Output file path (default: <input_stem>.html or .md).
        #[arg(short, long)]
        output: Option<String>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Html)]
        format: Format,

        /// Detail level for markdown output.
        #[arg(long, value_enum, default_value_t = DetailLevel::Full)]
        detail: DetailLevel,

        /// Compact mode: strip timestamps and horizontal rules (markdown only).
        #[arg(long, default_value_t = false)]
        compact: bool,

        /// Open the output file in the default browser.
        #[arg(long)]
        open_browser: bool,
    },
}

/// Output format for the export command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum Format {
    /// Self-contained HTML (default).
    Html,
    /// Markdown.
    #[clap(name = "md")]
    #[clap(alias = "markdown")]
    Markdown,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Some(Command::Stub { output }) => run_stub(&output),
            Some(Command::Export {
                input,
                output,
                format,
                detail,
                compact,
                open_browser,
            }) => run_export(&input, output.as_deref(), format, detail, compact, open_browser),
            None => {
                // If --input is provided, treat as single export.
                if let Some(ref input) = self.input {
                    run_export(input, None, Format::Html, DetailLevel::Full, false, false)
                } else {
                    // Default: all-projects export.
                    run_all_projects(self)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Single-session export (Phase 3–4)
// ---------------------------------------------------------------------------

fn run_stub(output: &str) -> anyhow::Result<()> {
    let ctx = crate::render::html::stub_context();
    let html = ctx.render()?;
    std::fs::write(output, &html)?;
    println!("Wrote stub HTML to {output}");
    println!("  Size: {} bytes", html.len());
    let self_contained = !html.contains("http://") && !html.contains("https://");
    println!("  Self-contained: {}", if self_contained { "yes" } else { "NO" });
    Ok(())
}

fn run_export(
    input: &Path,
    output: Option<&str>,
    format: Format,
    detail: DetailLevel,
    compact: bool,
    open_browser: bool,
) -> anyhow::Result<()> {
    let result = crate::parser::parse_file(input)?;
    if !result.warnings.is_empty() {
        for w in &result.warnings {
            eprintln!("Warning: line {}: {}", w.line, w.message);
        }
    }

    let session = crate::session::build_session(&result.entries);
    if session.messages.is_empty() {
        anyhow::bail!("No messages found in input file");
    }

    let agg = crate::aggregate::aggregate(&session);

    match format {
        Format::Html => {
            let ctx =
                crate::render::html::build_context(&session, &agg, crate::assets::CSS.to_string());
            let html = ctx.render()?;

            let output_path = match output {
                Some(o) => o.to_string(),
                None => {
                    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("session");
                    format!("{}.html", stem)
                }
            };

            std::fs::write(&output_path, &html)?;
            println!("Exported to {output_path}");
            println!("  Format: HTML");
            println!("  Messages: {}", agg.message_count);
            println!("  Tokens: {} in / {} out", agg.total_input_tokens, agg.total_output_tokens);
            let self_contained = !html.contains("http://") && !html.contains("https://");
            println!("  Self-contained: {}", if self_contained { "yes" } else { "NO" });

            if open_browser {
                let _ = std::process::Command::new("open").arg(&output_path).spawn();
            }
        }

        Format::Markdown => {
            let md =
                crate::render::markdown_export::render_session(&session, &agg, detail, compact);

            let output_path = match output {
                Some(o) => o.to_string(),
                None => {
                    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("session");
                    format!("{}.md", stem)
                }
            };

            std::fs::write(&output_path, &md)?;
            println!("Exported to {output_path}");
            println!("  Format: Markdown");
            println!("  Detail: {:?}", detail);
            println!("  Compact: {}", if compact { "yes" } else { "no" });
            println!("  Messages: {}", agg.message_count);
            println!("  Tokens: {} in / {} out", agg.total_input_tokens, agg.total_output_tokens);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// All-projects pipeline (Phase 5)
// ---------------------------------------------------------------------------

fn run_all_projects(cli: Cli) -> anyhow::Result<()> {
    let projects_dir = cli.projects_dir.unwrap_or_else(crate::project::default_projects_dir);

    if !projects_dir.is_dir() {
        anyhow::bail!(
            "Projects directory not found: {}. Use --projects-dir to specify a path.",
            projects_dir.display()
        );
    }

    let output_dir = cli.output_dir.unwrap_or_else(|| PathBuf::from("cclog-out"));

    if cli.clear_output && output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    // Cache.
    let cache_path = projects_dir.join("cclog-cache.db");
    if cli.clear_cache && cache_path.exists() {
        let _ = fs::remove_file(&cache_path);
    }

    let use_cache = !cli.no_cache;
    let cache: Option<Cache> = if use_cache { Cache::open(&cache_path).ok() } else { None };

    // Discover projects.
    let mut projects = crate::project::discover_projects(&projects_dir);

    // Filter by --session-id if provided.
    if let Some(ref sid) = cli.session_id {
        projects = filter_by_session_id(projects, sid)?;
    }

    if projects.is_empty() {
        println!("No sessions found in {}", projects_dir.display());
        return Ok(());
    }

    let css = crate::assets::CSS.to_string();
    let mut all_project_data: Vec<IndexProjectData> = Vec::new();
    let mut global_messages: u32 = 0;
    let mut global_tokens: u64 = 0;
    let mut global_earliest: Option<String> = None;
    let mut global_latest: Option<String> = None;
    let mut total_exported: usize = 0;

    for project in &projects {
        let project_out = output_dir.join(&project.name);
        fs::create_dir_all(&project_out)?;

        let mut session_datas: Vec<ProjectSessionData> = Vec::new();
        let mut project_messages: u32 = 0;
        let mut project_tokens: u64 = 0;

        for sf in &project.sessions {
            // Check cache first.
            let file_meta = fs::metadata(&sf.path).ok();
            let file_mtime = file_meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let file_size = file_meta.map(|m| m.len()).unwrap_or(0);

            let cached: Option<CachedSessionMeta> =
                cache.as_ref().and_then(|c| c.get(&sf.id, file_mtime));

            let (
                title,
                msg_count,
                input_tok,
                output_tok,
                cache_create,
                cache_read,
                first_ts,
                last_ts,
            ) = if let Some(ref cm) = cached {
                // Use cached metadata.
                (
                    cm.title.clone(),
                    cm.message_count,
                    cm.total_input_tokens,
                    cm.total_output_tokens,
                    cm.total_cache_creation_tokens,
                    cm.total_cache_read_tokens,
                    cm.first_timestamp.clone(),
                    cm.last_timestamp.clone(),
                )
            } else {
                // Parse and aggregate.
                let parse_result = match crate::parser::parse_file(&sf.path) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Warning: skipping {}: {}", sf.id, e);
                        continue;
                    }
                };
                let session = crate::session::build_session(&parse_result.entries);
                let agg = crate::aggregate::aggregate(&session);

                let title = agg.summaries.first().cloned();
                let msg_count = agg.message_count as u32;
                let it = agg.total_input_tokens;
                let ot = agg.total_output_tokens;
                let cc = agg.total_cache_creation_tokens;
                let cr = agg.total_cache_read_tokens;
                let first = agg.first_timestamp.map(|t| t.to_rfc3339());
                let last = agg.last_timestamp.map(|t| t.to_rfc3339());

                // Store in cache.
                if let Some(ref c) = cache {
                    c.put(
                        &CachedSessionMeta {
                            session_id: sf.id.clone(),
                            project_name: project.name.clone(),
                            title: title.clone(),
                            first_timestamp: first.clone(),
                            last_timestamp: last.clone(),
                            message_count: msg_count,
                            total_input_tokens: it,
                            total_output_tokens: ot,
                            total_cache_creation_tokens: cc,
                            total_cache_read_tokens: cr,
                        },
                        file_mtime,
                        file_size,
                    );
                }

                (title, msg_count, it, ot, cc, cr, first, last)
            };

            let total_session_tokens = input_tok + output_tok + cache_create + cache_read;

            // Export per-session HTML if not skipped.
            if !cli.no_individual_sessions {
                // Only re-parse if we used cache metadata.
                let session_html_path = project_out.join(format!("{}.html", sf.id));
                if !session_html_path.exists() {
                    let parse_result = crate::parser::parse_file(&sf.path)?;
                    let session = crate::session::build_session(&parse_result.entries);
                    let agg = crate::aggregate::aggregate(&session);
                    let ctx = crate::render::html::build_context(&session, &agg, css.clone());
                    let html = ctx.render()?;
                    fs::write(&session_html_path, &html)?;
                    total_exported += 1;
                }
            }

            session_datas.push(ProjectSessionData {
                id: sf.id.clone(),
                title,
                message_count: msg_count,
                total_tokens: total_session_tokens,
            });

            project_messages += msg_count;
            project_tokens += total_session_tokens;

            // Track global time range.
            if let Some(ref ts) = first_ts {
                if global_earliest.is_none()
                    || ts.as_str() < global_earliest.as_deref().unwrap_or("")
                {
                    global_earliest = Some(ts.clone());
                }
            }
            if let Some(ref ts) = last_ts {
                if global_latest.is_none() || ts.as_str() > global_latest.as_deref().unwrap_or("") {
                    global_latest = Some(ts.clone());
                }
            }
        }

        if session_datas.is_empty() {
            continue;
        }

        // Per-project combined page.
        let project_ctx =
            crate::render::project::build_context(css.clone(), project.name.clone(), session_datas);
        let project_html = project_ctx.render()?;
        let combined_path = project_out.join("combined_transcripts.html");
        fs::write(&combined_path, &project_html)?;
        println!(
            "  {}/combined_transcripts.html ({} sessions)",
            project.name,
            projects.iter().find(|p| p.name == project.name).map(|p| p.sessions.len()).unwrap_or(0)
        );

        all_project_data.push(IndexProjectData {
            name: project.name.clone(),
            session_count: project.sessions.len().try_into().unwrap_or(u32::MAX),
            message_count: project_messages,
            total_tokens: project_tokens,
        });

        global_messages += project_messages;
        global_tokens += project_tokens;
    }

    // Master index.
    let index_ctx = crate::render::index::build_context(
        css,
        all_project_data,
        global_messages,
        global_tokens,
        global_earliest,
        global_latest,
    );
    let index_html = index_ctx.render()?;
    fs::write(output_dir.join("index.html"), &index_html)?;
    println!("  index.html");

    let mode = if cli.no_individual_sessions { "combined + index" } else { "full" };
    println!(
        "Done: {} sessions exported ({} mode) to {}",
        total_exported,
        mode,
        output_dir.display()
    );

    Ok(())
}

/// Filter projects to only include the session matching `prefix`.
fn filter_by_session_id(
    projects: Vec<crate::project::Project>,
    prefix: &str,
) -> anyhow::Result<Vec<crate::project::Project>> {
    let mut matching: Vec<(&crate::project::Project, &crate::project::SessionFile)> = Vec::new();

    for p in &projects {
        for s in &p.sessions {
            if s.id.starts_with(prefix) {
                matching.push((p, s));
            }
        }
    }

    if matching.is_empty() {
        eprintln!("No session matches prefix \"{}\"", prefix);
        return Ok(Vec::new());
    }

    if matching.len() > 1 {
        eprintln!("Ambiguous prefix \"{}\" matches {} sessions:", prefix, matching.len());
        for (proj, sess) in &matching {
            eprintln!("  {}/{}", proj.name, sess.id);
        }
        anyhow::bail!("Ambiguous session-id prefix. Use a longer prefix to disambiguate.");
    }

    let (project, session) = matching.into_iter().next().unwrap();
    Ok(vec![crate::project::Project {
        name: project.name.clone(),
        path: project.path.clone(),
        sessions: vec![session.clone()],
    }])
}
