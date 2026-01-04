//! Self-update functionality for the IMACS CLI
//!
//! Provides:
//! - Non-blocking update check on startup
//! - Manual update command via `imacs update`

use std::time::Duration;

use update_informer::{registry, Check};

/// GitHub repository owner for release downloads
pub const REPO_OWNER: &str = "outboundlabs";

/// GitHub repository name for release downloads
pub const REPO_NAME: &str = "imacs";

/// Check for updates in a background thread (non-blocking).
///
/// Shows a notification to stderr if a new version is available.
/// Runs at most once per day per the update-informer interval.
pub fn check_for_updates_background() {
    std::thread::spawn(|| {
        let informer = update_informer::new(registry::Crates, "imacs", env!("CARGO_PKG_VERSION"))
            .timeout(Duration::from_secs(2))
            .interval(Duration::from_secs(60 * 60 * 24)); // Once per day

        if let Ok(Some(new_version)) = informer.check_version() {
            eprintln!(
                "\nUpdate available: {} -> {}",
                env!("CARGO_PKG_VERSION"),
                new_version
            );
            eprintln!("   Run: imacs update\n");
        }
    });
}

/// Perform a self-update from GitHub releases.
///
/// Downloads and replaces the current binary with the latest release.
pub fn run_update() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Checking for updates...");

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("imacs")
        .show_download_progress(true)
        .current_version(self_update::cargo_crate_version!())
        .build()?
        .update()?;

    match status {
        self_update::Status::UpToDate(v) => {
            eprintln!("Already at latest version: {}", v);
        }
        self_update::Status::Updated(v) => {
            eprintln!("Updated to version {}", v);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_constants() {
        assert_eq!(REPO_OWNER, "outboundlabs");
        assert_eq!(REPO_NAME, "imacs");
    }
}
