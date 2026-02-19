use anyhow::{Context, Result, anyhow};
use clap::Args;

use crate::download_client::DownloadClient;
use crate::project_specification::read_godot_version_file;
use crate::{config::Config, github::GitHubClient, godot_version::GodotVersion, installer, ui};

#[derive(Args)]
pub struct InstallCommand {
    /// The Godot version to install (e.g., 4.2.1, 4.1.0-stable)
    /// If not provided, reads from .godot-version file
    pub version: Option<String>,

    /// Install the .NET version of Godot
    #[arg(long, alias = "mono")]
    pub dotnet: bool,

    /// Force reinstall even if version is already installed
    #[arg(long, short)]
    pub force: bool,

    /// Install the latest stable release
    #[arg(long, conflicts_with_all = ["version", "latest_prerelease"])]
    pub latest: bool,

    /// Install the latest prerelease (beta, rc, etc.)
    #[arg(long, conflicts_with_all = ["version", "latest"])]
    pub latest_prerelease: bool,
}

impl InstallCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::setup()?;
        let github_client = GitHubClient::new();
        ui::info(&github_client.cache_status_message());

        // Fetch available releases from GitHub first (needed for --latest flags)
        let releases = github_client.godot_releases(false).await?;

        // Get the version to install
        let requested_version = if self.latest {
            // Find latest stable release (last one since it's sorted ascending)
            releases
                .iter()
                .rfind(|r| !r.version.is_prerelease())
                .map(|r| r.version.clone())
                .ok_or_else(|| anyhow!("No stable releases found"))?
        } else if self.latest_prerelease {
            // Find latest release (including prereleases)
            releases
                .last()
                .map(|r| r.version.clone())
                .ok_or_else(|| anyhow!("No releases found"))?
        } else {
            match GodotVersion::new(&self.version.clone().unwrap_or("".to_string()), self.dotnet) {
                Ok(v) => v,
                Err(_) => {
                    // Try to read from .godot-version file
                    GodotVersion::new(&read_godot_version_file()?, self.dotnet)?
                }
            }
        };

        ui::info(&format!("Installing Godot {requested_version}..."));

        let install_path =
            installer::ensure_installed(&config, &requested_version, &github_client, self.force)
                .await
                .context(format!(
                    "Failed to install Godot version {}",
                    requested_version
                ))?;

        ui::success(&format!("Installed to: {}", install_path.display()));

        // Only set as active version if no version is currently active
        if installer::get_active_version(&config)?.is_none() {
            installer::set_active_version(&config, &requested_version)?;
            ui::info(&format!(
                "Using Godot {requested_version} as active version (first installation)."
            ));
        } else {
            ui::tip(&format!(
                "Run `gdenv godot use {}` to switch to this version.",
                requested_version.as_godot_version_str()
            ));
        }
        ui::tip("Run `gdenv godot current` for PATH setup instructions.");

        Ok(())
    }
}
