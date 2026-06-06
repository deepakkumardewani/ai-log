//! Self-update from GitHub Releases.
//!
//! Provides:
//! - [`check_for_update`] — lightweight version check against GitHub Releases
//! - [`self_update`] — download and replace the current binary
//! - [`maybe_print_update_notice`] — passive, non-blocking notice on startup

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Home directory for the update-check cache file.
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("XDG_CACHE_HOME").ok().map(PathBuf::from))
}

/// Check GitHub Releases for a newer version.
///
/// Returns `Some(version)` if an update is available, `None` if already
/// current or if the check fails (offline, rate-limited, etc.).
pub fn check_for_update() -> Option<String> {
    let current = env!("CARGO_PKG_VERSION");

    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("deepakkumardewani")
        .repo_name("weavr")
        .build()
        .ok()?
        .fetch()
        .ok()?;

    let latest = releases.first()?;
    let latest_version = latest.version.trim_start_matches('v');

    if latest_version != current {
        Some(latest_version.to_string())
    } else {
        None
    }
}

/// Download and replace the current binary from GitHub Releases.
///
/// Returns a human-readable status message.
pub fn self_update() -> Result<String, anyhow::Error> {
    let updater = self_update::backends::github::Update::configure()
        .repo_owner("deepakkumardewani")
        .repo_name("weavr")
        .bin_name("weavr")
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .build()?;

    match updater.update()? {
        self_update::Status::UpToDate(v) => Ok(format!("weavr {v} is already the latest version.")),
        self_update::Status::Updated(v) => Ok(format!("Successfully updated to weavr {v}.")),
    }
}

/// Print a passive "new version available" notice to stderr, at most once
/// per 24 hours. Silently skips if offline or if any check fails.
pub fn maybe_print_update_notice() {
    // Respect opt-out via env var.
    if std::env::var("WEAVR_NO_UPDATE_CHECK").is_ok() {
        return;
    }

    // Throttle: only check once per 24 h.
    if let Some(cache_path) = home_dir().map(|d| d.join(".weavr-update-check")) {
        if let Ok(meta) = std::fs::metadata(&cache_path) {
            if let Ok(mtime) = meta.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(mtime) {
                    if elapsed < Duration::from_secs(86_400) {
                        return;
                    }
                }
            }
        }
        let _ = std::fs::write(&cache_path, "checked");
    }

    if let Some(new_version) = check_for_update() {
        eprintln!(
            "weavr: version {new_version} is available (you have {}). \
             Run `weavr self-update` to upgrade.",
            env!("CARGO_PKG_VERSION")
        );
    }
}
