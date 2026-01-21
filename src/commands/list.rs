use crate::config::Config;
use crate::godot::godot_installation_name;
use crate::installer::Installer;
use crate::{
    github::{GitHubClient, GitHubRelease},
    ui,
};
use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct ListCommand {
    /// Filter versions by query (e.g., "4" "4.2", "4.2-rc", "4.2-beta")
    pub query: Option<String>,

    /// Show all versions, including pre-releases
    #[arg(long)]
    pub pre: bool,
}

impl ListCommand {
    pub async fn run(self) -> Result<()> {
        let github_client = GitHubClient::new();
        let all_releases = github_client.get_godot_releases(false).await?;

        if let Some(q) = &self.query {
            Self::print_version_matches(&all_releases, q, self.pre);
        } else {
            Self::print_version_buffet(&all_releases);
            Self::print_installed_versions(true)?;
        }

        ui::info("Use 'gdenv list <string_pattern>' to search available versions");
        ui::info("Use 'gdenv install <version>' to install a new version from github");
        ui::info("Use 'gdenv use <version>' to set the active version");

        Ok(())
    }

    pub fn print_version_matches(all_releases: &[GitHubRelease], query: &str, all: bool) {
        println!();
        let filtered_all: Vec<&GitHubRelease> = all_releases
            .iter()
            .filter(|r| r.version.as_godot_version_str().contains(query))
            .collect();
        let filtered_releases: Vec<&GitHubRelease> = filtered_all
            .iter()
            .filter(|r| !r.version.is_prerelease())
            .copied()
            .collect();
        let count_all = filtered_all.len();
        let count_releases = filtered_releases.len();
        let smart_filtered = if all || count_releases == 0 {
            filtered_all
        } else {
            filtered_releases
        };
        for release in smart_filtered {
            println!("  • {}", release.version.as_godot_version_str());
        }
        if count_all == 0 {
            ui::info(&format!("0 matches found for '{}'", query));
        } else if !all && count_releases == 0 {
            ui::info(&format!(
                "{} matches found for '{}'. Note: 0 release matches, pre-releases added to results.",
                count_all, query,
            ));
        } else {
            ui::info(&format!(
                "Found {} matches for query '{}' {}.",
                count_all,
                query,
                if all { "" } else { "(pre-releases excluded)" }
            ));
        }
        println!();
    }

    fn print_version_buffet(all_releases: &[GitHubRelease]) {
        println!("🔽 Recent GitHub release versions:");
        if all_releases.last().is_some() {
            let mut most_recent_top: Vec<&GitHubRelease> = all_releases.iter().rev().collect();
            most_recent_top.dedup_by(|a, b| a.version.minor == b.version.minor);

            let mut major = 3; // Most users won't care about versions before 3.0
            while let major_releases = most_recent_top.iter().filter(|r| r.version.major == major)
                && major_releases.clone().count() > 0
            {
                println!("    Release series {}:", major);
                let top_4: Vec<&GitHubRelease> = major_releases.take(5).copied().collect();
                for release in top_4.iter().rev() {
                    if release.version.is_prerelease() {
                        println!(
                            "      • {} {}",
                            release.version.as_godot_version_str(),
                            "(pre-release)".yellow()
                        );
                    } else {
                        println!("      • {}", release.version.as_godot_version_str());
                    }
                }
                println!();
                major += 1;
            }
        } else {
            ui::warning("No releases found");
        }
    }

    fn print_installed_versions(show_paths: bool) -> Result<()> {
        let config = Config::new()?;
        let installer = Installer::new(config.clone());
        let installed = installer.list_installed()?;
        let active_version = installer.get_active_version()?;

        println!("📦 Installed versions:");

        if installed.is_empty() {
            ui::warning("    No Godot versions installed");
            ui::info("    Use 'gdenv install <version>' to install a version");
            return Ok(());
        }

        for version in &installed {
            let is_active = active_version.as_ref() == Some(version);
            let marker = if is_active {
                "★".green().bold()
            } else {
                " ".normal()
            };
            let version_str = if is_active {
                format!("{version}").green().bold()
            } else {
                format!("{version}").normal()
            };

            if show_paths {
                let install_path = config
                    .installations_dir
                    .join(godot_installation_name(version));
                println!("  {} {} -> {}", marker, version_str, install_path.display());
            } else {
                println!("  {marker} {version_str}");
            }
        }

        if let Some(active) = active_version {
            println!(
                "\n★ = active version: {}\n",
                active.to_string().green().bold()
            );
        } else {
            ui::warning("\nNo active version set. Use 'gdenv use <version>' to set one.\n");
        }

        Ok(())
    }
}
