//! Application state management
//!
//! This module provides centralized state management including
//! WebSocket connections and shared application state.

pub mod websocket;

pub use crate::models::BeadEvent;
pub use websocket::{ConnectionState, WebSocketError, init_websocket};
