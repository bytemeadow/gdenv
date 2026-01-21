use anyhow::Result;
use clap::Args;

use crate::{github::GitHubClient, ui};

#[derive(Args)]
pub struct UpdateCommand {
    /// Force update even if cache is recent
    #[arg(long, short)]
    pub force: bool,
}

impl UpdateCommand {
    pub async fn run(self) -> Result<()> {
        let github_client = GitHubClient::new();

        ui::info("Updating available Godot versions...");

        // Fetch releases from GitHub
        let releases = github_client.get_godot_releases(true).await?;

        ui::success(&format!("Found {} Godot releases", releases.len()));

        // Show latest stable and prerelease versions (sorted ascending, so last is latest)
        let stable_releases: Vec<_> = releases
            .iter()
            .filter(|r| !r.version.is_prerelease())
            .collect();
        let prerelease_releases: Vec<_> = releases
            .iter()
            .filter(|r| r.version.is_prerelease())
            .collect();

        if let Some(latest_stable) = stable_releases.last() {
            ui::info(&format!("Latest stable: {}", latest_stable.version));
        }

        if let Some(latest_prerelease) = prerelease_releases.last() {
            ui::info(&format!("Latest prerelease: {}", latest_prerelease.version));
        }

        ui::success(
            "Update complete! Use 'gdenv list' to see stable versions or 'gdenv list --include-prereleases' for all versions",
        );

        Ok(())
    }
}
