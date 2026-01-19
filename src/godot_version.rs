use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GodotVersion {
    major: u32,
    minor: u32,
    patch: u32,
    pre_release: PreReleaseTag,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PreReleaseTag {
    Alpha(u32),
    Beta(u32),
    Rc(u32),
    Dev(u32),
    Stable,
}

impl GodotVersion {
    pub fn parse(tag: &str) -> Option<Self> {
        let tag = tag.strip_prefix('v').unwrap_or(tag);
        let parts: Vec<&str> = tag.split('-').collect();
        let version_part = parts[0];

        let mut nums = version_part.split('.');
        let major = nums.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let minor = nums.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let patch = nums.next().and_then(|n| n.parse().ok()).unwrap_or(0);

        let pre_release = if parts.len() > 1 {
            let pre = parts[1].to_lowercase();
            if pre.contains("stable") {
                PreReleaseTag::Stable
            } else if pre.contains("rc") {
                PreReleaseTag::Rc(Self::extract_num(&pre, "rc"))
            } else if pre.contains("beta") {
                PreReleaseTag::Beta(Self::extract_num(&pre, "beta"))
            } else if pre.contains("alpha") {
                PreReleaseTag::Alpha(Self::extract_num(&pre, "alpha"))
            } else if pre.contains("dev") {
                PreReleaseTag::Dev(Self::extract_num(&pre, "dev"))
            } else {
                PreReleaseTag::Stable // Treat unknown tags as stable for sorting
            }
        } else {
            PreReleaseTag::Stable
        };

        Some(GodotVersion {
            major,
            minor,
            patch,
            pre_release,
        })
    }

    fn extract_num(s: &str, prefix: &str) -> u32 {
        if let Some(pos) = s.find(prefix) {
            let num_str = &s[pos + prefix.len()..];
            num_str.parse().ok().unwrap_or(0)
        } else {
            0
        }
    }
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

impl PreReleaseTag {
    pub fn rank(&self) -> u32 {
        match self {
            PreReleaseTag::Dev(_) => 0,
            PreReleaseTag::Alpha(_) => 1,
            PreReleaseTag::Beta(_) => 2,
            PreReleaseTag::Rc(_) => 3,
            PreReleaseTag::Stable => 4,
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
