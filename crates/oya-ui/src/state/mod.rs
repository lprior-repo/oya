//! Application state management
//!
//! This module provides centralized state management using Tauri IPC.
//! WebSocket support has been removed in favor of Tauri's native events
//! which provide ~10-50μs latency (vs ~100-500μs for WebSockets).
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::state::{init_backend, BackendState};
//!
//! let (state, event) = init_backend();
//! // state will be BackendState::Tauri when connected
//! ```

pub mod tauri_bridge;

pub use crate::models::BeadEvent;
pub use tauri_bridge::{
    TauriConnectionState, TauriError, init_tauri, invoke, is_tauri_available, listen,
};

use leptos::prelude::Set;

/// Backend connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected to Tauri backend
    Connected,
    /// Error occurred
    Error,
}

impl std::fmt::Display for BackendState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendState::Disconnected => write!(f, "Disconnected"),
            BackendState::Connecting => write!(f, "Connecting"),
            BackendState::Connected => write!(f, "Connected"),
            BackendState::Error => write!(f, "Error"),
        }
    }
}

/// Initialize the Tauri backend
///
/// Returns signals for connection state and events.
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

    if !is_tauri_available() {
        // In browser mode - set error state
        web_sys::console::warn_1(&"Tauri not available - running in limited browser mode".into());
        set_state.set(BackendState::Error);
        return (state, event);
    }

    // Desktop mode - use Tauri IPC
    let (tauri_state, _health) = init_tauri();

    // Map Tauri state to backend state
    Effect::new(move || {
        let ts = tauri_state.get();
        let new_state = match ts {
            TauriConnectionState::NotAvailable => BackendState::Error,
            TauriConnectionState::Connected => BackendState::Connected,
            TauriConnectionState::Error => BackendState::Error,
        };
        set_state.set(new_state);
    });

    // Set up Tauri event listeners for bead events
    setup_event_listeners(set_event);

    (state, event)
}

/// Set up Tauri event listeners
fn setup_event_listeners(set_event: leptos::prelude::WriteSignal<Option<BeadEvent>>) {
    // Listen for bead events from the backend
    if let Err(e) = listen("bead-event", move |event| {
        if let Ok(bead_event) = serde_wasm_bindgen::from_value::<BeadEvent>(event) {
            set_event.set(Some(bead_event));
        }
    }) {
        web_sys::console::error_1(&format!("Failed to set up bead event listener: {e}").into());
    }

    // Listen for stage events
    if let Err(e) = listen("stage-event", |event| {
        web_sys::console::log_1(&format!("Stage event received: {:?}", event).into());
        // Stage events are handled by pipeline components directly
    }) {
        web_sys::console::error_1(&format!("Failed to set up stage event listener: {e}").into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_state_display() {
        assert_eq!(BackendState::Disconnected.to_string(), "Disconnected");
        assert_eq!(BackendState::Connecting.to_string(), "Connecting");
        assert_eq!(BackendState::Connected.to_string(), "Connected");
        assert_eq!(BackendState::Error.to_string(), "Error");
    }

    #[test]
    fn test_backend_state_equality() {
        assert_eq!(BackendState::Connected, BackendState::Connected);
        assert_ne!(BackendState::Connected, BackendState::Error);
    }
}
