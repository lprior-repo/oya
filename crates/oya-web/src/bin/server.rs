//! OYA Web Server Binary
//!
//! Standalone binary to run the OYA web server with Tower middleware.

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,oya_web=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Server address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("Starting OYA Web Server");
    oya_web::server::run_server(addr).await?;

    Ok(())
}
