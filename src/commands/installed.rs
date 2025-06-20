use anyhow::Result;
use clap::Args;
use colored::*;

use crate::{config::Config, installer::Installer, ui};

#[derive(Args)]
pub struct InstalledCommand {
    /// Show the path to each installation
    #[arg(long, short)]
    pub path: bool,
}

impl InstalledCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let installer = Installer::new(config.clone());
        let installed = installer.list_installed()?;
        let active_version = installer.get_active_version()?;

        println!("📦 Installed Godot versions:");

        if installed.is_empty() {
            ui::warning("No Godot versions installed");
            ui::info("Use 'gdm install <version>' to install a version");
            return Ok(());
        }

        for version in &installed {
            let is_active = active_version.as_ref() == Some(version);
            let marker = if is_active {
                "★".green()
            } else {
                " ".normal()
            };
            let version_str = if is_active {
                format!("{}", version).green()
            } else {
                format!("{}", version).normal()
            };

            if self.path {
                let install_path = config.installations_dir.join(version.installation_name());
                println!("  {} {} -> {}", marker, version_str, install_path.display());
            } else {
                println!("  {} {}", marker, version_str);
            }
        }

        if let Some(active) = active_version {
            println!("\n★ = active version ({})", active.to_string().green());
        } else {
            ui::warning("\nNo active version set. Use 'gdm use <version>' to set one.");
        }

        Ok(())
    }
}
