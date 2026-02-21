use crate::cli::GlobalArgs;
use anyhow::{Context, Result, bail};
use clap::Args;
use gdenv_lib::config::Config;
use gdenv_lib::github::GitHubClient;
use gdenv_lib::godot_version::GodotVersion;
use gdenv_lib::installer;
use gdenv_lib::installer::ensure_installed;
use gdenv_lib::project_specification::{ProjectSpecification, load_godot_project_spec};

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
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;
        let github_client = GitHubClient::new(&config);

        let override_version = self
            .version
            .map(|v| GodotVersion::new(&v, self.dotnet))
            .transpose()?;
        let override_run_args = if self.godot_arguments.is_empty() {
            None
        } else {
            Some(self.godot_arguments)
        };
        let working_dir = global_args.project.unwrap_or(std::env::current_dir()?);
        let spec_from_file = load_godot_project_spec(&working_dir)?;
        let project_spec = ProjectSpecification {
            godot_version: override_version.unwrap_or(spec_from_file.godot_version),
            run_args: override_run_args.unwrap_or(spec_from_file.run_args),
            ..spec_from_file
        };

        ensure_installed(&config, &project_spec.godot_version, &github_client, false)
            .await
            .context(format!(
                "Failed to install Godot version {}",
                project_spec.godot_version
            ))?;

        let executable_path = installer::get_executable_path(&config, &project_spec.godot_version)?;

        if !executable_path.exists() {
            bail!("Executable not found at {}", executable_path.display());
        }

        let mut child = std::process::Command::new(executable_path)
            .current_dir(project_spec.project_path)
            .args(&project_spec.run_args)
            .spawn()
            .context("Failed to start Godot process")?;

        let status = child.wait()?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }

        Ok(())
    }
}
