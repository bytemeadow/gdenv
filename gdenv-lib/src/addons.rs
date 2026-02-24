use crate::file_sync::sync_recursive;
use crate::git::GitClient;
use crate::project_specification::{
    AddonSource, AddonSpec, GitAddonSource, LocalAddonSource, ProjectSpecification,
};
use anyhow::Result;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
pub async fn sync_addons<G: GitClient>(
    project_spec: ProjectSpecification,
    working_dir: &Path,
    git_client: &G,
) -> Result<()> {
    for (addon_name, addon_spec) in project_spec.addons {
        match &addon_spec.source {
            AddonSource::Git(git) => {
                sync_git_addon(
                    git_client,
                    &working_dir.join(&project_spec.project_dir),
                    &addon_name,
                    &addon_spec,
                    git,
                )
                .await?
            }
            AddonSource::Local(local) => sync_local_addon(
                &working_dir.join(&project_spec.project_dir),
                &addon_name,
                &addon_spec,
                local,
            )?,
        }
    }
    Ok(())
}

async fn sync_git_addon<G: GitClient>(
    git_client: &G,
    project_dir: &Path,
    addon_name: &str,
    addon_spec: &AddonSpec,
    addon_source: &GitAddonSource,
) -> Result<()> {
    let source_base = git_client
        .checkout(&addon_source.git, addon_source.rev.as_deref().unwrap_or(""))
        .await?
        .join(addon_source.subdir.as_deref().unwrap_or(Path::new("")));

    let dest_base = if let Some(destination) = &addon_spec.destination {
        project_dir.join(destination)
    } else {
        project_dir.join("addons").join(addon_name)
    };

    tracing::debug!(
        "Syncing addon {} from {:?} to {:?}",
        addon_name,
        source_base,
        dest_base
    );

    if !source_base.exists() {
        tracing::warn!(
            "Addon {} path {:?} does not exist, skipping",
            addon_name,
            source_base
        );
        return Ok(());
    }

    fs::create_dir_all(&dest_base)?;
    sync_recursive(
        &source_base,
        &dest_base,
        addon_spec.include.as_deref(),
        addon_spec.exclude.as_deref(),
    )?;
    Ok(())
}

fn sync_local_addon(
    project_dir: &Path,
    addon_name: &str,
    addon_spec: &AddonSpec,
    addon_source: &LocalAddonSource,
) -> Result<()> {
    let source_base = project_dir.join(&addon_source.path);
    let dest_base = project_dir.join("addons").join(addon_name);
    tracing::debug!(
        "Syncing addon {} from {:?} to {:?}",
        addon_name,
        source_base,
        dest_base
    );

    if !source_base.exists() {
        tracing::warn!(
            "Addon {} path {:?} does not exist, skipping",
            addon_name,
            source_base
        );
        return Ok(());
    }

    fs::create_dir_all(&dest_base)?;
    sync_recursive(
        &source_base,
        &dest_base,
        addon_spec.include.as_deref(),
        addon_spec.exclude.as_deref(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::project_specification::load_godot_project_spec;
    use crate::test_helpers::mock_git_client::MockGitClient;
    use std::fs;
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_sync_local_path_addons() -> Result<()> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .try_init();

        let tmp_dir = TempDir::new("gdenv-test")?;
        let tmp_data_dir = TempDir::new("gdenv-test-data-dir")?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let config = Config::setup(Some(&tmp_data_dir.path()))?;
        let git_client = MockGitClient::new(config);

        let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
        let test_addon2_path = test_data_dir.join("test-addon2-repo/addons/test-addon2");

        // Synchronize addon1 and addon2
        let test_addon1_path = test_data_dir.join("test-addon1-repo/addons/test-addon1");
        let str_spec_v1 = format!(
            r#"
[godot]
version = "4.6.0-stable"

[addon.test-addon1]
path = {}

[addon.test-addon2]
path = {}
        "#,
            toml::Value::String(test_addon1_path.to_string_lossy().to_string()),
            toml::Value::String(test_addon2_path.to_string_lossy().to_string()),
        );

        fs::write(&version_file, &str_spec_v1)?;
        let project_spec = load_godot_project_spec(tmp_dir.path())?;
        sync_addons(project_spec, tmp_dir.path(), &git_client).await?;

        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/plugin.cfg")
                .exists()
        );
        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon2/plugin.cfg")
                .exists()
        );
        assert!(!tmp_dir.path().join("file-not-part-of-addon.txt").exists());

        // Synchronize with an updated version of addon1 (addon1v2)
        let test_addon1v2_path = test_data_dir.join("test-addon1v2-repo/addons/test-addon1");
        let str_spec_v2 = format!(
            r#"
[godot]
version = "4.6.0-stable"

[addon.test-addon1]
path = {}

[addon.test-addon2]
path = {}
        "#,
            toml::Value::String(test_addon1v2_path.to_string_lossy().to_string()),
            toml::Value::String(test_addon2_path.to_string_lossy().to_string()),
        );
        fs::write(&version_file, &str_spec_v2)?;
        let project_spec = load_godot_project_spec(tmp_dir.path())?;
        sync_addons(project_spec, tmp_dir.path(), &git_client).await?;

        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/plugin.cfg")
                .exists()
        );
        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon2/plugin.cfg")
                .exists()
        );
        assert!(!tmp_dir.path().join("file-not-part-of-addon.txt").exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_sync_remote_git_addons() -> Result<()> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .try_init();

        let tmp_dir = TempDir::new("gdenv-test")?;
        let tmp_data_dir = TempDir::new("gdenv-test-data-dir")?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let config = Config::setup(Some(&tmp_data_dir.path()))?;
        let git_client = MockGitClient::new(config);

        // Synchronize addon
        let str_spec_v1 = r#"
[godot]
version = "4.6.0-stable"

[addon.test-addon1]
git = "https://github.com/GitHubUser/github_repo.git"
subdir = "addons/test-addon1"
destination = "addons/test-addon1/subfolder"
        "#;

        fs::write(&version_file, &str_spec_v1)?;
        let project_spec = load_godot_project_spec(tmp_dir.path())?;
        sync_addons(project_spec, tmp_dir.path(), &git_client).await?;

        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/subfolder/plugin.cfg")
                .exists()
        );
        assert!(!tmp_dir.path().join("file-not-part-of-addon.txt").exists());
        Ok(())
    }
}
