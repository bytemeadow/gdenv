mod cli;
mod commands;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use gdenv_lib::logging::initialize_logging;
use gdenv_lib::migrate::migrate;

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging();
    migrate().context("Failed to migrate data directory")?;
    let cli = Cli::parse();
    cli.run().await
}
