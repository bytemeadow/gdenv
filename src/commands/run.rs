use anyhow::{Result, anyhow};
use clap::Args;

use crate::github::GitHubClient;
use crate::{config::Config, godot_version::GodotVersion, installer::Installer, ui};

#[derive(Args)]
pub struct RunCommand {
    /// The Godot version to invoke
    /// If not provided, reads from .godot-version file
    pub version: Option<String>,

    /// Use the .NET version
    #[arg(long, alias = "mono")]
    pub dotnet: bool,

    /// Directory to look for a project version file (.godot-version)
    pub project: Option<String>,

    /// Arguments to pass to Godot
    #[arg(last = true)]
    godot_arguments: Vec<String>,
}

impl RunCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let installer = Installer::new(config.clone());
        let github_client = GitHubClient::new();

        // Get the version to use
        let version_string = match self.version {
            Some(v) => v,
            None => {
                // Try to read from .godot-version file
                self.read_godot_version_file()?
            }
        };

        let is_dotnet = self.dotnet;
        let target_version = GodotVersion::new(&version_string, is_dotnet)?;

        // Check if the version is installed
        let installed_versions = installer.list_installed()?;
        if !installed_versions.contains(&target_version) {
            let releases = github_client.get_godot_releases(false).await?;

            // Find the matching release
            let release = releases
                .iter()
                .find(|r| {
                    // Try to match both the normalized version and the original input
                    r.version == target_version
                })
                .ok_or_else(|| anyhow!("Godot version {} not found", target_version))?;

            // Find the appropriate asset for our platform
            let asset = release
                .find_godot_asset(is_dotnet)
                .ok_or_else(|| anyhow!("No compatible Godot build found for this platform"))?;

            ui::info(&format!("Found: {}", asset.name));
            ui::info(&format!("Size: {} MB", asset.size / 1024 / 1024));

            // Create cache directory
            let archive_file = config.cache_dir.join(&asset.name);

            // Download if not cached
            if !archive_file.exists() {
                ui::info("Downloading Godot...");
                github_client
                    .download_asset_with_progress(asset, &archive_file)
                    .await?;
            } else {
                ui::info("Using cached download");
            }
            installer
                .install_version_from_archive(&target_version, &archive_file)
                .await?;
        }

        let executable_path = installer.get_executable_path(&target_version)?;

        if !executable_path.exists() {
            return Err(anyhow!(
                "Executable not found at {}",
                executable_path.display()
            ));
        }

        ui::info(&format!("Running Godot {}...", target_version));

        let mut child = std::process::Command::new(executable_path)
            .args(&self.godot_arguments)
            .spawn()
            .map_err(|e| anyhow!("Failed to start Godot: {}", e))?;

        let status = child.wait()?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }

        Ok(())
    }

    fn read_godot_version_file(&self) -> Result<String> {
        use anyhow::anyhow;
        use std::fs;
        use std::path::Path;

        let version_file = Path::new(".godot-version");

        if !version_file.exists() {
            return Err(anyhow!(
                "No version specified and no .godot-version file found in current directory.\n\
                Create a .godot-version file or specify a version: gdenv use <version>"
            ));
        }

        let content = fs::read_to_string(version_file)?;
        let version = content.trim();

        if version.is_empty() {
            return Err(anyhow!(".godot-version file is empty"));
        }

        ui::info(&format!("Reading version from .godot-version: {version}"));

        Ok(version.to_string())
    }
}
