//! WebSocket client for real-time communication with OYA server
//!
//! This module provides WebSocket connectivity with connection state tracking
//! and error handling following the project's functional patterns.

use gloo_net::websocket::futures::WebSocket;
use leptos::prelude::*;
use std::fmt;
use wasm_bindgen_futures::spawn_local;

/// WebSocket connection states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Initial state, not yet connected
    Disconnected,
    /// Attempting to establish connection
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection error occurred
    Error,
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// WebSocket connection errors
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketError {
    /// Failed to open connection
    ConnectionFailed(String),
    /// Failed to send message
    SendFailed(String),
    /// Failed to receive message
    ReceiveFailed(String),
    /// Invalid URL
    InvalidUrl(String),
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            Self::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            Self::ReceiveFailed(msg) => write!(f, "Receive failed: {}", msg),
            Self::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
        }
    }
}

impl std::error::Error for WebSocketError {}

/// Result type for WebSocket operations
pub type Result<T> = std::result::Result<T, WebSocketError>;

/// Connect to WebSocket server at the given URL
///
/// # Arguments
/// * `url` - WebSocket URL (e.g., "ws://localhost:8080/api/ws")
/// * `state_signal` - Signal to update connection state
///
/// # Returns
/// Result indicating success or connection error
pub fn connect_websocket(url: &str, state_signal: WriteSignal<ConnectionState>) -> Result<()> {
    // Validate URL format
    if !url.starts_with("ws://") && !url.starts_with("wss://") {
        return Err(WebSocketError::InvalidUrl(url.to_string()));
    }

    // Set connecting state
    state_signal.set(ConnectionState::Connecting);

    // Store URL for closure
    let ws_url = url.to_string();

    // Spawn async connection task
    spawn_local(async move {
        // Log connection attempt
        web_sys::console::log_1(&format!("Connecting to WebSocket: {}", ws_url).into());

        match WebSocket::open(&ws_url) {
            Ok(ws) => {
                // Connection successful
                web_sys::console::log_1(&"WebSocket connection established".into());
                state_signal.set(ConnectionState::Connected);

                // Keep reference to prevent drop
                // In a real implementation, we'd store this in a resource
                // and handle message receiving here
                let _ws = ws;
            }
            Err(e) => {
                // Connection failed
                let error_msg = format!("Failed to connect: {:?}", e);
                web_sys::console::error_1(&error_msg.clone().into());
                state_signal.set(ConnectionState::Error);
            }
        }
    });

    Ok(())
}

/// Initialize WebSocket connection on app startup
///
/// This should be called once when the app initializes to establish
/// the WebSocket connection to the server.
///
/// # Returns
/// A tuple of (connection state signal, connection state setter)
pub fn init_websocket() -> (ReadSignal<ConnectionState>, WriteSignal<ConnectionState>) {
    let (state, set_state) = signal(ConnectionState::Disconnected);

    // Connect to WebSocket server
    let url = "ws://localhost:8080/api/ws";
    let _ = connect_websocket(url, set_state).map_err(|e| {
        web_sys::console::error_1(&format!("WebSocket initialization failed: {}", e).into());
        set_state.set(ConnectionState::Error);
    });

    (state, set_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_display() {
        assert_eq!(ConnectionState::Disconnected.to_string(), "Disconnected");
        assert_eq!(ConnectionState::Connecting.to_string(), "Connecting");
        assert_eq!(ConnectionState::Connected.to_string(), "Connected");
        assert_eq!(ConnectionState::Error.to_string(), "Error");
    }

    #[test]
    fn test_connection_state_equality() {
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
    }

    #[test]
    fn test_websocket_error_display() {
        let err = WebSocketError::ConnectionFailed("timeout".to_string());
        assert_eq!(err.to_string(), "Connection failed: timeout");

        let err = WebSocketError::InvalidUrl("http://example.com".to_string());
        assert_eq!(err.to_string(), "Invalid URL: http://example.com");
    }

    #[test]
    fn test_invalid_url_validation() {
        let (_, set_state) = signal(ConnectionState::Disconnected);

        let result = connect_websocket("http://localhost", set_state);
        assert!(result.is_err());

        if let Err(WebSocketError::InvalidUrl(url)) = result {
            assert_eq!(url, "http://localhost");
        }
    }

    #[test]
    fn test_valid_url_formats() {
        let (_, set_state) = signal(ConnectionState::Disconnected);

        // These should not return InvalidUrl error immediately
        // They may fail later in actual connection, but URL validation passes
        let result = connect_websocket("ws://localhost:8080", set_state);
        assert!(result.is_ok());

        let (_, set_state) = signal(ConnectionState::Disconnected);
        let result = connect_websocket("wss://localhost:8080", set_state);
        assert!(result.is_ok());
    }
}
