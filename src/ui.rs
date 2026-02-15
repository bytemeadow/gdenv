use colored::*;

// ---------------------------------------------------------------------------
// Marker kind – each variant has an emoji and an ASCII representation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    Success,
    Error,
    Info,
    Warning,
    Robot,
    Download,
    Package,
    Star,
}

impl Marker {
    /// Returns the emoji string for this marker.
    pub const fn emoji(self) -> &'static str {
        match self {
            Marker::Success => "✅",
            Marker::Error => "❌",
            Marker::Info => "ℹ️",
            Marker::Warning => "⚠️",
            Marker::Robot => "🤖",
            Marker::Download => "🔽",
            Marker::Package => "📦",
            Marker::Star => "★",
        }
    }

    /// Returns the plain-ASCII fallback for this marker.
    pub const fn ascii(self) -> &'static str {
        match self {
            Marker::Success => "[ok]",
            Marker::Error => "[error]",
            Marker::Info => "[info]",
            Marker::Warning => "[warn]",
            Marker::Robot => "==>",
            Marker::Download => "--",
            Marker::Package => "::",
            Marker::Star => "*",
        }
    }

    /// Picks the right representation based on `emoji_enabled`.
    pub const fn render(self, emoji_enabled: bool) -> &'static str {
        if emoji_enabled {
            self.emoji()
        } else {
            self.ascii()
        }
    }

    /// Picks the right representation using runtime terminal detection.
    pub fn auto(self) -> &'static str {
        self.render(emoji_supported())
    }
}

// ---------------------------------------------------------------------------
// Terminal capability detection
// ---------------------------------------------------------------------------

/// Reads environment variables and delegates to the pure detection logic.
pub fn emoji_supported() -> bool {
    detect_emoji(
        std::env::var("TERM").ok().as_deref(),
        std::env::var("WT_SESSION").ok().as_deref(),
        std::env::var("TERM_PROGRAM").ok().as_deref(),
    )
}

/// Pure detection logic — no env var reads, fully testable.
///
/// Rules:
/// 1. `TERM=dumb` → off (universally means "no fancy output")
/// 2. On Windows: only on inside Windows Terminal (`WT_SESSION` set)
///    or VS Code integrated terminal (`TERM_PROGRAM=vscode`).
///    Classic cmd.exe / PowerShell don't support emoji.
/// 3. On macOS / Linux: on by default
fn detect_emoji(term: Option<&str>, wt_session: Option<&str>, term_program: Option<&str>) -> bool {
    if term == Some("dumb") {
        return false;
    }

    #[cfg(windows)]
    {
        wt_session.is_some() || term_program == Some("vscode")
    }

    #[cfg(not(windows))]
    {
        let _ = (wt_session, term_program);
        true
    }
}

// ---------------------------------------------------------------------------
// Convenience printers
// ---------------------------------------------------------------------------

pub fn success(msg: &str) {
    println!("{} {}", "[✓]".green(), msg);
}

pub fn error(msg: &str) {
    println!("{} {}", "[E]".red(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", "[ ]".blue(), msg);
}

pub fn warning(msg: &str) {
    println!("{} {}", "[W]".yellow(), msg);
}

pub fn helpful(msg: &str) {
    println!("{} {}", "[i]".cyan(), msg);
}

pub fn question(msg: &str) {
    println!("{} {}", "[?]".magenta(), msg);
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Marker rendering tests ──

    #[test]
    fn render_picks_correct_variant() {
        for marker in [
            Marker::Success,
            Marker::Error,
            Marker::Info,
            Marker::Warning,
            Marker::Robot,
            Marker::Download,
            Marker::Package,
            Marker::Star,
        ] {
            assert_eq!(marker.render(true), marker.emoji());
            assert_eq!(marker.render(false), marker.ascii());
        }
    }

    #[test]
    fn ascii_markers_are_pure_ascii() {
        for marker in [
            Marker::Success,
            Marker::Error,
            Marker::Info,
            Marker::Warning,
            Marker::Robot,
            Marker::Download,
            Marker::Package,
            Marker::Star,
        ] {
            assert!(
                marker.ascii().is_ascii(),
                "Marker::{:?} ascii fallback contains non-ASCII: {}",
                marker,
                marker.ascii()
            );
        }
    }

    #[test]
    fn emoji_markers_contain_non_ascii() {
        for marker in [
            Marker::Success,
            Marker::Error,
            Marker::Info,
            Marker::Warning,
            Marker::Robot,
            Marker::Download,
            Marker::Package,
            Marker::Star,
        ] {
            assert!(
                !marker.emoji().is_ascii(),
                "Marker::{:?} emoji variant should contain non-ASCII",
                marker,
            );
        }
    }

    // ── Detection logic tests ──
    // These call the pure `detect_emoji` function directly so they don't
    // touch process-level env vars and are safe to run in parallel.

    #[test]
    fn dumb_terminal_disables_emoji() {
        assert!(!detect_emoji(Some("dumb"), None, None));
        assert!(!detect_emoji(Some("dumb"), Some("session-id"), None));
        assert!(!detect_emoji(Some("dumb"), None, Some("vscode")));
    }

    #[test]
    fn normal_term_does_not_block_emoji() {
        // On non-windows, any non-dumb TERM should still allow emoji.
        // On windows, it depends on WT_SESSION / TERM_PROGRAM.
        #[cfg(not(windows))]
        {
            assert!(detect_emoji(Some("xterm-256color"), None, None));
            assert!(detect_emoji(None, None, None));
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_defaults_to_emoji() {
        assert!(detect_emoji(None, None, None));
        assert!(detect_emoji(Some("xterm-256color"), None, None));
        assert!(detect_emoji(Some("screen"), None, None));
    }

    #[cfg(windows)]
    #[test]
    fn windows_plain_powershell_no_emoji() {
        assert!(!detect_emoji(None, None, None));
        assert!(!detect_emoji(Some("xterm"), None, None));
    }

    #[cfg(windows)]
    #[test]
    fn windows_terminal_has_emoji() {
        assert!(detect_emoji(None, Some("some-guid"), None));
    }

    #[cfg(windows)]
    #[test]
    fn windows_vscode_terminal_has_emoji() {
        assert!(detect_emoji(None, None, Some("vscode")));
    }
}
