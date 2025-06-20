use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    install::InstallCommand,
    list::ListCommand, 
    installed::InstalledCommand,
    use_cmd::UseCommand,
    uninstall::UninstallCommand,
    current::CurrentCommand,
    update::UpdateCommand,
    cache::CacheCommand,
};

#[derive(Parser)]
#[command(name = "gdm")]
#[command(about = "A beautiful terminal tool for managing Godot installations")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download and install a specific version of Godot
    Install(InstallCommand),
    
    /// List available Godot versions from remote
    List(ListCommand),
    
    /// List installed Godot versions
    Installed(InstalledCommand),
    
    /// Switch to a specific Godot version
    Use(UseCommand),
    
    /// Uninstall a specific Godot version
    Uninstall(UninstallCommand),
    
    /// Show the currently active Godot version
    Current(CurrentCommand),
    
    /// Update the cache of available Godot versions
    Update(UpdateCommand),
    
    /// Manage download cache
    Cache(CacheCommand),
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Install(cmd) => cmd.run().await,
            Commands::List(cmd) => cmd.run().await,
            Commands::Installed(cmd) => cmd.run().await,
            Commands::Use(cmd) => cmd.run().await,
            Commands::Uninstall(cmd) => cmd.run().await,
            Commands::Current(cmd) => cmd.run().await,
            Commands::Update(cmd) => cmd.run().await,
            Commands::Cache(cmd) => cmd.run().await,
        }
    }
}