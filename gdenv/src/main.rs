mod cli;
mod commands;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use gdenv_lib::migrate::migrate;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    // set up logging
    tracing_subscriber::registry()
        .with(
            fmt::layer() // Only output logged text without metadata
                .without_time()
                .with_target(false)
                .with_level(false)
                .compact(),
        )
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    migrate().context("Failed to migrate data directory")?;
    let cli = Cli::parse();
    cli.run().await
}
