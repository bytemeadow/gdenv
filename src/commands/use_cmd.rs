use anyhow::Result;
use clap::Args;

use crate::{config::Config, godot::GodotVersion, installer::Installer, ui};

#[derive(Args)]
pub struct UseCommand {
    /// The Godot version to switch to
    /// If not provided, reads from .godot-version file
    pub version: Option<String>,

    /// Use the .NET version
    #[arg(long)]
    pub dotnet: bool,
}

impl UseCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let installer = Installer::new(config);

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
            ui::error(&format!("Godot v{target_version} is not installed"));
            ui::info("Available installed versions:");

            for version in &installed_versions {
                println!("  • {version}");
            }

            if installed_versions.is_empty() {
                ui::info("No versions installed. Use 'gdenv install <version>' to install one.");
            } else {
                ui::info("Use 'gdenv installed' to see all installed versions");
            }

            return Ok(());
        }

        // Switch to the version
        installer.set_active_version(&target_version)?;

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
