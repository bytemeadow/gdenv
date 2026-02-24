use colored::*;

pub fn success(msg: &str) {
    tracing::info!("{} {}", "✓".green(), msg.green());
}

pub fn error(msg: &str) {
    tracing::info!("{} {}", "Error:".red(), msg.red());
}

pub fn info(msg: &str) {
    tracing::info!("{}", msg);
}

pub fn warning(msg: &str) {
    tracing::info!("{}", msg.yellow());
}

pub fn tip(msg: &str) {
    tracing::info!("{} {}", "Tip:".dimmed(), msg.dimmed());
}

pub fn question(msg: &str) {
    tracing::info!("{} {}", "[?]".magenta(), msg.magenta());
}
