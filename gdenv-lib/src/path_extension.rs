//! Utilities for working with paths.

use anyhow::Context;
use std::path::{Path, PathBuf};

pub trait PathExt {
    fn to_absolute(&self) -> anyhow::Result<PathBuf>;
}

impl PathExt for Path {
    fn to_absolute(&self) -> anyhow::Result<PathBuf> {
        if self.is_absolute() {
            Ok(self.to_path_buf())
        } else {
            let current_dir = std::env::current_dir().context("Failed to get current directory")?;
            Ok(current_dir.join(self))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_to_absolute_with_absolute_path() {
        let path = Path::new("/tmp/test_file.txt");
        let absolute = path.to_absolute().unwrap();

        #[cfg(windows)]
        let path = Path::new(r"C:\tmp\test_file.txt");

        assert!(absolute.is_absolute());
        if cfg!(unix) {
            assert_eq!(absolute, Path::new("/tmp/test_file.txt"));
        }
    }

    #[test]
    fn test_to_absolute_with_relative_path() {
        let path = Path::new("some/relative/path.rs");
        let absolute = path.to_absolute().expect("Should resolve to absolute");

        assert!(absolute.is_absolute());

        let current_dir = env::current_dir().unwrap();
        assert_eq!(absolute, current_dir.join("some/relative/path.rs"));
    }

    #[test]
    fn test_to_absolute_with_empty_path() {
        let path = Path::new("");
        let absolute = path.to_absolute().unwrap();

        assert!(absolute.is_absolute());
        assert_eq!(absolute, env::current_dir().unwrap());
    }
}
