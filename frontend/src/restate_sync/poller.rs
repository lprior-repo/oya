use crate::restate_client::types::InvocationStatus as RestateInvocationStatus;
use crate::restate_client::{InvocationFilter, RestateClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
async fn sleep_ms(ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(u64::from(ms))).await;
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: u32) {
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InvocationEvent {
    New {
        invocation_id: String,
    },
    Completed {
        invocation_id: String,
        result: Option<String>,
    },
    Failed {
        invocation_id: String,
        error: String,
    },
    StatusChanged {
        invocation_id: String,
        old_status: InvocationStatus,
        new_status: InvocationStatus,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InvocationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Suspended,
}

impl From<RestateInvocationStatus> for InvocationStatus {
    fn from(status: RestateInvocationStatus) -> Self {
        match status {
            RestateInvocationStatus::Pending
            | RestateInvocationStatus::Scheduled
            | RestateInvocationStatus::Ready => Self::Pending,
            RestateInvocationStatus::Running | RestateInvocationStatus::BackingOff => Self::Running,
            RestateInvocationStatus::Paused => Self::Failed,
            RestateInvocationStatus::Suspended => Self::Suspended,
            RestateInvocationStatus::Completed => Self::Completed,
        }
    }
}

impl From<InvocationStatus> for RestateInvocationStatus {
    fn from(status: InvocationStatus) -> Self {
        match status {
            InvocationStatus::Pending => Self::Pending,
            InvocationStatus::Running => Self::Running,
            InvocationStatus::Completed => Self::Completed,
            InvocationStatus::Failed => Self::Paused,
            InvocationStatus::Suspended => Self::Suspended,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollResult {
    pub events: Vec<InvocationEvent>,
    pub timestamp: i64,
}

impl PollResult {
    #[must_use]
    pub fn new(events: Vec<InvocationEvent>) -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        Self { events, timestamp }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

/// Poller state machine - explicit tracking of known invocations
#[derive(Debug, Clone, Default)]
pub enum PollerState {
    #[default]
    Initial,
    Tracking(HashMap<String, crate::restate_client::types::Invocation>),
}

impl PollerState {
    #[must_use]
    pub const fn is_tracking(&self) -> bool {
        matches!(self, Self::Tracking(_))
    }

    #[must_use]
    pub fn get_tracked_invocation(
        &self,
        id: &str,
    ) -> Option<crate::restate_client::types::Invocation> {
        match self {
            Self::Initial => None,
            Self::Tracking(map) => map.get(id).cloned(),
        }
    }

    pub fn update(&mut self, inv: crate::restate_client::types::Invocation) {
        match self {
            Self::Initial => {
                *self = Self::Tracking(HashMap::from([(inv.id.clone(), inv)]));
            }
            Self::Tracking(map) => {
                map.insert(inv.id.clone(), inv);
            }
        }
    }

    #[must_use]
    pub fn tracked_ids(&self) -> Vec<String> {
        match self {
            Self::Initial => Vec::new(),
            Self::Tracking(map) => map.keys().cloned().collect(),
        }
    }

    #[must_use]
    pub fn invocations(&self) -> Vec<crate::restate_client::types::Invocation> {
        match self {
            Self::Initial => Vec::new(),
            Self::Tracking(map) => map.values().cloned().collect(),
        }
    }
}

pub struct InvocationPoller {
    client: Arc<RestateClient>,
    poll_interval_ms: u32,
    state: PollerState,
}

impl InvocationPoller {
    #[must_use]
    pub fn new(client: Arc<RestateClient>) -> Self {
        Self { client, poll_interval_ms: 5000, state: PollerState::default() }
    }

    #[must_use]
    pub fn with_interval(client: Arc<RestateClient>, poll_interval_ms: u32) -> Self {
        Self { client, poll_interval_ms, state: PollerState::default() }
    }

    #[must_use]
    pub const fn state(&self) -> &PollerState {
        &self.state
    }

    /// Start polling for invocation events.
    ///
    /// # Errors
    ///
    /// Returns `PollerError` if polling fails.
    pub async fn start_polling<F>(mut self, mut callback: F) -> Result<(), PollerError>
    where
        F: FnMut(PollResult) + Send + Sync,
    {
        loop {
            sleep_ms(self.poll_interval_ms).await;

            match self.poll().await {
                Ok(result) => {
                    if !result.events.is_empty() {
                        callback(result);
                    }
                }
                Err(e) => {
                    eprintln!("Polling error: {e:?}");
                }
            }
        }
    }

    /// Poll for invocation events.
    ///
    /// # Errors
    ///
    /// Returns `PollerError` if the request fails.
    pub async fn poll(&mut self) -> Result<PollResult, PollerError> {
        let invocations = self
            .client
            .list_invocations(InvocationFilter::All)
            .await
            .map_err(|e| PollerError::RequestError(e.to_string()))?;

        let mut events = Vec::new();
        let mut new_state = HashMap::new();

        for inv in invocations {
            let id = inv.id.clone();
            let new_status = InvocationStatus::from(inv.status);

            if let Some(old_inv) = self.state.get_tracked_invocation(&id) {
                let old_status = InvocationStatus::from(old_inv.status);
                if old_status != new_status {
                    events.push(InvocationEvent::StatusChanged {
                        invocation_id: id.clone(),
                        old_status,
                        new_status,
                    });

                    if new_status == InvocationStatus::Completed {
                        events.push(InvocationEvent::Completed {
                            invocation_id: id.clone(),
                            result: None,
                        });
                    } else if new_status == InvocationStatus::Failed {
                        events.push(InvocationEvent::Failed {
                            invocation_id: id.clone(),
                            error: inv
                                .last_failure
                                .clone()
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        });
                    }
                }
            } else {
                events.push(InvocationEvent::New { invocation_id: id.clone() });
            }

            new_state.insert(id, inv);
        }

        self.state = PollerState::Tracking(new_state);

        Ok(PollResult::new(events))
    }

    #[must_use]
    pub const fn client(&self) -> &Arc<RestateClient> {
        &self.client
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PollerError {
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::clone_on_copy
)]
mod tests {
    use super::*;

    #[test]
    fn test_invocation_event_serialization() {
        let event = InvocationEvent::New { invocation_id: "test-123".to_string() };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("new"));
        assert!(json.contains("test-123"));
        assert!(json.contains("invocation_id"));
    }

    #[test]
    fn test_invocation_status_equality() {
        assert_eq!(InvocationStatus::Pending, InvocationStatus::Pending);
        assert_eq!(InvocationStatus::Completed, InvocationStatus::Completed);
        assert_ne!(InvocationStatus::Pending, InvocationStatus::Completed);
    }

    #[test]
    fn test_poll_result_empty() {
        let result = PollResult::empty();
        assert!(result.events.is_empty());
        assert!(result.timestamp > 0);
    }

    #[test]
    fn test_poll_result_with_events() {
        let events = vec![
            InvocationEvent::New { invocation_id: "inv-1".to_string() },
            InvocationEvent::Completed {
                invocation_id: "inv-2".to_string(),
                result: Some("success".to_string()),
            },
        ];
        let result = PollResult::new(events);
        assert_eq!(result.events.len(), 2);
        assert!(result.timestamp > 0);
    }

    #[test]
    fn test_invocation_event_completed_with_result() {
        let event = InvocationEvent::Completed {
            invocation_id: "inv-123".to_string(),
            result: Some("output data".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("completed"));
        assert!(json.contains("inv-123"));
        assert!(json.contains("output data"));
    }

    #[test]
    fn test_invocation_event_failed_with_error() {
        let event = InvocationEvent::Failed {
            invocation_id: "inv-456".to_string(),
            error: "Something went wrong".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("failed"));
        assert!(json.contains("inv-456"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_invocation_event_status_changed() {
        let event = InvocationEvent::StatusChanged {
            invocation_id: "inv-789".to_string(),
            old_status: InvocationStatus::Running,
            new_status: InvocationStatus::Completed,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("statusChanged"));
        assert!(json.contains("inv-789"));
    }

    #[test]
    fn test_all_invocation_statuses() {
        let statuses = vec![
            InvocationStatus::Pending,
            InvocationStatus::Running,
            InvocationStatus::Completed,
            InvocationStatus::Failed,
            InvocationStatus::Suspended,
        ];

        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_poll_result_timestamp_consistency() {
        let before = chrono::Utc::now().timestamp();
        let result = PollResult::empty();
        let after = chrono::Utc::now().timestamp();

        assert!(result.timestamp >= before);
        assert!(result.timestamp <= after);
    }

    #[test]
    fn test_poll_result_deserialization() {
        let json = r#"{"events":[{"new":{"invocation_id":"test-id"}}],"timestamp":1234567890}"#;
        let result: PollResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.timestamp, 1234567890);
    }

    #[test]
    fn test_poller_error_display() {
        let err = PollerError::RequestError("connection timeout".to_string());
        assert_eq!(err.to_string(), "Request error: connection timeout");

        let err = PollerError::ParseError("invalid json".to_string());
        assert_eq!(err.to_string(), "Parse error: invalid json");
    }

    #[test]
    fn test_invocation_event_clone() {
        let event = InvocationEvent::New { invocation_id: "test".to_string() };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn test_invocation_status_clone() {
        let status = InvocationStatus::Running;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_empty_invocation_id_edge_case() {
        let event = InvocationEvent::New { invocation_id: "".to_string() };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("invocation_id"));
    }

    #[test]
    fn test_large_error_message() {
        let long_error = "x".repeat(10000);
        let event = InvocationEvent::Failed {
            invocation_id: "inv-1".to_string(),
            error: long_error.clone(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: InvocationEvent = serde_json::from_str(&json).unwrap();
        if let InvocationEvent::Failed { error, .. } = parsed {
            assert_eq!(error.len(), 10000);
        } else {
            panic!("Expected Failed event");
        }
    }

    #[test]
    fn test_multiple_status_changes_serialization() {
        let events = vec![
            InvocationEvent::StatusChanged {
                invocation_id: "inv-1".to_string(),
                old_status: InvocationStatus::Pending,
                new_status: InvocationStatus::Running,
            },
            InvocationEvent::StatusChanged {
                invocation_id: "inv-1".to_string(),
                old_status: InvocationStatus::Running,
                new_status: InvocationStatus::Completed,
            },
        ];
        let result = PollResult::new(events);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("statusChanged"));
    }

    #[test]
    fn test_poll_result_timestamp_type() {
        let result = PollResult::empty();
        assert!(result.timestamp.is_positive());
    }

    #[test]
    fn test_invocation_status_suspended() {
        let status = InvocationStatus::Suspended;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("suspended"));
    }

    #[test]
    fn test_hashset_behavior() {
        use std::collections::HashSet;

        let mut ids: HashSet<String> = HashSet::new();
        assert!(ids.insert("inv-1".to_string()));
        assert!(!ids.insert("inv-1".to_string()));
        assert!(ids.contains("inv-1"));
        assert!(!ids.contains("inv-2"));
    }

    #[test]
    fn test_poller_transparency_logic() {
        use crate::restate_client::types::{
            Invocation, InvocationStatus as RestateStatus, InvokedBy, ServiceType,
        };

        let mut state;

        let inv1 = Invocation {
            id: "inv-1".to_string(),
            target: "wf".to_string(),
            target_service_name: "wf".to_string(),
            target_service_key: None,
            target_handler_name: "run".to_string(),
            target_service_ty: ServiceType::Workflow,
            status: RestateStatus::Pending,
            created_at: 1000,
            modified_at: 1000,
            completed_at: None,
            journal_size: 0,
            retry_count: 0,
            invoked_by: InvokedBy::Ingress,
            invoked_by_service_name: None,
            invoked_by_id: None,
            trace_id: None,
            last_failure: None,
            last_failure_error_code: None,
        };

        // 1. Initial state -> New invocation
        let mut new_state = HashMap::new();
        new_state.insert(inv1.id.clone(), inv1.clone());
        state = PollerState::Tracking(new_state);

        assert!(state.is_tracking());
        assert_eq!(state.tracked_ids(), vec!["inv-1".to_string()]);

        // 2. Status change
        let mut inv1_updated = inv1.clone();
        inv1_updated.status = RestateStatus::Running;
        inv1_updated.modified_at = 2000;

        if let PollerState::Tracking(ref mut map) = state {
            map.insert(inv1_updated.id.clone(), inv1_updated.clone());
        }

        let tracked = state.get_tracked_invocation("inv-1").expect("should be tracked");
        assert_eq!(tracked.status, RestateStatus::Running);
        assert_eq!(tracked.modified_at, 2000);
    }
}
