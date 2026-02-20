use anyhow::bail;
use std::fs;
use std::path::Path;

pub fn read_godot_version_file() -> anyhow::Result<String> {
    let version_file = Path::new(".godot-version");

    if !version_file.exists() {
        bail!(
            "No version specified and no .godot-version file found in current directory.\n\
                Create a .godot-version file or specify a version: gdenv use <version>"
        );
    }

    let content = fs::read_to_string(version_file)?;
    let version = content.trim();

    if version.is_empty() {
        bail!(".godot-version file is empty");
    }

    tracing::info!("Reading version from .godot-version: {version}");

    Ok(version.to_string())
}
