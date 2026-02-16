use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::run::RunCommand;
use crate::commands::{
    godot::cache::CacheCommand, godot::current::CurrentCommand, godot::fetch::FetchCommand,
    godot::install::InstallCommand, godot::list::ListCommand, godot::uninstall::UninstallCommand,
    godot::use_cmd::UseCommand,
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
    /// Manage Godot versions
    #[command(subcommand)]
    Godot(GodotCommands),

    /// Invoke a specific Godot version. Automatically installs the version. Will not affect the active version.
    Run(RunCommand),
}

#[derive(Subcommand)]
pub enum GodotCommands {
    /// Update the cache of available Godot versions
    #[command(alias = "update")]
    Fetch(FetchCommand),

    /// List installed and available Godot versions
    #[command(alias = "ls")]
    List(ListCommand),

    /// Download and install a specific version of Godot
    Install(InstallCommand),

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
            Commands::Godot(godot_command) => match godot_command {
                GodotCommands::Fetch(cmd) => cmd.run().await,
                GodotCommands::List(cmd) => cmd.run().await,
                GodotCommands::Install(cmd) => cmd.run().await,
                GodotCommands::Use(cmd) => cmd.run().await,
                GodotCommands::Current(cmd) => cmd.run().await,
                GodotCommands::Uninstall(cmd) => cmd.run().await,
                GodotCommands::Cache(cmd) => cmd.run().await,
            },
            Commands::Run(cmd) => cmd.run().await,
        }
    }
}
