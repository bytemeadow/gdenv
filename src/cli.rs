use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    cache::CacheCommand, current::CurrentCommand, install::InstallCommand,
    installed::InstalledCommand, list::ListCommand, uninstall::UninstallCommand,
    update::UpdateCommand, use_cmd::UseCommand,
};

#[derive(Parser)]
#[command(name = "gdenv")]
#[command(about = "A beautiful terminal tool for managing Godot installations")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Update the cache of available Godot versions
    #[command(alias = "update")]
    Fetch(UpdateCommand),

    /// List available Godot versions
    #[command(alias = "ls")]
    List(ListCommand),

    /// Download and install a specific version of Godot
    Install(InstallCommand),

    /// List installed Godot versions
    Installed(InstalledCommand),

    /// Switch to a specific Godot version
    Use(UseCommand),

    /// Show the currently active Godot version
    Current(CurrentCommand),

    /// Uninstall a specific Godot version
    #[command(alias = "remove")]
    Uninstall(UninstallCommand),

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
            Commands::Fetch(cmd) => cmd.run().await,
            Commands::Cache(cmd) => cmd.run().await,
        }
    }
}
