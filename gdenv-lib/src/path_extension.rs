//! Utilities for working with paths.

use std::path::{Path, PathBuf};

pub trait PathExt {
    fn to_absolute(&self, working_dir: &Path) -> anyhow::Result<PathBuf>;
}

impl PathExt for Path {
    fn to_absolute(&self, working_dir: &Path) -> anyhow::Result<PathBuf> {
        if self.is_absolute() {
            Ok(self.to_path_buf())
        } else {
            Ok(working_dir.join(self))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_to_absolute_with_absolute_path() -> Result<()> {
        let path = Path::new("/tmp/test_file.txt");
        let working_dir = Path::new("/home/user");
        let absolute = path.to_absolute(working_dir)?;

        #[cfg(windows)]
        let path = Path::new(r"C:\tmp\test_file.txt");

        assert!(absolute.is_absolute());
        if cfg!(unix) {
            assert_eq!(absolute, Path::new("/tmp/test_file.txt"));
        }
        Ok(())
    }

    #[test]
    fn test_to_absolute_with_relative_path() -> Result<()> {
        let path = Path::new("some/relative/path.rs");
        let working_dir = Path::new("/home/user");
        let absolute = path
            .to_absolute(working_dir)
            .expect("Should resolve to absolute");

        assert!(absolute.is_absolute());

        assert_eq!(absolute, working_dir.join("some/relative/path.rs"));
        Ok(())
    }

    #[test]
    fn test_to_absolute_with_empty_path() -> Result<()> {
        let path = Path::new("");
        let working_dir = Path::new("/home/user");
        let absolute = path.to_absolute(working_dir)?;

        assert!(absolute.is_absolute());
        assert_eq!(absolute, working_dir);
        Ok(())
    }
}
