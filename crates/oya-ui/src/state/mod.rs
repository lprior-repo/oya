//! Application state management
//!
//! This module provides centralized state management including
//! WebSocket connections, Tauri bridge, and shared application state.
//!
//! # Desktop vs Browser Mode
//!
//! The app supports two modes:
//! - **Desktop (Tauri)**: Uses Tauri IPC for high-performance communication
//! - **Browser (WebSocket)**: Falls back to WebSocket for web deployments
//!
//! Use `init_backend()` to initialize the appropriate backend.

pub mod tauri_bridge;
pub mod websocket;

pub use crate::models::BeadEvent;
pub use tauri_bridge::{TauriConnectionState, TauriError, is_tauri_available, init_tauri};
pub use websocket::{ConnectionState, WebSocketError, init_websocket};

/// Backend connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected to Tauri backend
    Tauri,
    /// Connected via WebSocket
    WebSocket,
    /// Error occurred
    Error,
}

impl std::fmt::Display for BackendState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendState::Disconnected => write!(f, "Disconnected"),
            BackendState::Connecting => write!(f, "Connecting"),
            BackendState::Tauri => write!(f, "Desktop"),
            BackendState::WebSocket => write!(f, "Web"),
            BackendState::Error => write!(f, "Error"),
        }
    }
}

/// Initialize the appropriate backend (Tauri or WebSocket)
///
/// Prefers Tauri if available, falls back to WebSocket for web deployments.
///
/// # Returns
/// A tuple of (backend state signal, event signal)
pub fn init_backend() -> (
    leptos::prelude::ReadSignal<BackendState>,
    leptos::prelude::ReadSignal<Option<BeadEvent>>,
) {
    use leptos::prelude::*;

    let (state, set_state) = signal(BackendState::Connecting);
    let (event, set_event) = signal(None);

    if is_tauri_available() {
        // Desktop mode - use Tauri IPC
        let (tauri_state, _health) = init_tauri();

        // Map Tauri state to backend state
        Effect::new(move || {
            let ts = tauri_state.get();
            let new_state = match ts {
                TauriConnectionState::NotAvailable => BackendState::Error,
                TauriConnectionState::Connected => BackendState::Tauri,
                TauriConnectionState::Error => BackendState::Error,
            };
            set_state.set(new_state);
        });

        // TODO: Set up Tauri event listeners for bead events
    } else {
        // Browser mode - use WebSocket
        let (ws_state, ws_event) = init_websocket();

        // Map WebSocket state to backend state
        Effect::new(move || {
            let ws = ws_state.get();
            let new_state = match ws {
                ConnectionState::Disconnected => BackendState::Disconnected,
                ConnectionState::Connecting => BackendState::Connecting,
                ConnectionState::Connected => BackendState::WebSocket,
                ConnectionState::Error => BackendState::Error,
            };
            set_state.set(new_state);
        });

        // Forward WebSocket events
        Effect::new(move || {
            if let Some(ev) = ws_event.get() {
                set_event.set(Some(ev));
            }
        });
    }

    (state, event)
}
