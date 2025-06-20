use anyhow::Result;
use clap::Args;
use std::io::{self, Write};

use crate::{
    config::Config,
    godot::GodotVersion,
    installer::Installer,
    ui,
};

#[derive(Args)]
pub struct UninstallCommand {
    /// The Godot version to uninstall
    pub version: String,
    
    /// Uninstall the .NET version
    #[arg(long)]
    pub dotnet: bool,
    
    /// Skip confirmation prompt
    #[arg(long, short)]
    pub yes: bool,
}

impl UninstallCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let installer = Installer::new(config);
        
        let is_dotnet = self.dotnet;
        let target_version = GodotVersion::new(&self.version, is_dotnet)?;
        
        // Check if the version is installed
        let installed_versions = installer.list_installed()?;
        if !installed_versions.contains(&target_version) {
            ui::warning(&format!("Godot v{} is not installed", target_version));
            return Ok(());
        }
        
        // Check if it's the active version
        let active_version = installer.get_active_version()?;
        let is_active = active_version.as_ref() == Some(&target_version);
        
        if is_active {
            ui::warning(&format!("Godot v{} is currently the active version", target_version));
        }
        
        // Confirmation prompt
        if !self.yes {
            print!("Are you sure you want to uninstall Godot v{}? [y/N]: ", target_version);
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            let confirmed = input.trim().to_lowercase();
            if confirmed != "y" && confirmed != "yes" {
                ui::info("Uninstall cancelled");
                return Ok(());
            }
        }
        
        // Uninstall the version
        installer.uninstall_version(&target_version)?;
        
        // If it was the active version, suggest setting a new one
        if is_active {
            let remaining_versions = installer.list_installed()?;
            if !remaining_versions.is_empty() {
                ui::info("Available versions to switch to:");
                for version in &remaining_versions {
                    println!("  • {}", version);
                }
                ui::info("Use 'gdm use <version>' to set a new active version");
                ui::info("Use 'gdm installed' to see all remaining versions");
            } else {
                ui::info("No Godot versions remaining. Use 'gdm install <version>' to install one.");
            }
        }
        
        Ok(())
    }
}