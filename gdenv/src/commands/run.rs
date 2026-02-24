use crate::cli::GlobalArgs;
use anyhow::{Context, Result, bail};
use clap::Args;
use gdenv_lib::config::Config;
use gdenv_lib::github::GitHubClient;
use gdenv_lib::godot_version::GodotVersion;
use gdenv_lib::installer;
use gdenv_lib::installer::ensure_installed;
use gdenv_lib::project_specification::read_godot_version_file;

#[derive(Args)]
pub struct RunCommand {
    /// Override the Godot version for this run
    #[arg(long)]
    pub version: Option<String>,

    /// Use the .NET version
    #[arg(long, alias = "mono")]
    pub dotnet: bool,

    /// Arguments to pass to Godot
    #[arg(last = true)]
    godot_arguments: Vec<String>,
}

impl RunCommand {
    pub async fn run(self, _global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup()?;
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

        ensure_installed(&config, &target_version, &github_client, false)
            .await
            .context(format!(
                "Failed to install Godot version {}",
                target_version
            ))?;

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
