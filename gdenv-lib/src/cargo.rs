//! Utilities for working with Rust/Cargo projects.

use anyhow::Context;
use std::path::{Path, PathBuf};

/// A function that provides the cargo target directory path
/// when given a cargo manifest path. This function is normally implemented
/// as the following, except for unit tests:
/// ```rust,no_run
/// # use std::path::Path;
/// # use std::path::PathBuf;
/// # use crate::gdenv_lib::cargo::CargoInfoProvider;
/// # use crate::gdenv_lib::cargo::cargo_info_provider;
/// # use crate::gdenv_lib::project_specification::load_godot_project_spec;
/// load_godot_project_spec(&PathBuf::new(), cargo_info_provider());
/// ```
pub trait CargoInfoProvider: Fn(&Path) -> anyhow::Result<CargoInfo> + Send + Sync {}

/// Blanket implementation: "Any type that fits the closure signature automatically implements this trait"
impl<T> CargoInfoProvider for T where T: Fn(&Path) -> anyhow::Result<CargoInfo> + Send + Sync {}

#[derive(Debug, Clone)]
pub struct CargoInfo {
    pub crate_name: String,
    pub target_dir: PathBuf,
}

pub fn cargo_info_provider() -> impl CargoInfoProvider {
    let provider: &dyn CargoInfoProvider = &|cargo_manifest_path| {
        let cargo_metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(cargo_manifest_path)
            .exec()
            .context(format!(
                "Failed to execute cargo metadata command. Manifest path: {}",
                cargo_manifest_path.display()
            ))?;

        let crate_name = cargo_metadata
            .packages
            .iter()
            .find_map(|package| {
                if package.manifest_path == cargo_manifest_path {
                    Some(package.name.to_string())
                } else {
                    None
                }
            })
            .context(format!(
                "Failed to find crate name \"{}\" for cargo manifest path: {}",
                cargo_manifest_path.display(),
                cargo_manifest_path.display()
            ))?;

        let target_dir = cargo_metadata.target_directory.into_std_path_buf();

        Ok(CargoInfo {
            crate_name,
            target_dir,
        })
    };
    provider
}
