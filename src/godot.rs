use crate::godot_version::GodotVersion;

/// Get the platform suffix for the current OS and architecture
pub fn platform_suffix() -> &'static str {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("windows", "x86_64") => "win64.exe",
        ("windows", "x86") => "win32.exe",
        ("macos", _) => "macos.universal", // macOS universal binaries work on both Intel and Apple Silicon
        ("linux", "x86_64") => "linux.x86_64",
        ("linux", "x86") => "linux.x86_32",
        ("linux", "arm") => "linux.arm32",
        ("linux", "aarch64") => "linux.arm64",
        // Fallbacks for common cases
        ("windows", _) => "win64.exe",  // Default to 64-bit on Windows
        ("linux", _) => "linux.x86_64", // Default to x86_64 on Linux
        _ => "linux.x86_64",            // Ultimate fallback
    }
}

/// Get the expected executable path within the extracted directory
pub fn godot_executable_path(version: &GodotVersion) -> String {
    let os = std::env::consts::OS;

    match os {
        "macos" => {
            if version.is_dotnet {
                "Godot_mono.app/Contents/MacOS/Godot".to_string()
            } else {
                "Godot.app/Contents/MacOS/Godot".to_string()
            }
        }
        "windows" => {
            let version_part = version.as_str();
            if version.is_dotnet {
                format!(
                    "Godot_v{}_mono_{}/Godot_v{}_mono_{}.exe",
                    version_part, "win64", version_part, "win64"
                )
            } else {
                format!("Godot_v{}_{}.exe", version_part, "win64")
            }
        }
        "linux" => {
            let version_part = version.as_str();
            let platform_suffix = platform_suffix();
            if version.is_dotnet {
                // Dotnet versions extract to a subfolder
                let folder_name = format!("Godot_v{version_part}_mono_{platform_suffix}");
                let exe_name = format!("Godot_v{version_part}_mono_{platform_suffix}");
                format!("{folder_name}/{exe_name}")
            } else {
                // Non-dotnet versions extract directly
                format!("Godot_v{version_part}_{platform_suffix}")
            }
        }
        _ => {
            // Fallback - just look for Godot executable
            "Godot".to_string()
        }
    }
}

pub fn godot_installation_name(version: &GodotVersion) -> String {
    if version.is_dotnet {
        format!("godot-{}-dotnet", version.as_str())
    } else {
        format!("godot-{}", version.as_str())
    }
}

#[allow(dead_code)]
pub fn godot_archive_name(version: &GodotVersion) -> String {
    let platform_suffix = platform_suffix();
    let version_part = version.as_str();

    if version.is_dotnet {
        format!("Godot_v{version_part}_mono_{platform_suffix}.zip")
    } else {
        format!("Godot_v{version_part}_{platform_suffix}.zip")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_names() {
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        let archive = godot_archive_name(&v1);
        assert!(archive.contains("Godot_v4.2.1-stable_"));
        assert!(archive.ends_with(".zip"));

        let v2 = GodotVersion::new("4.3.0-beta2", true).unwrap();
        let archive = godot_archive_name(&v2);
        assert!(archive.contains("Godot_v4.3.0-beta2_mono_"));
        assert!(archive.ends_with(".zip"));
    }

    #[test]
    fn test_platform_suffix_detection() {
        // Test that we get a valid platform suffix (this tests the current system)
        let suffix = platform_suffix();
        assert!(!suffix.is_empty());

        // Should be one of the expected patterns
        let valid_suffixes = [
            "win64.exe",
            "win32.exe",
            "macos.universal",
            "linux.x86_64",
            "linux.x86_32",
            "linux.arm32",
            "linux.arm64",
        ];
        assert!(
            valid_suffixes.contains(&suffix),
            "Got unexpected suffix: {suffix}"
        );
    }

    #[test]
    fn test_executable_path_construction() {
        // Test that we can construct executable paths
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        let exe_path = godot_executable_path(&v1);
        assert!(!exe_path.is_empty());

        let v2 = GodotVersion::new("4.2.1", true).unwrap();
        let dotnet_exe_path = godot_executable_path(&v2);
        assert!(!dotnet_exe_path.is_empty());

        // Paths should be different for dotnet vs non-dotnet
        assert_ne!(exe_path, dotnet_exe_path);
    }

    #[test]
    fn test_installation_name() {
        // Test .NET versions
        let v5 = GodotVersion::new("4.2.1", true).unwrap();
        assert_eq!(v5.to_string(), "4.2.1-stable (.NET)");
        assert_eq!(godot_installation_name(&v5), "godot-4.2.1-stable-dotnet");
    }
}
