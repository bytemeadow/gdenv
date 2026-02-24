use crate::config::Config;
use crate::git::GitClient;
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};

/// Mock implementation for testing purposes.
pub struct MockGitClient {
    config: Config,
}

impl MockGitClient {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl GitClient for MockGitClient {
    async fn init(&self, _path: &Path, _branch: Option<&str>) -> anyhow::Result<()> {
        todo!("Not yet implemented")
    }

    async fn checkout(&self, repo_url: &str, _git_ref: &str) -> anyhow::Result<PathBuf> {
        let repo_dir = crate::git::get_repo_dir(&self.config, repo_url);
        fs::create_dir_all(&repo_dir).context("Failed to create mock repository directory")?;

        let test_data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
        let source_repo = test_data_dir.join("test-addon1-repo");

        for entry in walkdir::WalkDir::new(&source_repo).min_depth(1) {
            let entry = entry.context("Failed to read test data entry")?;
            let path = entry.path();
            let relative_path = path
                .strip_prefix(&source_repo)
                .context("Failed to compute relative path")?;
            let dest_path = repo_dir.join(relative_path);

            if path.is_dir() {
                fs::create_dir_all(&dest_path)
                    .context("Failed to create directory in mock repo")?;
            } else {
                fs::copy(path, &dest_path).context("Failed to copy file to mock repo")?;
            }
        }

        Ok(repo_dir)
    }
}
