use crate::ui;

pub fn read_godot_version_file() -> anyhow::Result<String> {
    use anyhow::anyhow;
    use std::fs;
    use std::path::Path;

    let version_file = Path::new(".godot-version");

    if !version_file.exists() {
        return Err(anyhow!(
                "No version specified and no .godot-version file found in current directory.\n\
                Create a .godot-version file or specify a version: gdenv use <version>"
            ));
    }

    let content = fs::read_to_string(version_file)?;
    let version = content.trim();

    if version.is_empty() {
        return Err(anyhow!(".godot-version file is empty"));
    }

    ui::info(&format!("Reading version from .godot-version: {version}"));

    Ok(version.to_string())
}