use anyhow::{Context, Result, bail};
use clap::Args;

use crate::github::GitHubClient;
use crate::installer::ensure_installed;
use crate::project_specification::read_godot_version_file;
use crate::{data_dir_config::DataDirConfig, godot_version::GodotVersion, installer};

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
        let config = DataDirConfig::setup()?;
        let github_client = GitHubClient::new();

        // Get the version to use
        let version_string = match self.version {
            Some(v) => v,
            None => {
                // Try to read from .godot-version file
                read_godot_version_file()?
            }
        };

        let is_dotnet = self.dotnet;
        let target_version = GodotVersion::new(&version_string, is_dotnet)?;

        ensure_installed(&config, &target_version, &github_client, false).await?;

        let executable_path = installer::get_executable_path(&config, &target_version)?;

        if !executable_path.exists() {
            bail!("Executable not found at {}", executable_path.display());
        }

        let mut child = std::process::Command::new(executable_path)
            .args(&self.godot_arguments)
            .spawn()
            .context("Failed to start Godot process")?;

        let status = child.wait()?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }

        Ok(())
    }
}
