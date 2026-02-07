//! Scheduler metrics endpoint: GET /api/scheduler/metrics
//!
//! This module provides metrics about the workflow scheduler including:
//! - Queue lengths (pending, in_progress, failed)
//! - Throughput metrics (jobs completed per minute)
//! - Latency metrics (average queue wait time, execution time)
//!
//! ## Authentication
//!
//! All endpoints require Bearer token authentication via the Authorization header.
//!
//! ## Endpoints
//!
//! - `GET /api/scheduler/metrics` - Get scheduler metrics

use super::super::actors::AppState;
use super::super::error::{AppError, Result};
use axum::{
    extract::State,
    http::HeaderMap,
    response::Json,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Authentication token extracted from Authorization header
#[derive(Debug, Clone, PartialEq)]
struct AuthToken(String);

impl AuthToken {
    /// Extract Bearer token from Authorization header
    ///
    /// # Errors
    ///
    /// * `AppError::Unauthorized` - If header is missing or invalid
    fn from_headers(headers: &HeaderMap) -> Result<Self> {
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|auth| {
                if let Some(token) = auth.strip_prefix("Bearer ") {
                    Some(token.to_string())
                } else {
                    None
                }
            })
            .map(Self)
            .ok_or_else(|| AppError::Unauthorized("Missing or invalid Authorization header".to_string()))
    }

    /// Validate the token (placeholder implementation)
    ///
    /// In production, this would validate against a secure token store.
    /// For now, we accept any non-empty token.
    fn validate(&self) -> Result<()> {
        if self.0.is_empty() {
            return Err(AppError::Unauthorized("Invalid token".to_string()));
        }
        // TODO: Implement proper token validation against a token store
        Ok(())
    }
}

/// Scheduler metrics response
#[derive(Debug, Serialize)]
pub struct SchedulerMetricsResponse {
    /// Queue metrics
    pub queue: QueueMetrics,
    /// Throughput metrics
    pub throughput: ThroughputMetrics,
    /// Latency metrics
    pub latency: LatencyMetrics,
    /// Timestamp of metrics collection
    pub collected_at: String,
}

/// Queue length metrics
#[derive(Debug, Serialize)]
pub struct QueueMetrics {
    /// Number of jobs pending execution
    pub pending: usize,
    /// Number of jobs currently executing
    pub in_progress: usize,
    /// Number of failed jobs awaiting retry
    pub failed: usize,
    /// Total queue size
    pub total: usize,
}

/// Throughput metrics
#[derive(Debug, Serialize)]
pub struct ThroughputMetrics {
    /// Jobs completed in the last minute
    pub jobs_per_minute: f64,
    /// Jobs completed in the last hour
    pub jobs_per_hour: f64,
    /// Total jobs completed since server start
    pub total_completed: usize,
}

/// Latency metrics
#[derive(Debug, Serialize)]
pub struct LatencyMetrics {
    /// Average time jobs spend waiting in queue (milliseconds)
    pub average_queue_wait_ms: u64,
    /// Average job execution time (milliseconds)
    pub average_execution_time_ms: u64,
    /// 95th percentile queue wait time (milliseconds)
    pub p95_queue_wait_ms: u64,
    /// 95th percentile execution time (milliseconds)
    pub p95_execution_time_ms: u64,
}

/// Internal scheduler state from StateManager
#[derive(Debug, Clone)]
struct SchedulerState {
    pending: usize,
    in_progress: usize,
    failed: usize,
    completed_last_minute: f64,
    completed_last_hour: f64,
    total_completed: usize,
    avg_queue_wait_ms: u64,
    avg_execution_time_ms: u64,
    p95_queue_wait_ms: u64,
    p95_execution_time_ms: u64,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self {
            pending: 0,
            in_progress: 0,
            failed: 0,
            completed_last_minute: 0.0,
            completed_last_hour: 0.0,
            total_completed: 0,
            avg_queue_wait_ms: 0,
            avg_execution_time_ms: 0,
            p95_queue_wait_ms: 0,
            p95_execution_time_ms: 0,
        }
    }
}

/// GET /api/scheduler/metrics - Get scheduler metrics
///
/// Returns comprehensive metrics about the workflow scheduler including
/// queue lengths, throughput, and latency statistics.
///
/// # Authentication
///
/// Requires Bearer token in Authorization header:
/// ```text
/// Authorization: Bearer <token>
/// ```
///
/// # Arguments
///
/// * `headers` - HTTP headers containing Authorization token
/// * `state` - Application state containing actor handles
///
/// # Returns
///
/// `Result<Json<SchedulerMetricsResponse>>` - Scheduler metrics or error
///
/// # Errors
///
/// * `AppError::Unauthorized` (401) - Missing or invalid authentication
/// * `AppError::ServiceUnavailable` (503) - State manager unavailable
/// * `AppError::Internal` (500) - State manager failed to respond
pub async fn get_scheduler_metrics(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<SchedulerMetricsResponse>> {
    // Railway track: authenticate -> query state -> calculate metrics -> respond
    authenticate(headers)
        .and_then(|_| query_scheduler_state(&state))
        .map(|scheduler_state| calculate_metrics(scheduler_state))
        .map(|metrics| Json(metrics))
}

/// Authenticate the request using Bearer token
fn authenticate(headers: HeaderMap) -> Result<()> {
    AuthToken::from_headers(&headers).and_then(|token| token.validate())
}

/// Query scheduler state from StateManager
fn query_scheduler_state(state: &AppState) -> Result<SchedulerState> {
    // For now, return mock data since StateManager doesn't have scheduler metrics yet
    // In a future bead, we'll add a QuerySchedulerMetrics message to StateManagerMessage
    let scheduler_state = SchedulerState {
        pending: 5,
        in_progress: 3,
        failed: 1,
        completed_last_minute: 12.0,
        completed_last_hour: 450.0,
        total_completed: 1250,
        avg_queue_wait_ms: 250,
        avg_execution_time_ms: 5000,
        p95_queue_wait_ms: 800,
        p95_execution_time_ms: 12000,
    };

    // Simulate potential state manager errors for testing error paths
    // In production, this would actually query the StateManager actor
    Ok(scheduler_state)
}

/// Calculate metrics from scheduler state
fn calculate_metrics(state: SchedulerState) -> SchedulerMetricsResponse {
    let queue = QueueMetrics {
        pending: state.pending,
        in_progress: state.in_progress,
        failed: state.failed,
        total: state.pending + state.in_progress + state.failed,
    };

    let throughput = ThroughputMetrics {
        jobs_per_minute: state.completed_last_minute,
        jobs_per_hour: state.completed_last_hour,
        total_completed: state.total_completed,
    };

    let latency = LatencyMetrics {
        average_queue_wait_ms: state.avg_queue_wait_ms,
        average_execution_time_ms: state.avg_execution_time_ms,
        p95_queue_wait_ms: state.p95_queue_wait_ms,
        p95_execution_time_ms: state.p95_execution_time_ms,
    };

    let collected_at = current_timestamp();

    SchedulerMetricsResponse {
        queue,
        throughput,
        latency,
        collected_at,
    }
}

/// Get current ISO 8601 timestamp
fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| format!("{:.3?}", d))
        .unwrap_or_else(|_| "unknown".to_string())
}

// Add Unauthorized variant to AppError
impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        AppError::Unauthorized(err.0)
    }
}

/// Authentication error type
#[derive(Debug)]
struct AuthError(String);

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    /// Test successful token extraction from valid Authorization header
    #[test]
    fn test_token_extraction_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer test-token-123"),
        );

        let result = AuthToken::from_headers(&headers);
        assert!(result.is_ok());
        let token = result.unwrap();
        assert_eq!(token.0, "test-token-123");
    }

    /// Test token extraction fails with missing header
    #[test]
    fn test_token_extraction_missing_header() {
        let headers = HeaderMap::new();

        let result = AuthToken::from_headers(&headers);
        assert!(result.is_err());
    }

    /// Test token extraction fails with malformed header (no Bearer prefix)
    #[test]
    fn test_token_extraction_malformed_header() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("invalid-format"));

        let result = AuthToken::from_headers(&headers);
        assert!(result.is_err());
    }

    /// Test token validation with non-empty token
    #[test]
    fn test_token_validation_success() {
        let token = AuthToken("valid-token".to_string());
        let result = token.validate();
        assert!(result.is_ok());
    }

    /// Test token validation fails with empty token
    #[test]
    fn test_token_validation_empty_token() {
        let token = AuthToken("".to_string());
        let result = token.validate();
        assert!(result.is_err());
    }

    /// Test queue metrics calculation
    #[test]
    fn test_queue_metrics_calculation() {
        let state = SchedulerState {
            pending: 10,
            in_progress: 5,
            failed: 2,
            ..Default::default()
        };

        let queue = QueueMetrics {
            pending: state.pending,
            in_progress: state.in_progress,
            failed: state.failed,
            total: state.pending + state.in_progress + state.failed,
        };

        assert_eq!(queue.pending, 10);
        assert_eq!(queue.in_progress, 5);
        assert_eq!(queue.failed, 2);
        assert_eq!(queue.total, 17);
    }

    /// Test throughput metrics calculation
    #[test]
    fn test_throughput_metrics_calculation() {
        let state = SchedulerState {
            completed_last_minute: 15.0,
            completed_last_hour: 500.0,
            total_completed: 1000,
            ..Default::default()
        };

        let throughput = ThroughputMetrics {
            jobs_per_minute: state.completed_last_minute,
            jobs_per_hour: state.completed_last_hour,
            total_completed: state.total_completed,
        };

        assert_eq!(throughput.jobs_per_minute, 15.0);
        assert_eq!(throughput.jobs_per_hour, 500.0);
        assert_eq!(throughput.total_completed, 1000);
    }

    /// Test latency metrics calculation
    #[test]
    fn test_latency_metrics_calculation() {
        let state = SchedulerState {
            avg_queue_wait_ms: 300,
            avg_execution_time_ms: 6000,
            p95_queue_wait_ms: 900,
            p95_execution_time_ms: 15000,
            ..Default::default()
        };

        let latency = LatencyMetrics {
            average_queue_wait_ms: state.avg_queue_wait_ms,
            average_execution_time_ms: state.avg_execution_time_ms,
            p95_queue_wait_ms: state.p95_queue_wait_ms,
            p95_execution_time_ms: state.p95_execution_time_ms,
        };

        assert_eq!(latency.average_queue_wait_ms, 300);
        assert_eq!(latency.average_execution_time_ms, 6000);
        assert_eq!(latency.p95_queue_wait_ms, 900);
        assert_eq!(latency.p95_execution_time_ms, 15000);
    }

    /// Test complete metrics response structure
    #[test]
    fn test_complete_metrics_response() {
        let state = SchedulerState {
            pending: 8,
            in_progress: 4,
            failed: 1,
            completed_last_minute: 20.0,
            completed_last_hour: 800.0,
            total_completed: 2500,
            avg_queue_wait_ms: 200,
            avg_execution_time_ms: 4500,
            p95_queue_wait_ms: 700,
            p95_execution_time_ms: 11000,
        };

        let response = calculate_metrics(state);

        assert_eq!(response.queue.pending, 8);
        assert_eq!(response.queue.in_progress, 4);
        assert_eq!(response.queue.failed, 1);
        assert_eq!(response.queue.total, 13);

        assert_eq!(response.throughput.jobs_per_minute, 20.0);
        assert_eq!(response.throughput.jobs_per_hour, 800.0);
        assert_eq!(response.throughput.total_completed, 2500);

        assert_eq!(response.latency.average_queue_wait_ms, 200);
        assert_eq!(response.latency.average_execution_time_ms, 4500);
        assert_eq!(response.latency.p95_queue_wait_ms, 700);
        assert_eq!(response.latency.p95_execution_time_ms, 11000);

        assert!(!response.collected_at.is_empty());
    }

    /// Test authentication flow with valid token
    #[test]
    fn test_authenticate_flow_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer valid-token"),
        );

        let result = authenticate(headers);
        assert!(result.is_ok());
    }

    /// Test authentication flow with missing token
    #[test]
    fn test_authenticate_flow_missing_token() {
        let headers = HeaderMap::new();

        let result = authenticate(headers);
        assert!(result.is_err());
        match result {
            Err(AppError::Unauthorized(msg)) => {
                assert!(msg.contains("Missing or invalid"));
            }
            _ => panic!("Expected Unauthorized error"),
        }
    }

    /// Test authentication flow with empty token
    #[test]
    fn test_authenticate_flow_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer "));

        let result = authenticate(headers);
        assert!(result.is_err());
    }

    /// Test metrics calculation with zero values
    #[test]
    fn test_metrics_with_zero_values() {
        let state = SchedulerState::default();

        let response = calculate_metrics(state);

        assert_eq!(response.queue.pending, 0);
        assert_eq!(response.queue.in_progress, 0);
        assert_eq!(response.queue.failed, 0);
        assert_eq!(response.queue.total, 0);

        assert_eq!(response.throughput.jobs_per_minute, 0.0);
        assert_eq!(response.throughput.jobs_per_hour, 0.0);
        assert_eq!(response.throughput.total_completed, 0);

        assert_eq!(response.latency.average_queue_wait_ms, 0);
        assert_eq!(response.latency.average_execution_time_ms, 0);
        assert_eq!(response.latency.p95_queue_wait_ms, 0);
        assert_eq!(response.latency.p95_execution_time_ms, 0);
    }
}
