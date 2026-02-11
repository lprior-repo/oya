use oya_web::{ServerConfig, create_router};
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = ServerConfig::default();
    let bind_address: SocketAddr = config.bind_address.parse()?;

    info!("Starting oya-web server on {}", bind_address);
    info!("CORS origin: {}", config.cors_origin);

    let router = create_router(config)?;
    let listener = tokio::net::TcpListener::bind(bind_address).await?;

    axum::serve(listener, router).await?;

    Ok(())
}
