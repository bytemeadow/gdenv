use crate::cli::GlobalArgs;
use crate::ui;
use anyhow::Result;
use clap::Args;
use gdenv_lib::config::Config;
use gdenv_lib::godot_version::GodotVersion;
use gdenv_lib::installer;
use std::io::{self, Write};

#[derive(Args)]
pub struct UninstallCommand {
    /// The Godot version to uninstall
    pub version: String,

    /// Uninstall the .NET version
    #[arg(long, alias = "mono")]
    pub dotnet: bool,

    /// Skip confirmation prompt
    #[arg(long, short)]
    pub yes: bool,
}

impl UninstallCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;

        let is_dotnet = self.dotnet;
        let target_version = GodotVersion::new(&self.version, is_dotnet)?;

        // Check if the version is installed
        let installed_versions = installer::list_installed(&config)?;
        if !installed_versions.contains(&target_version) {
            ui::warning(&format!("Godot {target_version} is not installed."));
            return Ok(());
        }

        // Check if it's the active version
        let active_version = installer::get_active_version(&config)?;
        let is_active = active_version.as_ref() == Some(&target_version);

        if is_active {
            ui::warning(&format!(
                "Godot {target_version} is currently the active version, \
                uninstalling will break the `godot` command."
            ));
        }

        // Confirmation prompt
        if !self.yes {
            ui::question(&format!(
                "Are you sure you want to uninstall Godot {target_version}? [y/N]: "
            ));
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let confirmed = input.trim().to_lowercase();
            if confirmed != "y" && confirmed != "yes" {
                ui::warning("Uninstall cancelled.");
                return Ok(());
            }
        }

        ui::info(&format!("Uninstalling Godot {target_version}..."));

        // Uninstall the version
        installer::uninstall_version(&config, &target_version)?;

        ui::success(&format!("Uninstalled Godot {target_version}."));

        // If it was the active version, suggest setting a new one
        if is_active {
            let remaining_versions = installer::list_installed(&config)?;
            if !remaining_versions.is_empty() {
                ui::tip("Available versions to switch to:");
                for version in &remaining_versions {
                    tracing::info!("  - {version}");
                }
                ui::tip("Run `gdenv godot use <version>` to set a new active version");
            } else {
                ui::tip(
                    "No Godot versions remaining. Run `gdenv godot install <version>` to install one.",
                );
            }
        }

        Ok(())
    }
}
