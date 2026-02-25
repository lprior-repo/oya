#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod restate_oia;

use clap::{Parser, Subcommand};
use reqwest::Client;
use restate_oia::{StartRequest, StartResponse};
use std::net::SocketAddr;

const DEFAULT_BIND: &str = "127.0.0.1:9080";
const DEFAULT_INGRESS: &str = "http://127.0.0.1:8080";

#[derive(Debug, Parser)]
#[command(name = "oya")]
#[command(about = "OIA -> Restate -> OpenCode bridge")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve(ServeArgs),
    Invoke(InvokeArgs),
}

#[derive(Debug, clap::Args)]
struct ServeArgs {
    #[arg(long, default_value = DEFAULT_BIND)]
    bind: String,
}

#[derive(Debug, clap::Args)]
struct InvokeArgs {
    #[arg(long, default_value = DEFAULT_INGRESS)]
    ingress: String,
    #[arg(long, default_value = "default")]
    id: String,
    #[arg(long)]
    prompt: String,
    #[arg(long)]
    model: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve(args) => serve_command(args).await,
        Command::Invoke(args) => invoke_command(args).await,
    }
}

async fn serve_command(args: ServeArgs) -> anyhow::Result<()> {
    let bind = parse_socket_addr(args.bind)?;
    restate_oia::serve(bind).await
}

async fn invoke_command(args: InvokeArgs) -> anyhow::Result<()> {
    let request = StartRequest { prompt: args.prompt, model: args.model };
    let url = format!("{}/Oia/{}/start", args.ingress, args.id);
    let response = Client::new().post(url).json(&request).send().await?;
    let response = response.error_for_status()?;
    let body: StartResponse = response.json().await?;
    println!("{}", body.output);
    Ok(())
}

fn parse_socket_addr(value: String) -> anyhow::Result<SocketAddr> {
    value
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("invalid --bind '{}': {error}", value))
}
