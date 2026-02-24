use crate::cli::GlobalArgs;
use crate::ui;
use anyhow::{Context, Result, anyhow};
use clap::Args;
use gdenv_lib::config::Config;
use gdenv_lib::download_client::DownloadClient;
use gdenv_lib::github::GitHubClient;
use gdenv_lib::godot_version::GodotVersion;
use gdenv_lib::installer;
use gdenv_lib::project_specification::{ProjectSpecification, load_godot_project_spec};

#[derive(Args)]
pub struct InstallCommand {
    /// The Godot version to install (e.g., 4.2.1, 4.1.0-stable)
    /// If not provided, reads from .godot-version file
    pub version: Option<String>,

    /// Install the .NET version of Godot
    #[arg(long, alias = "mono")]
    pub dotnet: bool,

    /// Force reinstall even if version is already installed
    #[arg(long, short)]
    pub force: bool,

    /// Install the latest stable release
    #[arg(long, conflicts_with_all = ["version", "latest_prerelease"])]
    pub latest: bool,

    /// Install the latest prerelease (beta, rc, etc.)
    #[arg(long, conflicts_with_all = ["version", "latest"])]
    pub latest_prerelease: bool,
}

impl InstallCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;
        let github_client = GitHubClient::new(&config);
        ui::info(&github_client.cache_status_message());

        let project_spec = self.project_spec(global_args, &github_client).await?;

        let install_path = installer::ensure_installed(
            &config,
            &project_spec.godot_version,
            &github_client,
            self.force,
        )
        .await
        .context(format!(
            "Failed to install Godot version {}",
            project_spec.godot_version
        ))?;

        ui::success(&format!("Installed to: {}", install_path.display()));

        tracing::info!("");
        // Only set as active version if no version is currently active
        if installer::get_active_version(&config)?.is_none() {
            installer::set_active_version(&config, &project_spec.godot_version)?;
            ui::info(&format!(
                "Using Godot {} as active version (first installation).",
                project_spec.godot_version
            ));
        } else {
            ui::tip(&format!(
                "Run `gdenv godot use {}{}` to switch to this version.",
                project_spec.godot_version.as_godot_version_str(),
                if project_spec.godot_version.is_dotnet {
                    " --dotnet"
                } else {
                    ""
                }
            ));
        }
        ui::tip("Run `gdenv godot current` for PATH setup instructions.");

        Ok(())
    }

    async fn project_spec(
        &self,
        global_args: GlobalArgs,
        github_client: &GitHubClient,
    ) -> Result<ProjectSpecification> {
        // Fetch available releases from GitHub first (needed for --latest flags)
        let release_versions = github_client
            .godot_releases(false)
            .await?
            .iter()
            .map(|release| release.version.clone())
            .collect::<Vec<_>>();
        let version_override = self.override_version(release_versions)?;
        let working_dir = global_args.project.unwrap_or(std::env::current_dir()?);
        let spec_from_file = load_godot_project_spec(&working_dir)?;
        Ok(ProjectSpecification {
            godot_version: version_override.unwrap_or(spec_from_file.godot_version),
            ..spec_from_file
        })
    }

    fn override_version(
        &self,
        release_versions: Vec<GodotVersion>,
    ) -> Result<Option<GodotVersion>> {
        let version_override = if self.latest {
            // Find latest stable release (last one since it's sorted ascending)
            Some(
                release_versions
                    .iter()
                    .rfind(|v| !v.is_prerelease() && v.is_dotnet == self.dotnet)
                    .cloned()
                    .ok_or_else(|| anyhow!("No stable releases found"))?,
            )
        } else if self.latest_prerelease {
            // Find latest release (including prereleases)
            Some(
                release_versions
                    .iter()
                    .rfind(|v| v.is_dotnet == self.dotnet)
                    .cloned()
                    .ok_or_else(|| anyhow!("No releases found"))?,
            )
        } else {
            self.version
                .clone()
                .map(|v| GodotVersion::new(&v, self.dotnet))
                .transpose()?
        };
        Ok(version_override)
    }
}
