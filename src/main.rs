//! # OYA - Main CLI Entry Point

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

use anyhow::Result;
use tracing::error;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod commands;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing_from_cli(&cli);

    match commands::execute_command(cli.command).await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn init_tracing_from_cli(cli: &Cli) {
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else if cli.quiet {
        EnvFilter::new("warn")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
