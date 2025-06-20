use anyhow::Result;
use clap::Args;
use std::fs;

use crate::{
    config::Config,
    ui,
};

#[derive(Args)]
pub struct CacheCommand {
    /// Clear all cached downloads
    #[arg(long, short)]
    pub clear: bool,
    
    /// Show cache size and location
    #[arg(long, short)]
    pub info: bool,
}

impl CacheCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        
        if self.clear {
            self.clear_cache(&config)?;
        } else if self.info {
            self.show_cache_info(&config)?;
        } else {
            // Default to showing cache info
            self.show_cache_info(&config)?;
        }
        
        Ok(())
    }
    
    fn clear_cache(&self, config: &Config) -> Result<()> {
        if !config.cache_dir.exists() {
            ui::info("Cache directory does not exist - nothing to clear");
            return Ok(());
        }
        
        let cache_size = self.calculate_cache_size(config)?;
        
        if cache_size == 0 {
            ui::info("Cache is already empty");
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
            ui::info(&format!("Cached files: {}", file_count));
            ui::info("Run 'gdm cache --clear' to free up space");
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