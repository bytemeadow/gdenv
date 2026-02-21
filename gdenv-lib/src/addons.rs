use crate::project_specification::ProjectSpecification;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[allow(dead_code)]
fn sync_addons(project_spec: ProjectSpecification, working_dir: &Path) -> Result<()> {
    for (addon_name, addon_spec) in project_spec.addons {
        let addon_path_str = addon_spec.path.as_deref().unwrap_or(".");
        let source_base = working_dir.join(addon_path_str);
        let dest_base = working_dir.join(&project_spec.project_path);

        if !source_base.exists() {
            tracing::warn!("Addon {} path {:?} does not exist, skipping", addon_name, source_base);
            continue;
        }

        sync_recursive(
            &source_base,
            &dest_base,
            addon_spec.include.as_deref(),
            addon_spec.exclude.as_deref(),
        )?;
    }
    Ok(())
}

fn sync_recursive(
    source_base: &Path,
    dest_base: &Path,
    includes: Option<&[PathBuf]>,
    excludes: Option<&[PathBuf]>,
) -> Result<()> {
    // TODO: extend sync_recursive so that it will delete files from the destination
    //  that don't exist in the source and replace files that exist in both.
    for entry in WalkDir::new(source_base).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel_path = path.strip_prefix(source_base).context("Failed to strip prefix")?;

        // 1. Check Excludes: If it matches any exclude pattern, skip it
        if let Some(excludes) = excludes {
            if excludes.iter().any(|ex| rel_path.starts_with(ex)) {
                continue;
            }
        }

        // 2. Check Includes: If includes are specified, the path must be inside one of them
        if let Some(includes) = includes {
            let is_included = includes.iter().any(|inc| {
                rel_path.starts_with(inc) || inc.starts_with(rel_path)
            });
            if !is_included {
                continue;
            }
        }

        // 3. Perform Copy
        let target_path = dest_base.join(rel_path);
        if path.is_dir() {
            fs::create_dir_all(&target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target_path)
                .with_context(|| format!("Failed to copy {:?} to {:?}", path, target_path))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_specification::load_godot_project_spec;
    use std::fs;
    use std::path::PathBuf;
    use tempdir::TempDir;

    #[test]
    fn test_sync_local_path_addons() -> Result<()> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let test_addon1_path: PathBuf = [manifest_dir, "test-data", "test-addon1-repo"]
            .iter()
            .collect();
        let test_addon1v2_path: PathBuf = [manifest_dir, "test-data", "test-addon1v2-repo"]
            .iter()
            .collect();
        let test_addon2_path: PathBuf = [manifest_dir, "test-data", "test-addon2-repo"]
            .iter()
            .collect();
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let str_spec = format!(
            r#"
[godot]
version = "4.6.0-stable"

[addon.test-addon1]
path = "{}"
include = ["addons"]

[addon.test-addon2]
path = "{}"
exclude = ["file-not-part-of-addon.txt"]

[addon.test-addon1v2]
path = "{}"
include = ["addons"]
        "#,
            test_addon1_path.as_path().display(),
            test_addon2_path.as_path().display(),
            test_addon1v2_path.as_path().display(),
        );
        fs::write(version_file, str_spec)?;
        let project_spec = load_godot_project_spec(tmp_dir.path())?;

        sync_addons(project_spec, tmp_dir.path())?;

        assert!(tmp_dir.path().join("addons/test-addon1/plugin.cfg").exists());
        assert!(tmp_dir.path().join("addons/test-addon2/plugin.cfg").exists());
        assert!(!tmp_dir.path().join("file-not-part-of-addon.txt").exists());

        Ok(())
    }
}
