use anyhow::Result;
use clap::Args;

use crate::{
    config::Config,
    installer::Installer,
    ui,
};

#[derive(Args)]
pub struct CurrentCommand {
    /// Show the path to the current Godot executable
    #[arg(long, short)]
    pub path: bool,
}

impl CurrentCommand {
    pub async fn run(self) -> Result<()> {
        let config = Config::new()?;
        let installer = Installer::new(config.clone());
        
        match installer.get_active_version()? {
            Some(version) => {
                if self.path {
                    println!("{}", config.active_symlink.display());
                } else {
                    ui::success(&format!("Active Godot version: {}", version));
                    ui::info(&format!("Location: {}", config.active_symlink.display()));
                }
            }
            None => {
                ui::warning("No active Godot version set");
                ui::info("Use 'gdm install <version>' to install and activate a version");
            }
        }
        
        Ok(())
    }
}