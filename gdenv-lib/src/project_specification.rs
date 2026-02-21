use crate::godot_version::GodotVersion;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Godot configuration project specification
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct ProjectSpecification {
    pub godot_version: GodotVersion,
    pub project_path: PathBuf,
    pub run_args: Vec<String>,
    pub editor_args: Vec<String>,
    pub addons: HashMap<String, AddonSpec>,
}

/// Godot `gdenv.toml` file specification.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ProjectSpecificationToml {
    pub godot: SpecGodot,
    pub addon: Option<HashMap<String, AddonSpec>>,
}

/// `[godot]` toml section.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SpecGodot {
    pub version: String,
    pub dotnet: Option<bool>,
    pub project_path: Option<PathBuf>,
    pub run_args: Option<Vec<String>>,
    pub editor_args: Option<Vec<String>>,
}

/// Information about a Godot addon.
#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq, Clone)]
pub struct AddonSpec {
    pub include: Option<Vec<PathBuf>>,
    pub tag: Option<String>,
    pub branch: Option<String>,
    pub git: Option<String>,
    pub path: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
}

pub fn load_godot_project_spec(start_path: &Path) -> Result<ProjectSpecification> {
    let spec_file = find_godot_project_spec(start_path);
    match spec_file {
        SpecFileType::Toml(path) => {
            let str_spec = fs::read_to_string(path)?;
            let spec = toml::from_str::<ProjectSpecificationToml>(&str_spec)?;
            Ok(ProjectSpecification {
                godot_version: GodotVersion::new(
                    &spec.godot.version,
                    spec.godot.dotnet.unwrap_or(false),
                )?,
                project_path: spec.godot.project_path.unwrap_or(PathBuf::from_str(".")?),
                run_args: spec.godot.run_args.unwrap_or_default(),
                editor_args: spec.godot.editor_args.unwrap_or_default(),
                addons: spec.addon.unwrap_or_default(),
            })
        }
        SpecFileType::Version(path) => {
            let file_content = fs::read_to_string(path)?;
            let mut version_str = file_content.trim().split(' ');
            let version = version_str
                .next()
                .context("No version specified in .godot-version file.")?;
            let dotnet = version_str.next().unwrap_or("");
            Ok(ProjectSpecification {
                godot_version: GodotVersion::new(version, dotnet == "dotnet" || dotnet == "mono")?,
                project_path: PathBuf::from_str(".")?,
                run_args: vec![],
                editor_args: vec![],
                addons: HashMap::default(),
            })
        }
        SpecFileType::NotFound => Err(anyhow!(
            "No gdenv.toml or .godot-version file found in current directory or in parent directories."
        )),
    }
}

enum SpecFileType {
    Toml(PathBuf),
    Version(PathBuf),
    NotFound,
}

/// Searches for 'gdproject.toml' or '.godot-version' starting from `start_path`
/// and moving upwards towards the root. 'gdproject.toml' takes precedence.
fn find_godot_project_spec(start_path: &Path) -> SpecFileType {
    let mut current_dir = start_path.to_path_buf();

    loop {
        // 1. Check for the TOML file first (precedence)
        let toml_path = current_dir.join("gdenv.toml");
        if toml_path.exists() {
            return SpecFileType::Toml(toml_path);
        }

        // 2. Check for the .godot-version file
        let version_path = current_dir.join(".godot-version");
        if version_path.exists() {
            return SpecFileType::Version(version_path);
        }

        // Move to the parent directory
        if !current_dir.pop() {
            // Reached the filesystem root
            break;
        }
    }

    SpecFileType::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::godot_version::GodotVersion;
    use tempdir::TempDir;

    #[test]
    fn test_gdenv_toml_project_spec_full() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let str_spec = r#"
[godot]
version = "4.6.0-stable"
dotnet = true
project_path = "./godot"
run_args = ["arg1", "arg2"]
editor_args = ["arg3", "arg4"]

[addon.dialogic]
git = "https://github.com/dialogic-godot/dialogic"
rev = "main"

[addon.curtains]
git = "https://github.com/DragonAxe/gd-bvy-curtains"
rev = "other_ref"

[addon.gdunit4]
git = "https://github.com/godot-gdunit-labs/gdUnit4"
include = ["addons/gdUnit4"]

[addon.local-project]
path = "../local-project"
        "#;
        fs::write(version_file, str_spec)?;
        let spec = load_godot_project_spec(tmp_dir.path())?;
        let expected_spec = ProjectSpecification {
            godot_version: GodotVersion::new("4.6.0", true)?,
            project_path: PathBuf::from_str("./godot")?,
            run_args: vec!["arg1".to_string(), "arg2".to_string()],
            editor_args: vec!["arg3".to_string(), "arg4".to_string()],
            addons: HashMap::from([
                (
                    "dialogic".to_string(),
                    AddonSpec {
                        git: Some("https://github.com/dialogic-godot/dialogic".to_string()),
                        rev: Some("main".to_string()),
                        ..AddonSpec::default()
                    },
                ),
                (
                    "curtains".to_string(),
                    AddonSpec {
                        git: Some("https://github.com/DragonAxe/gd-bvy-curtains".to_string()),
                        rev: Some("other_ref".to_string()),
                        ..AddonSpec::default()
                    },
                ),
                (
                    "gdunit4".to_string(),
                    AddonSpec {
                        git: Some("https://github.com/godot-gdunit-labs/gdUnit4".to_string()),
                        include: Some(vec![PathBuf::from_str("addons/gdUnit4")?]),
                        ..AddonSpec::default()
                    },
                ),
                (
                    "local-project".to_string(),
                    AddonSpec {
                        path: Some("../local-project".to_string()),
                        ..AddonSpec::default()
                    },
                ),
            ]),
        };
        assert_eq!(spec, expected_spec);
        Ok(())
    }

    #[test]
    fn test_gdenv_toml_project_spec_minimal() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let str_spec = r#"
[godot]
version = "4.6.0"
        "#;
        fs::write(version_file, str_spec)?;
        let spec = load_godot_project_spec(tmp_dir.path())?;
        assert_eq!(spec.godot_version, GodotVersion::new("4.6.0", false)?);
        Ok(())
    }

    #[test]
    fn test_gdenv_toml_project_spec_empty() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let str_spec = r#""#;
        fs::write(version_file, str_spec)?;
        let spec = load_godot_project_spec(tmp_dir.path());
        assert!(spec.is_err());
        Ok(())
    }

    #[test]
    fn test_godot_version_file_full() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join(".godot-version");
        let str_spec = "4.6 dotnet";
        fs::write(version_file, str_spec)?;

        let spec = load_godot_project_spec(tmp_dir.path())?;

        assert_eq!(spec.godot_version, GodotVersion::new("4.6.0-stable", true)?);

        Ok(())
    }

    #[test]
    fn test_godot_version_file_version_only() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join(".godot-version");
        let str_spec = "4.6";
        fs::write(version_file, str_spec)?;

        let spec = load_godot_project_spec(tmp_dir.path())?;

        assert_eq!(
            spec.godot_version,
            GodotVersion::new("4.6.0-stable", false)?
        );

        Ok(())
    }

    #[test]
    fn test_godot_version_file_empty() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join(".godot-version");
        let str_spec = "";
        fs::write(version_file, str_spec)?;
        let spec = load_godot_project_spec(tmp_dir.path());
        assert!(spec.is_err());
        Ok(())
    }
}
