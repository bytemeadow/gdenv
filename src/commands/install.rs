use anyhow::{anyhow, Result};
use clap::Args;

use crate::{
    config::Config,
    github::GitHubClient,
    godot::GodotVersion,
    installer::Installer,
    ui,
};

#[derive(Args)]
pub struct InstallCommand {
    /// The Godot version to install (e.g., 4.2.1, 4.1.0-stable)
    /// If not provided, reads from .godot-version file
    pub version: Option<String>,
    
    /// Install the .NET version of Godot
    #[arg(long)]
    pub dotnet: bool,
    
    /// Force reinstall even if version is already installed
    #[arg(long, short)]
    pub force: bool,
}

impl InstallCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let github_client = GitHubClient::new(config.github_api_url.clone());
        let installer = Installer::new(config.clone());
        
        // Get the version to install
        let version_string = match self.version {
            Some(v) => v,
            None => {
                // Try to read from .godot-version file
                self.read_godot_version_file()?
            }
        };
        
        // Parse the requested version
        let is_dotnet = self.dotnet;
        let requested_version = GodotVersion::new(&version_string, is_dotnet)?;
        
        println!("🤖 Installing Godot v{}", requested_version);
        
        // Check if already installed (unless force flag is set)
        let install_path = config.installations_dir.join(requested_version.installation_name());
        if install_path.exists() && !self.force {
            ui::warning(&format!("Godot v{} is already installed", requested_version));
            ui::info("Use --force to reinstall");
            return Ok(());
        }
        
        // Fetch available releases from GitHub
        let releases = github_client.get_godot_releases(true).await?;
        
        // Find the matching release
        let release = releases.iter()
            .find(|r| {
                if let Some(version) = r.version() {
                    version == requested_version.godot_version_string()
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow!("Godot version {} not found", requested_version))?;
        
        // Find the appropriate asset for our platform
        let asset = release.find_godot_asset(is_dotnet)
            .ok_or_else(|| anyhow!("No compatible Godot build found for this platform"))?;
        
        ui::info(&format!("Found: {}", asset.name));
        ui::info(&format!("Size: {} MB", asset.size / 1024 / 1024));
        
        // Create cache directory
        let cache_file = config.cache_dir.join(&asset.name);
        
        // Download if not cached
        if !cache_file.exists() {
            ui::info("Downloading Godot...");
            github_client.download_asset_with_progress(asset, &cache_file).await?;
        } else {
            ui::info("Using cached download");
        }
        
        // Install the version
        let install_path = installer.install_version_from_archive(&requested_version, &cache_file).await?;
        
        // Set as active version
        installer.set_active_version(&requested_version)?;
        
        ui::success(&format!("Successfully installed Godot v{}", requested_version));
        ui::info(&format!("Installed to: {}", install_path.display()));
        ui::info("Run 'gdm current' for PATH setup instructions");
        
        Ok(())
    }
    
    fn read_godot_version_file(&self) -> Result<String> {
        use std::fs;
        use std::path::Path;
        
        let version_file = Path::new(".godot-version");
        
        if !version_file.exists() {
            return Err(anyhow!(
                "No version specified and no .godot-version file found in current directory.\n\
                Create a .godot-version file or specify a version: gdm install <version>"
            ));
        }
        
        let content = fs::read_to_string(version_file)?;
        let version = content.trim();
        
        if version.is_empty() {
            return Err(anyhow!(".godot-version file is empty"));
        }
        
        ui::info(&format!("Reading version from .godot-version: {}", version));
        
        Ok(version.to_string())
    }
}