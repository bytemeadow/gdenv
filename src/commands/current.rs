use anyhow::Result;
use clap::Args;
use std::path::Path;

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
                    
                    // Show executable path info
                    let godot_executable = config.bin_dir.join("godot");
                    if godot_executable.exists() {
                        ui::info(&format!("Executable: {}", godot_executable.display()));
                        show_path_instructions(&config.bin_dir);
                    }
                }
            }
            None => {
                ui::warning("No active Godot version set");
                ui::info("Use 'gdenv install <version>' to install and activate a version");
            }
        }
        
        Ok(())
    }
}

fn show_path_instructions(bin_dir: &Path) {
    ui::info("To use 'godot' from anywhere, add the following to your shell profile:");
    
    #[cfg(target_os = "windows")]
    {
        ui::info(&format!("  set PATH={};%PATH%", bin_dir.display()));
        ui::info("Or add it permanently through System Properties > Environment Variables");
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        let bin_path = bin_dir.display();
        ui::info(&format!("  export PATH=\"{}:$PATH\"", bin_path));
        ui::info("");
        ui::info("To add it to your shell profile, run:");
        
        // Detect common shells and show appropriate file
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("zsh") {
                ui::info(&format!("  echo 'export PATH=\"{}:$PATH\"' >> ~/.zshrc", bin_path));
                ui::info("Then restart your shell or run: source ~/.zshrc");
            } else if shell.contains("bash") {
                ui::info(&format!("  echo 'export PATH=\"{}:$PATH\"' >> ~/.bashrc", bin_path));
                ui::info("Then restart your shell or run: source ~/.bashrc");
            } else if shell.contains("fish") {
                ui::info(&format!("  fish_add_path {}", bin_path));
                ui::info("Then restart your shell");
            } else {
                ui::info(&format!("  echo 'export PATH=\"{}:$PATH\"' >> ~/.bashrc  # or ~/.zshrc", bin_path));
                ui::info("Then restart your shell or run: source ~/.bashrc");
            }
        } else {
            ui::info(&format!("  echo 'export PATH=\"{}:$PATH\"' >> ~/.bashrc  # or ~/.zshrc", bin_path));
            ui::info("Then restart your shell or run: source ~/.bashrc");
        }
    }
}