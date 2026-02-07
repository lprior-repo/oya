//! Actor examples demonstrating ractor patterns
//!
//! This module contains example implementations of common actor patterns:
//! - Ping/Pong: Basic message passing and reply
//! - Future: Request/response patterns, supervision, and error handling

pub mod ping_pong;

// Re-export example types
pub use ping_pong::{
    PingActor, PingMessage, PingPongExample, PingPongResult, PongActor, PongMessage,
};
