mod cli;
mod commands;
mod config;
mod error;
mod github;
mod godot;
mod godot_version;
mod installer;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run().await
}
