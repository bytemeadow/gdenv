use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

pub fn progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}

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