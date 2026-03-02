use crate::cargo::TargetPathProvider;
use crate::gdextension_config::GdExtensionConfig;
use crate::godot_version::GodotVersion;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectSpecError {
    #[error(
        "No gdenv.toml or .godot-version file found in current directory or in parent directories."
    )]
    NotFound,
    #[error("Failed to parse Godot project configuration file {0}: {1}")]
    ParseError(PathBuf, String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Godot configuration project specification
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ProjectSpecification {
    /// Godot version to use when running the project.
    pub godot_version: GodotVersion,
    /// Path to the Godot project directory.
    pub project_dir: PathBuf,
    /// Additional arguments to pass to the Godot executable.
    pub run_args: Vec<String>,
    /// Additional arguments to pass to Godot when launching in editor mode.
    pub editor_args: Vec<String>,
    /// Run the editor in headless import mode if the .godot folder doesn't exist.
    pub pre_import: bool,
    /// .gdextension file generation configuration.
    pub gdextension: HashMap<String, GdExtensionConfig>,
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
    /// .gdextension file generation configuration.
    pub gdextension: Option<HashMap<String, SpecGdExtensionGenerator>>,
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
    /// Run the editor in headless import mode if the .godot folder doesn't exist.
    pub pre_import: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub enum SpecGdExtensionGenerator {
    GodotBevy(SpecGodotBevyGdExtension),
}

/// `[gdextension.<name>.GodotBevy]` toml section.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct SpecGodotBevyGdExtension {
    /// Used to determine the name of the shared library for gdextension.
    pub crate_name: String,
    /// Used to find Cargo's build target directory.
    pub cargo_manifest_path: PathBuf,

    /// File name for the gdextension config: `<config_name>.gdextension`
    pub config_name: Option<String>,
    /// Standard gdextension attribute
    pub compatability_version: Option<String>,
    /// Standard gdextension attribute
    pub entry_symbol: Option<String>,
    /// Standard gdextension attribute
    pub reloadable: Option<bool>,
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
pub fn load_godot_project_spec<P: TargetPathProvider>(
    start_path: &Path,
    cargo_target_path_provider: P,
) -> Result<ProjectSpecification, ProjectSpecError> {
    let spec_file = find_godot_project_spec(start_path)?;
    match spec_file {
        SpecFileType::Toml(path) => {
            let str_spec = fs::read_to_string(&path)?;
            let spec = toml::from_str::<ProjectSpecificationToml>(&str_spec).context(format!(
                "Failed to parse Godot project configuration file gdenv.toml: {}",
                path.display()
            ))?;
            let project_dir = spec.godot.project_dir.unwrap_or(PathBuf::from("."));
            Ok(ProjectSpecification {
                godot_version: GodotVersion::new(
                    &spec.godot.version,
                    spec.godot.dotnet.unwrap_or(false),
                )?,
                project_dir: project_dir.clone(),
                run_args: spec.godot.run_args.unwrap_or_default(),
                editor_args: spec.godot.editor_args.unwrap_or_default(),
                pre_import: spec.godot.pre_import.unwrap_or(true),
                gdextension: gdextension_generator_to_config(
                    start_path,
                    spec.gdextension.unwrap_or_default(),
                    &project_dir,
                    cargo_target_path_provider,
                )?,
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
                project_dir: PathBuf::from("."),
                run_args: vec![],
                editor_args: vec![],
                pre_import: true,
                gdextension: HashMap::default(),
                addons: HashMap::default(),
            })
        }
    }
}

fn gdextension_generator_to_config<P: TargetPathProvider>(
    working_dir: &Path,
    generators: HashMap<String, SpecGdExtensionGenerator>,
    godot_project_path: &Path,
    cargo_target_path_provider: P,
) -> Result<HashMap<String, GdExtensionConfig>> {
    generators
        .into_iter()
        .map(|(name, generator)| -> Result<(String, GdExtensionConfig)> {
            match generator {
                SpecGdExtensionGenerator::GodotBevy(generator) => {
                    let mut config = GdExtensionConfig::start(
                        &generator.crate_name,
                        &working_dir.join(godot_project_path),
                        &cargo_target_path_provider(
                            &working_dir.join(&generator.cargo_manifest_path),
                        )?,
                    );
                    if let Some(config_name) = &generator.config_name {
                        config = config.config_file_name(&format!("{}.gdextension", config_name));
                    }
                    if let Some(compatability_version) = &generator.compatability_version {
                        config = config.compatability_version(compatability_version);
                    }
                    if let Some(entry_symbol) = &generator.entry_symbol {
                        config = config.entry_symbol(entry_symbol);
                    }
                    if let Some(reloadable) = generator.reloadable {
                        config = config.reloadable(reloadable);
                    }
                    Ok((name, config))
                }
            }
        })
        .collect()
}

enum SpecFileType {
    Toml(PathBuf),
    Version(PathBuf),
}

/// Searches for 'gdproject.toml' or '.godot-version' starting from `start_path`
/// and moving upwards towards the root. 'gdproject.toml' takes precedence.
fn find_godot_project_spec(start_path: &Path) -> Result<SpecFileType, ProjectSpecError> {
    let mut current_dir = start_path.to_path_buf();

    loop {
        // 1. Check for the TOML file first (precedence)
        let toml_path = current_dir.join("gdenv.toml");
        if toml_path.exists() {
            return Ok(SpecFileType::Toml(toml_path));
        }

        // 2. Check for the .godot-version file
        let version_path = current_dir.join(".godot-version");
        if version_path.exists() {
            return Ok(SpecFileType::Version(version_path));
        }

        // Move to the parent directory
        if !current_dir.pop() {
            // Reached the filesystem root
            break;
        }
    }

    Err(ProjectSpecError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::godot_version::GodotVersion;
    use anyhow::bail;
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
pre_import = false

[gdextension.config_a.GodotBevy]
crate_name = "my_gdextension"
cargo_manifest_path = "./rust/Cargo.toml"
config_name = "not_rust"
compatability_version = "4.2"
entry_symbol = "my_entry_symbol"
reloadable = true

[gdextension.config_b.GodotBevy]
crate_name = "my_gdextension"
cargo_manifest_path = "./rust/Cargo.toml"

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
        let cargo_target_path = PathBuf::from("/home/user/.cache/cargo/target");
        let spec =
            load_godot_project_spec(tmp_dir.path(), |_| Ok(cargo_target_path.to_path_buf()))?;
        let expected_spec = ProjectSpecification {
            godot_version: GodotVersion::new("4.6.0", true)?,
            project_dir: PathBuf::from("./godot"),
            run_args: vec!["arg1".to_string(), "arg2".to_string()],
            editor_args: vec!["arg3".to_string(), "arg4".to_string()],
            pre_import: false,
            gdextension: HashMap::from([
                (
                    "config_a".to_string(),
                    GdExtensionConfig::start(
                        "my_gdextension",
                        &tmp_dir.path().join("./godot"),
                        &cargo_target_path,
                    )
                    .config_file_name("not_rust.gdextension")
                    .compatability_version("4.2")
                    .entry_symbol("my_entry_symbol")
                    .reloadable(true),
                ),
                (
                    "config_b".to_string(),
                    GdExtensionConfig::start(
                        "my_gdextension",
                        &tmp_dir.path().join("./godot"),
                        &cargo_target_path,
                    ),
                ),
            ]),
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
                        include: Some(vec![PathBuf::from("addons/gdUnit4")]),
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
                            path: PathBuf::from("../local-project"),
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
        let spec =
            load_godot_project_spec(tmp_dir.path(), |_| bail!("Test lambda not implemented."))?;
        assert_eq!(spec.godot_version, GodotVersion::new("4.6.0", false)?);
        Ok(())
    }

    #[test]
    fn test_gdenv_toml_project_spec_empty() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let str_spec = r#""#;
        fs::write(version_file, str_spec)?;
        let spec =
            load_godot_project_spec(tmp_dir.path(), |_| bail!("Test lambda not implemented."));
        assert!(spec.is_err());
        Ok(())
    }

    #[test]
    fn test_godot_version_file_full() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join(".godot-version");
        let str_spec = "4.6 dotnet";
        fs::write(version_file, str_spec)?;

        let spec =
            load_godot_project_spec(tmp_dir.path(), |_| bail!("Test lambda not implemented."))?;

        assert_eq!(spec.godot_version, GodotVersion::new("4.6.0-stable", true)?);

        Ok(())
    }

    #[test]
    fn test_godot_version_file_version_only() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-test")?;
        let version_file = tmp_dir.path().join(".godot-version");
        let str_spec = "4.6";
        fs::write(version_file, str_spec)?;

        let spec =
            load_godot_project_spec(tmp_dir.path(), |_| bail!("Test lambda not implemented."))?;

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
        let spec =
            load_godot_project_spec(tmp_dir.path(), |_| bail!("Test lambda not implemented."));
        assert!(spec.is_err());
        Ok(())
    }
}
