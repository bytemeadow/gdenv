//! Integration tests that spawn the real `gdenv` binary in the current
//! environment and verify it picks the right markers for the platform.
//!
//! No env var mocking — the binary runs in the same environment as the
//! test runner, and we assert based on what that platform should produce.

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

/// On Unix (macOS / Linux) the default terminal supports emoji,
/// so the binary should emit emoji markers.
#[cfg(not(windows))]
#[test]
fn uses_emoji_on_unix() {
    let output = Command::new(gdenv_bin())
        .arg("current")
        .env("NO_COLOR", "1")
        // Ensure TERM isn't "dumb" which would suppress emoji
        .env("TERM", "xterm-256color")
        .output()
        .expect("failed to run gdenv");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        contains_any(&stdout, EMOJI_MARKERS),
        "Expected emoji markers on Unix, got:\n{stdout}"
    );
    assert!(
        !contains_any(&stdout, ASCII_MARKERS),
        "Should NOT contain ASCII markers on Unix, got:\n{stdout}"
    );
}

/// On Windows CI (and any plain cmd.exe / PowerShell without Windows Terminal),
/// WT_SESSION won't be set, so the binary should fall back to ASCII markers.
///
/// This test only compiles and runs on Windows — it exercises the real
/// platform detection, not a mock.
#[cfg(windows)]
#[test]
fn uses_ascii_on_plain_windows() {
    // We intentionally do NOT set WT_SESSION or TERM_PROGRAM.
    // On a real Windows CI runner (or plain PowerShell) these won't be set,
    // so the binary should detect that and use ASCII.
    let has_wt = std::env::var("WT_SESSION").is_ok();
    let has_vscode = std::env::var("TERM_PROGRAM").is_ok_and(|v| v == "vscode");

    if has_wt || has_vscode {
        // If we're running inside Windows Terminal or VS Code, emoji is
        // the correct behavior — skip this test rather than give a false failure.
        eprintln!("Skipping: running inside Windows Terminal or VS Code");
        return;
    }

    let output = Command::new(gdenv_bin())
        .arg("current")
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run gdenv");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        contains_any(&stdout, ASCII_MARKERS),
        "Expected ASCII markers on plain Windows, got:\n{stdout}"
    );
    assert!(
        !contains_any(&stdout, EMOJI_MARKERS),
        "Should NOT contain emoji on plain Windows, got:\n{stdout}"
    );
}
