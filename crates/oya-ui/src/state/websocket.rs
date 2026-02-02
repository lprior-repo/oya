//! WebSocket client for real-time communication with OYA server
//!
//! This module provides WebSocket connectivity with connection state tracking
//! and error handling following the project's functional patterns.

use futures::StreamExt;
use gloo_net::websocket::Message;
use gloo_net::websocket::futures::WebSocket;
use leptos::prelude::*;
use oya_events::BeadEvent;
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
    /// Failed to deserialize message
    DeserializationFailed(String),
    /// Received non-binary message
    UnexpectedMessageType,
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            Self::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            Self::ReceiveFailed(msg) => write!(f, "Receive failed: {}", msg),
            Self::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
            Self::DeserializationFailed(msg) => write!(f, "Deserialization failed: {}", msg),
            Self::UnexpectedMessageType => write!(f, "Unexpected message type (expected binary)"),
        }
    }
}

impl std::error::Error for WebSocketError {}

/// Result type for WebSocket operations
pub type Result<T> = std::result::Result<T, WebSocketError>;

/// Deserialize bincode frame to BeadEvent
///
/// Pure function that transforms raw bytes into a typed BeadEvent.
///
/// # Arguments
/// * `data` - Raw binary data from WebSocket
///
/// # Returns
/// Result with deserialized BeadEvent or deserialization error
///
/// # Errors
/// Returns `WebSocketError::DeserializationFailed` if bincode deserialization fails
fn deserialize_bead_event(data: &[u8]) -> Result<BeadEvent> {
    bincode::deserialize(data).map_err(|e| WebSocketError::DeserializationFailed(format!("{}", e)))
}

/// Handle incoming WebSocket message
///
/// Pure function that processes a WebSocket message and extracts binary data.
///
/// # Arguments
/// * `message` - WebSocket message (Text or Bytes)
///
/// # Returns
/// Result with binary data or error if message is not binary
///
/// # Errors
/// Returns `WebSocketError::UnexpectedMessageType` if message is not binary
fn extract_binary_data(message: Message) -> Result<Vec<u8>> {
    match message {
        Message::Bytes(data) => Ok(data),
        Message::Text(_) => Err(WebSocketError::UnexpectedMessageType),
    }
}

/// Process WebSocket message through the functional pipeline
///
/// Railway-oriented function that chains message extraction and deserialization.
///
/// # Arguments
/// * `message` - Incoming WebSocket message
///
/// # Returns
/// Result with deserialized BeadEvent or error from any stage
fn process_websocket_message(message: Message) -> Result<BeadEvent> {
    extract_binary_data(message).and_then(|data| deserialize_bead_event(&data))
}

/// Handle successful BeadEvent with side effects
///
/// Updates reactive signals and logs the event. This is the imperative shell
/// where side effects are permitted.
///
/// # Arguments
/// * `event` - Successfully deserialized BeadEvent
/// * `event_signal` - Signal to update with new event
fn handle_bead_event(event: BeadEvent, event_signal: WriteSignal<Option<BeadEvent>>) {
    web_sys::console::log_1(&format!("Received BeadEvent: {}", event.event_type()).into());
    event_signal.set(Some(event));
}

/// Handle WebSocket error with side effects
///
/// Logs error to console and updates connection state. This is the imperative shell.
///
/// # Arguments
/// * `error` - WebSocket error to handle
/// * `state_signal` - Signal to update connection state
fn handle_websocket_error(error: WebSocketError, state_signal: WriteSignal<ConnectionState>) {
    web_sys::console::error_1(&format!("WebSocket error: {}", error).into());
    state_signal.set(ConnectionState::Error);
}

/// Start WebSocket message event loop
///
/// Spawns async task that processes incoming WebSocket messages using
/// Railway-Oriented Programming patterns.
///
/// # Arguments
/// * `ws` - WebSocket connection stream
/// * `state_signal` - Signal to update connection state
/// * `event_signal` - Signal to update with received events
fn start_message_loop(
    mut ws: WebSocket,
    state_signal: WriteSignal<ConnectionState>,
    event_signal: WriteSignal<Option<BeadEvent>>,
) {
    spawn_local(async move {
        while let Some(msg) = ws.next().await {
            msg.map_err(|e| WebSocketError::ReceiveFailed(format!("{:?}", e)))
                .and_then(process_websocket_message)
                .map(|event| handle_bead_event(event, event_signal))
                .map_err(|e| handle_websocket_error(e, state_signal))
                .ok();
        }

        // Connection closed
        web_sys::console::log_1(&"WebSocket connection closed".into());
        state_signal.set(ConnectionState::Disconnected);
    });
}

/// Connect to WebSocket server at the given URL
///
/// # Arguments
/// * `url` - WebSocket URL (e.g., "ws://localhost:8080/api/ws")
/// * `state_signal` - Signal to update connection state
/// * `event_signal` - Signal to update with received events
///
/// # Returns
/// Result indicating success or connection error
pub fn connect_websocket(
    url: &str,
    state_signal: WriteSignal<ConnectionState>,
    event_signal: WriteSignal<Option<BeadEvent>>,
) -> Result<()> {
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

                // Start message processing loop
                start_message_loop(ws, state_signal, event_signal);
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
/// A tuple of (connection state signal, event signal)
pub fn init_websocket() -> (ReadSignal<ConnectionState>, ReadSignal<Option<BeadEvent>>) {
    let (state, set_state) = signal(ConnectionState::Disconnected);
    let (event, set_event) = signal(None);

    // Connect to WebSocket server
    let url = "ws://localhost:8080/api/ws";
    let _ = connect_websocket(url, set_state, set_event).map_err(|e| {
        web_sys::console::error_1(&format!("WebSocket initialization failed: {}", e).into());
        set_state.set(ConnectionState::Error);
    });

    (state, event)
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
        let (_, set_event) = signal(None);

        let result = connect_websocket("http://localhost", set_state, set_event);
        assert!(result.is_err());

        if let Err(WebSocketError::InvalidUrl(url)) = result {
            assert_eq!(url, "http://localhost");
        }
    }

    #[test]
    fn test_valid_url_formats() {
        let (_, set_state) = signal(ConnectionState::Disconnected);
        let (_, set_event) = signal(None);

        // These should not return InvalidUrl error immediately
        // They may fail later in actual connection, but URL validation passes
        let result = connect_websocket("ws://localhost:8080", set_state, set_event);
        assert!(result.is_ok());

        let (_, set_state) = signal(ConnectionState::Disconnected);
        let (_, set_event) = signal(None);
        let result = connect_websocket("wss://localhost:8080", set_state, set_event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deserialize_bead_event_with_invalid_data() {
        let invalid_data = vec![0xFF, 0xFF, 0xFF];
        let result = deserialize_bead_event(&invalid_data);
        assert!(result.is_err());

        if let Err(WebSocketError::DeserializationFailed(_)) = result {
            // Expected error type
        } else {
            panic!("Expected DeserializationFailed error");
        }
    }

    #[test]
    fn test_extract_binary_data_with_text_message() {
        let text_msg = Message::Text("hello".to_string());
        let result = extract_binary_data(text_msg);
        assert!(result.is_err());
        assert_eq!(result, Err(WebSocketError::UnexpectedMessageType));
    }

    #[test]
    fn test_extract_binary_data_with_binary_message() {
        let data = vec![1, 2, 3, 4];
        let binary_msg = Message::Bytes(data.clone());
        let result = extract_binary_data(binary_msg);
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(data));
    }
}
