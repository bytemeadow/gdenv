use crate::config::Config;
use crate::godot::get_platform_patterns;
use crate::godot_version::GodotVersion;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::Path;
use tokio::io::AsyncWriteExt;

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
    pub fn find_godot_asset(&self, is_dotnet: bool) -> Option<&GitHubAsset> {
        let platform_patterns = get_platform_patterns();

        // Try to find an asset matching our platform patterns (in order of preference)
        for pattern in platform_patterns {
            if let Some(asset) = self.assets.iter().find(|asset| {
                let name = asset.name.to_lowercase();
                let has_platform = name.contains(pattern);
                let has_godot = name.contains("godot");
                let has_mono = name.contains("mono");
                let is_zip = name.ends_with(".zip");

                has_platform && has_godot && is_zip && (is_dotnet == has_mono)
            }) {
                return Some(asset);
            }
        }
        None
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
    client: Client,
}

impl GitHubClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("gdenv/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Returns a sorted list of all available Godot releases.
    /// If `force_refresh` is true, fetches the latest list from GitHub.
    /// Otherwise, uses a cached list if it exists and was modified less than 6 months ago.
    pub async fn get_godot_releases(&self, force_refresh: bool) -> Result<Vec<GitHubRelease>> {
        let cache_file = Config::new()?.cache_dir.join("releases_cache.json");

        if !force_refresh && self.is_cache_valid(&cache_file) {
            return self
                .load_cache(&cache_file)
                .context("Failed to load releases cache. Use `gdenv update` to refresh it.");
        }

        let releases = self.fetch_all_releases_from_api().await?;

        let mut sorted_releases = releases;
        sorted_releases.sort();

        if let Err(e) = self.save_cache(&cache_file, &sorted_releases) {
            eprintln!("⚠️ Failed to save releases cache: {}", e);
        }

        Ok(sorted_releases)
    }

    async fn fetch_all_releases_from_api(&self) -> Result<Vec<GitHubRelease>> {
        let mut releases = Vec::new();
        let mut next_url = Some(
            "https://api.github.com/repos/godotengine/godot-builds/releases?per_page=100"
                .to_string(),
        );

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}")?);
        pb.set_message("Fetching Godot releases from GitHub...");
        pb.enable_steady_tick(std::time::Duration::from_millis(120));

        while let Some(url) = next_url {
            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(anyhow!("GitHub API request failed: {}", response.status()));
            }

            let link_header = response
                .headers()
                .get("link")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            let page_releases: Vec<GitHubReleaseJson> = response.json().await?;
            releases.extend(page_releases);

            if releases.len() >= 1000 {
                break;
            }

            next_url = link_header.and_then(|h| self.parse_next_link(&h));
        }

        pb.finish_and_clear();
        Ok(releases.iter().filter_map(|json| {
            match GitHubRelease::from_json_struct(json) {
                Ok(release) => Some(release),
                Err(e) => {
                    eprintln!("Warn: Failed to parse release from GitHub API response; this release will be unavailable to download: {}, reason: {}", json.tag_name, e);
                    None
                }
            }
        }).collect())
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

    /// A cache file is valid if it exists and was modified less than 6 months ago.
    fn is_cache_valid(&self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let now = std::time::SystemTime::now();
                if let Ok(duration) = now.duration_since(modified) {
                    // 6 months is roughly 180 days
                    return duration.as_secs() < 180 * 24 * 60 * 60;
                }
            }
        }
        false
    }

    fn load_cache(&self, path: &Path) -> Result<Vec<GitHubRelease>> {
        let content = std::fs::read_to_string(path)?;
        let releases: Vec<GitHubRelease> = serde_json::from_str(&content)?;

        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let datetime: DateTime<Utc> = modified.into();
                let local_time = datetime.with_timezone(&chrono::Local);

                let now = chrono::Local::now();
                let days_ago = now.signed_duration_since(local_time).num_days().max(0);

                println!(
                    "✨ Releases cache last updated: {} ({} days ago)",
                    local_time.format("%Y-%m-%d %I:%M%P"),
                    days_ago
                );
            }
        }

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

    pub async fn download_asset_with_progress(
        &self,
        asset: &GitHubAsset,
        path: &Path,
    ) -> Result<()> {
        println!("📥 Downloading {}", asset.name);

        let response = self.client.get(&asset.browser_download_url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Download failed: {}", response.status()));
        }

        let total_size = asset.size;

        // Create progress bar
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
                .progress_chars("#>-"),
        );

        // Create the file
        let mut file = tokio::fs::File::create(path).await?;
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        file.flush().await?;
        pb.finish_with_message("✅ Download complete");

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

    #[test]
    fn test_find_godot_asset() {
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
        })
        .unwrap();

        // Test finding regular asset
        let asset = release.find_godot_asset(false);
        assert!(asset.is_some());
        let asset = asset.unwrap();
        assert!(asset.name.to_lowercase().contains("godot"));
        assert!(!asset.name.to_lowercase().contains("mono"));

        // Test finding .NET asset
        let dotnet_asset = release.find_godot_asset(true);
        assert!(dotnet_asset.is_some());
        let dotnet_asset = dotnet_asset.unwrap();
        assert!(dotnet_asset.name.to_lowercase().contains("mono"));
    }

    #[test]
    fn test_version_sorting() {
        let v1 = GodotVersion::new("3.5.3-stable", false).unwrap();
        let v2 = GodotVersion::new("4.0-alpha1", false).unwrap();
        let v3 = GodotVersion::new("4.0-beta1", false).unwrap();
        let v4 = GodotVersion::new("4.0-rc1", false).unwrap();
        let v5 = GodotVersion::new("4.0-stable", false).unwrap();
        let v6 = GodotVersion::new("4.1-stable", false).unwrap();
        let v7 = GodotVersion::new("4.2-dev1", false).unwrap();
        let v8 = GodotVersion::new("4.2", false).unwrap();

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
    }
}
