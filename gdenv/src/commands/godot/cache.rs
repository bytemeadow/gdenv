use crate::cli::GlobalArgs;
use crate::ui;
use anyhow::Result;
use clap::{Args, Subcommand};
use gdenv_lib::config::Config;
use std::fs;

#[derive(Args)]
pub struct CacheCommand {
    #[command(subcommand)]
    pub action: Option<CacheAction>,
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Show cache size and location
    Info,
    /// Clear all cached downloads
    Clear,
}

impl CacheCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;

        match self.action {
            Some(CacheAction::Clear) => self.clear_cache(&config)?,
            Some(CacheAction::Info) => self.show_cache_info(&config)?,
            None => {
                // Default to showing cache info
                self.show_cache_info(&config)?;
            }
        }

        Ok(())
    }

    fn clear_cache(&self, config: &Config) -> Result<()> {
        if !config.cache_dir.exists() {
            ui::success("Cache directory does not exist - nothing to clear");
            return Ok(());
        }

        let cache_size = self.calculate_cache_size(config)?;

        if cache_size == 0 {
            ui::success("Cache is already empty");
            return Ok(());
        }

        ui::info(&format!("Clearing cache ({})...", format_size(cache_size)));

        // Remove all files in cache directory
        for entry in fs::read_dir(&config.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(&path)?;
            }
        }

        ui::success("Cache cleared successfully");

        Ok(())
    }

    fn show_cache_info(&self, config: &Config) -> Result<()> {
        ui::info(&format!("Cache location: {}", config.cache_dir.display()));

        if !config.cache_dir.exists() {
            ui::info("Cache directory does not exist");
            return Ok(());
        }

        let cache_size = self.calculate_cache_size(config)?;
        let file_count = self.count_cache_files(config)?;

        if cache_size == 0 {
            ui::info("Cache is empty");
        } else {
            ui::info(&format!("Cache size: {}", format_size(cache_size)));
            ui::info(&format!("Cached files: {file_count}"));
            ui::tip("Run `gdenv godot cache clear` to free up space");
        }

        Ok(())
    }

    fn calculate_cache_size(&self, config: &Config) -> Result<u64> {
        let mut total_size = 0;

        if !config.cache_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&config.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                total_size += fs::metadata(&path)?.len();
            }
        }

        Ok(total_size)
    }

    fn count_cache_files(&self, config: &Config) -> Result<usize> {
        let mut count = 0;

        if !config.cache_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&config.cache_dir)? {
            let entry = entry?;
            if entry.path().is_file() {
                count += 1;
            }
        }

        Ok(count)
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
