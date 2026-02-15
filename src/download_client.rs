use crate::github::{GitHubAsset, GitHubRelease};
use anyhow::Result;
use std::path::Path;

pub trait DownloadClient {
    async fn godot_releases(&self, force_refresh: bool) -> Result<Vec<GitHubRelease>>;

    async fn download_asset(&self, asset: &GitHubAsset, output_path: &Path) -> Result<()>;
}
