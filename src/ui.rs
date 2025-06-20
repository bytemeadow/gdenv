use colored::*;

pub fn success(msg: &str) {
    println!("{} {}", "✅".green(), msg);
}

pub fn error(msg: &str) {
    println!("{} {}", "❌".red(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", "ℹ️".blue(), msg);
}

pub fn warning(msg: &str) {
    println!("{} {}", "⚠️".yellow(), msg);
}
