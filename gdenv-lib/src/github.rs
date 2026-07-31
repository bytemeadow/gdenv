//! Download client implementation for GitHub releases.

use crate::config::Config;
use crate::download_client::DownloadClient;
use crate::godot::get_platform_patterns;
use crate::godot_version::GodotVersion;
use crate::logging::{progress_bar_style, spinner_style};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use colored::Colorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tracing::instrument;
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub const CACHE_VALIDITY_DAYS: u64 = 7;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GitHubRelease {
    pub version: GodotVersion,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Matches the GitHub API JSON response for a single release
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct GitHubReleaseJson {
    pub tag_name: String,
    pub assets: Vec<GitHubAssetJson>,
}

/// Matches the GitHub API JSON response for a single release asset
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct GitHubAssetJson {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

impl GitHubRelease {
    /// Find a Godot asset for the current platform
    pub fn find_godot_asset(&self, is_dotnet: bool, os: &str, arch: &str) -> Result<&GitHubAsset> {
        if self.assets.is_empty() {
            bail!("There are no assets available to search for.");
        }

        let platform_patterns = get_platform_patterns(os, arch);

        // Try to find an asset matching our platform patterns (in order of preference)
        for pattern in &platform_patterns {
            if let Some(asset) = self.assets.iter().find(|asset| {
                let name = asset.name.to_lowercase();
                let has_platform = name.contains(pattern);
                let has_godot = name.contains("godot");
                let has_mono = name.contains("mono");
                let is_zip = name.ends_with(".zip");

                has_platform && has_godot && is_zip && (is_dotnet == has_mono)
            }) {
                return Ok(asset);
            }
        }

        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        bail!(
            "No matching Godot asset found for the current platform: OS={}, ARCH={}",
            os,
            arch
        );
    }

    fn from_json_struct(json: &GitHubReleaseJson) -> Result<Self> {
        let version =
            GodotVersion::new(&json.tag_name, false).context("Failed to parse Godot version")?;
        let assets = json
            .assets
            .iter()
            .map(|a| GitHubAsset {
                name: a.name.clone(),
                browser_download_url: a.browser_download_url.clone(),
                size: a.size,
            })
            .collect();
        Ok(GitHubRelease { version, assets })
    }
}

pub struct GitHubClient {
    config: Config,
    client: Client,
}

impl DownloadClient for GitHubClient {
    /// Returns a sorted list of all available Godot releases.
    /// If `force_refresh` is true, fetches the latest list from GitHub.
    /// Otherwise, uses a cached list if it exists and was modified less than 6 months ago.
    /// If `partial_fetch` is true, fetches only the latest 100 releases (1 page) from GitHub.
    async fn godot_releases(
        &self,
        force_refresh: bool,
        partial_fetch: bool,
    ) -> Result<Vec<GitHubRelease>> {
        let cache_file = self.config.cache_dir.join("releases_cache.json");

        if !force_refresh && self.is_cache_valid(&cache_file) {
            return self
                .load_cache(&cache_file)
                .context("Failed to load releases cache. Use `gdenv godot fetch` to refresh it.");
        }

        let cache_exists = cache_file.exists();
        let new_releases = self.fetch_releases_from_api(partial_fetch).await?;

        let mut all_releases = if cache_exists && partial_fetch {
            self.merge_with_cache(new_releases, &cache_file)?
        } else {
            new_releases
        };

        all_releases.sort();

        if let Err(e) = self.save_cache(&cache_file, &all_releases) {
            bail!("Failed to save releases cache: {}", e);
        }

        Ok(all_releases)
    }

    #[instrument(skip_all)]
    async fn download_asset(&self, asset: &GitHubAsset, path: &Path) -> Result<()> {
        let current_span = tracing::Span::current();
        current_span.pb_set_style(&progress_bar_style()?);
        current_span.pb_set_length(asset.size);
        current_span.pb_set_message(&format!("Downloading {}...", asset.name));
        current_span.pb_set_finish_message(&format!("Downloading {}... Complete!", asset.name));

        let response = self.client.get(&asset.browser_download_url).send().await?;

        if !response.status().is_success() {
            bail!("Download failed: {}", response.status());
        }

        // Create the file
        let mut file = tokio::fs::File::create(path).await?;
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            // Update the span field so a subscriber can see progress
            tracing::Span::current().pb_set_position(downloaded);
        }

        file.flush().await?;
        Ok(())
    }
}

impl GitHubClient {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .user_agent("gdenv/0.1.0")
            .build()
            .expect("Failed to create HTTP client");
        Self { config, client }
    }

    pub fn cache_status_message(&self) -> String {
        let cache_file = self.config.cache_dir.join("releases_cache.json");

        if let Ok(metadata) = std::fs::metadata(cache_file)
            && let Ok(modified) = metadata.modified()
        {
            let datetime: DateTime<Utc> = modified.into();
            let local_time = datetime.with_timezone(&chrono::Local);

            let now = chrono::Local::now();
            let days_ago = now.signed_duration_since(local_time).num_days().max(0);

            format!(
                "{} {} {} {}",
                "GitHub release cache:".cyan(),
                "Last fetch:".dimmed(),
                format!("{days_ago}").green().bold(),
                "days ago.".dimmed(),
            )
        } else {
            format!(
                "{} {}",
                "GitHub release cache:".cyan(),
                "Cache is empty.".dimmed(),
            )
        }
    }

    #[instrument(skip_all)]
    async fn fetch_releases_from_api(&self, partial_fetch: bool) -> Result<Vec<GitHubRelease>> {
        let message = if partial_fetch {
            "Fetching first page of Godot releases from GitHub..."
        } else {
            "Fetching Godot releases from GitHub..."
        };
        let finish_message = if partial_fetch {
            "Fetching first page of Godot releases from GitHub... Done"
        } else {
            "Fetching Godot releases from GitHub... Done"
        };

        let current_span = tracing::Span::current();
        current_span.pb_set_style(&spinner_style("{msg} [Fetch count: {pos}]")?);
        current_span.pb_set_message(message);
        current_span.pb_set_finish_message(finish_message);

        let mut releases = Vec::new();
        let mut next_url = Some(
            "https://api.github.com/repos/godotengine/godot-builds/releases?per_page=100"
                .to_string(),
        );

        while let Some(url) = next_url {
            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                bail!("GitHub API request failed: {}", response.status());
            }

            let link_header = response
                .headers()
                .get("link")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            let page_releases: Vec<GitHubReleaseJson> = response.json().await?;
            releases.extend(page_releases);

            current_span.pb_set_position(releases.len() as u64);

            if partial_fetch || releases.len() >= 1000 {
                break;
            }

            next_url = link_header.and_then(|h| self.parse_next_link(&h));
        }

        let mut all_releases = Vec::new();

        for json in releases {
            match GitHubRelease::from_json_struct(&json) {
                Ok(release) => {
                    // Add the standard version
                    all_releases.push(release.clone());

                    // Add the .NET version
                    let mut dotnet_release = release;
                    dotnet_release.version.is_dotnet = true;
                    all_releases.push(dotnet_release);
                }
                Err(e) => {
                    tracing::error!(
                        "Warn: Failed to parse release from GitHub API response; this release will be unavailable to download: {}, reason: {}",
                        json.tag_name,
                        e
                    );
                }
            }
        }

        Ok(all_releases)
    }

    fn parse_next_link(&self, link_header: &str) -> Option<String> {
        for part in link_header.split(',') {
            if part.contains("rel=\"next\"") {
                return part
                    .split(';')
                    .next()
                    .map(|s| s.trim().trim_matches(|c| c == '<' || c == '>').to_string());
            }
        }
        None
    }

    /// Merge the fetched releases with the cached ones, ensuring no duplicates
    fn merge_with_cache(
        &self,
        releases: Vec<GitHubRelease>,
        cache_file: &Path,
    ) -> Result<Vec<GitHubRelease>> {
        let mut all = releases;
        all.extend(
            self.load_cache(cache_file)
                .context("Failed to load releases cache for partial fetch.")?,
        );
        all.sort();
        all.dedup_by(|a, b| a.version == b.version);
        Ok(all)
    }

    /// A cache file is valid if it exists.
    fn is_cache_valid(&self, path: &Path) -> bool {
        path.exists()
    }

    pub fn is_cache_stale(&self) -> bool {
        let path = self.config.cache_dir.join("releases_cache.json");
        if !path.exists() {
            return false;
        }

        if let Ok(metadata) = std::fs::metadata(path)
            && let Ok(modified) = metadata.modified()
        {
            let now = std::time::SystemTime::now();
            if let Ok(duration) = now.duration_since(modified) {
                return duration.as_secs() >= CACHE_VALIDITY_DAYS * 24 * 60 * 60;
            }
        }
        false
    }

    fn load_cache(&self, path: &Path) -> Result<Vec<GitHubRelease>> {
        let content = std::fs::read_to_string(path)?;
        let mut releases: Vec<GitHubRelease> = serde_json::from_str(&content)?;
        releases.sort();

        Ok(releases)
    }

    fn save_cache(&self, path: &Path, releases: &[GitHubRelease]) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(releases)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Ord for GitHubRelease {
    fn cmp(&self, other: &Self) -> Ordering {
        self.version.cmp(&other.version)
    }
}

impl PartialOrd for GitHubRelease {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_find_godot_asset() -> Result<()> {
        // Create a mock release with various assets for all platforms
        let assets = vec![
            GitHubAssetJson {
                name: "Godot_v4.2.1-stable_linux.x86_64.zip".to_string(),
                browser_download_url: "https://example.com/linux64".to_string(),
                size: 1000,
            },
            GitHubAssetJson {
                name: "Godot_v4.2.1-stable_linux.arm32.zip".to_string(),
                browser_download_url: "https://example.com/arm32".to_string(),
                size: 1000,
            },
            GitHubAssetJson {
                name: "Godot_v4.2.1-stable_mono_linux_x86_64.zip".to_string(),
                browser_download_url: "https://example.com/mono-linux".to_string(),
                size: 1000,
            },
            GitHubAssetJson {
                name: "Godot_v4.2.1-stable_win64.exe.zip".to_string(),
                browser_download_url: "https://example.com/win64".to_string(),
                size: 1000,
            },
            GitHubAssetJson {
                name: "Godot_v4.2.1-stable_mono_win64.exe.zip".to_string(),
                browser_download_url: "https://example.com/mono-win".to_string(),
                size: 1000,
            },
            GitHubAssetJson {
                name: "Godot_v4.2.1-stable_macos.universal.zip".to_string(),
                browser_download_url: "https://example.com/macos".to_string(),
                size: 1000,
            },
            GitHubAssetJson {
                name: "Godot_v4.2.1-stable_mono_macos.universal.zip".to_string(),
                browser_download_url: "https://example.com/mono-macos".to_string(),
                size: 1000,
            },
        ];

        let release = GitHubRelease::from_json_struct(&GitHubReleaseJson {
            tag_name: "4.2.1-stable".to_string(),
            assets,
        })?;

        // Test finding regular asset
        let asset = release.find_godot_asset(false, std::env::consts::OS, std::env::consts::ARCH);
        assert!(asset.is_ok());
        let asset = asset?;
        assert!(asset.name.to_lowercase().contains("godot"));
        assert!(!asset.name.to_lowercase().contains("mono"));

        // Test finding .NET asset
        let dotnet_asset =
            release.find_godot_asset(true, std::env::consts::OS, std::env::consts::ARCH);
        assert!(dotnet_asset.is_ok());
        let dotnet_asset = dotnet_asset?;
        assert!(dotnet_asset.name.to_lowercase().contains("mono"));
        Ok(())
    }

    #[test]
    fn test_version_sorting() -> Result<()> {
        let v1 = GodotVersion::new("3.5.3-stable", false)?;
        let v2 = GodotVersion::new("4.0-alpha1", false)?;
        let v3 = GodotVersion::new("4.0-beta1", false)?;
        let v4 = GodotVersion::new("4.0-rc1", false)?;
        let v5 = GodotVersion::new("4.0-stable", false)?;
        let v6 = GodotVersion::new("4.1-stable", false)?;
        let v7 = GodotVersion::new("4.2-dev1", false)?;
        let v8 = GodotVersion::new("4.2", false)?;

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v4 < v5);
        assert!(v5 < v6);
        assert!(v7 > v6);
        assert!(v8 > v7);

        let mut versions = vec![
            v6.clone(),
            v1.clone(),
            v5.clone(),
            v3.clone(),
            v2.clone(),
            v4.clone(),
            v7.clone(),
            v8.clone(),
        ];
        versions.sort();

        assert_eq!(versions, vec![v1, v2, v3, v4, v5, v6, v7, v8]);
        Ok(())
    }

    #[test]
    fn test_dedup_bug_repro() -> Result<()> {
        let v_normal = GodotVersion::new("4.2.1", false)?;
        let v_dotnet = GodotVersion::new("4.2.1", true)?;

        let r1 = GitHubRelease {
            version: v_normal.clone(),
            assets: vec![],
        };
        let r2 = GitHubRelease {
            version: v_dotnet.clone(),
            assets: vec![],
        };
        let r3 = GitHubRelease {
            version: v_normal.clone(),
            assets: vec![],
        };

        let mut releases = vec![r1, r2, r3];
        releases.sort();

        // If bug exists, releases might be [r1, r2, r3] because all are "equal" in cmp
        // and sort is stable. dedup_by only checks neighbors.
        releases.dedup_by(|a, b| a.version == b.version);

        // We expect r1 and r3 to be deduped, leaving 2 releases (normal and dotnet)
        assert_eq!(
            releases.len(),
            2,
            "Duplicate normal releases were not deduped!"
        );
        Ok(())
    }
}
