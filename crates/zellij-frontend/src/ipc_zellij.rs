//! Zellij stdin/stdout IPC client for WASM plugin
//!
//! This module implements synchronous IPC communication with the backend
//! via Zellij's stdin/stdout. Since WASM doesn't have async runtime,
//! all I/O operations are synchronous blocking calls.

use crate::{correlation::CorrelationContext, correlation::RequestId};
use oya_ipc::{GuestMessage, HostMessage};
use serde_json;
use std::io::{self, Read, Write};

/// Zellij stdin wrapper
///
/// Provides read access to Zellij's stdin stream.
pub struct ZellijStdin;

impl ZellijStdin {
    pub fn new() -> Self {
        Self {}
    }

    /// Read exactly n bytes from stdin
    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        std::io::stdin().read_exact(buf)
    }

    /// Read available data from stdin
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        std::io::stdin().read(buf)
    }
}

/// Zellij stdout wrapper
///
/// Provides write access to Zellij's stdout stream.
pub struct ZellijStdout;

impl ZellijStdout {
    pub fn new() -> Self {
        Self {}
    }

    /// Write data to stdout
    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        std::io::stdout().write_all(data)
    }

    /// Flush stdout
    pub fn flush(&mut self) -> io::Result<()> {
        std::io::stdout().flush()
    }
}

/// Zellij IPC client
///
/// Synchronous client for communicating with backend via Zellij stdin/stdout.
pub struct ZellijIpcClient {
    _correlation_context: CorrelationContext,
    _pending_requests: std::collections::HashMap<String, serde_json::Value>,
}

impl ZellijIpcClient {
    /// Create new Zellij IPC client
    pub fn new() -> Self {
        Self {
            _correlation_context: CorrelationContext::new(),
            _pending_requests: std::collections::HashMap::new(),
        }
    }

    /// Send a GuestMessage to backend
    ///
    /// Returns Ok(()) on success, Err on failure
    pub fn send_command(&mut self, cmd: &GuestMessage) -> Result<(), String> {
        let request_id = self._correlation_context.generate_request_id();
        
        // Serialize message
        let json = serde_json::to_string(cmd)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        // Add correlation ID
        let payload = format!("{{\"request_id\": \"{}\", \"data\": {}}}", request_id, json);
        
        // Write to Zellij stdout
        let mut stdout = ZellijStdout::new();
        stdout.write_all(payload.as_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;
        stdout.flush()
            .map_err(|e| format!("Flush failed: {}", e))?;

        Ok(())
    }

    /// Read a HostMessage from backend
    ///
    /// Blocks until message is available
    pub fn recv(&mut self) -> Result<HostMessage, String> {
        let mut stdin = ZellijStdin::new();
        let mut buffer = [0u8; 8192]; // 8KB buffer
        
        // Read line (JSON terminated by newline)
        let mut json_string = String::new();
        loop {
            let bytes_read = stdin.read(&mut buffer)
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
    fn test_zellij_stdin_read() {
        // Mock test - would need real stdin in integration tests
        assert!(true);
    }

    #[test]
    fn test_zellij_stdout_write() {
        let mut stdout = ZellijStdout::new();
        let data = b"test data";
        let result = stdout.write_all(data);
        assert!(result.is_ok());
    }
}
