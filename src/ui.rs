use colored::*;
use std::io::{self, IsTerminal};

#[derive(Debug, Clone, Copy)]
struct TerminalCapabilities {
    color: bool,
    unicode: bool,
    emoji: bool,
}

fn detect_terminal_capabilities(is_terminal: bool, is_windows: bool) -> TerminalCapabilities {
    detect_terminal_capabilities_with_env(is_terminal, is_windows, |key| {
        std::env::var_os(key).and_then(|value| value.into_string().ok())
    })
}

fn detect_terminal_capabilities_with_env(
    is_terminal: bool,
    is_windows: bool,
    get_env: impl Fn(&str) -> Option<String>,
) -> TerminalCapabilities {
    let no_color = get_env("NO_COLOR").is_some();
    let force_color = get_env("CLICOLOR_FORCE").is_some_and(|v| v != "0");
    let color = if force_color {
        true
    } else if no_color {
        false
    } else {
        is_terminal
    };

    let locale_supports_unicode = ["LC_ALL", "LC_CTYPE", "LANG"]
        .into_iter()
        .filter_map(&get_env)
        .any(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("utf-8") || value.contains("utf8")
        });

    let windows_unicode_hints = ["WT_SESSION", "ANSICON", "ConEmuANSI"]
        .into_iter()
        .any(|key| get_env(key).is_some())
        || get_env("TERM").is_some_and(|term| term.to_ascii_lowercase().contains("xterm"));

    let unicode = if is_windows {
        locale_supports_unicode || windows_unicode_hints
    } else {
        locale_supports_unicode
    };

    TerminalCapabilities {
        color,
        unicode,
        emoji: unicode,
    }
}

fn format_marker(
    emoji: &str,
    unicode: &str,
    ascii: &str,
    style: impl Fn(&str) -> ColoredString,
) -> String {
    let caps = detect_terminal_capabilities(io::stdout().is_terminal(), cfg!(windows));
    let marker = if caps.emoji {
        emoji
    } else if caps.unicode {
        unicode
    } else {
        ascii
    };

    if caps.color {
        style(marker).to_string()
    } else {
        marker.to_string()
    }
}

pub fn success(msg: &str) {
    println!(
        "{} {}",
        format_marker("✅", "✓", "[OK]", |marker| marker.green()),
        msg
    );
}

pub fn error(msg: &str) {
    println!(
        "{} {}",
        format_marker("❌", "✗", "[X]", |marker| marker.red()),
        msg
    );
}

pub fn info(msg: &str) {
    println!(
        "{} {}",
        format_marker("ℹ️", "ℹ", "[i]", |marker| marker.blue()),
        msg
    );
}

pub fn warning(msg: &str) {
    println!(
        "{} {}",
        format_marker("⚠️", "⚠", "[!]", |marker| marker.yellow()),
        msg
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn detects_windows_unicode_when_terminal_hint_present() {
        let mut env = HashMap::new();
        env.insert("WT_SESSION", "1");
        let caps = detect_terminal_capabilities_with_env(true, true, |key| {
            env.get(key).map(ToString::to_string)
        });

        assert!(caps.unicode);
        assert!(caps.emoji);
    }

    #[test]
    fn detects_ascii_fallback_without_unicode_support() {
        let mut env = HashMap::new();
        env.insert("LANG", "C");
        let caps = detect_terminal_capabilities_with_env(true, true, |key| {
            env.get(key).map(ToString::to_string)
        });

        assert!(caps.color);
        assert!(!caps.unicode);
        assert!(!caps.emoji);
    }

    #[test]
    fn disables_color_for_non_terminal_output() {
        let caps = detect_terminal_capabilities_with_env(false, false, |_| None);

        assert!(!caps.color);
        assert!(!caps.unicode);
        assert!(!caps.emoji);
    }
}
