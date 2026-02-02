//! Application state management
//!
//! This module provides centralized state management including
//! WebSocket connections and shared application state.

pub mod websocket;

pub use oya_events::BeadEvent;
pub use websocket::{ConnectionState, WebSocketError, init_websocket};
