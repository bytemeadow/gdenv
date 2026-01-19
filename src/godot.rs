use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum PreReleaseTag {
    Dev(u32),
    Alpha(u32),
    Beta(u32),
    Rc(u32),
    Stable,
    Unknown(String),
}

impl PreReleaseTag {
    pub fn new(version_str: &str) -> Result<Self> {
        let parts: Vec<&str> = version_str.split('-').collect();
        if parts.len() > 1 {
            let pre = parts[1].to_lowercase();
            if pre.contains("stable") {
                Ok(PreReleaseTag::Stable)
            } else if pre.contains("rc") {
                Ok(PreReleaseTag::Rc(Self::extract_num(&pre, "rc")))
            } else if pre.contains("beta") {
                Ok(PreReleaseTag::Beta(Self::extract_num(&pre, "beta")))
            } else if pre.contains("alpha") {
                Ok(PreReleaseTag::Alpha(Self::extract_num(&pre, "alpha")))
            } else if pre.contains("dev") {
                Ok(PreReleaseTag::Dev(Self::extract_num(&pre, "dev")))
            } else {
                Ok(PreReleaseTag::Unknown(pre.to_string()))
            }
        } else {
            Ok(PreReleaseTag::Stable)
        }
    }

    pub fn rank(&self) -> u32 {
        match self {
            PreReleaseTag::Unknown(_) => 0,
            PreReleaseTag::Dev(_) => 1,
            PreReleaseTag::Alpha(_) => 2,
            PreReleaseTag::Beta(_) => 3,
            PreReleaseTag::Rc(_) => 4,
            PreReleaseTag::Stable => 5,
        }
    }

    fn extract_num(s: &str, prefix: &str) -> u32 {
        if let Some(pos) = s.find(prefix) {
            let num_str = &s[pos + prefix.len()..];
            // Remove any dots before parsing, e.g., ".1" -> "1"
            let num_str = num_str.trim_start_matches('.');
            num_str.parse().ok().unwrap_or(0)
        } else {
            0
        }
    }
}

impl PartialOrd for PreReleaseTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PreReleaseTag {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.rank().cmp(&other.rank()) {
            Ordering::Equal => match (self, other) {
                (PreReleaseTag::Dev(a), PreReleaseTag::Dev(b)) => a.cmp(b),
                (PreReleaseTag::Alpha(a), PreReleaseTag::Alpha(b)) => a.cmp(b),
                (PreReleaseTag::Beta(a), PreReleaseTag::Beta(b)) => a.cmp(b),
                (PreReleaseTag::Rc(a), PreReleaseTag::Rc(b)) => a.cmp(b),
                _ => Ordering::Equal,
            },
            ord => ord,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GodotVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: PreReleaseTag,
    pub is_dotnet: bool,
}

impl PartialOrd for GodotVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GodotVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then(self.pre_release.cmp(&other.pre_release))
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
        let tag = version_str.strip_prefix('v').unwrap_or(version_str);
        let parts: Vec<&str> = tag.split('-').collect();
        let version_part = parts[0];

        let mut nums = version_part.split('.');
        let major = nums
            .next()
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| anyhow!("Invalid major version"))?;
        let minor = nums.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let patch = nums.next().and_then(|n| n.parse().ok()).unwrap_or(0);

        let pre_release = PreReleaseTag::new(version_str)?;

        Ok(GodotVersion {
            major,
            minor,
            patch,
            pre_release,
            is_dotnet,
        })
    }

    pub fn godot_version_string(&self) -> String {
        let base = if self.patch == 0 {
            format!("{}.{}", self.major, self.minor)
        } else {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        };

        match &self.pre_release {
            PreReleaseTag::Stable => base,
            PreReleaseTag::Dev(n) => {
                if *n == 0 {
                    format!("{}-dev", base)
                } else {
                    format!("{}-dev{}", base, n)
                }
            }
            PreReleaseTag::Alpha(n) => {
                if *n == 0 {
                    format!("{}-alpha", base)
                } else {
                    format!("{}-alpha{}", base, n)
                }
            }
            PreReleaseTag::Beta(n) => {
                if *n == 0 {
                    format!("{}-beta", base)
                } else {
                    format!("{}-beta{}", base, n)
                }
            }
            PreReleaseTag::Rc(n) => {
                if *n == 0 {
                    format!("{}-rc", base)
                } else {
                    format!("{}-rc{}", base, n)
                }
            }
            PreReleaseTag::Unknown(s) => format!("{}-{}", base, s),
        }
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
                let version_part = if matches!(self.pre_release, PreReleaseTag::Stable) {
                    format!("{}-stable", self.godot_version_string())
                } else {
                    self.godot_version_string()
                };

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
                let version_part = if matches!(self.pre_release, PreReleaseTag::Stable) {
                    format!("{}-stable", self.godot_version_string())
                } else {
                    self.godot_version_string()
                };

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
            format!("godot-{}-dotnet", self.godot_version_string())
        } else {
            format!("godot-{}", self.godot_version_string())
        }
    }

    #[allow(dead_code)]
    pub fn archive_name(&self) -> String {
        let platform_suffix = Self::get_platform_suffix();

        let version_part = if matches!(self.pre_release, PreReleaseTag::Stable) {
            format!("{}-stable", self.godot_version_string())
        } else {
            self.godot_version_string()
        };

        if self.is_dotnet {
            format!("Godot_v{version_part}_mono_{platform_suffix}.zip")
        } else {
            format!("Godot_v{version_part}_{platform_suffix}.zip")
        }
    }

    #[allow(dead_code)]
    pub fn is_prerelease(&self) -> bool {
        !matches!(self.pre_release, PreReleaseTag::Stable)
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
        assert_eq!(v3.godot_version_string(), "4.3-beta2");
        assert!(v3.is_prerelease());

        // Test rc versions
        let v4 = GodotVersion::new("4.1.0-rc.1", false).unwrap();
        assert_eq!(v4.godot_version_string(), "4.1-rc1");
        assert!(v4.is_prerelease());

        // Test .NET versions
        let v5 = GodotVersion::new("4.2.1", true).unwrap();
        assert_eq!(v5.to_string(), "4.2.1 (.NET)");
        assert_eq!(v5.installation_name(), "godot-4.2.1-dotnet");

        // Test short prerelease versions like "4.5-beta1"
        let v6 = GodotVersion::new("4.5-beta1", false).unwrap();
        assert_eq!(v6.godot_version_string(), "4.5-beta1");
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
        assert!(archive.contains("Godot_v4.3-beta2_mono_"));
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
