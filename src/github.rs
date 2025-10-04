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
    /// Get platform patterns for asset matching, in order of preference
    pub fn get_platform_patterns() -> Vec<&'static str> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match (os, arch) {
            ("windows", "x86_64") => vec!["win64"],
            ("windows", "x86") => vec!["win32", "win64"], // Fallback to 64-bit if 32-bit not available
            ("macos", _) => vec!["macos"],                // macOS universal binaries
            ("linux", "x86_64") => vec!["linux.x86_64", "linux_x86_64", "linux"], // Prefer specific, fallback to generic
            ("linux", "x86") => vec![
                "linux.x86_32",
                "linux_x86_32",
                "linux.x86_64",
                "linux_x86_64",
                "linux",
            ],
            ("linux", "arm") => vec![
                "linux.arm32",
                "linux_arm32",
                "linux.arm64",
                "linux_arm64",
                "linux",
            ], // ARM32 preferred, but ARM64 compatible
            ("linux", "aarch64") => vec![
                "linux.arm64",
                "linux_arm64",
                "linux.x86_64",
                "linux_x86_64",
                "linux",
            ], // ARM64 preferred
            // Fallbacks
            ("windows", _) => vec!["win64", "win32"],
            ("linux", _) => vec!["linux.x86_64", "linux"],
            _ => vec!["linux.x86_64", "linux"], // Ultimate fallback
        }
    }
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
        let platform_patterns = Self::get_platform_patterns();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_patterns_detection() {
        // Test that we get valid platform patterns (this tests the current system)
        let patterns = GitHubRelease::get_platform_patterns();
        assert!(!patterns.is_empty());

        // All patterns should be non-empty strings
        for pattern in &patterns {
            assert!(!pattern.is_empty());
        }

        // Should contain at least one valid pattern
        let valid_patterns = [
            "win64",
            "win32",
            "macos",
            "linux.x86_64",
            "linux.x86_32",
            "linux.arm32",
            "linux.arm64",
            "linux",
        ];

        let has_valid_pattern = patterns.iter().any(|p| valid_patterns.contains(p));
        assert!(
            has_valid_pattern,
            "No valid patterns found in: {patterns:?}"
        );
    }

    #[test]
    fn test_find_godot_asset() {
        // Create a mock release with various assets for all platforms
        let assets = vec![
            GitHubAsset {
                name: "Godot_v4.2.1-stable_linux.x86_64.zip".to_string(),
                browser_download_url: "https://example.com/linux64".to_string(),
                size: 1000,
            },
            GitHubAsset {
                name: "Godot_v4.2.1-stable_linux.arm32.zip".to_string(),
                browser_download_url: "https://example.com/arm32".to_string(),
                size: 1000,
            },
            GitHubAsset {
                name: "Godot_v4.2.1-stable_mono_linux_x86_64.zip".to_string(),
                browser_download_url: "https://example.com/mono-linux".to_string(),
                size: 1000,
            },
            GitHubAsset {
                name: "Godot_v4.2.1-stable_win64.exe.zip".to_string(),
                browser_download_url: "https://example.com/win64".to_string(),
                size: 1000,
            },
            GitHubAsset {
                name: "Godot_v4.2.1-stable_mono_win64.exe.zip".to_string(),
                browser_download_url: "https://example.com/mono-win".to_string(),
                size: 1000,
            },
            GitHubAsset {
                name: "Godot_v4.2.1-stable_macos.universal.zip".to_string(),
                browser_download_url: "https://example.com/macos".to_string(),
                size: 1000,
            },
            GitHubAsset {
                name: "Godot_v4.2.1-stable_mono_macos.universal.zip".to_string(),
                browser_download_url: "https://example.com/mono-macos".to_string(),
                size: 1000,
            },
        ];

        let release = GitHubRelease {
            tag_name: "4.2.1-stable".to_string(),
            name: "Godot 4.2.1".to_string(),
            published_at: chrono::Utc::now(),
            prerelease: false,
            assets,
        };

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
    fn test_version_parsing() {
        let release = GitHubRelease {
            tag_name: "4.2.1-stable".to_string(),
            name: "Godot 4.2.1".to_string(),
            published_at: chrono::Utc::now(),
            prerelease: false,
            assets: vec![],
        };

        assert_eq!(release.version(), Some("4.2.1".to_string()));

        // Test with v prefix
        let release_v = GitHubRelease {
            tag_name: "v4.3.0-beta2".to_string(),
            name: "Godot 4.3.0 Beta 2".to_string(),
            published_at: chrono::Utc::now(),
            prerelease: true,
            assets: vec![],
        };

        assert_eq!(release_v.version(), Some("4.3.0-beta2".to_string()));
    }
}
