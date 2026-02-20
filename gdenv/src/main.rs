mod cli;
mod commands;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use gdenv_lib::logging::initialize_logging;

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging();
    let cli = Cli::parse();
    cli.run().await
}
