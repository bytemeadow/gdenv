//! The `gdenv-lib` public API consists of the [`api`] module.
//! The most useful class is [`api::godot_runner::GodotRunner`],
//! which makes it a good starting point for exploring this API.

#[cfg(test)]
pub mod test_helpers;

pub mod addons;
pub mod api;
pub mod cargo;
pub mod command_runner;
pub mod config;
pub mod download_client;
pub mod file_sync;
pub mod gdextension_config;
pub mod git;
pub mod github;
pub mod godot;
pub mod godot_version;
pub mod installer;
pub mod logging;
pub mod migrate;
pub mod path_extension;
pub mod project_specification;
