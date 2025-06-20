use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub published_at: DateTime<Utc>,
    pub prerelease: bool,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

impl GitHubRelease {
    /// Parse the version from the tag name (e.g., "4.2.1-stable" -> "4.2.1")
    pub fn version(&self) -> Option<String> {
        // Godot release tags are typically like "4.2.1-stable", "4.3.0-beta2", etc.
        let tag = self.tag_name.strip_prefix("v").unwrap_or(&self.tag_name);

        // For stable releases, remove "-stable" suffix
        if let Some(version) = tag.strip_suffix("-stable") {
            Some(version.to_string())
        } else {
            // For pre-releases, keep the full tag
            Some(tag.to_string())
        }
    }

    /// Find a Godot asset for the current platform
    pub fn find_godot_asset(&self, is_dotnet: bool) -> Option<&GitHubAsset> {
        let platform = std::env::consts::OS;

        let platform_pattern = match platform {
            "windows" => "win64",
            "macos" => "macos",
            "linux" => "linux",
            _ => return None,
        };

        self.assets.iter().find(|asset| {
            let name = asset.name.to_lowercase();
            let has_platform = name.contains(platform_pattern);
            let has_godot = name.contains("godot");
            let has_mono = name.contains("mono");
            let is_zip = name.ends_with(".zip");

            has_platform && has_godot && is_zip && (is_dotnet == has_mono)
        })
    }
}

pub struct GitHubClient {
    client: Client,
    api_url: String,
}

impl GitHubClient {
    pub fn new(api_url: String) -> Self {
        let client = Client::builder()
            .user_agent("gdenv/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_url }
    }

    pub async fn get_godot_releases(
        &self,
        include_prereleases: bool,
    ) -> Result<Vec<GitHubRelease>> {
        let url = format!("{}/repos/godotengine/godot-builds/releases", self.api_url);

        println!("🔍 Fetching available Godot versions...");

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("GitHub API request failed: {}", response.status()));
        }

        let mut releases: Vec<GitHubRelease> = response.json().await?;

        if !include_prereleases {
            releases.retain(|r| !r.prerelease);
        }

        // Sort by published date (newest first)
        releases.sort_by(|a, b| b.published_at.cmp(&a.published_at));

        Ok(releases)
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
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
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
