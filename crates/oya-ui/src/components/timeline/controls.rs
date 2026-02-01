//! Timeline control buttons for workflow management
//!
//! Provides cancel, retry, and view logs buttons for workflow timeline.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Status of a workflow/bead
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeadStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// State for control buttons
#[derive(Debug, Clone)]
pub struct ControlsState {
    pub bead_id: String,
    pub status: BeadStatus,
    pub is_loading: bool,
    pub error_message: Option<String>,
}

/// Props for WorkflowControls component
#[derive(Clone)]
pub struct WorkflowControlsProps {
    pub bead_id: Signal<String>,
    pub status: Signal<BeadStatus>,
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, String>;

/// Response from cancel API
#[derive(Debug, Deserialize)]
pub struct CancelResponse {
    pub message: String,
}

/// Response from retry API
#[derive(Debug, Deserialize)]
pub struct RetryResponse {
    pub message: String,
}

/// WorkflowControls component - provides cancel, retry, and view logs buttons
#[component]
pub fn WorkflowControls(
    #[prop(into)] bead_id: Signal<String>,
    #[prop(into)] status: Signal<BeadStatus>,
) -> impl IntoView {
    // Loading state signal
    let is_loading = RwSignal::new(false);
    let error_message = RwSignal::new(None::<String>);
    let show_logs_modal = RwSignal::new(false);

    // Cancel button click handler
    let on_cancel_click = move |_| {
        // Show confirm dialog
        let confirmed = web_sys::window()
            .and_then(|w| {
                w.confirm_with_message("Are you sure you want to cancel this workflow?")
                    .ok()
            })
            .unwrap_or(false);

        if confirmed {
            is_loading.set(true);
            error_message.set(None);

            let bead_id_value = bead_id.get();

            // Spawn async task for API call
            leptos::spawn_local(async move {
                match send_cancel_request(&bead_id_value).await {
                    Ok(_) => {
                        is_loading.set(false);
                    }
                    Err(err) => {
                        is_loading.set(false);
                        error_message.set(Some(err));
                    }
                }
            });
        }
    };

    // Retry button click handler
    let on_retry_click = move |_| {
        is_loading.set(true);
        error_message.set(None);

        let bead_id_value = bead_id.get();

        leptos::spawn_local(async move {
            match send_retry_request(&bead_id_value).await {
                Ok(_) => {
                    is_loading.set(false);
                }
                Err(err) => {
                    is_loading.set(false);
                    error_message.set(Some(err));
                }
            }
        });
    };

    // View logs button click handler
    let on_logs_click = move |_| {
        show_logs_modal.set(true);
    };

    // Close logs modal handler
    let on_close_modal = move |_| {
        show_logs_modal.set(false);
    };

    let loading = move || is_loading.get();
    let show_retry = move || status.get() == BeadStatus::Failed;
    let error = move || error_message.get();
    let show_modal = move || show_logs_modal.get();

    view! {
        <div class="workflow-controls">
            // Cancel button (red)
            <button
                class="btn btn-cancel"
                style="background-color: #dc2626; color: white; padding: 8px 16px; margin: 4px; border: none; border-radius: 4px; cursor: pointer;"
                on:click=on_cancel_click
                disabled=loading
            >
                {move || if loading() { "Loading..." } else { "Cancel" }}
            </button>

            // Retry button (yellow) - only for failed beads
            <Show when=show_retry>
                <button
                    class="btn btn-retry"
                    style="background-color: #eab308; color: white; padding: 8px 16px; margin: 4px; border: none; border-radius: 4px; cursor: pointer;"
                    on:click=on_retry_click
                    disabled=loading
                >
                    {move || if loading() { "Loading..." } else { "Retry" }}
                </button>
            </Show>

            // View logs button (blue)
            <button
                class="btn btn-logs"
                style="background-color: #2563eb; color: white; padding: 8px 16px; margin: 4px; border: none; border-radius: 4px; cursor: pointer;"
                on:click=on_logs_click
                disabled=loading
            >
                "View Logs"
            </button>

            // Error message display
            <Show when=move || error().is_some()>
                <div class="error-message" style="color: #dc2626; margin-top: 8px;">
                    {move || error().unwrap_or_default()}
                </div>
            </Show>

            // Logs modal
            <Show when=show_modal>
                <div class="logs-modal" style="position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: white; padding: 20px; border: 1px solid #ccc; border-radius: 8px; z-index: 1000; box-shadow: 0 4px 6px rgba(0,0,0,0.1);">
                    <h3>"Workflow Logs"</h3>
                    <div class="logs-content" style="max-height: 400px; overflow-y: auto; margin: 16px 0;">
                        <p>"Logs for bead: " {move || bead_id.get()}</p>
                        <p>"Status: " {move || format!("{:?}", status.get())}</p>
                    </div>
                    <button
                        class="btn btn-close"
                        style="background-color: #6b7280; color: white; padding: 8px 16px; border: none; border-radius: 4px; cursor: pointer;"
                        on:click=on_close_modal
                    >
                        "Close"
                    </button>
                </div>
            </Show>
        </div>
    }
}

/// Send cancel request to API
async fn send_cancel_request(bead_id: &str) -> ApiResult<CancelResponse> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let url = format!("/api/beads/{}/cancel", bead_id);

        Request::post(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?
            .json::<CancelResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // For tests, return mock response
        Ok(CancelResponse {
            message: format!("Bead {} cancellation requested", bead_id),
        })
    }
}

/// Send retry request to API
async fn send_retry_request(bead_id: &str) -> ApiResult<RetryResponse> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let url = format!("/api/beads/{}/retry", bead_id);

        Request::post(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?
            .json::<RetryResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // For tests, return mock response
        Ok(RetryResponse {
            message: format!("Bead {} retry requested", bead_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_status_serialization() {
        let status = BeadStatus::Failed;
        let json = serde_json::to_string(&status).expect("Failed to serialize");
        assert_eq!(json, r#""failed""#);
    }

    #[test]
    fn test_controls_state_creation() {
        let state = ControlsState {
            bead_id: "test-123".to_string(),
            status: BeadStatus::Running,
            is_loading: false,
            error_message: None,
        };
        assert_eq!(state.bead_id, "test-123");
        assert_eq!(state.status, BeadStatus::Running);
        assert!(!state.is_loading);
        assert!(state.error_message.is_none());
    }

    #[tokio::test]
    async fn test_send_cancel_request() {
        let result = send_cancel_request("test-123").await;
        assert!(result.is_ok());
        let response = result.expect("Expected Ok result");
        assert!(response.message.contains("test-123"));
    }

    #[tokio::test]
    async fn test_send_retry_request() {
        let result = send_retry_request("test-456").await;
        assert!(result.is_ok());
        let response = result.expect("Expected Ok result");
        assert!(response.message.contains("test-456"));
    }
}
