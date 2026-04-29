#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Restate client types for OYA Frontend integration.
//!
//! These types map to Restate's introspection API at localhost:9070/query

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Invocation status from Restate's `sys_invocation` table
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvocationStatus {
    #[default]
    Pending,
    Scheduled,
    Ready,
    Running,
    Paused,
    #[serde(rename = "backing-off")]
    BackingOff,
    Suspended,
    Completed,
}

impl InvocationStatus {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed)
    }

    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Pending
                | Self::Scheduled
                | Self::Ready
                | Self::Running
                | Self::BackingOff
                | Self::Suspended
        )
    }
}

/// Filter for listing invocations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationFilter {
    ActiveOnly,
    All,
}

impl InvocationFilter {
    #[must_use]
    pub const fn include_completed(self) -> bool {
        matches!(self, Self::All)
    }
}

/// Who invoked the handler
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvokedBy {
    Ingress,
    Service,
    Subscription,
    #[serde(rename = "restart_as_new")]
    RestartAsNew,
}

/// Service type in Restate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Service,
    #[serde(rename = "virtual_object")]
    VirtualObject,
    Workflow,
}

/// Invocation from `sys_invocation` table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    pub id: String,
    pub target: String,
    pub target_service_name: String,
    pub target_service_key: Option<String>,
    pub target_handler_name: String,
    pub target_service_ty: ServiceType,
    pub status: InvocationStatus,
    pub created_at: i64,
    pub modified_at: i64,
    pub completed_at: Option<i64>,
    pub journal_size: u32,
    pub retry_count: u64,
    pub invoked_by: InvokedBy,
    pub invoked_by_service_name: Option<String>,
    pub invoked_by_id: Option<String>,
    pub trace_id: Option<String>,
    pub last_failure: Option<String>,
    pub last_failure_error_code: Option<String>,
}

impl Invocation {
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.created_at).unwrap_or_else(|| {
            // Safe unwrap: timestamp 0 is always valid
            chrono::DateTime::from_timestamp(0, 0).unwrap_or(chrono::DateTime::UNIX_EPOCH)
        })
    }

    #[must_use]
    pub fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at.and_then(DateTime::from_timestamp_millis)
    }
}

/// Journal entry from `sys_journal` table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub index: u32,
    pub entry_type: JournalEntryType,
    pub raw_entry_type: String,
    pub name: Option<String>,
    pub completed: bool,
    pub invoked_id: Option<String>,
    pub invoked_target: Option<String>,
    pub sleep_wakeup_at: Option<i64>,
    pub promise_name: Option<String>,
    pub entry_json: Option<String>,
    pub entry_lite_json: Option<String>,
    pub appended_at: Option<i64>,
}

/// Journal entry types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JournalEntryType {
    Call,
    OneWayCall,
    Sleep,
    Awakeable,
    GetPromise,
    PeekPromise,
    CompletePromise,
    GetState,
    SetState,
    ClearState,
    ClearAll,
    Custom,
    Unknown(String),
}

impl From<&str> for JournalEntryType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "call" => Self::Call,
            "onewaycall" | "one-way-call" => Self::OneWayCall,
            "sleep" => Self::Sleep,
            "awakeable" => Self::Awakeable,
            "getpromise" | "get-promise" => Self::GetPromise,
            "peekpromise" | "peek-promise" => Self::PeekPromise,
            "completepromise" | "complete-promise" => Self::CompletePromise,
            "getstate" | "get-state" => Self::GetState,
            "setstate" | "set-state" => Self::SetState,
            "clearstate" | "clear-state" => Self::ClearState,
            "clearall" | "clear-all" => Self::ClearAll,
            "custom" => Self::Custom,
            other => Self::Unknown(other.to_string()),
        }
    }
}

/// State entry from state table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateEntry {
    pub service_name: String,
    pub service_key: Option<String>,
    pub key: String,
    pub value_utf8: Option<String>,
    pub value: Option<Vec<u8>>,
}

/// Service info from `sys_service` table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub ty: ServiceType,
    pub revision: u64,
    pub public: bool,
    pub deployment_id: String,
}

/// Deployment type in Restate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentType {
    Http,
    Lambda,
    Unknown,
}

impl From<&str> for DeploymentType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "http" => Self::Http,
            "lambda" => Self::Lambda,
            _ => Self::Unknown,
        }
    }
}

/// Deployment info from `sys_deployment` table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub id: String,
    pub ty: DeploymentType,
    pub raw_ty: String,
    pub endpoint: String,
    pub created_at: i64,
}

/// Virtual object status from `sys_keyed_service_status`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyedServiceStatus {
    pub service_name: String,
    pub service_key: String,
    pub invocation_id: String,
}

/// Promise info from `sys_promise`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseInfo {
    pub service_name: String,
    pub service_key: String,
    pub key: String,
    pub completed: bool,
    pub completion_success_value: Option<Vec<u8>>,
    pub completion_failure: Option<String>,
}

/// Journal event from `sys_journal_events`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEvent {
    pub id: String,
    pub after_journal_entry_index: u32,
    pub appended_at: i64,
    pub event_type: String,
    pub event_json: Option<String>,
}

/// Map Restate's `InvocationStatus` to OYA's `ExecutionState`
impl From<crate::graph::ExecutionState> for InvocationStatus {
    fn from(state: crate::graph::ExecutionState) -> Self {
        match state {
            crate::graph::ExecutionState::Idle => Self::Pending,
            crate::graph::ExecutionState::Queued => Self::Scheduled,
            crate::graph::ExecutionState::Running => Self::Running,
            crate::graph::ExecutionState::Completed | crate::graph::ExecutionState::Skipped => {
                Self::Completed
            }
            crate::graph::ExecutionState::Failed => Self::Paused,
        }
    }
}

impl From<InvocationStatus> for crate::graph::ExecutionState {
    fn from(status: InvocationStatus) -> Self {
        match status {
            InvocationStatus::Pending | InvocationStatus::Scheduled | InvocationStatus::Ready => {
                Self::Queued
            }
            InvocationStatus::Running
            | InvocationStatus::BackingOff
            | InvocationStatus::Suspended => Self::Running,
            InvocationStatus::Paused => Self::Failed,
            InvocationStatus::Completed => Self::Completed,
        }
    }
}

/// Admin API action on an invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvocationAction {
    Cancel,
    Kill,
    Pause,
    Resume,
    Purge,
}

/// Response from an invocation mutation action via the Admin API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvocationActionResponse {
    pub action: InvocationAction,
    pub invocation_id: String,
    pub success: bool,
}

/// SQL query response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlQueryResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Invocation with journal - full detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationDetail {
    #[serde(flatten)]
    pub invocation: Invocation,
    pub journal: Vec<JournalEntry>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::no_effect_underscore_binding
)]
mod tests {
    use super::*;
    use crate::canonical_ports::default_ingress_url;
    use chrono::TimeZone;

    #[test]
    fn invocation_status_pending_is_active() {
        assert!(InvocationStatus::Pending.is_active());
        assert!(!InvocationStatus::Pending.is_terminal());
    }

    #[test]
    fn invocation_status_completed_is_terminal() {
        assert!(InvocationStatus::Completed.is_terminal());
        assert!(!InvocationStatus::Completed.is_active());
    }

    #[test]
    fn invocation_status_running_is_active() {
        assert!(InvocationStatus::Running.is_active());
    }

    #[test]
    fn invocation_status_suspended_is_active() {
        assert!(InvocationStatus::Suspended.is_active());
    }

    #[test]
    fn journal_entry_type_from_str() {
        assert_eq!(JournalEntryType::from("call"), JournalEntryType::Call);
        assert_eq!(JournalEntryType::from("sleep"), JournalEntryType::Sleep);
        assert_eq!(JournalEntryType::from("GetState"), JournalEntryType::GetState);
    }

    #[test]
    fn journal_entry_type_unknown() {
        let unknown = JournalEntryType::from("someUnknownType");
        assert!(matches!(unknown, JournalEntryType::Unknown(s) if s == "someunknowntype"));
    }

    #[test]
    fn service_type_has_service_variant() {
        let _service = ServiceType::Service;
        // Just verify the variant exists - compile-time check
        assert_eq!(format!("{:?}", ServiceType::Service), "Service");
    }

    #[test]
    fn service_type_has_virtual_object_variant() {
        let _virtual_object = ServiceType::VirtualObject;
        assert_eq!(format!("{:?}", ServiceType::VirtualObject), "VirtualObject");
    }

    #[test]
    fn service_type_has_workflow_variant() {
        let _workflow = ServiceType::Workflow;
        assert_eq!(format!("{:?}", ServiceType::Workflow), "Workflow");
    }

    #[test]
    fn invoked_by_has_ingress_variant() {
        let _ingress = InvokedBy::Ingress;
        assert_eq!(format!("{:?}", InvokedBy::Ingress), "Ingress");
    }

    #[test]
    fn invoked_by_has_service_variant() {
        let _service = InvokedBy::Service;
        assert_eq!(format!("{:?}", InvokedBy::Service), "Service");
    }

    #[test]
    fn invoked_by_has_subscription_variant() {
        let _subscription = InvokedBy::Subscription;
        assert_eq!(format!("{:?}", InvokedBy::Subscription), "Subscription");
    }

    #[test]
    fn invoked_by_has_restart_as_new_variant() {
        let _restart_as_new = InvokedBy::RestartAsNew;
        assert_eq!(format!("{:?}", InvokedBy::RestartAsNew), "RestartAsNew");
    }

    #[test]
    fn invocation_status_all_variants() {
        assert!(InvocationStatus::Pending.is_active());
        assert!(InvocationStatus::Scheduled.is_active());
        assert!(InvocationStatus::Ready.is_active());
        assert!(InvocationStatus::Running.is_active());
        assert!(!InvocationStatus::Paused.is_active());
        assert!(InvocationStatus::BackingOff.is_active());
        assert!(InvocationStatus::Suspended.is_active());
        assert!(!InvocationStatus::Completed.is_active());

        assert!(!InvocationStatus::Pending.is_terminal());
        assert!(!InvocationStatus::Running.is_terminal());
        assert!(InvocationStatus::Completed.is_terminal());
    }

    #[test]
    fn invocation_status_scheduled_is_active() {
        assert!(InvocationStatus::Scheduled.is_active());
    }

    #[test]
    fn invocation_status_ready_is_active() {
        assert!(InvocationStatus::Ready.is_active());
    }

    #[test]
    fn invocation_status_paused_is_not_active() {
        assert!(!InvocationStatus::Paused.is_active());
    }

    #[test]
    fn invocation_status_backing_off_is_active() {
        assert!(InvocationStatus::BackingOff.is_active());
    }

    #[test]
    fn journal_entry_type_all_variants() {
        assert_eq!(JournalEntryType::from("call"), JournalEntryType::Call);
        assert_eq!(JournalEntryType::from("onewaycall"), JournalEntryType::OneWayCall);
        assert_eq!(JournalEntryType::from("one-way-call"), JournalEntryType::OneWayCall);
        assert_eq!(JournalEntryType::from("sleep"), JournalEntryType::Sleep);
        assert_eq!(JournalEntryType::from("awakeable"), JournalEntryType::Awakeable);
        assert_eq!(JournalEntryType::from("getpromise"), JournalEntryType::GetPromise);
        assert_eq!(JournalEntryType::from("peekpromise"), JournalEntryType::PeekPromise);
        assert_eq!(JournalEntryType::from("completepromise"), JournalEntryType::CompletePromise);
        assert_eq!(JournalEntryType::from("getstate"), JournalEntryType::GetState);
        assert_eq!(JournalEntryType::from("setstate"), JournalEntryType::SetState);
        assert_eq!(JournalEntryType::from("clearstate"), JournalEntryType::ClearState);
        assert_eq!(JournalEntryType::from("custom"), JournalEntryType::Custom);
    }

    #[test]
    fn journal_entry_type_case_insensitive() {
        assert_eq!(JournalEntryType::from("CALL"), JournalEntryType::Call);
        assert_eq!(JournalEntryType::from("SLEEP"), JournalEntryType::Sleep);
        assert_eq!(JournalEntryType::from("CaLl"), JournalEntryType::Call);
    }

    #[test]
    fn service_type_serialization() {
        use serde_json;

        let service = ServiceType::Service;
        let json = serde_json::to_string(&service).unwrap();
        assert!(json.contains("\"service\""));

        let virtual_object = ServiceType::VirtualObject;
        let json = serde_json::to_string(&virtual_object).unwrap();
        assert!(json.contains("\"virtual_object\""));

        let workflow = ServiceType::Workflow;
        let json = serde_json::to_string(&workflow).unwrap();
        assert!(json.contains("\"workflow\""));
    }

    #[test]
    fn invocation_status_serialization() {
        use serde_json;

        let status = InvocationStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"running\""));

        let status = InvocationStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"completed\""));
    }

    #[test]
    fn state_entry_default() {
        let entry = StateEntry {
            service_name: "TestService".to_string(),
            service_key: Some("key1".to_string()),
            key: "mykey".to_string(),
            value_utf8: Some("value".to_string()),
            value: Some(vec![1, 2, 3]),
        };
        assert_eq!(entry.service_name, "TestService");
        assert_eq!(entry.service_key, Some("key1".to_string()));
    }

    #[test]
    fn state_entry_optional_fields() {
        let entry = StateEntry {
            service_name: "TestService".to_string(),
            service_key: None,
            key: "mykey".to_string(),
            value_utf8: None,
            value: None,
        };
        assert_eq!(entry.service_key, None);
        assert_eq!(entry.value_utf8, None);
        assert_eq!(entry.value, None);
    }

    #[test]
    fn invocation_default() {
        let inv = Invocation {
            id: "inv_123".to_string(),
            target: "Service".to_string(),
            target_service_name: "Service".to_string(),
            target_service_key: None,
            target_handler_name: "handler".to_string(),
            target_service_ty: ServiceType::Service,
            status: InvocationStatus::Pending,
            created_at: 0,
            modified_at: 0,
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
        assert_eq!(inv.id, "inv_123");
        assert!(inv.completed_at.is_none());
    }

    #[test]
    fn journal_entry_default() {
        let entry = JournalEntry {
            id: "je_123".to_string(),
            index: 0,
            entry_type: JournalEntryType::Call,
            raw_entry_type: "call".to_string(),
            name: None,
            completed: false,
            invoked_id: None,
            invoked_target: None,
            sleep_wakeup_at: None,
            promise_name: None,
            entry_json: None,
            entry_lite_json: None,
            appended_at: None,
        };
        assert_eq!(entry.index, 0);
        assert!(!entry.completed);
        assert_eq!(entry.entry_type, JournalEntryType::Call);
    }

    #[test]
    fn service_info_default() {
        let info = ServiceInfo {
            name: "MyService".to_string(),
            ty: ServiceType::Service,
            revision: 1,
            public: true,
            deployment_id: "dep_123".to_string(),
        };
        assert_eq!(info.revision, 1);
        assert!(info.public);
    }

    #[test]
    fn deployment_info_default() {
        let info = DeploymentInfo {
            id: "dep_123".to_string(),
            ty: DeploymentType::Http,
            raw_ty: "http".to_string(),
            endpoint: default_ingress_url(),
            created_at: 1000,
        };
        assert_eq!(info.ty, DeploymentType::Http);
    }

    #[test]
    fn keyed_service_status_default() {
        let status = KeyedServiceStatus {
            service_name: "MyService".to_string(),
            service_key: "key1".to_string(),
            invocation_id: "inv_123".to_string(),
        };
        assert_eq!(status.service_key, "key1");
    }

    #[test]
    fn journal_event_default() {
        let event = JournalEvent {
            id: "je_123".to_string(),
            after_journal_entry_index: 5,
            appended_at: 1000,
            event_type: "entry".to_string(),
            event_json: None,
        };
        assert_eq!(event.after_journal_entry_index, 5);
    }

    #[test]
    fn promise_info_default() {
        let info = PromiseInfo {
            service_name: "MyService".to_string(),
            service_key: "key1".to_string(),
            key: "promise1".to_string(),
            completed: false,
            completion_success_value: None,
            completion_failure: None,
        };
        assert!(!info.completed);
    }

    #[test]
    fn sql_query_response_default() {
        let resp = SqlQueryResponse {
            columns: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![],
        };
        assert_eq!(resp.columns.len(), 2);
        assert!(resp.rows.is_empty());
    }

    #[test]
    fn invocation_detail_default() {
        let detail = InvocationDetail {
            invocation: Invocation {
                id: "inv_123".to_string(),
                target: "Service".to_string(),
                target_service_name: "Service".to_string(),
                target_service_key: None,
                target_handler_name: "handler".to_string(),
                target_service_ty: ServiceType::Service,
                status: InvocationStatus::Pending,
                created_at: 0,
                modified_at: 0,
                completed_at: None,
                journal_size: 0,
                retry_count: 0,
                invoked_by: InvokedBy::Ingress,
                invoked_by_service_name: None,
                invoked_by_id: None,
                trace_id: None,
                last_failure: None,
                last_failure_error_code: None,
            },
            journal: vec![],
        };
        assert!(detail.journal.is_empty());
    }

    #[test]
    fn invocation_status_deserialization() {
        use serde_json;

        let pending: InvocationStatus = serde_json::from_str("\"pending\"").unwrap();
        assert_eq!(pending, InvocationStatus::Pending);

        let running: InvocationStatus = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(running, InvocationStatus::Running);

        let completed: InvocationStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(completed, InvocationStatus::Completed);

        let backing_off: InvocationStatus = serde_json::from_str("\"backing-off\"").unwrap();
        assert_eq!(backing_off, InvocationStatus::BackingOff);
    }

    #[test]
    fn service_type_deserialization() {
        use serde_json;

        let service: ServiceType = serde_json::from_str("\"service\"").unwrap();
        assert_eq!(service, ServiceType::Service);

        let virtual_object: ServiceType = serde_json::from_str("\"virtual_object\"").unwrap();
        assert_eq!(virtual_object, ServiceType::VirtualObject);

        let workflow: ServiceType = serde_json::from_str("\"workflow\"").unwrap();
        assert_eq!(workflow, ServiceType::Workflow);
    }

    #[test]
    fn invoked_by_deserialization() {
        use serde_json;

        let ingress: InvokedBy = serde_json::from_str("\"ingress\"").unwrap();
        assert_eq!(ingress, InvokedBy::Ingress);

        let service: InvokedBy = serde_json::from_str("\"service\"").unwrap();
        assert_eq!(service, InvokedBy::Service);

        let subscription: InvokedBy = serde_json::from_str("\"subscription\"").unwrap();
        assert_eq!(subscription, InvokedBy::Subscription);

        let restart: InvokedBy = serde_json::from_str("\"restart_as_new\"").unwrap();
        assert_eq!(restart, InvokedBy::RestartAsNew);
    }

    #[test]
    fn invocation_serialization_roundtrip() {
        use serde_json;

        let inv = Invocation {
            id: "inv_123".to_string(),
            target: "Service".to_string(),
            target_service_name: "Service".to_string(),
            target_service_key: Some("key1".to_string()),
            target_handler_name: "handler".to_string(),
            target_service_ty: ServiceType::VirtualObject,
            status: InvocationStatus::Running,
            created_at: 1000,
            modified_at: 2000,
            completed_at: Some(3000),
            journal_size: 10,
            retry_count: 2,
            invoked_by: InvokedBy::Service,
            invoked_by_service_name: Some("Caller".to_string()),
            invoked_by_id: Some("inv_456".to_string()),
            trace_id: Some("trace_abc".to_string()),
            last_failure: Some("error".to_string()),
            last_failure_error_code: Some("ERR_001".to_string()),
        };

        let json = serde_json::to_string(&inv).unwrap();
        let restored: Invocation = serde_json::from_str(&json).unwrap();

        assert_eq!(inv.id, restored.id);
        assert_eq!(inv.target_service_key, restored.target_service_key);
        assert_eq!(inv.status, restored.status);
    }

    #[test]
    fn state_entry_serialization_roundtrip() {
        use serde_json;

        let entry = StateEntry {
            service_name: "MyService".to_string(),
            service_key: Some("key1".to_string()),
            key: "state_key".to_string(),
            value_utf8: Some("{\"foo\":\"bar\"}".to_string()),
            value: Some(vec![1, 2, 3, 4]),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let restored: StateEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.service_name, restored.service_name);
        assert_eq!(entry.key, restored.key);
    }

    #[test]
    fn journal_entry_type_serialization_roundtrip() {
        use serde_json;

        for entry_type in [
            JournalEntryType::Call,
            JournalEntryType::Sleep,
            JournalEntryType::GetState,
            JournalEntryType::SetState,
            JournalEntryType::Custom,
            JournalEntryType::Unknown("custom_type".to_string()),
        ] {
            let json = serde_json::to_string(&entry_type).unwrap();
            let restored: JournalEntryType = serde_json::from_str(&json).unwrap();
            assert_eq!(entry_type, restored);
        }
    }

    #[test]
    fn sql_query_response_with_data() {
        let resp = SqlQueryResponse {
            columns: vec!["id".to_string(), "name".to_string(), "status".to_string()],
            rows: vec![
                vec![
                    serde_json::json!("inv_1"),
                    serde_json::json!("Service1"),
                    serde_json::json!("running"),
                ],
                vec![
                    serde_json::json!("inv_2"),
                    serde_json::json!("Service2"),
                    serde_json::json!("completed"),
                ],
            ],
        };

        assert_eq!(resp.columns.len(), 3);
        assert_eq!(resp.rows.len(), 2);
        assert_eq!(resp.rows[0][0].as_str(), Some("inv_1"));
        assert_eq!(resp.rows[1][1].as_str(), Some("Service2"));
    }

    #[test]
    fn sql_query_response_empty() {
        let resp = SqlQueryResponse { columns: vec![], rows: vec![] };

        assert!(resp.columns.is_empty());
        assert!(resp.rows.is_empty());
    }

    #[test]
    fn journal_entry_with_all_fields() {
        let entry = JournalEntry {
            id: "je_123".to_string(),
            index: 5,
            entry_type: JournalEntryType::Call,
            raw_entry_type: "call".to_string(),
            name: Some("getUser".to_string()),
            completed: true,
            invoked_id: Some("inv_456".to_string()),
            invoked_target: Some("UserService".to_string()),
            sleep_wakeup_at: Some(10000),
            promise_name: Some("my_promise".to_string()),
            entry_json: Some("{\"key\":\"value\"}".to_string()),
            entry_lite_json: Some("{}".to_string()),
            appended_at: Some(5000),
        };

        assert_eq!(entry.index, 5);
        assert!(entry.completed);
        assert!(entry.name.is_some());
        assert_eq!(entry.entry_type, JournalEntryType::Call);
    }

    #[test]
    fn promise_info_with_completion() {
        let info = PromiseInfo {
            service_name: "Workflow".to_string(),
            service_key: "key1".to_string(),
            key: "promise1".to_string(),
            completed: true,
            completion_success_value: Some(vec![1, 2, 3]),
            completion_failure: None,
        };

        assert!(info.completed);
        assert!(info.completion_success_value.is_some());
        assert!(info.completion_failure.is_none());
    }

    #[test]
    fn promise_info_with_failure() {
        let info = PromiseInfo {
            service_name: "Workflow".to_string(),
            service_key: "key1".to_string(),
            key: "promise1".to_string(),
            completed: true,
            completion_success_value: None,
            completion_failure: Some("Something went wrong".to_string()),
        };

        assert!(info.completed);
        assert!(info.completion_failure.is_some());
    }

    #[test]
    fn invocation_started_at_returns_correct_timestamp() {
        use chrono::Utc;

        let inv = Invocation {
            id: "inv_123".to_string(),
            target: "Service".to_string(),
            target_service_name: "Service".to_string(),
            target_service_key: None,
            target_handler_name: "handler".to_string(),
            target_service_ty: ServiceType::Service,
            status: InvocationStatus::Pending,
            created_at: 1_700_000_000_000,
            modified_at: 1_700_000_000_000,
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

        let started_at = inv.started_at();
        let expected = Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
        assert_eq!(started_at, expected);
    }

    #[test]
    fn invocation_finished_at_with_value() {
        let inv = Invocation {
            id: "inv_123".to_string(),
            target: "Service".to_string(),
            target_service_name: "Service".to_string(),
            target_service_key: None,
            target_handler_name: "handler".to_string(),
            target_service_ty: ServiceType::Service,
            status: InvocationStatus::Completed,
            created_at: 1000,
            modified_at: 2000,
            completed_at: Some(3000),
            journal_size: 10,
            retry_count: 0,
            invoked_by: InvokedBy::Ingress,
            invoked_by_service_name: None,
            invoked_by_id: None,
            trace_id: None,
            last_failure: None,
            last_failure_error_code: None,
        };

        assert!(inv.finished_at().is_some());
    }

    #[test]
    fn invocation_finished_at_none_when_incomplete() {
        let inv = Invocation {
            id: "inv_123".to_string(),
            target: "Service".to_string(),
            target_service_name: "Service".to_string(),
            target_service_key: None,
            target_handler_name: "handler".to_string(),
            target_service_ty: ServiceType::Service,
            status: InvocationStatus::Running,
            created_at: 1000,
            modified_at: 2000,
            completed_at: None,
            journal_size: 5,
            retry_count: 0,
            invoked_by: InvokedBy::Ingress,
            invoked_by_service_name: None,
            invoked_by_id: None,
            trace_id: None,
            last_failure: None,
            last_failure_error_code: None,
        };

        assert!(inv.finished_at().is_none());
    }

    #[test]
    fn state_entry_with_binary_value() {
        let entry = StateEntry {
            service_name: "Service".to_string(),
            service_key: Some("key".to_string()),
            key: "binary".to_string(),
            value_utf8: None,
            value: Some(vec![0x00, 0x01, 0x02, 0xFF]),
        };

        assert!(entry.value_utf8.is_none());
        assert!(entry.value.is_some());
        assert_eq!(entry.value.unwrap().len(), 4);
    }

    #[test]
    fn keyed_service_status_empty_invocation() {
        let status = KeyedServiceStatus {
            service_name: "Service".to_string(),
            service_key: "key".to_string(),
            invocation_id: "".to_string(),
        };

        assert_eq!(status.invocation_id, "");
    }

    // --- InvocationAction tests (U-001..U-003) ---

    #[test]
    fn invocation_action_all_variants_serialized() {
        let cases = [
            (InvocationAction::Cancel, "\"cancel\""),
            (InvocationAction::Kill, "\"kill\""),
            (InvocationAction::Pause, "\"pause\""),
            (InvocationAction::Resume, "\"resume\""),
            (InvocationAction::Purge, "\"purge\""),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected, "InvocationAction::{variant:?} serialization mismatch");
        }
    }

    #[test]
    fn invocation_action_all_variants_deserialized() {
        let cases = [
            ("\"cancel\"", InvocationAction::Cancel),
            ("\"kill\"", InvocationAction::Kill),
            ("\"pause\"", InvocationAction::Pause),
            ("\"resume\"", InvocationAction::Resume),
            ("\"purge\"", InvocationAction::Purge),
        ];
        for (json, expected) in cases {
            let parsed: InvocationAction = serde_json::from_str(json).unwrap();
            assert_eq!(parsed, expected, "Deserialization mismatch for {json}");
        }
    }

    #[test]
    fn invocation_action_copy_semantics() {
        let a = InvocationAction::Cancel;
        let b = a;
        assert_eq!(a, b);
    }

    // --- InvocationActionResponse tests (U-004..U-007) ---

    #[test]
    fn invocation_action_response_success_construction() {
        let resp = InvocationActionResponse {
            action: InvocationAction::Cancel,
            invocation_id: "inv_123".to_string(),
            success: true,
        };
        assert_eq!(resp.action, InvocationAction::Cancel);
        assert_eq!(resp.invocation_id, "inv_123");
        assert!(resp.success);
    }

    #[test]
    fn invocation_action_response_roundtrip() {
        let original = InvocationActionResponse {
            action: InvocationAction::Kill,
            invocation_id: "inv_abc".to_string(),
            success: true,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: InvocationActionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn invocation_action_response_debug_format() {
        let resp = InvocationActionResponse {
            action: InvocationAction::Pause,
            invocation_id: "inv_xyz".to_string(),
            success: false,
        };
        let debug = format!("{resp:?}");
        assert!(debug.contains("action"), "Debug missing 'action'");
        assert!(debug.contains("invocation_id"), "Debug missing 'invocation_id'");
        assert!(debug.contains("success"), "Debug missing 'success'");
    }

    #[test]
    fn invocation_action_response_equality() {
        let a = InvocationActionResponse {
            action: InvocationAction::Resume,
            invocation_id: "inv_1".to_string(),
            success: true,
        };
        let b = InvocationActionResponse {
            action: InvocationAction::Resume,
            invocation_id: "inv_1".to_string(),
            success: true,
        };
        assert_eq!(a, b);
    }
}
