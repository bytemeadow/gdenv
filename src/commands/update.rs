use anyhow::Result;
use clap::Args;

use crate::{config::Config, github::GitHubClient, ui};

#[derive(Args)]
pub struct UpdateCommand {
    /// Force update even if cache is recent
    #[arg(long, short)]
    pub force: bool,
}

impl UpdateCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let github_client = GitHubClient::new(config.github_api_url);

        ui::info("Updating available Godot versions...");

        // Fetch releases from GitHub
        let releases = github_client.get_godot_releases(true, true).await?;

        ui::success(&format!("Found {} Godot releases", releases.len()));

        // Show latest stable and prerelease versions (sorted ascending, so last is latest)
        let stable_releases: Vec<_> = releases.iter().filter(|r| !r.prerelease).collect();
        let prerelease_releases: Vec<_> = releases.iter().filter(|r| r.prerelease).collect();

        if let Some(latest_stable) = stable_releases.last() {
            if let Some(version) = latest_stable.version() {
                ui::info(&format!("Latest stable: {version}"));
            }
        }

        if let Some(latest_prerelease) = prerelease_releases.last() {
            if let Some(version) = latest_prerelease.version() {
                ui::info(&format!("Latest prerelease: {version}"));
            }
        }

        ui::success("Update complete! Use 'gdenv list' to see stable versions or 'gdenv list --include-prereleases' for all versions");

        Ok(())
    }
}
