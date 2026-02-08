//! Oya IPC - Length-prefixed buffer transport layer for Zellij plugin IPC
//!
//! This crate provides a transport layer for reading/writing length-prefixed messages
//! over stdin/stdout buffers for Zellij plugin IPC.
//!
//! # Protocol
//!
//! Every message frame consists of:
//! - **Length prefix**: 4-byte big-endian u32 indicating payload size
//! - **Payload**: Bincode-encoded message data
//!
//! ```text
//! +--------+--------+--------+--------+--------------------------+
//! | Byte 0 | Byte 1 | Byte 2 | Byte 3 | Bytes 4..(4+N)           |
//! |--------+--------+--------+--------+--------------------------|
//! |          Length (big-endian u32)       |    Bincode Payload      |
//! |           N = payload size             |    (N bytes)             |
//! +--------+--------+--------+--------+--------------------------+
//! ```
//!
//! # Constraints
//!
//! - Maximum message size: 1MB (1,048,576 bytes)
//! - Length prefix is big-endian byte order
//! - Empty payloads (length = 0) are rejected
//!
//! # Example
//!
//! ```rust
//! use oya_ipc::IpcTransport;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, PartialEq, Serialize, Deserialize)]
//! struct TestMessage {
//!     field: String,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create transport pair
//! let (mut client, mut server) = IpcTransport::pair();
//!
//! // Send message
//! let msg = TestMessage {
//!     field: "hello".to_string(),
//! };
//! client.send(&msg)?;
//!
//! // Receive message
//! let received = server.recv::<TestMessage>()?;
//! assert_eq!(received, msg);
//! # Ok(())
//! # }
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

mod error;
mod messages;
mod transport;

pub use error::{TransportError, TransportResult};
pub use messages::{
    AlertLevel, BeadDetail, BeadSummary, ComponentHealth, GraphEdge, GraphNode, GuestMessage,
    HealthStatus, HostMessage,
};
pub use transport::IpcTransport;

/// Maximum allowed payload size in bytes (1MB)
pub const MAX_PAYLOAD_SIZE: usize = 1_048_576;

/// Length prefix size in bytes
pub const LENGTH_PREFIX_SIZE: usize = 4;

/// Maximum frame size (length prefix + max payload)
pub const MAX_FRAME_SIZE: usize = LENGTH_PREFIX_SIZE + MAX_PAYLOAD_SIZE;
