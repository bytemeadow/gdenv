use crate::cli::GlobalArgs;
use crate::ui;
use anyhow::{Result, bail};
use clap::Args;
use gdenv_lib::config::Config;
use gdenv_lib::godot_version::GodotVersion;
use gdenv_lib::installer;

#[derive(Args)]
pub struct UseCommand {
    /// The Godot version to switch to
    /// If not provided, reads from .godot-version file
    pub version: Option<String>,

    /// Use the .NET version
    #[arg(long, alias = "mono")]
    pub dotnet: bool,
}

impl UseCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;

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
        let installed_versions = installer::list_installed(&config)?;
        if !installed_versions.contains(&target_version) {
            ui::error(&format!("Godot {target_version} is not installed"));
            ui::info("Available installed versions:");

            for version in &installed_versions {
                ui::info(&format!("  - {version}"));
            }

            if installed_versions.is_empty() {
                ui::info(
                    "No versions installed. Use `gdenv godot install <version>` to install one.",
                );
            } else {
                ui::tip("Use `gdenv godot installed` to see all installed versions");
            }

            return Ok(());
        }

        // Switch to the version
        installer::set_active_version(&config, &target_version)?;

        ui::success(&format!(
            "Switched active Godot version to {target_version}."
        ));

        Ok(())
    }

    fn read_godot_version_file(&self) -> Result<String> {
        use std::fs;
        use std::path::Path;

        let version_file = Path::new(".godot-version");

        if !version_file.exists() {
            bail!(
                "No version specified and no .godot-version file found in current directory.\n\
                Create a .godot-version file or specify a version: gdenv use <version>"
            );
        }

        let content = fs::read_to_string(version_file)?;
        let version = content.trim();

        if version.is_empty() {
            bail!(".godot-version file is empty");
        }

        ui::info(&format!("Reading version from .godot-version: {version}"));

        Ok(version.to_string())
    }
}
