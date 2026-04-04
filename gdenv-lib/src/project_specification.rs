use crate::cargo::CargoInfoProvider;
use crate::gdextension_config::GdExtensionConfig;
use crate::godot_version::GodotVersion;
use anyhow::{Context, Result};
use documented::{Documented, DocumentedFieldsOpt};
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
    /// Path to the project specification file.
    pub spec_file_path: Option<PathBuf>,
    /// Godot version to use when running the project.
    pub godot_version: GodotVersion,
    /// Path to the Godot project directory.
    pub godot_project_dir: PathBuf,
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

/// # The Godot project management file: `gdenv.toml`
/// # The following sections are available:
#[derive(Serialize, Deserialize, Documented, Debug, Default)]
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

/// # --------------------------------------------------------------------------------
/// # Describes the Godot project.
/// # Required.
/// [godot]
#[derive(Serialize, Deserialize, Documented, DocumentedFieldsOpt, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct SpecGodot {
    /// # Godot version to use when running the project.
    /// # Required.
    /// version = "4.6.0-stable"
    pub version: String,

    /// # Whether to use the .NET version of Godot. Optional.
    /// #dotnet = false
    pub dotnet: Option<bool>,

    /// # Path to the Godot project directory. Optional. Example: "./godot"
    /// #project_dir = "."
    pub project_dir: Option<PathBuf>,

    /// # Additional arguments to pass to the Godot executable. Optional.
    /// # Example: ["--debug", "--no-window", "--headless"]
    /// #run_args = []
    pub run_args: Option<Vec<String>>,

    /// # Additional arguments to pass to Godot when launching in editor mode. Optional.
    /// # Example: ["--debug", "--no-window", "--headless"]
    /// #editor_args = []
    pub editor_args: Option<Vec<String>>,

    /// # Before opening the project, run the editor in headless import mode
    /// # to import the project when the `.godot` folder doesn't yet exist.
    /// # Useful when opening newly Git cloned projects. Optional.
    /// #pre_import = true
    pub pre_import: Option<bool>,
}

/// # --------------------------------------------------------------------------------
/// # Describes how one-or-more gdextension files should be generated.
/// # The <name> field is only for your convenience.
/// # The gdextension file can be generated for various project <type>s, covered below.
/// # More project types can be added in the future. Optional.
/// [gdextension.<name>.<type>]
#[derive(Serialize, Deserialize, Documented, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub enum SpecGdExtensionGenerator {
    /// Generate a gdextension file for a rust project.
    Rust(SpecRustGdExtension),
}

/// # Generate a gdextension file for a rust project.
/// [gdextension.<name>.Rust]
#[derive(
    Serialize, Deserialize, Documented, DocumentedFieldsOpt, Debug, Eq, PartialEq, Clone, Default,
)]
#[serde(deny_unknown_fields)]
pub struct SpecRustGdExtension {
    /// # Path to the folder containing Cargo.toml. Used to find Cargo's build target directory. Required.
    /// # Example: "./rust"
    /// cargo_crate_path = "."
    pub cargo_crate_path: PathBuf,

    /// # File name for the gdextension config: `<config_name>.gdextension`. Optional.
    /// #config_name = "rust"
    pub config_name: Option<String>,

    /// # GdExtension API version compatability. Optional.
    /// #compatability_version = 4.1
    pub compatability_version: Option<String>,

    /// # GdExtension entry symbol for the shared library. Optional.
    /// #entry_symbol = "gdext_rust_init"
    pub entry_symbol: Option<String>,

    /// # Is the shared library hot reloadable? Optional.
    /// #reloadable = false
    pub reloadable: Option<bool>,
}

/// # --------------------------------------------------------------------------------
/// # Describes how one-or-more Godot addons should be synchronized.
/// # The <name> field determines the addon's default directory name,
/// # e.g. `addons/<name>/...files...`. Optional.
/// [addon.<name>]
#[derive(Serialize, Deserialize, Documented, DocumentedFieldsOpt, Debug, Eq, PartialEq, Clone)]
pub struct AddonSpec {
    /// # Paths to include from the addon's source directory. Optional.
    /// #include = []
    pub include: Option<Vec<PathBuf>>,

    /// # Paths to exclude from the addon's source directory. Optional.
    /// #exclude = []
    pub exclude: Option<Vec<PathBuf>>,

    /// # Path relative to project_dir to place addon files.
    /// # Defaults to <godot_project_dir>/addons/<addon_name>.
    /// #destination = "./custom/location"
    pub destination: Option<PathBuf>,

    #[serde(flatten)]
    pub source: AddonSource,
}

/// # Addons can be sourced from one of the following options:
/// #  - Git repository.
/// #  - Local directory.
#[derive(Serialize, Deserialize, Documented, Debug, Eq, PartialEq, Clone)]
#[serde(untagged, rename = "addon source type")]
pub enum AddonSource {
    /// Addon sourced from a Git repository.
    Git(GitAddonSource),
    /// Addon sourced from a local directory.
    Local(LocalAddonSource),
}

/// # -- Git repository specific addon fields:
#[derive(Serialize, Deserialize, Documented, DocumentedFieldsOpt, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct GitAddonSource {
    /// # Git repository URL. Required.
    /// #git = "https://github.com/bytemeadow/gdenv.git"
    pub git: String,

    /// # Git reference to 'checkout' (branch, tag, commit hash, etc). Optional.
    /// #rev = "main"
    pub rev: Option<String>,

    /// # Sub-directory, relative to the repository root, to source the addon files from. Optional.
    /// #subdir = ""
    pub subdir: Option<PathBuf>,
}

/// # -- Local directory specific addon fields:
#[derive(Serialize, Deserialize, Documented, DocumentedFieldsOpt, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct LocalAddonSource {
    /// # Path to the directory whose contents will be copied to the destination directory. Required.
    /// #path = "/path/to/local/addon"
    pub path: PathBuf,
}

pub fn spec_documentation() -> Result<String> {
    let out = [
        struct_doc::<ProjectSpecificationToml>(),
        struct_doc_f::<SpecGodot>(),
        struct_doc::<SpecGdExtensionGenerator>(),
        struct_doc_f::<SpecRustGdExtension>(),
        struct_doc_f::<AddonSpec>(),
        struct_doc_f::<GitAddonSource>(),
        struct_doc_f::<LocalAddonSource>(),
    ];
    Ok(out.join("\n"))
}

fn struct_doc_f<T: Documented + DocumentedFieldsOpt>() -> String {
    let fields = T::FIELD_DOCS
        .iter()
        .filter_map(|x| *x)
        .map(|doc| format!("{}\n", doc));
    [
        T::DOCS.to_string(),
        fields.collect::<Vec<String>>().join("\n"),
    ]
    .join("\n\n")
}

fn struct_doc<T: Documented>() -> String {
    [T::DOCS, ""].join("\n")
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
pub fn load_godot_project_spec<P: CargoInfoProvider>(
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
                spec_file_path: Some(path.clone()),
                godot_version: GodotVersion::new(
                    &spec.godot.version,
                    spec.godot.dotnet.unwrap_or(false),
                )?,
                godot_project_dir: project_dir.clone(),
                run_args: spec.godot.run_args.unwrap_or_default(),
                editor_args: spec.godot.editor_args.unwrap_or_default(),
                pre_import: spec.godot.pre_import.unwrap_or(true),
                gdextension: gdextension_generator_to_config(
                    path.parent().unwrap_or(start_path),
                    spec.gdextension.unwrap_or_default(),
                    &project_dir,
                    cargo_target_path_provider,
                )?,
                addons: spec.addon.unwrap_or_default(),
            })
        }
        SpecFileType::Version(path) => {
            let file_content = fs::read_to_string(&path)?;
            let mut version_str = file_content.trim().split(' ');
            let version = version_str
                .next()
                .context("No version specified in .godot-version file.")?;
            let dotnet = version_str.next().unwrap_or("");
            Ok(ProjectSpecification {
                spec_file_path: Some(path),
                godot_version: GodotVersion::new(version, dotnet == "dotnet" || dotnet == "mono")?,
                godot_project_dir: PathBuf::from("."),
                run_args: vec![],
                editor_args: vec![],
                pre_import: true,
                gdextension: HashMap::default(),
                addons: HashMap::default(),
            })
        }
    }
}

fn gdextension_generator_to_config<P: CargoInfoProvider>(
    working_dir: &Path,
    generators: HashMap<String, SpecGdExtensionGenerator>,
    godot_project_path: &Path,
    cargo_info_provider: P,
) -> Result<HashMap<String, GdExtensionConfig>> {
    generators
        .into_iter()
        .map(|(name, generator)| -> Result<(String, GdExtensionConfig)> {
            match generator {
                SpecGdExtensionGenerator::Rust(generator) => {
                    let cargo_info = &cargo_info_provider(
                        &working_dir
                            .join(&generator.cargo_crate_path)
                            .join("Cargo.toml"),
                    )?;
                    let mut config = GdExtensionConfig::start(
                        &cargo_info.crate_name,
                        &working_dir.join(godot_project_path),
                        &cargo_info.target_dir,
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
    use crate::cargo::CargoInfo;
    use crate::godot_version::GodotVersion;
    use anyhow::bail;

    #[test]
    fn test_gdenv_toml_project_spec_full() -> Result<()> {
        let tmp_dir = tempfile::Builder::new().prefix("gdenv-test").tempdir()?;
        let version_file = tmp_dir.path().join("gdenv.toml");
        let str_spec = r#"
[godot]
version = "4.6.0-stable"
dotnet = true
project_dir = "./godot"
run_args = ["arg1", "arg2"]
editor_args = ["arg3", "arg4"]
pre_import = false

[gdextension.config_a.Rust]
cargo_crate_path = "rust"
config_name = "not_rust"
compatability_version = "4.2"
entry_symbol = "my_entry_symbol"
reloadable = true

[gdextension.config_b.Rust]
cargo_crate_path = "rust"

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
        fs::write(&version_file, str_spec)?;
        let cargo_info = CargoInfo {
            crate_name: "my_gdextension".to_string(),
            target_dir: PathBuf::from("/home/user/.cache/cargo/target"),
        };
        let spec = load_godot_project_spec(tmp_dir.path(), |_| Ok(cargo_info.clone()))?;
        let expected_spec = ProjectSpecification {
            spec_file_path: Some(version_file),
            godot_version: GodotVersion::new("4.6.0", true)?,
            godot_project_dir: PathBuf::from("./godot"),
            run_args: vec!["arg1".to_string(), "arg2".to_string()],
            editor_args: vec!["arg3".to_string(), "arg4".to_string()],
            pre_import: false,
            gdextension: HashMap::from([
                (
                    "config_a".to_string(),
                    GdExtensionConfig::start(
                        &cargo_info.crate_name,
                        &tmp_dir.path().join("./godot"),
                        &cargo_info.target_dir,
                    )
                    .config_file_name("not_rust.gdextension")
                    .compatability_version("4.2")
                    .entry_symbol("my_entry_symbol")
                    .reloadable(true),
                ),
                (
                    "config_b".to_string(),
                    GdExtensionConfig::start(
                        &cargo_info.crate_name,
                        &tmp_dir.path().join("./godot"),
                        &cargo_info.target_dir,
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
        let tmp_dir = tempfile::Builder::new().prefix("gdenv-test").tempdir()?;
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
        let tmp_dir = tempfile::Builder::new().prefix("gdenv-test").tempdir()?;
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
        let tmp_dir = tempfile::Builder::new().prefix("gdenv-test").tempdir()?;
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
        let tmp_dir = tempfile::Builder::new().prefix("gdenv-test").tempdir()?;
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
        let tmp_dir = tempfile::Builder::new().prefix("gdenv-test").tempdir()?;
        let version_file = tmp_dir.path().join(".godot-version");
        let str_spec = "";
        fs::write(version_file, str_spec)?;
        let spec =
            load_godot_project_spec(tmp_dir.path(), |_| bail!("Test lambda not implemented."));
        assert!(spec.is_err());
        Ok(())
    }
}
