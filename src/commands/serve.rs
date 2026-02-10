//! Serve command - starts the IPC server in background

use std::net::TcpStream;
use std::time::Duration;

use anyhow::Result;
use orchestrator::actors::{IpcWorkerActorDef, IpcWorkerArguments, IpcWorkerMessage};
use orchestrator::ipc_messages::{GuestMessage, HostMessage};
use oya_ipc::{IpcTransport, TransportError};
use ractor::{Actor, ActorRef};
use tokio::net::TcpListener;
use tracing::{info, warn};

const DEFAULT_ADDR: &str = "127.0.0.1:5555";

/// Start the IPC server
pub fn serve_command(address: Option<String>) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("Failed to create runtime: {e}"))?;

    runtime
        .block_on(async { run_server(address.unwrap_or_else(|| DEFAULT_ADDR.to_string())).await })
}

async fn run_server(address: String) -> Result<()> {
    let (ipc_worker, _handle) = Actor::spawn(None, IpcWorkerActorDef, IpcWorkerArguments::new())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to spawn IPC worker: {e:?}"))?;

    let listener = TcpListener::bind(&address)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {address}: {e}"))?;

    info!("🔌 OYA IPC server listening on {}", address);

    loop {
        let (stream, addr) = listener
            .accept()
            .await
            .map_err(|e| anyhow::anyhow!("Accept failed: {e}"))?;

        let ipc_worker = ipc_worker.clone();
        info!("Accepted IPC connection from {}", addr);

        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, ipc_worker).await {
                warn!("IPC client {} disconnected: {}", addr, err);
            }
        });
    }
}

async fn handle_client(
    stream: tokio::net::TcpStream,
    ipc_worker: ActorRef<IpcWorkerMessage>,
) -> Result<()> {
    let std_stream = stream
        .into_std()
        .map_err(|e| anyhow::anyhow!("Failed to convert stream: {e}"))?;

    let handle = tokio::runtime::Handle::current();

    tokio::task::spawn_blocking(move || run_client(std_stream, ipc_worker, handle))
        .await
        .map_err(|e| anyhow::anyhow!("IPC client task failed: {e}"))?
}

fn run_client(
    stream: TcpStream,
    ipc_worker: ActorRef<IpcWorkerMessage>,
    handle: tokio::runtime::Handle,
) -> Result<()> {
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|e| anyhow::anyhow!("Failed to set read timeout: {e}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(15)))
        .map_err(|e| anyhow::anyhow!("Failed to set write timeout: {e}"))?;

    let reader = stream
        .try_clone()
        .map_err(|e| anyhow::anyhow!("Failed to clone stream: {e}"))?;
    let writer = stream;
    let mut transport = IpcTransport::new(reader, writer);

    loop {
        let message = match transport.recv::<GuestMessage>() {
            Ok(message) => message,
            Err(TransportError::UnexpectedEof { .. }) => return Ok(()),
            Err(err) => {
                return Err(anyhow::anyhow!("IPC receive failed: {err}"));
            }
        };

        let response = handle.block_on(async {
            ractor::call_t!(
                ipc_worker,
                |reply| IpcWorkerMessage::HandleGuestMessage { message, reply },
                10_000
            )
        });

        let host_message = match response {
            Ok(Ok(message)) => message,
            Ok(Err(err)) => HostMessage::Error {
                message: err.to_string(),
            },
            Err(err) => HostMessage::Error {
                message: err.to_string(),
            },
        };

        transport
            .send(&host_message)
            .map_err(|e| anyhow::anyhow!("IPC send failed: {e}"))?;
    }
}
