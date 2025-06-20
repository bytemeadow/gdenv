use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum GdmError {
    #[error("Godot version '{0}' is invalid")]
    InvalidVersion(String),
    
    #[error("Godot version '{0}' is not installed")]
    VersionNotInstalled(String),
    
    #[error("Godot version '{0}' is already installed")]
    VersionAlreadyInstalled(String),
    
    #[error("Failed to download Godot: {0}")]
    DownloadError(String),
    
    #[error("Failed to extract archive: {0}")]
    ExtractionError(String),
    
    #[error("Failed to create symlink: {0}")]
    SymlinkError(String),
    
    #[error("GitHub API error: {0}")]
    GitHubApiError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}