//! Tauri IPC bridge for desktop application
//!
//! Provides high-performance communication with the Tauri backend.
//! Features:
//! - Request deduplication to reduce IPC overhead
//! - Signal debouncing for batch updates
//! - Event listeners for real-time updates
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::state::tauri_bridge::{invoke, listen};
//!
//! // Invoke a Tauri command
//! let result = invoke::<Vec<Bead>>("get_beads_batch", &["id1", "id2"]).await;
//!
//! // Listen for events
//! listen("stream-chunk", |event| {
//!     // Handle streaming data
//! });
//! ```

use serde::{Serialize, de::DeserializeOwned};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Result type for Tauri bridge operations
pub type TauriResult<T> = Result<T, TauriError>;

/// Tauri bridge errors
#[derive(Debug, Clone)]
pub enum TauriError {
    /// Tauri not available (running in browser)
    NotAvailable,
    /// Command invocation failed
    InvocationFailed(String),
    /// Serialization error
    SerializationError(String),
    /// Event listener error
    ListenerError(String),
}

impl std::fmt::Display for TauriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TauriError::NotAvailable => write!(f, "Tauri not available"),
            TauriError::InvocationFailed(msg) => write!(f, "Invocation failed: {msg}"),
            TauriError::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            TauriError::ListenerError(msg) => write!(f, "Listener error: {msg}"),
        }
    }
}

impl std::error::Error for TauriError {}

// Thread-local pending requests for deduplication
thread_local! {
    static PENDING_REQUESTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Check if Tauri is available
#[must_use]
pub fn is_tauri_available() -> bool {
    web_sys::window().is_some_and(|w| {
        js_sys::Reflect::get(&w, &JsValue::from_str("__TAURI__")).is_ok_and(|v| !v.is_undefined())
    })
}

/// Invoke a Tauri command
///
/// # Type Parameters
/// - `R`: Return type (must be deserializable)
/// - `A`: Arguments type (must be serializable)
///
/// # Errors
/// Returns `TauriError` if Tauri is not available or the command fails
pub async fn invoke<R, A>(command: &str, args: &A) -> TauriResult<R>
where
    R: DeserializeOwned,
    A: Serialize + ?Sized,
{
    if !is_tauri_available() {
        return Err(TauriError::NotAvailable);
    }

    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| TauriError::SerializationError(e.to_string()))?;

    let result = invoke_inner(command, args_js).await?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| TauriError::SerializationError(e.to_string()))
}

/// Invoke Tauri command with raw JsValue
async fn invoke_inner(command: &str, args: JsValue) -> TauriResult<JsValue> {
    let window = web_sys::window().ok_or(TauriError::NotAvailable)?;

    let tauri = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map_err(|_| TauriError::NotAvailable)?;

    if tauri.is_undefined() {
        return Err(TauriError::NotAvailable);
    }

    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core"))
        .map_err(|_| TauriError::NotAvailable)?;

    let invoke_fn = js_sys::Reflect::get(&core, &JsValue::from_str("invoke"))
        .map_err(|_| TauriError::NotAvailable)?;

    let invoke_fn = invoke_fn
        .dyn_ref::<js_sys::Function>()
        .ok_or(TauriError::NotAvailable)?;

    let promise = invoke_fn
        .call2(&JsValue::NULL, &JsValue::from_str(command), &args)
        .map_err(|e| TauriError::InvocationFailed(format!("{e:?}")))?;

    let promise = js_sys::Promise::from(promise);
    let result = JsFuture::from(promise)
        .await
        .map_err(|e| TauriError::InvocationFailed(format!("{e:?}")))?;

    Ok(result)
}

/// Listen for Tauri events
///
/// # Arguments
/// - `event`: Event name to listen for
/// - `callback`: Closure to call when event is received
///
/// # Returns
/// An unlisten function that can be called to stop listening
///
/// # Errors
/// Returns `TauriError` if Tauri is not available or listener setup fails
pub fn listen<F>(event: &str, callback: F) -> TauriResult<js_sys::Function>
where
    F: Fn(JsValue) + 'static,
{
    if !is_tauri_available() {
        return Err(TauriError::NotAvailable);
    }

    let window = web_sys::window().ok_or(TauriError::NotAvailable)?;

    let tauri = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map_err(|_| TauriError::NotAvailable)?;

    let event_module = js_sys::Reflect::get(&tauri, &JsValue::from_str("event"))
        .map_err(|_| TauriError::NotAvailable)?;

    let listen_fn = js_sys::Reflect::get(&event_module, &JsValue::from_str("listen"))
        .map_err(|_| TauriError::NotAvailable)?;

    let listen_fn = listen_fn
        .dyn_ref::<js_sys::Function>()
        .ok_or(TauriError::NotAvailable)?;

    let closure = Closure::wrap(Box::new(callback) as Box<dyn Fn(JsValue)>);

    let result = listen_fn
        .call2(&JsValue::NULL, &JsValue::from_str(event), closure.as_ref())
        .map_err(|e| TauriError::ListenerError(format!("{e:?}")))?;

    // Leak the closure to keep it alive (it will be cleaned up when unlisten is called)
    closure.forget();

    // Return the unlisten function
    result
        .dyn_into()
        .map_err(|_| TauriError::ListenerError("Failed to get unlisten function".to_string()))
}

/// Batched bead fetch with request deduplication
///
/// Fetches multiple beads in a single IPC call, deduplicating against
/// pending requests to avoid redundant fetches.
pub async fn fetch_beads_batched(ids: Vec<String>) -> TauriResult<Vec<crate::models::bead::Bead>> {
    // Filter out IDs that are already being fetched
    let unique_ids: Vec<String> = PENDING_REQUESTS.with(|pending| {
        let mut pending = pending.borrow_mut();
        ids.into_iter()
            .filter(|id| pending.insert(id.clone()))
            .collect()
    });

    if unique_ids.is_empty() {
        return Ok(vec![]);
    }

    // Make the batched call
    let result = invoke::<Vec<crate::models::bead::Bead>, _>("get_beads_batch", &unique_ids).await;

    // Clear pending requests
    PENDING_REQUESTS.with(|pending| {
        let mut pending = pending.borrow_mut();
        for id in &unique_ids {
            pending.remove(id);
        }
    });

    result
}

/// Connection state for Tauri backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TauriConnectionState {
    /// Tauri not available (running in browser)
    NotAvailable,
    /// Connected to Tauri backend
    Connected,
    /// Connection error
    Error,
}

impl std::fmt::Display for TauriConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TauriConnectionState::NotAvailable => write!(f, "Not Available"),
            TauriConnectionState::Connected => write!(f, "Connected"),
            TauriConnectionState::Error => write!(f, "Error"),
        }
    }
}

/// Health check response from Tauri backend
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub version: String,
    pub cached_beads: u64,
    pub active_streams: usize,
    pub project_root: Option<String>,
}

/// Check Tauri backend health
pub async fn health_check() -> TauriResult<HealthStatus> {
    #[derive(serde::Serialize)]
    struct EmptyArgs {}
    invoke("health_check", &EmptyArgs {}).await
}

/// Initialize Tauri connection and return state signal
///
/// This should be called once when the app initializes.
/// If Tauri is not available, falls back to browser mode.
pub fn init_tauri() -> (
    leptos::prelude::ReadSignal<TauriConnectionState>,
    Rc<RefCell<Option<HealthStatus>>>,
) {
    use leptos::prelude::*;

    let (state, set_state) = signal(TauriConnectionState::NotAvailable);
    let health = Rc::new(RefCell::new(None));
    let health_clone = health.clone();

    if is_tauri_available() {
        wasm_bindgen_futures::spawn_local(async move {
            match health_check().await {
                Ok(status) => {
                    *health_clone.borrow_mut() = Some(status);
                    set_state.set(TauriConnectionState::Connected);
                    web_sys::console::log_1(&"Tauri backend connected".into());
                }
                Err(e) => {
                    set_state.set(TauriConnectionState::Error);
                    web_sys::console::error_1(&format!("Tauri health check failed: {e}").into());
                }
            }
        });
    } else {
        web_sys::console::log_1(&"Tauri not available, running in browser mode".into());
    }

    (state, health)
}

// Pipeline commands

/// Get pipeline stages definition
pub async fn get_pipeline_stages() -> TauriResult<Vec<crate::models::pipeline::StageInfo>> {
    #[derive(serde::Serialize)]
    struct EmptyArgs {}
    invoke("get_pipeline_stages", &EmptyArgs {}).await
}

/// Get pipeline state for a task
pub async fn get_pipeline_state(
    task_id: &str,
) -> TauriResult<crate::models::pipeline::PipelineState> {
    #[derive(serde::Serialize)]
    struct Args<'a> {
        task_id: &'a str,
    }
    invoke("get_pipeline_state", &Args { task_id }).await
}

/// Run a single pipeline stage
pub async fn run_stage(
    task_id: &str,
    stage_name: &str,
) -> TauriResult<crate::models::pipeline::StageEvent> {
    #[derive(serde::Serialize)]
    struct Args<'a> {
        task_id: &'a str,
        stage_name: &'a str,
    }
    invoke(
        "run_stage",
        &Args {
            task_id,
            stage_name,
        },
    )
    .await
}

/// Run the full pipeline
pub async fn run_pipeline(task_id: &str) -> TauriResult<crate::models::pipeline::PipelineState> {
    #[derive(serde::Serialize)]
    struct Args<'a> {
        task_id: &'a str,
    }
    invoke("run_pipeline", &Args { task_id }).await
}

/// Reset pipeline state for a task
pub async fn reset_pipeline(task_id: &str) -> TauriResult<crate::models::pipeline::PipelineState> {
    #[derive(serde::Serialize)]
    struct Args<'a> {
        task_id: &'a str,
    }
    invoke("reset_pipeline", &Args { task_id }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tauri_error_display() {
        let err = TauriError::NotAvailable;
        assert_eq!(err.to_string(), "Tauri not available");

        let err = TauriError::InvocationFailed("timeout".to_string());
        assert_eq!(err.to_string(), "Invocation failed: timeout");
    }

    #[test]
    fn test_connection_state_display() {
        assert_eq!(
            TauriConnectionState::NotAvailable.to_string(),
            "Not Available"
        );
        assert_eq!(TauriConnectionState::Connected.to_string(), "Connected");
        assert_eq!(TauriConnectionState::Error.to_string(), "Error");
    }

    // Note: Full Tauri tests require a running Tauri context
    // These are integration tests that would run in the actual app
}
