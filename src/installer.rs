use crate::{config::Config, godot::GodotVersion, ui};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct Installer {
    config: Config,
}

impl Installer {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn install_version_from_archive(
        &self,
        version: &GodotVersion,
        archive_path: &Path,
    ) -> Result<PathBuf> {
        let install_path = self
            .config
            .installations_dir
            .join(version.installation_name());

        // Remove existing installation if it exists
        if install_path.exists() {
            ui::info("Removing existing installation...");
            fs::remove_dir_all(&install_path)?;
        }

        // Create installation directory
        fs::create_dir_all(&install_path)?;

        ui::info("Extracting archive...");
        self.extract_zip(archive_path, &install_path)?;

        // Make the Godot executable... executable (Unix only)
        #[cfg(unix)]
        self.make_executable(&install_path)?;

        ui::success("Installation complete");
        Ok(install_path)
    }

    fn extract_zip(&self, archive_path: &Path, destination: &Path) -> Result<()> {
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
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
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
    fn make_executable(&self, install_path: &Path) -> Result<()> {
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
                    ui::info(&format!("Made {} executable", name));
                }
            }
        }

        Ok(())
    }

    pub fn uninstall_version(&self, version: &GodotVersion) -> Result<()> {
        let install_path = self
            .config
            .installations_dir
            .join(version.installation_name());

        if !install_path.exists() {
            ui::warning(&format!("Godot v{} is not installed", version));
            return Ok(());
        }

        fs::remove_dir_all(&install_path)?;
        ui::success(&format!("Uninstalled Godot v{}", version));

        Ok(())
    }

    pub fn set_active_version(&self, version: &GodotVersion) -> Result<()> {
        self.set_active_version_with_message(version, true)
    }

    pub fn set_active_version_with_message(
        &self,
        version: &GodotVersion,
        show_message: bool,
    ) -> Result<()> {
        let install_path = self
            .config
            .installations_dir
            .join(version.installation_name());

        if !install_path.exists() {
            return Err(anyhow::anyhow!("Godot v{} is not installed", version));
        }

        // Remove existing symlink if it exists
        if self.config.active_symlink.exists() {
            if self.config.active_symlink.is_symlink() {
                fs::remove_file(&self.config.active_symlink)?;
            } else {
                fs::remove_dir_all(&self.config.active_symlink)?;
            }
        }

        // Create new symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(&install_path, &self.config.active_symlink)?;

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&install_path, &self.config.active_symlink)?;

        // Create executable symlink in bin directory
        self.create_executable_symlink(&install_path, version)?;

        if show_message {
            ui::success(&format!("Switched to Godot v{}", version));
        }

        Ok(())
    }

    fn create_executable_symlink(
        &self,
        install_path: &std::path::Path,
        version: &GodotVersion,
    ) -> Result<()> {
        let godot_executable_symlink = self.config.bin_dir.join("godot");

        // Remove existing symlink if it exists
        if godot_executable_symlink.exists() {
            if godot_executable_symlink.is_symlink() {
                fs::remove_file(&godot_executable_symlink)?;
            } else {
                ui::warning(
                    "Found non-symlink 'godot' executable in bin directory - not overwriting",
                );
                return Ok(());
            }
        }

        // Find the actual Godot executable in the installation
        let godot_exe_path = self.find_godot_executable(install_path, version)?;

        // Create symlink to the executable
        #[cfg(unix)]
        std::os::unix::fs::symlink(&godot_exe_path, &godot_executable_symlink)?;

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&godot_exe_path, &godot_executable_symlink)?;

        ui::info(&format!(
            "Created 'godot' executable symlink in {}",
            self.config.bin_dir.display()
        ));

        Ok(())
    }

    fn find_godot_executable(
        &self,
        install_path: &std::path::Path,
        version: &GodotVersion,
    ) -> Result<PathBuf> {
        // First try the expected path based on version info
        let expected_path = version.get_executable_path();
        let expected_exe = install_path.join(&expected_path);

        if expected_exe.exists() && expected_exe.is_file() {
            return Ok(expected_exe);
        }

        // If the expected path doesn't work, fall back to searching
        ui::warning(&format!(
            "Expected executable at {} not found, searching...",
            expected_path
        ));

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
            for entry in WalkDir::new(install_path).max_depth(2) {
                let entry = entry?;
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("Godot") && name.ends_with(".exe") {
                        return Ok(path.into());
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // On Linux, look for executable files starting with Godot
            for entry in WalkDir::new(install_path).max_depth(2) {
                let entry = entry?;
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if it's a Godot executable (may have version info in name)
                    if name.starts_with("Godot") && path.is_file() {
                        // Check if it's executable
                        use std::os::unix::fs::PermissionsExt;
                        let metadata = fs::metadata(&path)?;
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

    pub fn get_active_version(&self) -> Result<Option<GodotVersion>> {
        if !self.config.active_symlink.exists() {
            return Ok(None);
        }

        // Read the symlink target
        let target = fs::read_link(&self.config.active_symlink)?;

        // Parse version from the directory name
        if let Some(dir_name) = target.file_name().and_then(|n| n.to_str()) {
            if let Some(version_part) = dir_name.strip_prefix("godot-") {
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
        }

        Ok(None)
    }

    pub fn list_installed(&self) -> Result<Vec<GodotVersion>> {
        let mut versions = Vec::new();

        if !self.config.installations_dir.exists() {
            return Ok(versions);
        }

        for entry in fs::read_dir(&self.config.installations_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            if let Some(dir_name) = entry.file_name().to_str() {
                if let Some(version_part) = dir_name.strip_prefix("godot-") {
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
        }

        versions.sort();
        Ok(versions)
    }
}
