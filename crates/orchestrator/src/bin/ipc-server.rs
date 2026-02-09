//! Orchestrator IPC server binary.
//!
//! TCP server for Zellij plugin IPC using length-prefixed bincode frames.

use std::net::TcpStream;
use std::time::Duration;

use orchestrator::actors::{IpcWorkerActorDef, IpcWorkerArguments, IpcWorkerMessage};
use orchestrator::ipc_messages::{GuestMessage, HostMessage};
use oya_ipc::{IpcTransport, TransportError};
use ractor::{Actor, ActorRef, RpcReplyPort};
use tokio::net::TcpListener;
use tracing::{info, warn};

const DEFAULT_ADDR: &str = "127.0.0.1:5555";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let address = std::env::var("OYA_IPC_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let (ipc_worker, _handle) = Actor::spawn(None, IpcWorkerActorDef, IpcWorkerArguments::new())
        .await
        .map_err(|err| format!("Failed to spawn IPC worker: {err:?}"))?;

    let listener = TcpListener::bind(&address).await?;
    info!("IPC server listening on {}", address);

    loop {
        let (stream, addr) = listener.accept().await?;
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let std_stream = stream.into_std()?;
    let handle = tokio::runtime::Handle::current();

    tokio::task::spawn_blocking(move || run_client(std_stream, ipc_worker, handle))
        .await
        .map_err(|err| format!("IPC client task failed: {err}"))?
}

fn run_client(
    stream: TcpStream,
    ipc_worker: ActorRef<IpcWorkerMessage>,
    handle: tokio::runtime::Handle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(15)))?;

    let reader = stream.try_clone()?;
    let writer = stream;
    let mut transport = IpcTransport::new(reader, writer);

    loop {
        let message = match transport.recv::<GuestMessage>() {
            Ok(message) => message,
            Err(TransportError::UnexpectedEof { .. }) => {
                return Ok(());
            }
            Err(err) => {
                return Err(format!("IPC receive failed: {err}").into());
            }
        };

        let response = handle.block_on(async {
            let reply = RpcReplyPort::new();
            ractor::call_t!(
                ipc_worker,
                IpcWorkerMessage::HandleGuestMessage { message, reply },
                Duration::from_secs(10)
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
            .map_err(|err| format!("IPC send failed: {err}"))?;
    }
}
