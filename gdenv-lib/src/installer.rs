use crate::download_client::DownloadClient;
use crate::godot::{godot_executable_path, godot_installation_name};
use crate::logging::spinner_style;
use crate::{config::Config, godot_version::GodotVersion};
use anyhow::{Result, anyhow, bail};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::instrument;
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub async fn ensure_installed<D: DownloadClient>(
    config: &Config,
    version: &GodotVersion,
    download_client: &D,
    force: bool,
) -> Result<PathBuf> {
    if !force && list_installed(config)?.contains(version) {
        return get_executable_path(config, version);
    }

    // 1. Fetch releases
    let releases = download_client.godot_releases(false).await?;

    // 2. Find release & asset
    let release = releases
        .iter()
        .find(|r| r.version == *version)
        .ok_or_else(|| anyhow!("Version {} not found", version))?;

    let asset = release.find_godot_asset(version.is_dotnet, &config.os, &config.arch)?;

    // 3. Download to cache
    let cache_path = config.cache_dir.join(&asset.name);
    if !cache_path.exists() {
        download_client.download_asset(asset, &cache_path).await?;
    }

    // 4. Install
    install_version_from_archive(config, version, &cache_path).await
}

#[instrument(skip_all)]
pub async fn install_version_from_archive(
    config: &Config,
    version: &GodotVersion,
    archive_path: &Path,
) -> Result<PathBuf> {
    let current_span = tracing::Span::current();
    current_span.pb_set_style(&spinner_style("{msg}")?);
    current_span.pb_set_message("Installing...");
    current_span.pb_set_finish_message("Installing... Done");

    let install_path = config
        .installations_dir
        .join(godot_installation_name(version));

    // Remove existing installation if it exists
    if install_path.exists() {
        fs::remove_dir_all(&install_path)?;
    }

    // Create installation directory
    fs::create_dir_all(&install_path)?;

    tracing::debug!("Extracting archive...");
    extract_zip(archive_path, &install_path)?;

    // Make the Godot executable... executable (Unix only)
    #[cfg(unix)]
    make_executable(&install_path)?;

    Ok(install_path)
}

fn extract_zip(archive_path: &Path, destination: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => destination.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            // Directory
            fs::create_dir_all(&outpath)?;
        } else {
            // File
            if let Some(p) = outpath.parent()
                && !p.exists()
            {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Set file permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}

#[cfg(unix)]
fn make_executable(install_path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Find the Godot executable and make it executable
    for entry in fs::read_dir(install_path)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Look for files that look like Godot executables
            if name.starts_with("Godot") && path.is_file() {
                let mut perms = fs::metadata(&path)?.permissions();
                perms.set_mode(perms.mode() | 0o755); // Add execute permissions
                fs::set_permissions(&path, perms)?;
            }
        }
    }

    Ok(())
}

pub fn uninstall_version(config: &Config, version: &GodotVersion) -> Result<()> {
    let install_path = config
        .installations_dir
        .join(godot_installation_name(version));

    if !install_path.exists() {
        bail!("Godot {} is not installed", version);
    }

    fs::remove_dir_all(&install_path)?;

    Ok(())
}

pub fn set_active_version(config: &Config, version: &GodotVersion) -> Result<()> {
    let install_path = config
        .installations_dir
        .join(godot_installation_name(version));

    if !install_path.exists() {
        bail!("Godot {} is not installed", version);
    }

    // Create new symlink
    update_symlink(&install_path, &config.active_symlink)?;

    // Create executable symlink in bin directory
    create_executable_symlink(config, &install_path, version)?;

    Ok(())
}

fn create_executable_symlink(
    config: &Config,
    install_path: &Path,
    version: &GodotVersion,
) -> Result<()> {
    let godot_executable_symlink = config.bin_dir.join("godot");
    let godot_exe_path = find_godot_executable(install_path, version, &config.os, &config.arch)?;
    update_symlink(&godot_exe_path, &godot_executable_symlink)?;
    Ok(())
}

fn find_godot_executable(
    install_path: &Path,
    version: &GodotVersion,
    os: &str,
    arch: &str,
) -> Result<PathBuf> {
    // First try the expected path based on version info
    let expected_path = godot_executable_path(version, os, arch);
    let expected_exe = install_path.join(&expected_path);

    if expected_exe.exists() && expected_exe.is_file() {
        return Ok(expected_exe);
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, the executable is inside Godot.app/Contents/MacOS/Godot
        let godot_app_exe = install_path.join("Godot.app/Contents/MacOS/Godot");
        if godot_app_exe.exists() {
            return Ok(godot_app_exe);
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, look for .exe files
        for entry in walkdir::WalkDir::new(install_path).max_depth(2) {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with("Godot")
                && name.ends_with(".exe")
            {
                return Ok(path.into());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, look for executable files starting with Godot
        for entry in walkdir::WalkDir::new(install_path).max_depth(2) {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Check if it's a Godot executable (may have version info in name)
                if name.starts_with("Godot") && path.is_file() {
                    // Check if it's executable
                    use std::os::unix::fs::PermissionsExt;
                    let metadata = fs::metadata(path)?;
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o111 != 0 {
                        return Ok(path.into());
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Could not find Godot executable in installation"
    ))
}

pub fn get_active_version(config: &Config) -> Result<Option<GodotVersion>> {
    if !config.active_symlink.exists() {
        return Ok(None);
    }

    // Read the symlink target
    let target = fs::read_link(&config.active_symlink)?;

    // Parse version from the directory name
    if let Some(dir_name) = target.file_name().and_then(|n| n.to_str())
        && let Some(version_part) = dir_name.strip_prefix("godot-")
    {
        let is_dotnet = version_part.ends_with("-dotnet");
        let version_str = if is_dotnet {
            version_part.strip_suffix("-dotnet").unwrap()
        } else {
            version_part
        };

        if let Ok(version) = GodotVersion::new(version_str, is_dotnet) {
            return Ok(Some(version));
        }
    }

    Ok(None)
}

pub fn list_installed(config: &Config) -> Result<Vec<GodotVersion>> {
    let mut versions = Vec::new();

    if !config.installations_dir.exists() {
        return Ok(versions);
    }

    for entry in fs::read_dir(&config.installations_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        if let Some(dir_name) = entry.file_name().to_str()
            && let Some(version_part) = dir_name.strip_prefix("godot-")
        {
            let is_dotnet = version_part.ends_with("-dotnet");
            let version_str = if is_dotnet {
                version_part.strip_suffix("-dotnet").unwrap()
            } else {
                version_part
            };

            if let Ok(version) = GodotVersion::new(version_str, is_dotnet) {
                versions.push(version);
            }
        }
    }

    versions.sort();
    Ok(versions)
}

pub fn get_executable_path(config: &Config, version: &GodotVersion) -> Result<PathBuf> {
    let install_path = config
        .installations_dir
        .join(godot_installation_name(version));

    if !install_path.exists() {
        bail!("Godot {} is not installed", version);
    }

    find_godot_executable(&install_path, version, &config.os, &config.arch)
}

pub fn update_symlink(original: &Path, link: &Path) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(link) {
        if metadata.file_type().is_symlink() {
            fs::remove_file(link)?;
        } else {
            tracing::warn!(
                "Won't create symlink: Found non-symlink '{}' not overwriting",
                link.to_str().unwrap_or("<unknown_path>")
            );
            return Ok(());
        }
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(original, link)?;

    #[cfg(windows)]
    {
        if original.is_dir() {
            std::os::windows::fs::symlink_dir(original, link)?;
        } else {
            std::os::windows::fs::symlink_file(original, link)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{GitHubAsset, GitHubRelease};
    use anyhow::Context;
    use std::fs::File;
    use std::io::Cursor;
    use tempdir::TempDir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    struct TestDownloadClient;
    impl DownloadClient for TestDownloadClient {
        async fn godot_releases(&self, _force_refresh: bool) -> Result<Vec<GitHubRelease>> {
            Ok(vec![GitHubRelease {
                version: GodotVersion::new("4.2.1-stable", false)?,
                assets: vec![GitHubAsset {
                    name: "Godot_v4.2.1-stable_linux.x86_64.zip".to_string(),
                    browser_download_url: "https://example.com/linux64".to_string(),
                    size: 1000,
                }],
            }])
        }

        async fn download_asset(&self, _asset: &GitHubAsset, output_path: &Path) -> Result<()> {
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

    #[tokio::test]
    async fn test_installation_lifecycle() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let config = Config::setup(Some(tmp_dir.path()))?;
        let config = Config {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            ..config
        };
        let client = TestDownloadClient;
        let version = GodotVersion::new("4.2.1", false)?;
        assert_eq!(list_installed(&config)?.len(), 0);
        ensure_installed(&config, &version, &client, false).await?;
        assert_eq!(list_installed(&config)?.len(), 1);
        ensure_installed(&config, &version, &client, false).await?;
        assert_eq!(list_installed(&config)?.len(), 1);
        set_active_version(&config, &version)?;
        assert!(get_active_version(&config)?.is_some());
        get_executable_path(&config, &version)?;
        uninstall_version(&config, &version)?;
        assert_eq!(list_installed(&config)?.len(), 0);
        Ok(())
    }

    #[test]
    fn test_update_symlink_create_new() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let dir = tmp_dir.path();

        let original = dir.join("original");
        let link = dir.join("link");

        File::create(&original)?;

        update_symlink(&original, &link)?;

        assert!(link.exists());
        assert!(fs::symlink_metadata(&link)?.file_type().is_symlink());
        assert_eq!(
            fs::read_link(&link)?.canonicalize()?,
            original.canonicalize()?
        );

        Ok(())
    }

    #[test]
    fn test_update_symlink_no_overwrite_regular_file() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let dir = tmp_dir.path();

        let original = dir.join("original");
        let link = dir.join("non-symlink-file-test");

        File::create(&original)?;
        File::create(&link)?;

        update_symlink(&original, &link)?;

        assert!(link.exists());
        assert!(!fs::symlink_metadata(&link)?.file_type().is_symlink());

        Ok(())
    }

    #[test]
    fn test_update_symlink_replace_existing() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let dir = tmp_dir.path();

        let original = dir.join("original");
        let other = dir.join("other");
        let link = dir.join("link");

        File::create(&original)?;
        File::create(&other)?;

        // Create initial symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(&other, &link)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&other, &link)?;

        assert_eq!(fs::read_link(&link)?.canonicalize()?, other.canonicalize()?);

        update_symlink(&original, &link)?;

        assert!(link.exists());
        assert!(fs::symlink_metadata(&link)?.file_type().is_symlink());
        assert_eq!(
            fs::read_link(&link)?.canonicalize()?,
            original.canonicalize()?
        );

        Ok(())
    }
}
