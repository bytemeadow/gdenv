mod cli;
mod commands;
mod config;
mod error;
mod github;
mod godot;
mod godot_version;
mod installer;
mod migrate;
mod project_specification;
mod ui;

use crate::migrate::migrate;
use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    migrate().context("Failed to migrate data directory")?;
    let cli = Cli::parse();
    cli.run().await
}
