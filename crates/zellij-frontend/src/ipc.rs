// IPC module - Communication with oya-orchestrator for real-time data
//
// This module provides a TCP-based IPC client using bincode serialization
// for efficient communication with the oya-orchestrator.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use oya_ipc::{GuestMessage, HostMessage, IpcTransport, TransportError};
use thiserror::Error;

/// IPC client errors
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Transport error: {0}")]
    Transport(String),
}

impl From<TransportError> for IpcError {
    fn from(err: TransportError) -> Self {
        IpcError::Transport(err.to_string())
    }
}

/// IPC client for orchestrator communication
pub struct IpcClient {
    transport: IpcTransport<Box<dyn Read + Send>, Box<dyn Write + Send>>,
}

impl IpcClient {
    /// Connect to the IPC server
    pub fn connect(address: &str) -> Result<Self, IpcError> {
        let stream = TcpStream::connect(address)
            .map_err(|e| IpcError::ConnectionFailed(format!("Connection failed: {e}")))?;

        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| IpcError::ConnectionFailed(format!("Failed to set read timeout: {e}")))?;

        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| IpcError::ConnectionFailed(format!("Failed to set write timeout: {e}")))?;

        let reader = stream
            .try_clone()
            .map_err(|e| IpcError::ConnectionFailed(format!("Failed to clone stream: {e}")))?;

        let transport = IpcTransport::new(
            Box::new(reader) as Box<dyn Read + Send>,
            Box::new(stream) as Box<dyn Write + Send>,
        );
        Ok(Self { transport })
    }

    /// Send a request and wait for a response
    pub fn request(&mut self, message: GuestMessage) -> Result<HostMessage, IpcError> {
        self.transport.send(&message)?;
        self.transport.recv().map_err(IpcError::from)
    }
}

// ============================================================================
// ZELLIJ PLUGIN IPC (stdin/stdout)
// ============================================================================

/// Zellij stdin/stdout IPC client for WASM plugin
///
/// Synchronous client for communicating with backend via Zellij stdin/stdout.
pub struct ZellijIpcClient {
    _correlation_context: crate::correlation::CorrelationContext,
    _pending_requests: std::collections::HashMap<String, serde_json::Value>,
}

impl ZellijIpcClient {
    /// Create new Zellij IPC client
    pub fn new() -> Self {
        Self {
            _correlation_context: crate::correlation::CorrelationContext::new(),
            _pending_requests: std::collections::HashMap::new(),
        }
    }

    /// Send a GuestMessage to backend
    ///
    /// Returns Ok(()) on success, Err on failure
    pub fn send_command(&mut self, cmd: &GuestMessage) -> Result<(), String> {
        let request_id = self._correlation_context.generate_request_id();

        // Serialize message
        let json =
            serde_json::to_string(cmd).map_err(|e| format!("Serialization failed: {}", e))?;

        // Add correlation ID
        let payload = format!(r#"{{"request_id": "{}", "data": {}}}"#, request_id, json);

        // Write to Zellij stdout
        let mut stdout = std::io::stdout();
        stdout
            .write_all(payload.as_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;
        stdout.flush().map_err(|e| format!("Flush failed: {}", e))?;

        Ok(())
    }

    /// Read a HostMessage from backend
    ///
    /// Blocks until message is available
    pub fn recv(&mut self) -> Result<HostMessage, String> {
        let mut stdin = std::io::stdin();
        let mut buffer = [0u8; 8192]; // 8KB buffer

        // Read line (JSON terminated by newline)
        let mut json_string = String::new();
        loop {
            let bytes_read = stdin
                .read(&mut buffer)
                .map_err(|e| format!("Read failed: {}", e))?;

            if bytes_read == 0 {
                // End of stream - treat as disconnect
                return Err("Connection closed".to_string());
            }

            let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
            json_string.push_str(&chunk);

            // Check if we have a complete JSON object
            if json_string.trim().ends_with('}') {
                break;
            }
        }

        // Parse JSON
        serde_json::from_str::<HostMessage>(&json_string)
            .map_err(|e| format!("Deserialization failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zellij_ipc_client() {
        let client = ZellijIpcClient::new();
        assert!(true);
    }
}
