//! WebSocket connection handler
//!
//! Handles real-time bidirectional communication for workflow updates and events.
//! Uses functional patterns with Result-based error handling.

use super::super::actors::{AppState, BroadcastEvent};
use super::super::error::{AppError, Result};
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// WebSocket message types sent from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to workflow/bead updates
    Subscribe { bead_id: String },
    /// Unsubscribe from workflow/bead updates
    Unsubscribe { bead_id: String },
    /// Ping to keep connection alive
    Ping,
}

/// WebSocket message types sent from server to client
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Subscription confirmation
    Subscribed { bead_id: String },
    /// Unsubscription confirmation
    Unsubscribed { bead_id: String },
    /// Workflow/bead status update
    StatusUpdate {
        bead_id: String,
        status: String,
        phase: String,
    },
    /// Event notification
    Event { bead_id: String, event: String },
    /// Broadcast event from the system
    Broadcast { event: BroadcastEvent },
    /// Pong response to ping
    Pong,
    /// Error message
    Error { message: String },
}

/// GET /api/ws - WebSocket upgrade endpoint
///
/// Handles WebSocket connection upgrade and manages the connection lifecycle.
/// Uses Railway-Oriented Programming:
/// 1. Accept WebSocket upgrade
/// 2. Handle connection lifecycle
/// 3. Process messages with proper error handling
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle WebSocket connection lifecycle
///
/// Railway track: connect -> message loop -> broadcast loop -> disconnect
/// All errors are logged and handled gracefully
async fn handle_socket(socket: WebSocket, state: AppState) {
    info!("WebSocket connection established");

    // Split socket into sender and receiver for independent handling
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // Track subscriptions for this connection
    let mut subscriptions: Vec<String> = Vec::new();

    // Run both message processing and broadcast forwarding concurrently
    loop {
        tokio::select! {
            // Handle incoming client messages
            msg_result = receiver.next() => {
                match msg_result {
                    Some(msg) => {
                        match process_message(msg, &state, &mut subscriptions).await {
                            Ok(Some(response)) => {
                                if let Err(e) = send_message(&mut sender, response).await {
                                    error!("Failed to send message: {}", e);
                                    break;
                                }
                            }
                            Ok(None) => {
                                // No response needed (e.g., close message)
                                debug!("Connection closing gracefully");
                                break;
                            }
                            Err(e) => {
                                error!("Error processing message: {}", e);
                                let error_msg = ServerMessage::Error {
                                    message: e.to_string(),
                                };
                                if send_message(&mut sender, error_msg).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    None => {
                        // Client disconnected
                        debug!("Client disconnected");
                        break;
                    }
                }
            }
            // Forward broadcast events to this client
            broadcast_result = broadcast_rx.recv() => {
                match broadcast_result {
                    Ok(event) => {
                        // Filter events based on subscriptions if needed
                        if should_forward_event(&event, &subscriptions) {
                            let msg = ServerMessage::Broadcast { event };
                            if let Err(e) = send_message(&mut sender, msg).await {
                                error!("Failed to send broadcast message: {}", e);
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        // Client is too slow, missed some messages
                        warn!("Client lagged behind, missed {} messages", count);
                        // Continue processing, client will recover
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Broadcast channel closed, should not happen
                        error!("Broadcast channel closed unexpectedly");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup subscriptions on disconnect
    cleanup_subscriptions(&subscriptions).await;
    info!("WebSocket connection closed");
}

/// Process incoming WebSocket message
///
/// Railway-Oriented Programming:
/// - Parse message -> Handle command -> Return response
/// - Errors are propagated up for centralized handling
async fn process_message(
    msg_result: std::result::Result<Message, axum::Error>,
    state: &AppState,
    subscriptions: &mut Vec<String>,
) -> Result<Option<ServerMessage>> {
    let msg = msg_result.map_err(|e| AppError::Internal(format!("WebSocket error: {}", e)))?;

    match msg {
        Message::Text(text) => {
            debug!("Received text message: {}", text);
            handle_text_message(&text, state, subscriptions).await
        }
        Message::Binary(_) => {
            warn!("Received binary message (not supported)");
            Err(AppError::BadRequest(
                "Binary messages not supported".to_string(),
            ))
        }
        Message::Ping(_) => {
            debug!("Received ping");
            Ok(Some(ServerMessage::Pong))
        }
        Message::Pong(_) => {
            debug!("Received pong");
            Ok(None)
        }
        Message::Close(_) => {
            debug!("Received close message");
            Ok(None)
        }
    }
}

/// Handle text message from client
///
/// Parses JSON and dispatches to appropriate handler
async fn handle_text_message(
    text: &str,
    state: &AppState,
    subscriptions: &mut Vec<String>,
) -> Result<Option<ServerMessage>> {
    let client_msg: ClientMessage = serde_json::from_str(text)
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    match client_msg {
        ClientMessage::Subscribe { bead_id } => {
            handle_subscribe(bead_id, state, subscriptions).await
        }
        ClientMessage::Unsubscribe { bead_id } => handle_unsubscribe(bead_id, subscriptions).await,
        ClientMessage::Ping => Ok(Some(ServerMessage::Pong)),
    }
}

/// Handle subscribe command
///
/// Adds bead_id to subscription list
async fn handle_subscribe(
    bead_id: String,
    _state: &AppState,
    subscriptions: &mut Vec<String>,
) -> Result<Option<ServerMessage>> {
    if !subscriptions.contains(&bead_id) {
        subscriptions.push(bead_id.clone());
        info!("Subscribed to bead: {}", bead_id);
    }

    Ok(Some(ServerMessage::Subscribed { bead_id }))
}

/// Handle unsubscribe command
///
/// Removes bead_id from subscription list
async fn handle_unsubscribe(
    bead_id: String,
    subscriptions: &mut Vec<String>,
) -> Result<Option<ServerMessage>> {
    subscriptions.retain(|id| id != &bead_id);
    info!("Unsubscribed from bead: {}", bead_id);

    Ok(Some(ServerMessage::Unsubscribed { bead_id }))
}

/// Send message to WebSocket client
///
/// Serializes ServerMessage to JSON and sends as text
async fn send_message(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    msg: ServerMessage,
) -> Result<()> {
    let json = serde_json::to_string(&msg)
        .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;

    sender
        .send(Message::Text(json.into()))
        .await
        .map_err(|e| AppError::Internal(format!("Failed to send message: {}", e)))
}

/// Determine if a broadcast event should be forwarded to this client
///
/// If subscriptions list is empty, forward all events (broadcast mode)
/// Otherwise, only forward events matching subscribed bead_ids
fn should_forward_event(event: &BroadcastEvent, subscriptions: &[String]) -> bool {
    // If no specific subscriptions, forward all events
    if subscriptions.is_empty() {
        return true;
    }

    // Check if event matches any subscription
    match event {
        BroadcastEvent::BeadStatusChanged { bead_id, .. } => subscriptions.contains(bead_id),
        BroadcastEvent::BeadEvent { bead_id, .. } => subscriptions.contains(bead_id),
        BroadcastEvent::SystemEvent { .. } => true, // Always forward system events
    }
}

/// Cleanup subscriptions when connection closes
///
/// In a full implementation, this would notify the state manager
/// to remove this connection from active subscribers
async fn cleanup_subscriptions(subscriptions: &[String]) {
    if !subscriptions.is_empty() {
        info!("Cleaning up {} subscriptions", subscriptions.len());
        // TODO: Notify state manager to remove subscriptions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_parsing() {
        let json = r#"{"type":"subscribe","bead_id":"test-123"}"#;
        let msg: std::result::Result<ClientMessage, _> = serde_json::from_str(json);
        assert!(msg.is_ok());
        if let Ok(ClientMessage::Subscribe { bead_id }) = msg {
            assert_eq!(bead_id, "test-123");
        }
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Subscribed {
            bead_id: "test-123".to_string(),
        };
        let json = serde_json::to_string(&msg);
        assert!(json.is_ok());
        if let Ok(j) = json {
            assert!(j.contains("subscribed"));
            assert!(j.contains("test-123"));
        }
    }

    #[test]
    fn test_should_forward_event_no_subscriptions() {
        let subscriptions: Vec<String> = vec![];
        let event = BroadcastEvent::BeadStatusChanged {
            bead_id: "bead-123".to_string(),
            status: "running".to_string(),
            phase: "build".to_string(),
        };
        assert!(should_forward_event(&event, &subscriptions));
    }

    #[test]
    fn test_should_forward_event_with_matching_subscription() {
        let subscriptions = vec!["bead-123".to_string()];
        let event = BroadcastEvent::BeadStatusChanged {
            bead_id: "bead-123".to_string(),
            status: "running".to_string(),
            phase: "build".to_string(),
        };
        assert!(should_forward_event(&event, &subscriptions));
    }

    #[test]
    fn test_should_not_forward_event_without_matching_subscription() {
        let subscriptions = vec!["bead-456".to_string()];
        let event = BroadcastEvent::BeadStatusChanged {
            bead_id: "bead-123".to_string(),
            status: "running".to_string(),
            phase: "build".to_string(),
        };
        assert!(!should_forward_event(&event, &subscriptions));
    }

    #[test]
    fn test_should_forward_system_event_always() {
        let subscriptions = vec!["bead-123".to_string()];
        let event = BroadcastEvent::SystemEvent {
            message: "System maintenance".to_string(),
        };
        assert!(should_forward_event(&event, &subscriptions));
    }

    #[test]
    fn test_broadcast_message_serialization() {
        let event = BroadcastEvent::BeadStatusChanged {
            bead_id: "bead-123".to_string(),
            status: "completed".to_string(),
            phase: "deploy".to_string(),
        };
        let msg = ServerMessage::Broadcast { event };
        let json = serde_json::to_string(&msg);
        assert!(json.is_ok());
        if let Ok(j) = json {
            assert!(j.contains("broadcast"));
            assert!(j.contains("bead_status_changed"));
            assert!(j.contains("bead-123"));
        }
    }
}
