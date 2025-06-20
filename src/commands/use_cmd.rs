use anyhow::Result;
use clap::Args;

use crate::{
    config::Config,
    godot::GodotVersion,
    installer::Installer,
    ui,
};

#[derive(Args)]
pub struct UseCommand {
    /// The Godot version to switch to
    pub version: String,
    
    /// Use the .NET version
    #[arg(long)]
    pub dotnet: bool,
}

impl UseCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let installer = Installer::new(config);
        
        let is_dotnet = self.dotnet;
        let target_version = GodotVersion::new(&self.version, is_dotnet)?;
        
        // Check if the version is installed
        let installed_versions = installer.list_installed()?;
        if !installed_versions.contains(&target_version) {
            ui::error(&format!("Godot v{} is not installed", target_version));
            ui::info("Available installed versions:");
            
            for version in &installed_versions {
                println!("  • {}", version);
            }
            
            if installed_versions.is_empty() {
                ui::info("No versions installed. Use 'gdm install <version>' to install one.");
            } else {
                ui::info("Use 'gdm installed' to see all installed versions");
            }
            
            return Ok(());
        }
        
        // Switch to the version
        installer.set_active_version(&target_version)?;
        
        Ok(())
    }
}