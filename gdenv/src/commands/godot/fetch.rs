use crate::cli::GlobalArgs;
use crate::ui;
use anyhow::Result;
use clap::Args;
use gdenv_lib::config::Config;
use gdenv_lib::download_client::DownloadClient;
use gdenv_lib::github::GitHubClient;

#[derive(Args)]
pub struct FetchCommand {
    /// Force update even if cache is recent
    #[arg(long, short)]
    pub force: bool,
}

impl FetchCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;
        let github_client = GitHubClient::new(&config);

        ui::info("Fetching available Godot versions from GitHub...");

        // Fetch releases from GitHub
        let releases = github_client.godot_releases(true).await?;

        ui::success(&format!("Found {} Godot releases", releases.len()));

        // Show the latest stable and prerelease versions (sorted ascending, so last is latest)
        let stable_releases: Vec<_> = releases
            .iter()
            .filter(|r| !r.version.is_prerelease() && !r.version.is_dotnet)
            .collect();
        let prerelease_releases: Vec<_> = releases
            .iter()
            .filter(|r| r.version.is_prerelease() && !r.version.is_dotnet)
            .collect();

        if let Some(latest_stable) = stable_releases.last() {
            ui::info(&format!("Latest stable: {}", latest_stable.version));
        }

        if let Some(latest_prerelease) = prerelease_releases.last() {
            ui::info(&format!("Latest prerelease: {}", latest_prerelease.version));
        }

        ui::success("Update complete!\n");
        ui::info(&github_client.cache_status_message());
        ui::tip("Use `gdenv godot fetch` to refresh the cache.");
        ui::tip("Use 'gdenv godot list' to see available versions.");

        Ok(())
    }
}
