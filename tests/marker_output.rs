//! Integration tests that spawn the real `gdenv` binary and verify
//! it produces the correct markers based on actual terminal environment
//! signals (TERM, WT_SESSION, etc.) — not mock flags.

use std::process::Command;

fn gdenv_bin() -> String {
    let mut path = std::env::current_exe().expect("current_exe");
    path.pop(); // remove test binary name
    path.pop(); // remove `deps/`
    path.push("gdenv");

    #[cfg(windows)]
    {
        path.set_extension("exe");
    }

    path.to_string_lossy().to_string()
}

const ASCII_MARKERS: &[&str] = &["[ok]", "[error]", "[info]", "[warn]"];
const EMOJI_MARKERS: &[&str] = &["✅", "❌", "ℹ️", "⚠️"];

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Build a Command with a clean env (only PATH preserved) so we fully
/// control which terminal signals the subprocess sees.
fn clean_gdenv_cmd() -> Command {
    let mut cmd = Command::new(gdenv_bin());
    cmd.env_clear();

    // Preserve PATH so the binary can find system libs / itself
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }

    // On Windows, SYSTEMROOT / USERPROFILE are needed for basic operation
    #[cfg(windows)]
    {
        if let Ok(v) = std::env::var("SYSTEMROOT") {
            cmd.env("SYSTEMROOT", v);
        }
        if let Ok(v) = std::env::var("USERPROFILE") {
            cmd.env("USERPROFILE", v);
        }
        if let Ok(v) = std::env::var("LOCALAPPDATA") {
            cmd.env("LOCALAPPDATA", v);
        }
        if let Ok(v) = std::env::var("APPDATA") {
            cmd.env("APPDATA", v);
        }
    }

    // On Unix, HOME is needed for config dirs
    #[cfg(not(windows))]
    {
        if let Ok(v) = std::env::var("HOME") {
            cmd.env("HOME", v);
        }
    }

    cmd.env("NO_COLOR", "1");
    cmd
}

#[test]
fn dumb_terminal_uses_ascii_markers() {
    let output = clean_gdenv_cmd()
        .arg("current")
        .env("TERM", "dumb")
        .output()
        .expect("failed to run gdenv");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        contains_any(&stdout, ASCII_MARKERS),
        "Expected ASCII markers with TERM=dumb, got:\n{stdout}"
    );
    assert!(
        !contains_any(&stdout, EMOJI_MARKERS),
        "Should NOT contain emoji with TERM=dumb, got:\n{stdout}"
    );
}

#[cfg(not(windows))]
#[test]
fn unix_normal_terminal_uses_emoji() {
    let output = clean_gdenv_cmd()
        .arg("current")
        .env("TERM", "xterm-256color")
        .output()
        .expect("failed to run gdenv");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        contains_any(&stdout, EMOJI_MARKERS),
        "Expected emoji markers on Unix with TERM=xterm-256color, got:\n{stdout}"
    );
    assert!(
        !contains_any(&stdout, ASCII_MARKERS),
        "Should NOT contain ASCII markers on Unix with TERM=xterm-256color, got:\n{stdout}"
    );
}

#[cfg(windows)]
#[test]
fn windows_plain_powershell_uses_ascii() {
    // No WT_SESSION, no TERM_PROGRAM → plain PowerShell / cmd.exe
    let output = clean_gdenv_cmd()
        .arg("current")
        .output()
        .expect("failed to run gdenv");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        contains_any(&stdout, ASCII_MARKERS),
        "Expected ASCII markers on plain Windows shell, got:\n{stdout}"
    );
    assert!(
        !contains_any(&stdout, EMOJI_MARKERS),
        "Should NOT contain emoji on plain Windows shell, got:\n{stdout}"
    );
}

#[cfg(windows)]
#[test]
fn windows_terminal_uses_emoji() {
    // WT_SESSION signals Windows Terminal which supports emoji
    let output = clean_gdenv_cmd()
        .arg("current")
        .env("WT_SESSION", "test-session-guid")
        .output()
        .expect("failed to run gdenv");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        contains_any(&stdout, EMOJI_MARKERS),
        "Expected emoji markers with WT_SESSION set, got:\n{stdout}"
    );
    assert!(
        !contains_any(&stdout, ASCII_MARKERS),
        "Should NOT contain ASCII markers with WT_SESSION set, got:\n{stdout}"
    );
}

#[cfg(windows)]
#[test]
fn windows_vscode_terminal_uses_emoji() {
    let output = clean_gdenv_cmd()
        .arg("current")
        .env("TERM_PROGRAM", "vscode")
        .output()
        .expect("failed to run gdenv");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        contains_any(&stdout, EMOJI_MARKERS),
        "Expected emoji markers with TERM_PROGRAM=vscode, got:\n{stdout}"
    );
    assert!(
        !contains_any(&stdout, ASCII_MARKERS),
        "Should NOT contain ASCII markers with TERM_PROGRAM=vscode, got:\n{stdout}"
    );
}
