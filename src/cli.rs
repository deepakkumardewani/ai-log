//! CLI argument parsing and command dispatch.

use std::path::{Path, PathBuf};

use askama::Template;
use clap::{Parser, Subcommand};

/// cclog — Claude Code transcript exporter.
#[derive(Parser)]
#[command(name = "cclog", version = "0.1.0-dev")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Input JSONL file path (shorthand for `export <INPUT>`).
    #[arg(short, long)]
    pub input: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Emit a stub transcript HTML file for design verification.
    Stub {
        /// Output file path (default: cclog-stub.html in current dir).
        #[arg(short, long, default_value = "cclog-stub.html")]
        output: String,
    },

    /// Export a JSONL session file to HTML.
    Export {
        /// Input JSONL file path.
        input: PathBuf,

        /// Output file path (default: <input_stem>.html).
        #[arg(short, long)]
        output: Option<String>,

        /// Open the output file in the default browser.
        #[arg(long)]
        open_browser: bool,
    },
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Some(Command::Stub { output }) => run_stub(&output),
            Some(Command::Export {
                input,
                output,
                open_browser,
            }) => run_export(&input, output.as_deref(), open_browser),
            None => {
                // If --input is provided directly, treat as export.
                if let Some(ref input) = self.input {
                    run_export(input, None, false)
                } else {
                    // Default: print version and usage hint.
                    println!("cclog v0.1.0-dev");
                    println!("Usage: cclog <COMMAND>");
                    println!("  stub     Emit a stub HTML for design review");
                    println!("  export   Export a JSONL session to HTML");
                    println!("  help     Print help information");
                    Ok(())
                }
            }
        }
    }
}

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

fn run_export(input: &Path, output: Option<&str>, open_browser: bool) -> anyhow::Result<()> {
    // Parse the JSONL file.
    let result = crate::parser::parse_file(input)?;
    if !result.warnings.is_empty() {
        for w in &result.warnings {
            eprintln!("Warning: line {}: {}", w.line, w.message);
        }
    }

    // Build session DAG.
    let session = crate::session::build_session(&result.entries);
    if session.messages.is_empty() {
        anyhow::bail!("No messages found in input file");
    }

    // Aggregate.
    let agg = crate::aggregate::aggregate(&session);

    // Build render context.
    let ctx = crate::render::html::build_context(&session, &agg, crate::assets::CSS.to_string());

    // Render HTML.
    let html = ctx.render()?;

    // Determine output path.
    let output_path = match output {
        Some(o) => o.to_string(),
        None => {
            let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("session");
            format!("{}.html", stem)
        }
    };

    std::fs::write(&output_path, &html)?;
    println!("Exported to {output_path}");
    println!("  Messages: {}", agg.message_count);
    println!("  Tokens: {} in / {} out", agg.total_input_tokens, agg.total_output_tokens);
    let self_contained = !html.contains("http://") && !html.contains("https://");
    println!("  Self-contained: {}", if self_contained { "yes" } else { "NO" });

    if open_browser {
        let _ = std::process::Command::new("open").arg(&output_path).spawn();
    }

    Ok(())
}
