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
    /// Godot version to use when running the project.
    pub godot_version: GodotVersion,
    /// Path to the Godot project directory.
    pub project_dir: PathBuf,
    /// Additional arguments to pass to the Godot executable.
    pub run_args: Vec<String>,
    /// Additional arguments to pass to Godot when launching in editor mode.
    pub editor_args: Vec<String>,
    /// Godot addon specifications. The name given in this field will
    /// be used as the addon's name in the project's `addons` directory.
    pub addons: HashMap<String, AddonSpec>,
}

/// Godot `gdenv.toml` file specification.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectSpecificationToml {
    /// Specifications for the Godot project.
    pub godot: SpecGodot,
    /// Godot addon specifications. The name given in this field will
    /// be used as the addon's name in the project's `addons` directory.
    pub addon: Option<HashMap<String, AddonSpec>>,
}

/// `[godot]` toml section.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct SpecGodot {
    /// Godot version to use when running the project.
    pub version: String,
    /// Whether to use the .NET version of Godot.
    pub dotnet: Option<bool>,
    /// Path to the Godot project directory.
    pub project_dir: Option<PathBuf>,
    /// Additional arguments to pass to the Godot executable.
    pub run_args: Option<Vec<String>>,
    /// Additional arguments to pass to Godot when launching in editor mode.
    pub editor_args: Option<Vec<String>>,
}

/// Information about a Godot addon.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct AddonSpec {
    /// Paths to include in the addon's source directory. Relative to the project_dir.
    pub include: Option<Vec<PathBuf>>,
    /// Paths to exclude from the addon's source directory. Relative to the project_dir.
    pub exclude: Option<Vec<PathBuf>>,
    /// Path relative to project_dir to place addon files.
    /// Defaults to <godot_project_dir>/addons/<addon_name>.
    pub destination: Option<PathBuf>,
    /// Where to get the addon's source code from.
    #[serde(flatten)]
    pub source: AddonSource,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(untagged, rename = "addon source type")]
pub enum AddonSource {
    Git(GitAddonSource),
    Local(LocalAddonSource),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct GitAddonSource {
    /// Git repository URL.
    pub git: String,
    /// Git reference to 'checkout' (branch, tag, commit hash, etc).
    pub rev: Option<String>,
    /// Directory inside the repository to synchronize to the addon's directory.
    pub subdir: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct LocalAddonSource {
    pub path: PathBuf,
}

/// Loads the Godot project specification from a given starting path.
///
/// This function attempts to locate and parse a Godot project configuration file within the
/// directory tree starting from the given `start_path`. It supports two types of configuration
/// files:
///
/// - `gdenv.toml`: A TOML-based configuration file that defines various project settings.
/// - `.godot-version`: A simple file that specifies the Godot version information.
///
/// # Arguments
///
/// * `start_path` - A reference to the starting directory path where the search for the
///   project configuration file begins.
pub fn load_godot_project_spec(start_path: &Path) -> Result<ProjectSpecification> {
    let spec_file = find_godot_project_spec(start_path);
    match spec_file {
        SpecFileType::Toml(path) => {
            let str_spec = fs::read_to_string(&path)?;
            let spec = toml::from_str::<ProjectSpecificationToml>(&str_spec).context(format!(
                "Failed to parse Godot project configuration file gdenv.toml: {}",
                path.display()
            ))?;
            Ok(ProjectSpecification {
                godot_version: GodotVersion::new(
                    &spec.godot.version,
                    spec.godot.dotnet.unwrap_or(false),
                )?,
                project_dir: spec.godot.project_dir.unwrap_or(PathBuf::from_str(".")?),
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
                project_dir: PathBuf::from_str(".")?,
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
project_dir = "./godot"
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
            project_dir: PathBuf::from_str("./godot")?,
            run_args: vec!["arg1".to_string(), "arg2".to_string()],
            editor_args: vec!["arg3".to_string(), "arg4".to_string()],
            addons: HashMap::from([
                (
                    "dialogic".to_string(),
                    AddonSpec {
                        include: None,
                        exclude: None,
                        destination: None,
                        source: AddonSource::Git(GitAddonSource {
                            git: "https://github.com/dialogic-godot/dialogic".to_string(),
                            rev: Some("main".to_string()),
                            subdir: None,
                        }),
                    },
                ),
                (
                    "curtains".to_string(),
                    AddonSpec {
                        include: None,
                        exclude: None,
                        destination: None,
                        source: AddonSource::Git(GitAddonSource {
                            git: "https://github.com/DragonAxe/gd-bvy-curtains".to_string(),
                            rev: Some("other_ref".to_string()),
                            subdir: None,
                        }),
                    },
                ),
                (
                    "gdunit4".to_string(),
                    AddonSpec {
                        include: Some(vec![PathBuf::from_str("addons/gdUnit4")?]),
                        exclude: None,
                        destination: None,
                        source: AddonSource::Git(GitAddonSource {
                            git: "https://github.com/godot-gdunit-labs/gdUnit4".to_string(),
                            rev: None,
                            subdir: None,
                        }),
                    },
                ),
                (
                    "local-project".to_string(),
                    AddonSpec {
                        include: None,
                        exclude: None,
                        destination: None,
                        source: AddonSource::Local(LocalAddonSource {
                            path: PathBuf::from_str("../local-project")?,
                        }),
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
