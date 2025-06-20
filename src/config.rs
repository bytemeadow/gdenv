use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// Directory where Godot installations are stored
    pub installations_dir: PathBuf,
    
    /// Directory for download cache
    pub cache_dir: PathBuf,
    
    /// Path to the active Godot symlink
    pub active_symlink: PathBuf,
    
    /// Directory for executable symlinks (to be added to PATH)
    pub bin_dir: PathBuf,
    
    /// GitHub API base URL
    pub github_api_url: String,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
            .join("gdm");
            
        Self {
            installations_dir: data_dir.join("installations"),
            cache_dir: data_dir.join("cache"),
            active_symlink: data_dir.join("current"),
            bin_dir: data_dir.join("bin"),
            github_api_url: "https://api.github.com".to_string(),
        }
    }
}

impl Config {
    pub fn new() -> Result<Self> {
        let config = Self::default();
        
        // Ensure directories exist
        std::fs::create_dir_all(&config.installations_dir)?;
        std::fs::create_dir_all(&config.cache_dir)?;
        std::fs::create_dir_all(&config.bin_dir)?;
        
        Ok(config)
    }
}