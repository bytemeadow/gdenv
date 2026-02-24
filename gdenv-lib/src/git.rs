use crate::config::Config;
use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait GitClient: Send + Sync {
    /// Initializes a new git repository at the specified path.
    fn init(&self, path: &Path, branch: Option<&str>) -> impl Future<Output = Result<()>> + Send;

    /// Clones or updates a repository and checks out the specified ref.
    /// Returns the path to the checked-out repository.
    fn checkout(
        &self,
        repo_url: &str,
        git_ref: &str,
    ) -> impl Future<Output = Result<PathBuf>> + Send;
}

pub struct SystemGitClient {
    config: Config,
}

impl SystemGitClient {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl GitClient for SystemGitClient {
    async fn init(&self, path: &Path, branch: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("init");

        if let Some(b) = branch {
            cmd.arg(format!("--initial-branch={}", b));
        }

        let output = cmd
            .arg(path)
            .output()
            .context("Failed to execute git init")?;

        if !output.status.success() {
            bail!(
                "git init failed at {:?}. Reason: {}",
                path,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    async fn checkout(&self, repo_url: &str, git_ref: &str) -> Result<PathBuf> {
        let repo_dir = get_repo_dir(&self.config, repo_url);

        if !repo_dir.exists() {
            // Clone the repository if it doesn't exist
            let output = Command::new("git")
                .args(["clone", "--no-checkout", "--filter=blob:none", repo_url])
                .arg(&repo_dir)
                .output()
                .context("Failed to execute git clone")?;

            if !output.status.success() {
                bail!(
                    "git clone failed for {}. Reason: {}",
                    repo_url,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        // Fetch and checkout the specific ref
        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["fetch", "origin", git_ref])
            .output()
            .context("Failed to execute git fetch")?;

        if !output.status.success() {
            return Err(anyhow!(
                "git fetch failed for {} at {}. Reason: {}",
                repo_url,
                git_ref,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let output = Command::new("git")
            .current_dir(&repo_dir)
            .args(["checkout", "FETCH_HEAD"])
            .output()
            .context("Failed to execute git checkout")?;

        if !output.status.success() {
            bail!(
                "git checkout failed for {}. Reason: {}",
                git_ref,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(repo_dir)
    }
}

pub fn get_repo_dir(config: &Config, repo_url: &str) -> PathBuf {
    // Create a unique directory name based on the URL to avoid collisions in cache
    let safe_name = repo_url
        .replace("://", "_")
        .replace("/", "_")
        .replace(":", "_");
    config.git_cache_dir.join(safe_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_system_git_client_local_checkout() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let tmp_data_dir = TempDir::new("gdenv-test-data-dir")?;
        let source_repo = tmp_dir.path().join("source_repo");

        fs::create_dir_all(&source_repo)?;

        let config = Config::setup(Some(&tmp_data_dir.path()))?;
        let git_client = SystemGitClient::new(config);

        // 1. Initialize the source repository
        git_client.init(&source_repo, Some("main")).await?;

        // 2. Create an initial commit so there is a ref to check out
        // Git needs a user identity to commit
        Command::new("git")
            .current_dir(&source_repo)
            .args(["config", "user.email", "test@example.com"])
            .status()?;
        Command::new("git")
            .current_dir(&source_repo)
            .args(["config", "user.name", "test"])
            .status()?;

        let file_path = source_repo.join("hello.txt");
        fs::write(&file_path, "world")?;

        Command::new("git")
            .current_dir(&source_repo)
            .args(["add", "."])
            .status()?;
        Command::new("git")
            .current_dir(&source_repo)
            .args(["commit", "-m", "initial commit"])
            .status()?;

        // 3. Use checkout to clone the local path
        // We use the absolute path as the "URL"
        let repo_url = source_repo.to_str().unwrap();
        let checked_out_path = git_client.checkout(repo_url, "main").await?; // Handle different default branch names

        // 4. Verify the file exists in the checked-out location
        assert!(checked_out_path.exists());
        assert!(checked_out_path.join("hello.txt").exists());

        let content = fs::read_to_string(checked_out_path.join("hello.txt"))?;
        assert_eq!(content, "world");

        Ok(())
    }
}
