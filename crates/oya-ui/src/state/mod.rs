//! Application state management
//!
//! This module provides centralized state management including
//! WebSocket connections and shared application state.

pub mod websocket;

pub use websocket::{ConnectionState, WebSocketError, init_websocket};
