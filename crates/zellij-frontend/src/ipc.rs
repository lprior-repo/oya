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
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// IPC client error types.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("invalid IPC address: {0}")]
    InvalidAddress(String),

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
    pub fn connect(address: &str) -> IpcResult<Self> {
        validate_address(address)?;

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

fn validate_address(address: &str) -> IpcResult<()> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(IpcError::InvalidAddress("IPC address is empty".to_string()));
    }

    if trimmed.chars().any(char::is_control) {
        return Err(IpcError::InvalidAddress(format!(
            "Address contains control characters: {}",
            sanitize_for_display(trimmed)
        )));
    }

    if trimmed.eq_ignore_ascii_case("stdio") || trimmed.starts_with("stdio://") {
        return Ok(());
    }

    if trimmed.parse::<SocketAddr>().is_ok() {
        return Ok(());
    }

    let (host_raw, port_raw) = trimmed.rsplit_once(':').ok_or_else(|| {
        IpcError::InvalidAddress(format!(
            "Missing port in address: {}",
            sanitize_for_display(trimmed)
        ))
    })?;

    let host = host_raw
        .strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host_raw);

    let port = port_raw.parse::<u16>().map_err(|_| {
        IpcError::InvalidAddress(format!(
            "Invalid port in address: {}",
            sanitize_for_display(trimmed)
        ))
    })?;

    if port == 0 {
        return Err(IpcError::InvalidAddress(format!(
            "Port must be greater than zero: {}",
            sanitize_for_display(trimmed)
        )));
    }

    if host.is_empty() || !is_valid_host(host) {
        return Err(IpcError::InvalidAddress(format!(
            "Invalid host in address: {}",
            sanitize_for_display(trimmed)
        )));
    }

    Ok(())
}

fn is_valid_host(host: &str) -> bool {
    host.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.'))
}

fn sanitize_for_display(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { '?' } else { ch })
        .collect()
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

    #[test]
    fn test_ipc_client_connect_rejects_empty_address() {
        let result = IpcClient::connect("");
        assert!(result.is_err());
    }

    #[test]
    fn test_ipc_client_connect_rejects_invalid_address() {
        let result = IpcClient::connect("not-an-address");
        assert!(result.is_err());
    }

    #[test]
    fn test_ipc_client_connect_accepts_stdio_transport() {
        let result = IpcClient::connect("stdio://zellij");
        assert!(result.is_ok());
    }

    #[test]
    fn test_ipc_client_connect_rejects_zero_port() {
        let result = IpcClient::connect("localhost:0");
        assert!(result.is_err());
    }

    #[test]
    fn test_ipc_client_connect_rejects_control_chars() {
        let result = IpcClient::connect("stdio://zellij\n");
        assert!(result.is_err());
    }
}
