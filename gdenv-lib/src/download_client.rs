use crate::github::{GitHubAsset, GitHubRelease};
use anyhow::Result;
use std::path::Path;

pub trait DownloadClient {
    fn godot_releases(
        &self,
        force_refresh: bool,
    ) -> impl Future<Output = Result<Vec<GitHubRelease>>> + Send;

    fn download_asset(
        &self,
        asset: &GitHubAsset,
        output_path: &Path,
    ) -> impl Future<Output = Result<()>> + Send;
}
