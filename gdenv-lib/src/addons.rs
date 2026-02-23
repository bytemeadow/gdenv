use crate::file_sync::sync_recursive;
use crate::project_specification::ProjectSpecification;
use anyhow::Result;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
fn sync_addons(project_spec: ProjectSpecification, working_dir: &Path) -> Result<()> {
    for (addon_name, addon_spec) in project_spec.addons {
        let addon_path_str = addon_spec.path.as_deref().unwrap_or(".");
        let source_base = working_dir.join(addon_path_str);
        let dest_base = working_dir
            .join(&project_spec.project_path)
            .join("addons")
            .join(&addon_name);
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
            continue;
        }

        fs::create_dir_all(&dest_base)?;
        sync_recursive(
            &source_base,
            &dest_base,
            addon_spec.include.as_deref(),
            addon_spec.exclude.as_deref(),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_specification::load_godot_project_spec;
    use std::fs;
    use tempdir::TempDir;

    #[test]
    fn test_sync_local_path_addons() -> Result<()> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .try_init();

        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join("gdenv.toml");

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
        sync_addons(project_spec, tmp_dir.path())?;

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
        sync_addons(project_spec, tmp_dir.path())?;

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
}
