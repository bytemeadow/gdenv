use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^v?(\d+)\.(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-([a-zA-Z]+)(\d+)?)?(.*?)$").unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GodotVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
    pub sub_patch: Option<u32>,
    pub release_tag: String,
    pub tag_version: Option<u32>,
    pub extra: Option<String>,
    pub is_dotnet: bool,
}

impl PartialOrd for GodotVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn get_tag_rank(tag: &str) -> u32 {
    match tag.to_lowercase().as_str() {
        "stable" => 100,
        "rc" => 80,
        "beta" => 60,
        "alpha" => 40,
        "dev" => 20,
        _ => 0, // Other tags like 'custom' or unknown ones
    }
}

impl Ord for GodotVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.unwrap_or(0).cmp(&other.patch.unwrap_or(0)))
            .then(
                self.sub_patch
                    .unwrap_or(0)
                    .cmp(&other.sub_patch.unwrap_or(0)),
            )
            .then(get_tag_rank(&self.release_tag).cmp(&get_tag_rank(&other.release_tag)))
            .then(self.release_tag.cmp(&other.release_tag)) // Alphabetical if ranks are same
            .then(
                self.tag_version
                    .unwrap_or(0)
                    .cmp(&other.tag_version.unwrap_or(0)),
            )
            .then(self.extra.cmp(&other.extra))
    }
}

impl GodotVersion {
    /// Get the platform suffix for the current OS and architecture
    pub fn get_platform_suffix() -> &'static str {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match (os, arch) {
            ("windows", "x86_64") => "win64.exe",
            ("windows", "x86") => "win32.exe",
            ("macos", _) => "macos.universal", // macOS universal binaries work on both Intel and Apple Silicon
            ("linux", "x86_64") => "linux.x86_64",
            ("linux", "x86") => "linux.x86_32",
            ("linux", "arm") => "linux.arm32",
            ("linux", "aarch64") => "linux.arm64",
            // Fallbacks for common cases
            ("windows", _) => "win64.exe", // Default to 64-bit on Windows
            ("linux", _) => "linux.x86_64", // Default to x86_64 on Linux
            _ => "linux.x86_64",           // Ultimate fallback
        }
    }

    pub fn new(version_str: &str, is_dotnet: bool) -> Result<Self> {
        let caps = VERSION_REGEX
            .captures(version_str)
            .ok_or_else(|| anyhow!("Invalid Godot version format: {}", version_str))?;

        let major = caps
            .get(1)
            .ok_or_else(|| anyhow!("Invalid major version"))?
            .as_str()
            .parse()?;
        let minor = caps
            .get(2)
            .ok_or_else(|| anyhow!("Invalid minor version"))?
            .as_str()
            .parse()?;
        let patch = caps.get(3).map(|m| m.as_str().parse()).transpose()?;
        let sub_patch = caps.get(4).map(|m| m.as_str().parse()).transpose()?;
        let release_tag = caps
            .get(5)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "stable".to_string());
        let tag_version = caps.get(6).map(|m| m.as_str().parse()).transpose()?;
        let extra = caps
            .get(7)
            .map(|m| m.as_str().to_string())
            .filter(|s| !s.is_empty());

        Ok(GodotVersion {
            major,
            minor,
            patch,
            sub_patch,
            release_tag,
            tag_version,
            extra,
            is_dotnet,
        })
    }

    pub fn as_str(&self) -> String {
        let mut out = self.as_str_no_release_tag();
        out.push_str(&format!("-{}", self.release_tag));
        if let Some(tag_v) = self.tag_version {
            out.push_str(&tag_v.to_string());
        }
        if let Some(extra) = &self.extra {
            out.push_str(extra);
        }
        out
    }
    
    pub fn as_str_no_release_tag(&self) -> String {
        let mut out = format!("{}.{}", self.major, self.minor);
        if let Some(patch) = self.patch {
            out.push_str(&format!(".{}", patch));
            if let Some(sub_patch) = self.sub_patch {
                out.push_str(&format!(".{}", sub_patch));
            }
        }
        out
    }

    /// Get the expected executable path within the extracted directory
    pub fn get_executable_path(&self) -> String {
        let os = std::env::consts::OS;

        match os {
            "macos" => {
                if self.is_dotnet {
                    "Godot_mono.app/Contents/MacOS/Godot".to_string()
                } else {
                    "Godot.app/Contents/MacOS/Godot".to_string()
                }
            }
            "windows" => {
                let version_part = self.as_str();
                if self.is_dotnet {
                    format!(
                        "Godot_v{}_mono_{}/Godot_v{}_mono_{}.exe",
                        version_part, "win64", version_part, "win64"
                    )
                } else {
                    format!("Godot_v{}_{}.exe", version_part, "win64")
                }
            }
            "linux" => {
                let version_part = self.as_str();
                let platform_suffix = Self::get_platform_suffix();
                if self.is_dotnet {
                    // Dotnet versions extract to a subfolder
                    let folder_name = format!("Godot_v{version_part}_mono_{platform_suffix}");
                    let exe_name = format!("Godot_v{version_part}_mono_{platform_suffix}");
                    format!("{folder_name}/{exe_name}")
                } else {
                    // Non-dotnet versions extract directly
                    format!("Godot_v{version_part}_{platform_suffix}")
                }
            }
            _ => {
                // Fallback - just look for Godot executable
                "Godot".to_string()
            }
        }
    }

    pub fn installation_name(&self) -> String {
        if self.is_dotnet {
            format!("godot-{}-dotnet", self.as_str())
        } else {
            format!("godot-{}", self.as_str())
        }
    }

    #[allow(dead_code)]
    pub fn archive_name(&self) -> String {
        let platform_suffix = Self::get_platform_suffix();
        let version_part = self.as_str();

        if self.is_dotnet {
            format!("Godot_v{version_part}_mono_{platform_suffix}.zip")
        } else {
            format!("Godot_v{version_part}_{platform_suffix}.zip")
        }
    }

    #[allow(dead_code)]
    pub fn is_prerelease(&self) -> bool {
        self.release_tag != "stable"
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
            write!(f, "{} (.NET)", self.as_str())
        } else {
            write!(f, "{}", self.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        // Test stable versions
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        assert_eq!(v1.as_str(), "4.2.1-stable");
        assert!(!v1.is_prerelease());

        // Test stable with suffix
        let v2 = GodotVersion::new("4.2.1-stable", false).unwrap();
        assert_eq!(v2.as_str(), "4.2.1-stable");
        assert!(!v2.is_prerelease());

        // Test beta versions
        let v3 = GodotVersion::new("4.3.0-beta2", false).unwrap();
        assert_eq!(v3.as_str(), "4.3.0-beta2");
        assert!(v3.is_prerelease());

        // Test rc versions
        let v4 = GodotVersion::new("4.1.0-rc.1", false).unwrap();
        assert_eq!(v4.as_str(), "4.1.0-rc.1");
        assert!(v4.is_prerelease());

        // Test four part version
        let v7 = GodotVersion::new("4.3.0.1", false).unwrap();
        assert_eq!(v7.as_str(), "4.3.0.1-stable");
        assert_eq!(v7.major, 4);
        assert_eq!(v7.minor, 3);
        assert_eq!(v7.patch, Some(0));
        assert_eq!(v7.sub_patch, Some(1));

        // Test extra info
        let v8 = GodotVersion::new("4.4.stable.official.8981fd6c1", false).unwrap();
        assert_eq!(v8.as_str(), "4.4-stable.stable.official.8981fd6c1");
        assert_eq!(v8.extra, Some(".stable.official.8981fd6c1".to_string()));

        // Test .NET versions
        let v5 = GodotVersion::new("4.2.1", true).unwrap();
        assert_eq!(v5.to_string(), "4.2.1-stable (.NET)");
        assert_eq!(v5.installation_name(), "godot-4.2.1-stable-dotnet");

        // Test short prerelease versions like "4.5-beta1"
        let v6 = GodotVersion::new("4.5-beta1", false).unwrap();
        assert_eq!(v6.as_str(), "4.5-beta1");
        assert!(v6.is_prerelease());
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

    #[test]
    fn test_platform_suffix_detection() {
        // Test that we get a valid platform suffix (this tests the current system)
        let suffix = GodotVersion::get_platform_suffix();
        assert!(!suffix.is_empty());

        // Should be one of the expected patterns
        let valid_suffixes = [
            "win64.exe",
            "win32.exe",
            "macos.universal",
            "linux.x86_64",
            "linux.x86_32",
            "linux.arm32",
            "linux.arm64",
        ];
        assert!(
            valid_suffixes.contains(&suffix),
            "Got unexpected suffix: {suffix}"
        );
    }

    #[test]
    fn test_executable_path_construction() {
        // Test that we can construct executable paths
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        let exe_path = v1.get_executable_path();
        assert!(!exe_path.is_empty());

        let v2 = GodotVersion::new("4.2.1", true).unwrap();
        let dotnet_exe_path = v2.get_executable_path();
        assert!(!dotnet_exe_path.is_empty());

        // Paths should be different for dotnet vs non-dotnet
        assert_ne!(exe_path, dotnet_exe_path);
    }
}
