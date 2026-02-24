use crate::download_client::DownloadClient;
use crate::github::{GitHubAsset, GitHubRelease};
use crate::godot_version::GodotVersion;
use anyhow::Context;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub struct MockDownloadClient;
impl DownloadClient for MockDownloadClient {
    async fn godot_releases(&self, _force_refresh: bool) -> anyhow::Result<Vec<GitHubRelease>> {
        Ok(vec![GitHubRelease {
            version: GodotVersion::new("4.2.1-stable", false)?,
            assets: vec![GitHubAsset {
                name: "Godot_v4.2.1-stable_linux.x86_64.zip".to_string(),
                browser_download_url: "https://example.com/linux64".to_string(),
                size: 1000,
            }],
        }])
    }

    async fn download_asset(&self, _asset: &GitHubAsset, output_path: &Path) -> anyhow::Result<()> {
        // We'll use a Vec<u8> to store the zip in memory,
        // but you could use a std::fs::File instead.
        let mut zip_buffer = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut zip_buffer));

        // Define the options.
        // 0o755 is a standard permission for an executable (rwxr-xr-x).
        #[cfg(unix)]
        let options = SimpleFileOptions::default().unix_permissions(0o755);

        #[cfg(not(unix))]
        let options = SimpleFileOptions::default();

        // Create the 'godot' file inside the zip
        zip.start_file("Godot_v4.2.1-stable_linux.x86_64", options)?;

        // The file content is empty, so we don't need to write anything here.
        // If you wanted content, you'd do: zip.write_all(b"content")?;
        zip.finish()?;

        // For testing: Write the result to an actual file to verify
        fs::write(&output_path, zip_buffer)
            .context(format!("Failed to write zip file: {:?}", output_path))?;

        Ok(())
    }
}
