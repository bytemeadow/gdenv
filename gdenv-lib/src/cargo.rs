//! Utilities for working with Rust/Cargo projects.

use anyhow::Context;
use std::path::{Path, PathBuf};

/// A function that provides the cargo target directory path
/// when given a cargo manifest path. This function is normally implemented
/// as the following, except for unit tests:
/// ```rust,no_run
/// # use std::path::Path;
/// # use std::path::PathBuf;
/// # use crate::gdenv_lib::cargo::TargetPathProvider;
/// # use crate::gdenv_lib::project_specification::load_godot_project_spec;
/// let provider: &dyn TargetPathProvider = &|cargo_manifest_path| {
///     Ok(cargo_metadata::MetadataCommand::new()
///         .manifest_path(&cargo_manifest_path)
///         .exec()?
///         .target_directory
///         .into_std_path_buf())
/// };
/// load_godot_project_spec(&PathBuf::new(), provider);
/// ```
pub trait TargetPathProvider: Fn(&Path) -> anyhow::Result<PathBuf> + Send + Sync {}

/// Blanket implementation: "Any type that fits the closure signature automatically implements this trait"
impl<T> TargetPathProvider for T where T: Fn(&Path) -> anyhow::Result<PathBuf> + Send + Sync {}

pub fn cargo_target_path_provider() -> impl TargetPathProvider {
    let provider: &dyn TargetPathProvider = &|cargo_manifest_path| {
        Ok(cargo_metadata::MetadataCommand::new()
            .manifest_path(cargo_manifest_path)
            .exec()
            .context(format!(
                "Failed to execute cargo metadata command. Manifest path: {}",
                cargo_manifest_path.display()
            ))?
            .target_directory
            .into_std_path_buf())
    };
    provider
}
