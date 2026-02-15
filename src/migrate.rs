//! Utilities for data directory migration between versions of gdenv

use crate::config::Config;
use crate::ui;
use anyhow::{Context, Result};
use semver::Version;
use std::fs;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn migrate() -> Result<()> {
    let new_version = Version::parse(VERSION)?;

    // No need to migrate if the data directory doesn't exist
    if !Config::default().data_dir.exists() {
        write_data_format_version(&new_version)?;
        return Ok(());
    }

    struct Migration {
        to_version: Version,
        migrate_fn: fn() -> Result<()>,
    }
    let migrations: Vec<Migration> = vec![
        Migration {
            to_version: Version::parse("0.2.0")?,
            migrate_fn: v0_1_6_to_v0_2_0::migrate,
        },
        // Add future migrations here
    ];

    let old_version = get_data_format_version();

    if old_version == Some(new_version.clone()) {
        return Ok(());
    }

    ui::info(&format!(
        "Migrating gdenv data directory to latest format. {} -> {}",
        old_version
            .clone()
            .map(|v| v.to_string())
            .unwrap_or("?".to_string()),
        new_version
    ));

    for migration in &migrations {
        // Run the migration if there's no version or if the current version is less than what's required for this step.
        if old_version
            .as_ref()
            .is_none_or(|v| v < &migration.to_version)
        {
            (migration.migrate_fn)()?;
        }
    }

    write_data_format_version(&new_version)?;

    ui::success("Migration successful!");

    Ok(())
}

fn write_data_format_version(version: &Version) -> Result<()> {
    let config = Config::setup()?;
    let data_format_version_file = config.data_dir_format_version_file;
    fs::write(&data_format_version_file, version.to_string()).context(format!(
        "Could not write the data format version file: {}",
        &data_format_version_file.to_str().unwrap_or("?")
    ))
}

fn get_data_format_version() -> Option<Version> {
    let config = Config::setup().ok()?;
    let data_format_version_file = config.data_dir_format_version_file;
    if data_format_version_file.exists() {
        fs::read_to_string(data_format_version_file)
            .ok()?
            .parse()
            .ok()
    } else {
        None
    }
}

mod v0_1_6_to_v0_2_0 {
    use crate::config::Config;
    use crate::godot::godot_installation_name;
    use crate::godot_version::GodotVersion;
    use crate::installer;
    use anyhow::Result;
    use regex::Regex;
    use std::fs;

    pub fn migrate() -> Result<()> {
        migrate_installations_dir()?;
        migrate_symlinks()
    }

    pub fn migrate_installations_dir() -> Result<()> {
        let config = Config::setup()?;
        let installations_dir = &config.installations_dir;

        for entry_result in fs::read_dir(installations_dir)? {
            let entry = entry_result?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let dir_name = entry.file_name();
            let Some(dir_name) = dir_name.to_str() else {
                continue;
            };

            if let Some(expected_dir_name) = try_get_new_name(dir_name) {
                let src = installations_dir.join(dir_name);
                let dst = installations_dir.join(&expected_dir_name);

                // Only attempt a rename if the target doesn't already exist
                if !dst.exists() {
                    fs::rename(&src, &dst)?;
                }
            }
        }
        Ok(())
    }

    fn try_get_new_name(name: &str) -> Option<String> {
        // Only migrate "godot-..." directories/files
        let version_part = name.strip_prefix("godot-")?;

        let (version_str, is_dotnet) = if let Some(v) = version_part.strip_suffix("-dotnet") {
            (v, true)
        } else {
            (version_part, false)
        };

        // Skip unparseable versions
        let version = GodotVersion::new(version_str, is_dotnet).ok()?;

        let expected_name = godot_installation_name(&version);

        if name != expected_name {
            Some(expected_name)
        } else {
            None
        }
    }

    fn migrate_symlinks() -> Result<()> {
        let config = Config::setup()?;
        let bin_dir = &config.bin_dir;
        let godot_symlink = bin_dir.join("godot");

        let Ok(target) = fs::read_link(&godot_symlink) else {
            return Ok(());
        };

        if let Some(target_str) = target.to_str()
            && let Some(version_str) = extract_godot_version(target_str)
        {
            let version = GodotVersion::new(&version_str, false)?;
            // It's possible that the installation directory hasn't been migrated yet if we are in the middle of migrations,
            // but migrate_installations_dir() is called before migrate_simlinks() in migrate().
            // However, Installer::set_active_version checks if the directory exists.
            installer::set_active_version(&config, &version)?;
        }

        Ok(())
    }

    fn extract_godot_version(target: &str) -> Option<String> {
        // This will match a path segment like "/godot-4.2.0/"
        let re = Regex::new(r"/godot-(.*?)/").unwrap();
        re.captures(target)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    }
}
