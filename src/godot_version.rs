use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^v?(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:\.(\d+))?(?:-([a-zA-Z]+)(\d+)?)?(.*?)$")
        .unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GodotVersion {
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub sub_patch: Option<u32>,
    pub release_tag: String,
    pub tag_version: Option<u32>,
    pub extra: Option<String>,
    pub is_dotnet: bool,
}

impl GodotVersion {
    pub fn new(version_str: &str, is_dotnet: bool) -> Result<Self> {
        let caps = VERSION_REGEX
            .captures(version_str)
            .ok_or_else(|| anyhow!("Invalid Godot version format: {}", version_str))?;

        let major = caps
            .get(1)
            .ok_or_else(|| anyhow!("Invalid major version"))?
            .as_str()
            .parse()?;
        let minor_opt = caps.get(2).map(|m| m.as_str().parse()).transpose()?;
        let patch_opt = caps.get(3).map(|m| m.as_str().parse()).transpose()?;
        let sub_patch_opt = caps.get(4).map(|m| m.as_str().parse()).transpose()?;
        let release_tag = caps
            .get(5)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "stable".to_string());
        let tag_version = caps.get(6).map(|m| m.as_str().parse()).transpose()?;
        let extra = caps
            .get(7)
            .map(|m| m.as_str().to_string())
            .filter(|s| !s.is_empty());

        // Remove trailing zeros from the main version
        let sub_patch = sub_patch_opt.filter(|value| *value > 0);
        let patch = patch_opt.filter(|value| sub_patch.is_some() || *value > 0);
        let minor = minor_opt.filter(|value| patch.is_some() || *value > 0);

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

    /// Example outputs:
    /// - 4.0-stable
    /// - 4.2.1-rc5
    pub fn as_full_version_str(&self) -> String {
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

    /// Example outputs:
    /// - 4.0
    /// - 4.2.1
    pub fn as_str_no_release_tag(&self) -> String {
        let mut out = format!("{}.{}", self.major, self.minor.unwrap_or(0));
        if let Some(patch) = self.patch {
            out.push_str(&format!(".{}", patch));
            if let Some(sub_patch) = self.sub_patch {
                out.push_str(&format!(".{}", sub_patch));
            }
        }
        out
    }

    #[allow(dead_code)]
    pub fn is_prerelease(&self) -> bool {
        self.release_tag != "stable"
    }
}

impl PartialOrd for GodotVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GodotVersion {
    fn cmp(&self, other: &Self) -> Ordering {
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
        self.major
            .cmp(&other.major)
            .then(self.minor.unwrap_or(0).cmp(&other.minor.unwrap_or(0)))
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
            write!(f, "{} (.NET)", self.as_full_version_str())
        } else {
            write!(f, "{}", self.as_full_version_str())
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
        assert_eq!(v1.as_full_version_str(), "4.2.1-stable");
        assert!(!v1.is_prerelease());

        // Test stable with suffix
        let v2 = GodotVersion::new("4.2.1-stable", false).unwrap();
        assert_eq!(v2.as_full_version_str(), "4.2.1-stable");
        assert!(!v2.is_prerelease());

        // Test beta versions
        let v3 = GodotVersion::new("4.3.0-beta2", false).unwrap();
        assert_eq!(v3.as_full_version_str(), "4.3-beta2");
        assert!(v3.is_prerelease());

        // Test rc versions
        let v4 = GodotVersion::new("4.1.0-rc.1", false).unwrap();
        assert_eq!(v4.as_full_version_str(), "4.1-rc.1");
        assert!(v4.is_prerelease());

        // Test four part version
        let v7 = GodotVersion::new("4.3.0.1", false).unwrap();
        assert_eq!(v7.as_full_version_str(), "4.3.0.1-stable");
        assert_eq!(v7.major, 4);
        assert_eq!(v7.minor, Some(3));
        assert_eq!(v7.patch, Some(0));
        assert_eq!(v7.sub_patch, Some(1));

        // Test extra info
        let v8 = GodotVersion::new("4.4.stable.official.8981fd6c1", false).unwrap();
        assert_eq!(
            v8.as_full_version_str(),
            "4.4-stable.stable.official.8981fd6c1"
        );
        assert_eq!(v8.extra, Some(".stable.official.8981fd6c1".to_string()));

        // Test short prerelease versions like "4.5-beta1"
        let v6 = GodotVersion::new("4.5-beta1", false).unwrap();
        assert_eq!(v6.as_full_version_str(), "4.5-beta1");
        assert!(v6.is_prerelease());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = GodotVersion::new("4.2", false).unwrap();
        let v2 = GodotVersion::new("4.2.0-stable", false).unwrap();
        let v3 = GodotVersion::new("4.2.1-rc5", false).unwrap();
        let v4 = GodotVersion::new("4.2.1-rc5", false).unwrap();
        assert!(v1.cmp(&v2).is_eq());
        assert!(v2.cmp(&v3).is_lt());
        assert!(v3.cmp(&v4).is_eq());
    }
}
