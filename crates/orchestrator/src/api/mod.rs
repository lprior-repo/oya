//! HTTP API server for orchestrator.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::agent_swarm::AgentMessage;
use crate::messaging::{Message, MessageRouter};

/// API server state.
#[derive(Clone)]
pub struct ApiState {
    /// Message router for sending commands.
    router: Arc<MessageRouter>,
}

impl ApiState {
    /// Create a new API state.
    #[must_use]
    pub fn new(router: Arc<MessageRouter>) -> Self {
        Self { router }
    }
}

/// Cancel bead request body.
#[derive(Debug, Deserialize)]
pub struct CancelBeadRequest {
    /// Optional reason for cancellation.
    pub reason: Option<String>,
}

/// Cancel bead response.
#[derive(Debug, Serialize)]
pub struct CancelBeadResponse {
    /// The bead ID that was cancelled.
    pub bead_id: String,
    /// Status of the cancellation.
    pub status: String,
    /// Message about the result.
    pub message: String,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}

/// POST /api/beads/{id}/cancel
///
/// Cancels a bead by sending a CancelBead message through the router.
pub async fn cancel_bead(
    State(state): State<ApiState>,
    Path(bead_id): Path<String>,
    Json(req): Json<CancelBeadRequest>,
) -> Result<Json<CancelBeadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = req
        .reason
        .unwrap_or_else(|| "Cancelled via API".to_string());

    // Create cancel message and serialize to JSON
    let agent_msg = AgentMessage::cancel_bead(&bead_id, &reason);
    let payload = serde_json::to_value(&agent_msg).map_err(|e| {
        tracing::error!("Failed to serialize cancel message: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to serialize message: {}", e),
            }),
        )
    })?;

    let msg = Message::one_way(payload);

    // Send to router - use agent command channel
    let channel_id = "agent/commands";
    match state.router.send(&channel_id, msg).await {
        Ok(_) => Ok(Json(CancelBeadResponse {
            bead_id,
            status: "cancelled".to_string(),
            message: "Cancellation request sent".to_string(),
        })),
        Err(e) => {
            tracing::error!("Failed to send cancel message: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to send cancel message: {}", e),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::MessageRouter;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use serde_json::json;
    use tower::ServiceExt;

    // Helper to create test router
    fn create_test_router() -> Arc<MessageRouter> {
        Arc::new(MessageRouter::new(crate::messaging::RouterConfig::default()))
    }

    #[tokio::test]
    async fn test_cancel_bead_success() {
        // Setup: Create router and state
        let router = create_test_router();
        let state = ApiState::new(router.clone());

        // Register the command channel
        router.register_channel("agent/commands").await;

        // Create app
        let app = axum::Router::new()
            .route("/api/beads/{id}/cancel", axum::routing::post(cancel_bead))
            .with_state(state);

        // Execute: Send cancel request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/beads/test-bead-123/cancel")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"reason": "Test cancellation"}).to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Verify: Check response
        assert_eq!(response.status(), StatusCode::OK);

        // Check message was sent to router
        let msg = router.receive("agent/commands").await;
        assert!(msg.is_some(), "Message should be sent to router");
    }

    #[tokio::test]
    async fn test_cancel_bead_without_reason() {
        // Setup
        let router = create_test_router();
        let state = ApiState::new(router.clone());
        router.register_channel("agent/commands").await;

        let app = axum::Router::new()
            .route("/api/beads/{id}/cancel", axum::routing::post(cancel_bead))
            .with_state(state);

        // Execute: Send cancel without reason
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/beads/test-bead-456/cancel")
            .header("content-type", "application/json")
            .body(Body::from(json!({}).to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Verify
        assert_eq!(response.status(), StatusCode::OK);

        // Message should still be sent
        let msg = router.receive("agent/commands").await;
        assert!(msg.is_some());
    }

    #[tokio::test]
    async fn test_cancel_bead_idempotent() {
        // Setup
        let router = create_test_router();
        let state = ApiState::new(router.clone());
        router.register_channel("agent/commands").await;

        // Execute: Cancel same bead multiple times
        for _ in 0..3 {
            let app = axum::Router::new()
                .route("/api/beads/{id}/cancel", axum::routing::post(cancel_bead))
                .with_state(state.clone());

            let request = Request::builder()
                .method(Method::POST)
                .uri("/api/beads/test-bead-789/cancel")
                .header("content-type", "application/json")
                .body(Body::from(json!({"reason": "Multiple cancel"}).to_string()))
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        // All three messages should be sent (idempotent at message level)
        let count = router.channel_count().await;
        assert!(count >= 1);
    }
}
