use anyhow::Result;
use clap::Args;
use colored::*;

use crate::{github::GitHubClient, ui};

#[derive(Args)]
pub struct ListCommand {
    /// Include pre-release versions (beta, rc, etc.)
    #[arg(long)]
    pub include_prereleases: bool,
}

impl ListCommand {
    pub async fn run(self) -> Result<()> {
        let github_client = GitHubClient::new();
        let mut releases = github_client.get_godot_releases(false).await?;
        releases.retain(|r| self.include_prereleases || !r.version.is_prerelease());

        println!("\n📋 Available Godot versions:");

        if releases.is_empty() {
            ui::warning("No releases found");
            return Ok(());
        }

        for release in releases.iter().rev().take(20) {
            // Show only latest 20 (they are sorted ascending, so rev() for latest)
            let version = release.version.as_full_version_str();
            let status = if release.version.is_prerelease() {
                " (prerelease)".yellow()
            } else {
                " (stable)".green()
            };
            println!("  • {version}{status}");
        }

        if releases.len() > 20 {
            ui::info(&format!("... and {} more versions", releases.len() - 20));
        }

        if !self.include_prereleases {
            ui::info("Use --include-prereleases to see beta/rc versions");
        }

        ui::info("Use 'gdenv installed' to see installed versions");

        Ok(())
    }
}
