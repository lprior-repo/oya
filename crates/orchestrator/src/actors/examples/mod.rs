//! Actor examples demonstrating ractor patterns
//!
//! Lightweight example placeholders.

/// Messages sent to a ping actor in examples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PingMessage {
    /// Start the ping-pong flow.
    Start,
}

/// Messages sent to a pong actor in examples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PongMessage {
    /// Pong response.
    Pong,
}

/// Placeholder type for ping-pong example wiring.
#[derive(Debug, Clone, Copy, Default)]
pub struct PingPongExample;
