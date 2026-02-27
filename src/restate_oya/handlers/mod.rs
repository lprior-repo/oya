#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod memory;
mod oya;
mod runtime;
mod service;
mod status;

#[cfg(test)]
mod tests;

pub use memory::OyaMemoryBridge;
pub use oya::OyaBridge;
pub use service::OyaServiceBridge;

use restate_sdk::prelude::*;
use std::net::SocketAddr;

#[restate_sdk::object]
trait OyaMemory {
    async fn start(
        req: Json<super::types::StartRequest>,
    ) -> Result<Json<super::types::StartResponse>, HandlerError>;
    async fn sync_bead(
        req: Json<super::types::BeadSyncRequest>,
    ) -> Result<Json<super::types::StartResponse>, HandlerError>;
    async fn run_pipeline(
        req: Json<super::types::PipelineRequest>,
    ) -> Result<Json<super::types::StartResponse>, HandlerError>;
    #[shared]
    async fn get_state() -> Result<Json<super::types::MemorySnapshot>, HandlerError>;
    #[shared]
    async fn get_bead() -> Result<Json<super::types::BeadSnapshot>, HandlerError>;
    async fn request_cancel() -> Result<Json<super::types::CancelResponse>, HandlerError>;
}

#[restate_sdk::service]
trait OyaService {
    async fn get_state(
        req: Json<super::types::KeyRequest>,
    ) -> Result<Json<super::types::MemorySnapshot>, HandlerError>;
    async fn get_bead(
        req: Json<super::types::KeyRequest>,
    ) -> Result<Json<super::types::BeadSnapshot>, HandlerError>;
    async fn get_lifecycle(
        req: Json<super::types::KeyRequest>,
    ) -> Result<Json<super::types::LifecycleStatusSnapshot>, HandlerError>;
    async fn cancel(
        req: Json<super::types::KeyRequest>,
    ) -> Result<Json<super::types::CancelResponse>, HandlerError>;
}

#[restate_sdk::workflow]
trait Oya {
    async fn run(
        req: Json<super::types::LifecycleRequest>,
    ) -> Result<Json<super::types::StartResponse>, HandlerError>;
    #[shared]
    async fn status() -> Result<Json<super::types::LifecycleStatusSnapshot>, HandlerError>;
}

pub async fn serve(bind: SocketAddr) -> anyhow::Result<()> {
    let endpoint = Endpoint::builder()
        .bind(OyaMemoryBridge.serve())
        .bind(OyaBridge.serve())
        .bind(OyaServiceBridge.serve())
        .build();
    HttpServer::new(endpoint).listen_and_serve(bind).await;
    Ok(())
}
