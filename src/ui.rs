use colored::*;

pub fn success(msg: &str) {
    println!("{} {}", "✓".green(), msg.green());
}

pub fn error(msg: &str) {
    println!("{} {}", "Error:".red(), msg.red());
}

pub fn info(msg: &str) {
    println!("{}", msg);
}

pub fn warning(msg: &str) {
    println!("{}", msg.yellow());
}

pub fn tip(msg: &str) {
    println!("{} {}", "Tip:".cyan(), msg.cyan());
}

pub fn question(msg: &str) {
    println!("{} {}", "[?]".magenta(), msg.magenta());
}
