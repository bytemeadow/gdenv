use crate::github::{GitHubAsset, GitHubRelease};
use std::path::Path;

pub trait DownloadClient {
    async fn get_godot_releases(&self, force_refresh: bool) -> anyhow::Result<Vec<GitHubRelease>>;

    async fn download_asset_with_progress(
        &self,
        asset: &GitHubAsset,
        output_path: &Path,
    ) -> anyhow::Result<()>;
}
