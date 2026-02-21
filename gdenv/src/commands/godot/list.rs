use crate::cli::GlobalArgs;
use crate::ui;
use anyhow::Result;
use clap::Args;
use colored::Colorize;
use gdenv_lib::config::Config;
use gdenv_lib::download_client::DownloadClient;
use gdenv_lib::github::GitHubClient;
use gdenv_lib::godot_version::{GodotVersion, version_buffet};
use gdenv_lib::installer;

#[derive(Args)]
pub struct ListCommand {
    /// Filter versions by query (e.g., "4" "4.2", "4.2-rc", "4.2-beta")
    pub query: Option<String>,

    /// Show all versions, including pre-releases
    #[arg(long)]
    pub pre: bool,
}

impl ListCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;
        let github_client = GitHubClient::new(&config);
        let all_releases = github_client.godot_releases(false).await?;
        let installed = installer::list_installed(&config)?;
        let active_version = installer::get_active_version(&config)?;
        let all_versions: Vec<GodotVersion> = all_releases
            .iter()
            .map(|release| release.version.clone())
            .collect();

        if let Some(q) = &self.query {
            Self::print_version_matches(&all_versions, &installed, &active_version, q, self.pre);
        } else {
            Self::print_version_buffet(&all_versions, &installed, &active_version);
        }

        tracing::info!("");
        if self.query.is_none() && self.pre {
            ui::warning("Note: --pre flag only applies to version queries.");
        }
        ui::info(&github_client.cache_status_message());
        ui::tip("Use `gdenv godot fetch` to refresh the cache.");
        ui::tip("Use `gdenv godot list <string_pattern>` to filter available versions");
        ui::tip("Use `gdenv godot install <version>` to install a new version from github");
        ui::tip("Use `gdenv godot use <version>` to set the active version");

        Ok(())
    }

    pub fn print_version_matches(
        all_releases: &[GodotVersion],
        installed: &[GodotVersion],
        active_version: &Option<GodotVersion>,
        query: &str,
        all: bool,
    ) {
        let filtered_all: Vec<&GodotVersion> = all_releases
            .iter()
            .filter(|v| v.as_godot_version_str().contains(query) && !v.is_dotnet)
            .collect();
        let filtered_releases: Vec<&GodotVersion> = filtered_all
            .iter()
            .filter(|v| !v.is_prerelease())
            .copied()
            .collect();
        let count_all = filtered_all.len();
        let count_releases = filtered_releases.len();
        let smart_filtered = if all || count_releases == 0 {
            filtered_all
        } else {
            filtered_releases
        };

        // Print version matches
        Self::print_versions(&smart_filtered, installed, active_version.as_ref());

        // Print statistics
        if count_all == 0 {
            ui::info(&format!("0 matches found for '{}'", query));
        } else if !all && count_releases == 0 {
            ui::info(&format!(
                "{} matches found for '{}'. Note: 0 release matches, pre-releases added to results.",
                count_all, query,
            ));
        } else {
            ui::info(&format!(
                "Found {} matches for query: '{}'{}",
                count_all,
                query,
                if all { "" } else { " (pre-releases excluded)" }
            ));
        }
        tracing::info!("");
    }

    pub fn print_version_buffet(
        all_versions: &[GodotVersion],
        installed: &[GodotVersion],
        active_version: &Option<GodotVersion>,
    ) {
        ui::info(&format!(
            "{}",
            &"Recent Godot releases available from GitHub:"
                .underline()
                .green()
        ));
        let mut buffet = version_buffet(all_versions);
        buffet.extend(installed);
        buffet.sort();
        buffet.dedup();
        Self::print_versions(&buffet, installed, active_version.as_ref());
    }

    fn print_versions(
        versions: &[&GodotVersion],
        installed: &[GodotVersion],
        active_version: Option<&GodotVersion>,
    ) {
        if versions.is_empty() {
            ui::warning("No releases found");
            return;
        }
        let width = versions
            .iter()
            .map(|release| release.to_string().len())
            .max()
            .unwrap_or(0);
        for release in versions {
            let version_str = release.to_string();
            let pre_release_str = if release.is_prerelease() {
                " (pre-release)".yellow()
            } else {
                "".to_string().normal()
            };
            let is_installed_str = if installed.contains(release) {
                " (installed)".green()
            } else {
                "".to_string().normal()
            };
            let is_active_str = if active_version == Some(release) {
                " (active)".bright_blue()
            } else {
                "".to_string().normal()
            };
            ui::info(
                format!(
                    "{:width$}{}{}{}",
                    version_str,
                    pre_release_str,
                    is_installed_str,
                    is_active_str,
                    width = width,
                )
                .trim_end(),
            );
        }
    }
}
