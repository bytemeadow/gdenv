//! Utilities for generating a `.gdextension` file for Godot.

use crate::path_extension::PathExt;
use anyhow::{Context, Result};
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};

/// A validated GDExtension configuration ready to be writen to a `.gdextension` file.
/// Construct me using the builder `GdExtensionConfig::start`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidGdExtensionConfig {
    config_file_name: String,
    compatability_version: String,
    entry_symbol: String,
    reloadable: bool,
    release_target: Option<String>,
    debug_target: Option<String>,
    godot_project_path: PathBuf,
    relative_target_path: String,
    library_name: String,
}

/// Used to configure a `.gdextension` file for Godot that can be written to disk.
/// This class is a builder for `ValidGdExtensionConfig`.
///
/// Example usage:
/// ```rust,no_run
/// # fn example() -> anyhow::Result<()> {
/// # use std::path::PathBuf;
/// # use crate::gdenv_lib::gdextension_config::GdExtensionConfig;
/// # let crate_name = "test-library";
/// # let godot_project_path = &PathBuf::from("/home/user/projects/godot_project_path");
/// # let target_directory = &PathBuf::from("/home/user/.cache/cargo/target");
/// GdExtensionConfig::start(crate_name, godot_project_path, target_directory)
///     .build()?
///     .write()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GdExtensionConfig {
    config_file_name: String,
    compatability_version: String,
    entry_symbol: String,
    reloadable: bool,
    release_target: Option<String>,
    debug_target: Option<String>,
    target_path: Option<PathBuf>,
    godot_project_path: Option<PathBuf>,
    library_name: Option<String>,
}

impl Default for GdExtensionConfig {
    fn default() -> Self {
        Self {
            config_file_name: "rust.gdextension".to_string(),
            compatability_version: "4.1".to_string(),
            entry_symbol: "gdext_rust_init".to_string(),
            reloadable: true,
            release_target: Some("release".to_string()),
            debug_target: Some("debug".to_string()),
            target_path: None,
            godot_project_path: None,
            library_name: None,
        }
    }
}

impl GdExtensionConfig {
    /// Start building a `ValidGdExtensionConfig` from the given parameters.
    ///
    /// Note: `crate_name` will have dashes replaced with underscores
    /// to match cargo file naming conventions.
    pub fn start(crate_name: &str, godot_project_path: &Path, target_directory: &Path) -> Self {
        Self {
            library_name: Some(crate_name.replace("-", "_")),
            target_path: Some(target_directory.to_path_buf()),
            godot_project_path: Some(godot_project_path.to_path_buf()),
            ..Self::default()
        }
    }

    /// Validate builder parameters and return a `ValidGdExtensionConfig`.
    pub fn build(&self) -> Result<ValidGdExtensionConfig> {
        let target_path = self
            .target_path
            .as_deref()
            .context("Missing target path")?
            .to_absolute()
            .with_context(|| {
                format!(
                    "Failed to calculate absolute target path: {:?}",
                    self.target_path
                )
            })?;
        let godot_project_path = self
            .godot_project_path
            .as_ref()
            .context("Missing godot project path")?
            .to_absolute()
            .with_context(|| {
                format!(
                    "Failed to calculate absolute godot project path: {:?}",
                    self.godot_project_path
                )
            })?;
        let library_name = self.library_name.as_ref().context("Missing library name")?;
        let relative_target_path = diff_paths(&target_path, &godot_project_path)
            .with_context(|| {
                format!(
                    "Failed to calculate relative target path: target={:?} -> godot_project={:?}",
                    target_path, godot_project_path
                )
            })?
            .to_str()
            .context("Failed to convert relative target path to string")?
            .to_string()
            .replace('\\', "/"); // Godot res:// paths are always forward slashes.

        Ok(ValidGdExtensionConfig {
            config_file_name: self.config_file_name.clone(),
            reloadable: self.reloadable,
            compatability_version: self.compatability_version.clone(),
            entry_symbol: self.entry_symbol.clone(),
            release_target: self.release_target.clone(),
            debug_target: self.debug_target.clone(),
            godot_project_path,
            relative_target_path,
            library_name: library_name.clone(),
        })
    }

    /// Only include 'release' library configuration.
    /// The default is to include both 'release' and 'debug'.
    pub fn release_target(self, name: Option<String>) -> Self {
        Self {
            release_target: name,
            ..self
        }
    }

    /// Only include 'debug' library configuration.
    /// The default is to include both 'release' and 'debug'.
    pub fn debug_target(self, name: Option<String>) -> Self {
        Self {
            debug_target: name,
            ..self
        }
    }

    /// Configure the minimum compatibility version for the generated `.gdextension` file.
    /// The default is `4.1`.
    pub fn compatability_version(self, version: &str) -> Self {
        Self {
            compatability_version: version.to_string(),
            ..self
        }
    }

    /// Configure the name of the entry symbol for the generated `.gdextension` file.
    /// The default is `gdext_rust_init`.
    pub fn entry_symbol(self, symbol: &str) -> Self {
        Self {
            entry_symbol: symbol.to_string(),
            ..self
        }
    }

    /// Configure the name of the generated `.gdextension` file.
    /// The default is `rust.gdextension`.
    pub fn config_file_name(self, name: &str) -> Self {
        Self {
            config_file_name: name.to_string(),
            ..self
        }
    }

    /// Configure whether the `.gdextension` library is hot reloadable.
    /// The default is `true`.
    pub fn reloadable(self, reloadable: bool) -> Self {
        Self { reloadable, ..self }
    }
}

impl ValidGdExtensionConfig {
    /// Generate a `.gdextension` file as a string.
    pub fn create(&self) -> String {
        let release = if let Some(release_target) = &self.release_target {
            format!(
                r#"
linux.release.x86_64 =   "res://{target}/{release_target}/lib{pkgname}.so"
windows.release.x86_64 = "res://{target}/{release_target}/{pkgname}.dll"
macos.release =          "res://{target}/{release_target}/lib{pkgname}.dylib"
macos.release.arm64 =    "res://{target}/{release_target}/lib{pkgname}.dylib"
"#,
                target = self.relative_target_path,
                release_target = release_target,
                pkgname = self.library_name,
            )
            .trim_start()
            .to_string()
        } else {
            "".to_string()
        };

        let debug = if let Some(debug_target) = &self.debug_target {
            format!(
                r#"
linux.debug.x86_64 =     "res://{target}/{debug_target}/lib{pkgname}.so"
windows.debug.x86_64 =   "res://{target}/{debug_target}/{pkgname}.dll"
macos.debug =            "res://{target}/{debug_target}/lib{pkgname}.dylib"
macos.debug.arm64 =      "res://{target}/{debug_target}/lib{pkgname}.dylib"
"#,
                target = self.relative_target_path,
                debug_target = debug_target,
                pkgname = self.library_name,
            )
            .trim_start()
            .to_string()
        } else {
            "".to_string()
        };

        let preamble = format!(
            r#"
[configuration]
entry_symbol = "{entry_symbol}"
compatibility_minimum = {compatability_version}
reloadable = {reloadable}

[libraries]
"#,
            entry_symbol = self.entry_symbol,
            compatability_version = self.compatability_version,
            reloadable = if self.reloadable { "true" } else { "false" },
        )
        .trim_start()
        .to_string();

        preamble + &release + &debug
    }

    /// The full path to the generated `.gdextension` file including the file name.
    pub fn full_config_path(&self) -> PathBuf {
        self.godot_project_path.join(&self.config_file_name)
    }

    /// Write a generated `.gdextension` file to disk.
    pub fn write(&self) -> std::io::Result<()> {
        std::fs::write(self.full_config_path(), self.create())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tempdir::TempDir;

    fn create_test_directories() -> Result<(TempDir, PathBuf, PathBuf)> {
        let tempdir = TempDir::new("gdenv-test")?;
        let godot_project_path = tempdir.path().join("home/user/projects/godot_project_path");
        std::fs::create_dir_all(&godot_project_path)?;
        let target_path = tempdir.path().join("home/user/.cache/cargo/target");
        std::fs::create_dir_all(&target_path)?;

        Ok((tempdir, godot_project_path, target_path))
    }

    #[test]
    fn test_create() -> Result<()> {
        let (_tempdir, godot_project_path, target_path) = create_test_directories()?;
        let config = GdExtensionConfig::start("test_library", &godot_project_path, &target_path)
            .build()
            .expect("Successful build");
        let file_string = config.create();

        assert!(!file_string.contains('\\'));
        assert_eq!(
            file_string,
            r#"
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1
reloadable = true

[libraries]
linux.release.x86_64 =   "res://../../.cache/cargo/target/release/libtest_library.so"
windows.release.x86_64 = "res://../../.cache/cargo/target/release/test_library.dll"
macos.release =          "res://../../.cache/cargo/target/release/libtest_library.dylib"
macos.release.arm64 =    "res://../../.cache/cargo/target/release/libtest_library.dylib"
linux.debug.x86_64 =     "res://../../.cache/cargo/target/debug/libtest_library.so"
windows.debug.x86_64 =   "res://../../.cache/cargo/target/debug/test_library.dll"
macos.debug =            "res://../../.cache/cargo/target/debug/libtest_library.dylib"
macos.debug.arm64 =      "res://../../.cache/cargo/target/debug/libtest_library.dylib"
"#
            .trim_start()
            .to_string()
        );
        Ok(())
    }

    #[test]
    fn test_create_release_only() -> Result<()> {
        let (_tempdir, godot_project_path, target_path) = create_test_directories()?;
        let config = GdExtensionConfig::start("test_library", &godot_project_path, &target_path)
            .release_target(Some("release".to_string()))
            .debug_target(None)
            .build()
            .expect("Successful build");
        let file_string = config.create();

        assert!(!file_string.contains('\\'));
        assert_eq!(
            file_string,
            r#"
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1
reloadable = true

[libraries]
linux.release.x86_64 =   "res://../../.cache/cargo/target/release/libtest_library.so"
windows.release.x86_64 = "res://../../.cache/cargo/target/release/test_library.dll"
macos.release =          "res://../../.cache/cargo/target/release/libtest_library.dylib"
macos.release.arm64 =    "res://../../.cache/cargo/target/release/libtest_library.dylib"
"#
            .trim_start()
            .to_string()
        );
        Ok(())
    }

    #[test]
    fn test_create_debug_only() -> Result<()> {
        let (_tempdir, godot_project_path, target_path) = create_test_directories()?;
        let config = GdExtensionConfig::start("test_library", &godot_project_path, &target_path)
            .release_target(None)
            .debug_target(Some("debug".to_string()))
            .build()
            .expect("Successful build");
        let file_string = config.create();

        assert!(!file_string.contains('\\'));
        assert_eq!(
            file_string,
            r#"
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1
reloadable = true

[libraries]
linux.debug.x86_64 =     "res://../../.cache/cargo/target/debug/libtest_library.so"
windows.debug.x86_64 =   "res://../../.cache/cargo/target/debug/test_library.dll"
macos.debug =            "res://../../.cache/cargo/target/debug/libtest_library.dylib"
macos.debug.arm64 =      "res://../../.cache/cargo/target/debug/libtest_library.dylib"
"#
            .trim_start()
            .to_string()
        );
        Ok(())
    }

    #[test]
    fn test_entry_symbol() -> Result<()> {
        let (_tempdir, godot_project_path, target_path) = create_test_directories()?;
        let config = GdExtensionConfig::start("test_library", &godot_project_path, &target_path)
            .entry_symbol("custom_entry_point")
            .build()
            .expect("Successful build");
        let file_string = config.create();

        assert!(!file_string.contains('\\'));
        assert_eq!(
            file_string,
            r#"
[configuration]
entry_symbol = "custom_entry_point"
compatibility_minimum = 4.1
reloadable = true

[libraries]
linux.release.x86_64 =   "res://../../.cache/cargo/target/release/libtest_library.so"
windows.release.x86_64 = "res://../../.cache/cargo/target/release/test_library.dll"
macos.release =          "res://../../.cache/cargo/target/release/libtest_library.dylib"
macos.release.arm64 =    "res://../../.cache/cargo/target/release/libtest_library.dylib"
linux.debug.x86_64 =     "res://../../.cache/cargo/target/debug/libtest_library.so"
windows.debug.x86_64 =   "res://../../.cache/cargo/target/debug/test_library.dll"
macos.debug =            "res://../../.cache/cargo/target/debug/libtest_library.dylib"
macos.debug.arm64 =      "res://../../.cache/cargo/target/debug/libtest_library.dylib"
"#
            .trim_start()
            .to_string()
        );
        Ok(())
    }
}
