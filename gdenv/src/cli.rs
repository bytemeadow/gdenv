use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

    #[command(flatten)]
    pub global_args: GlobalArgs,
}

#[derive(clap::Args, Clone)]
pub struct GlobalArgs {
    /// Path to a gdenv managed project (defaults to current directory)
    #[arg(short, long, global = true)]
    pub project: Option<PathBuf>,

    /// Use a different location for gdenv's data, where downloads and installations are kept (useful for testing)
    #[arg(long, global = true)]
    pub datadir: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Invoke Godot for the current project
    Run(RunCommand),

    /// Manage Godot versions
    #[command(subcommand)]
    Godot(GodotCommands),
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
                GodotCommands::Fetch(cmd) => cmd.run(self.global_args).await,
                GodotCommands::List(cmd) => cmd.run(self.global_args).await,
                GodotCommands::Install(cmd) => cmd.run(self.global_args).await,
                GodotCommands::Use(cmd) => cmd.run(self.global_args).await,
                GodotCommands::Current(cmd) => cmd.run(self.global_args).await,
                GodotCommands::Uninstall(cmd) => cmd.run(self.global_args).await,
                GodotCommands::Cache(cmd) => cmd.run(self.global_args).await,
            },
            Commands::Run(cmd) => cmd.run(self.global_args).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::Cli;
    use clap::CommandFactory;

    #[test]
    fn test_cli() {
        Cli::command().debug_assert();
    }
}
