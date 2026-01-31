//! Juggernaut SDLC Factory CLI entrypoint.

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

use clap::Parser;

#[derive(Parser)]
#[command(name = "juggernaut")]
#[command(about = "Juggernaut SDLC Factory - 100x developer throughput")]
#[command(version)]
struct Cli {
    /// Output as JSON (AI-native mode)
    #[arg(long, default_value = "false")]
    json: bool,
}

fn main() {
    let _cli = Cli::parse();

    // TODO: Implement subcommands
    println!("Juggernaut SDLC Factory v0.1.0");
    println!("Ready to build.");
}
