use crate::migrate::migrate;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    /// Root directory for gdenv data (installations, cache, symlinks, etc.)
    pub data_dir: PathBuf,

    /// Directory where Godot installations are stored
    pub installations_dir: PathBuf,

    /// Directory for download cache
    pub cache_dir: PathBuf,

    /// Path to the active Godot symlink
    pub active_symlink: PathBuf,

    /// Directory for executable symlinks (to be added to PATH)
    pub bin_dir: PathBuf,

    /// Path to the gdenv version file (used to detect if migration is needed)
    pub data_dir_format_version_file: PathBuf,

    /// Platform-specific operating system string.
    pub os: String,

    /// Platform-specific architecture string.
    pub arch: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::new_for_path(&Self::default_data_dir())
    }
}

impl Config {
    pub fn new_for_path(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            installations_dir: data_dir.join("installations"),
            cache_dir: data_dir.join("cache"),
            active_symlink: data_dir.join("current"),
            bin_dir: data_dir.join("bin"),
            data_dir_format_version_file: data_dir.join("gdenv_version.txt"),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }

    /// Sets up a new Config for the given data directory.
    /// See also [Self::default_data_dir].
    pub fn setup(data_dir: Option<&Path>) -> Result<Self> {
        let data_dir = data_dir
            .map(|d| d.to_path_buf())
            .unwrap_or_else(Self::default_data_dir);
        let config = Self::new_for_path(&data_dir);

        migrate().context("Failed to migrate data directory")?;

        // Ensure directories exist
        std::fs::create_dir_all(&config.installations_dir)?;
        std::fs::create_dir_all(&config.cache_dir)?;
        std::fs::create_dir_all(&config.bin_dir)?;

        Ok(config)
    }

    pub fn default_data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
            .join("gdenv")
    }
}
