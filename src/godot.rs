use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use semver::Version;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GodotVersion {
    pub version: Version,
    pub is_dotnet: bool,
}

impl GodotVersion {
    pub fn new(version_str: &str, is_dotnet: bool) -> Result<Self> {
        let normalized = Self::normalize_version_string(version_str)?;
        let version = Version::parse(&normalized)?;
        Ok(Self {
            version,
            is_dotnet,
        })
    }
    
    /// Normalize Godot version strings to be semver compatible
    /// Examples:
    /// - "4.2.1" -> "4.2.1"
    /// - "4.3.0-beta2" -> "4.3.0-beta.2"
    /// - "4.1.0-rc.1" -> "4.1.0-rc.1"
    /// - "4.2.1-stable" -> "4.2.1"
    fn normalize_version_string(version_str: &str) -> Result<String> {
        let version_str = version_str.trim();
        
        // Remove common suffixes that aren't standard semver
        let cleaned = version_str
            .strip_suffix("-stable")
            .unwrap_or(version_str);
        
        // Handle beta/rc versions to be semver compatible
        if cleaned.contains("-beta") && !cleaned.contains("-beta.") {
            // Convert "4.3.0-beta2" to "4.3.0-beta.2"
            if let Some((base, beta_part)) = cleaned.split_once("-beta") {
                if let Ok(beta_num) = beta_part.parse::<u32>() {
                    return Ok(format!("{}-beta.{}", base, beta_num));
                } else if beta_part.is_empty() {
                    return Ok(format!("{}-beta", base));
                }
            }
        }
        
        if cleaned.contains("-rc") && !cleaned.contains("-rc.") {
            // Convert "4.1.0-rc1" to "4.1.0-rc.1"
            if let Some((base, rc_part)) = cleaned.split_once("-rc") {
                if let Ok(rc_num) = rc_part.parse::<u32>() {
                    return Ok(format!("{}-rc.{}", base, rc_num));
                } else if rc_part.is_empty() {
                    return Ok(format!("{}-rc", base));
                }
            }
        }
        
        if cleaned.contains("-alpha") && !cleaned.contains("-alpha.") {
            // Convert "4.3.0-alpha1" to "4.3.0-alpha.1"
            if let Some((base, alpha_part)) = cleaned.split_once("-alpha") {
                if let Ok(alpha_num) = alpha_part.parse::<u32>() {
                    return Ok(format!("{}-alpha.{}", base, alpha_num));
                } else if alpha_part.is_empty() {
                    return Ok(format!("{}-alpha", base));
                }
            }
        }
        
        Ok(cleaned.to_string())
    }
    
    pub fn version_string(&self) -> String {
        self.version.to_string()
    }
    
    pub fn godot_version_string(&self) -> String {
        // Convert back to Godot's preferred format
        let version_str = self.version.to_string();
        
        // Convert semver format back to Godot format for display
        version_str
            .replace("-beta.", "-beta")
            .replace("-rc.", "-rc")
            .replace("-alpha.", "-alpha")
    }
    
    pub fn installation_name(&self) -> String {
        if self.is_dotnet {
            format!("godot-{}-dotnet", self.godot_version_string())
        } else {
            format!("godot-{}", self.godot_version_string())
        }
    }
    
    pub fn archive_name(&self) -> String {
        let platform = std::env::consts::OS;
        
        let platform_suffix = match platform {
            "windows" => "win64.exe",
            "macos" => "macos.universal", 
            "linux" => "linux.x86_64",
            _ => "linux.x86_64", // fallback
        };
        
        let version_part = if self.version.pre.is_empty() {
            format!("{}-stable", self.version)
        } else {
            self.godot_version_string()
        };
        
        if self.is_dotnet {
            format!("Godot_v{}_mono_{}.zip", version_part, platform_suffix)
        } else {
            format!("Godot_v{}_{}.zip", version_part, platform_suffix)
        }
    }
    
    pub fn is_prerelease(&self) -> bool {
        !self.version.pre.is_empty()
    }
}

impl FromStr for GodotVersion {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        // Default to non-.NET version
        Self::new(s, false)
    }
}

impl fmt::Display for GodotVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dotnet {
            write!(f, "{} (.NET)", self.godot_version_string())
        } else {
            write!(f, "{}", self.godot_version_string())
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstalledGodot {
    pub version: GodotVersion,
    pub path: std::path::PathBuf,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_parsing() {
        // Test stable versions
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        assert_eq!(v1.godot_version_string(), "4.2.1");
        assert!(!v1.is_prerelease());
        
        // Test stable with suffix
        let v2 = GodotVersion::new("4.2.1-stable", false).unwrap();
        assert_eq!(v2.godot_version_string(), "4.2.1");
        assert!(!v2.is_prerelease());
        
        // Test beta versions
        let v3 = GodotVersion::new("4.3.0-beta2", false).unwrap();
        assert_eq!(v3.godot_version_string(), "4.3.0-beta2");
        assert!(v3.is_prerelease());
        
        // Test rc versions
        let v4 = GodotVersion::new("4.1.0-rc.1", false).unwrap();
        assert_eq!(v4.godot_version_string(), "4.1.0-rc1");
        assert!(v4.is_prerelease());
        
        // Test .NET versions
        let v5 = GodotVersion::new("4.2.1", true).unwrap();
        assert_eq!(v5.to_string(), "4.2.1 (.NET)");
        assert_eq!(v5.installation_name(), "godot-4.2.1-dotnet");
    }
    
    #[test]
    fn test_archive_names() {
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        let archive = v1.archive_name();
        assert!(archive.contains("Godot_v4.2.1-stable_"));
        assert!(archive.ends_with(".zip"));
        
        let v2 = GodotVersion::new("4.3.0-beta2", true).unwrap();
        let archive = v2.archive_name();
        assert!(archive.contains("Godot_v4.3.0-beta2_mono_"));
        assert!(archive.ends_with(".zip"));
    }
}