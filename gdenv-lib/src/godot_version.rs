use anyhow::{Context, Result};
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
    pub release_tag: Option<String>,
    pub tag_version: Option<u32>,
    pub extra: Option<String>,
    pub is_dotnet: bool,
}

impl GodotVersion {
    pub fn new(version_str: &str, is_dotnet: bool) -> Result<Self> {
        let caps = VERSION_REGEX
            .captures(version_str)
            .context("Invalid Godot version format")?;

        let major = caps
            .get(1)
            .context("Invalid major version")?
            .as_str()
            .parse()
            .context("Invalid major version")?;
        let minor_opt = caps
            .get(2)
            .map(|m| m.as_str().parse())
            .transpose()
            .context("Invalid minor version")?;
        let patch_opt = caps
            .get(3)
            .map(|m| m.as_str().parse())
            .transpose()
            .context("Invalid patch version")?;
        let sub_patch_opt = caps
            .get(4)
            .map(|m| m.as_str().parse())
            .transpose()
            .context("Invalid sub-patch version")?;
        let release_tag = Some(
            caps.get(5)
                .map_or("stable".to_string(), |m| m.as_str().to_string()),
        );
        let tag_version = caps
            .get(6)
            .map(|m| m.as_str().parse())
            .transpose()
            .context("Invalid tag version")?;
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
    pub fn as_godot_version_str(&self) -> String {
        let mut out = self.as_str_no_release_tag();
        if let Some(release_tag) = &self.release_tag {
            out.push_str(&format!("-{}", release_tag));
        }
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
        self.release_tag.as_ref().is_none_or(|tag| tag != "stable")
    }
}

impl PartialOrd for GodotVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GodotVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        fn get_tag_rank(tag: &Option<String>) -> u32 {
            match tag
                .clone()
                .unwrap_or("stable".to_string())
                .to_lowercase()
                .as_str()
            {
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
            write!(f, "{} (.NET)", self.as_godot_version_str())
        } else {
            write!(f, "{}", self.as_godot_version_str())
        }
    }
}

pub fn version_buffet(all_releases: &[GodotVersion]) -> Vec<&GodotVersion> {
    // Most users will not care about version solder than 3.x for the buffet.
    let mut most_recent_top: Vec<&GodotVersion> = all_releases
        .iter()
        .filter(|v| v.major >= 3 && !v.is_dotnet)
        .rev()
        .collect();

    // Reduce to the most recent minor version, except when there is a newer
    // pre-release version, then show both stable and pre-release versions.
    most_recent_top.dedup_by(|a, b| {
        a.minor == b.minor && b.release_tag.as_ref().is_some_and(|tag| tag == "stable")
    });

    let max_major_version = most_recent_top
        .iter()
        .max_by_key(|v| v.major)
        .map(|v| v.major)
        .unwrap_or(0);

    (3..=max_major_version) // Range of major versions we care about
        .flat_map(|major| {
            // For each major version, find its releases, take 5, and reverse them
            most_recent_top
                .iter()
                .filter(|r| r.major == major)
                .take(5)
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        // Test stable versions
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        assert_eq!(v1.as_godot_version_str(), "4.2.1-stable");
        assert!(!v1.is_prerelease());

        // Test stable with suffix
        let v2 = GodotVersion::new("4.2.1-stable", false).unwrap();
        assert_eq!(v2.as_godot_version_str(), "4.2.1-stable");
        assert!(!v2.is_prerelease());

        // Test beta versions
        let v3 = GodotVersion::new("4.3.0-beta2", false).unwrap();
        assert_eq!(v3.as_godot_version_str(), "4.3-beta2");
        assert!(v3.is_prerelease());

        // Test rc versions
        let v4 = GodotVersion::new("4.1.0-rc.1", false).unwrap();
        assert_eq!(v4.as_godot_version_str(), "4.1-rc.1");
        assert!(v4.is_prerelease());

        // Test four part version
        let v7 = GodotVersion::new("4.3.0.1", false).unwrap();
        assert_eq!(v7.as_godot_version_str(), "4.3.0.1-stable");
        assert_eq!(v7.major, 4);
        assert_eq!(v7.minor, Some(3));
        assert_eq!(v7.patch, Some(0));
        assert_eq!(v7.sub_patch, Some(1));

        // Test extra info
        let v8 = GodotVersion::new("4.4.stable.official.8981fd6c1", false).unwrap();
        assert_eq!(
            v8.as_godot_version_str(),
            "4.4-stable.stable.official.8981fd6c1"
        );
        assert_eq!(v8.extra, Some(".stable.official.8981fd6c1".to_string()));

        // Test short prerelease versions like "4.5-beta1"
        let v6 = GodotVersion::new("4.5-beta1", false).unwrap();
        assert_eq!(v6.as_godot_version_str(), "4.5-beta1");
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

    #[test]
    fn test_version_buffet() {
        let releases: Vec<GodotVersion> = [
            "4.5-beta7",
            "4.5-rc1",
            "4.5-rc2",
            "4.5-stable",
            "4.5.1-rc1",
            "4.5.1-rc2",
            "4.5.1-stable",
            "4.5.2-rc1",
            "4.6-dev1",
            "4.6-dev2",
            "4.6-dev3",
            "4.6-dev4",
            "4.6-dev5",
            "4.6-dev6",
            "4.6-beta1",
            "4.6-beta2",
            "4.6-beta3",
            "4.6-rc1",
            "4.6-rc2",
            "4.6-stable",
            "4.6.1-rc1",
            "4.7-dev1",
        ]
        .iter()
        .flat_map(|s| {
            [
                GodotVersion::new(s, false).unwrap(),
                GodotVersion::new(s, true).unwrap(),
            ]
        })
        .collect();

        let expected: Vec<GodotVersion> = [
            "4.5.1-stable",
            "4.5.2-rc1",
            "4.6-stable",
            "4.6.1-rc1",
            "4.7-dev1",
        ]
        .iter()
        .map(|s| GodotVersion::new(s, false).unwrap())
        .collect::<Vec<_>>();

        let buffet = version_buffet(&releases);
        tracing::info!(
            "{:#?}",
            buffet.iter().map(|v| v.to_string()).collect::<Vec<_>>()
        );
        assert_eq!(buffet.len(), 5);

        buffet
            .iter()
            .zip(expected)
            .for_each(|(v1, v2)| assert_eq!(**v1, v2));
    }
}
