//! IPC client abstraction for oya-orchestrator communication.
//!
//! This module provides a unified interface for communicating with the OYA
//! orchestrator, supporting both direct socket connections and Zellij stdin/stdout.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]

use crate::ipc_zellij::ZellijIpcClient;
use oya_ipc::{GuestMessage, HostMessage};
use std::sync::{Arc, Mutex};

/// IPC client error types.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("send failed: {0}")]
    SendFailed(String),

    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    #[error("unexpected response: {0:?}")]
    UnexpectedResponse(Box<HostMessage>),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Result type for IPC operations.
pub type IpcResult<T> = Result<T, IpcError>;

/// IPC client for communicating with oya-orchestrator.
///
/// Wraps the Zellij IPC client and provides a higher-level request/response API.
pub struct IpcClient {
    inner: Arc<Mutex<ZellijIpcClient>>,
}

impl IpcClient {
    /// Connect to the orchestrator via address.
    ///
    /// For Zellij stdin/stdout, the address is ignored and we use stdin/stdout.
    pub fn connect(_address: &str) -> IpcResult<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(ZellijIpcClient::new())),
        })
    }

    /// Send a request and wait for a response.
    pub fn request(&mut self, message: GuestMessage) -> IpcResult<HostMessage> {
        let mut client = self
            .inner
            .lock()
            .map_err(|e| IpcError::SendFailed(format!("lock failed: {}", e)))?;

        // Send the request
        client
            .send_command(&message)
            .map_err(IpcError::SendFailed)?;

        // Receive the response
        client.recv().map_err(IpcError::ReceiveFailed)
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::connect("127.0.0.1:5555").unwrap_or_else(|_| Self {
            inner: Arc::new(Mutex::new(ZellijIpcClient::new())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_client_connect() {
        let _client = IpcClient::connect("127.0.0.1:5555");
    }

    #[test]
    fn test_ipc_client_default() {
        let _client = IpcClient::default();
    }
}
