//! Actor examples demonstrating ractor patterns
//!
//! This module contains example implementations of common actor patterns:
//! - Ping/Pong: Basic message passing and reply
//! - Messaging: call(), cast(), and send() patterns
//! - Future: Request/response patterns, supervision, and error handling

pub mod messaging;
pub mod ping_pong;

// Re-export example types
pub use messaging::{
    CalculatorActor, CalculatorError, CalculatorMessage, CalculatorResult, CalculatorState,
    CalculatorStats, LogEntry, LogLevel, LoggerActor, LoggerMessage, LoggerState,
};
pub use ping_pong::{
    PingActor, PingMessage, PingPongExample, PingPongResult, PongActor, PongMessage,
};
