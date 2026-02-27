#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod cli;
pub mod lifecycle;
pub mod restate_oya;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli::dispatch_command(cli.command).await
}

#[cfg(test)]
mod main_tests;
