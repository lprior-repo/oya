#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! SQL queries for Restate introspection API.

use crate::restate_client::types::{InvocationFilter, InvocationStatus};

const INVOCATION_PROJECTION: &str =
    "id, target, target_service_name, target_service_key, target_handler_name, target_service_ty, status, created_at, modified_at, completed_at, journal_size, retry_count, invoked_by, invoked_by_service_name, invoked_by_id, trace_id, last_failure, last_failure_error_code";

/// Pre-built SQL queries for Restate introspection.
pub struct SqlQueries;

impl SqlQueries {
    /// List all invocations.
    #[must_use]
    pub fn list_invocations(filter: InvocationFilter) -> String {
        if filter.include_completed() {
            format!("SELECT {INVOCATION_PROJECTION} FROM sys_invocation ORDER BY created_at DESC")
        } else {
            format!(
                "SELECT {INVOCATION_PROJECTION} FROM sys_invocation WHERE status != 'completed' ORDER BY created_at DESC"
            )
        }
    }

    /// Get single invocation by ID.
    #[must_use]
    pub fn invocation(id: &str) -> String {
        format!(
            "SELECT {INVOCATION_PROJECTION} FROM sys_invocation WHERE id = '{}'",
            escape_sql(id)
        )
    }

    /// Get invocations by status.
    #[must_use]
    pub fn invocations_by_status(status: InvocationStatus) -> String {
        format!(
            "SELECT {INVOCATION_PROJECTION} FROM sys_invocation WHERE status = '{}' ORDER BY created_at DESC",
            invocation_status_literal(status)
        )
    }

    /// Get journal entries for an invocation (ordered by index).
    #[must_use]
    pub fn journal(invocation_id: &str) -> String {
        format!(
            "SELECT id, index, entry_type, name, completed, invoked_id, invoked_target, sleep_wakeup_at, promise_name, entry_json, entry_lite_json, appended_at FROM sys_journal WHERE id = '{}' ORDER BY index",
            escape_sql(invocation_id)
        )
    }

    /// Get journal events since a given index.
    #[must_use]
    pub fn journal_events_since(invocation_id: &str, since_index: u32) -> String {
        format!(
            "SELECT id, after_journal_entry_index, appended_at, event_type, event_json FROM sys_journal_events WHERE id = '{}' AND after_journal_entry_index > {} ORDER BY appended_at",
            escape_sql(invocation_id),
            since_index
        )
    }

    /// Get all state for a service.
    #[must_use]
    pub fn service_state(service_name: &str) -> String {
        format!(
            "SELECT service_name, service_key, key, value_utf8, value FROM state WHERE service_name = '{}'",
            escape_sql(service_name)
        )
    }

    /// Get state for a specific key.
    #[must_use]
    pub fn keyed_state(service_name: &str, service_key: &str) -> String {
        format!(
            "SELECT service_name, service_key, key, value_utf8, value FROM state WHERE service_name = '{}' AND service_key = '{}'",
            escape_sql(service_name),
            escape_sql(service_key)
        )
    }

    /// List all services.
    #[must_use]
    pub fn services() -> String {
        "SELECT name, ty, revision, public, deployment_id FROM sys_service ORDER BY name"
            .to_string()
    }

    /// List all deployments.
    #[must_use]
    pub fn deployments() -> String {
        "SELECT id, ty, endpoint, created_at FROM sys_deployment ORDER BY created_at DESC"
            .to_string()
    }

    /// Get keyed service status (blocking invocations).
    #[must_use]
    pub fn keyed_service_status() -> String {
        "SELECT service_name, service_key, invocation_id FROM sys_keyed_service_status".to_string()
    }

    /// Get promises for a workflow.
    #[must_use]
    pub fn promises(service_name: &str, service_key: &str) -> String {
        format!(
            "SELECT service_name, service_key, key, completed, completion_success_value, completion_failure FROM sys_promise WHERE service_name = '{}' AND service_key = '{}'",
            escape_sql(service_name),
            escape_sql(service_key)
        )
    }

    /// Get invocations for a specific service.
    #[must_use]
    pub fn invocations_for_service(service_name: &str) -> String {
        format!(
            "SELECT {INVOCATION_PROJECTION} FROM sys_invocation WHERE target_service_name = '{}' ORDER BY created_at DESC",
            escape_sql(service_name)
        )
    }

    /// Get invocations in backing-off status (retries).
    #[must_use]
    pub fn retrying_invocations() -> String {
        format!(
            "SELECT {INVOCATION_PROJECTION} FROM sys_invocation WHERE status = 'backing-off' ORDER BY modified_at"
        )
    }

    /// Get oldest stuck invocations using an absolute cutoff timestamp in epoch ms.
    #[must_use]
    pub fn stuck_invocations(cutoff_epoch_ms: u64) -> String {
        format!(
            "SELECT {INVOCATION_PROJECTION} FROM sys_invocation WHERE status IN ('pending', 'scheduled', 'suspended') AND modified_at <= {cutoff_epoch_ms} ORDER BY modified_at, created_at"
        )
    }
}

const fn invocation_status_literal(status: InvocationStatus) -> &'static str {
    match status {
        InvocationStatus::Pending => "pending",
        InvocationStatus::Scheduled => "scheduled",
        InvocationStatus::Ready => "ready",
        InvocationStatus::Running => "running",
        InvocationStatus::Paused => "paused",
        InvocationStatus::BackingOff => "backing-off",
        InvocationStatus::Suspended => "suspended",
        InvocationStatus::Completed => "completed",
    }
}

/// Escape single quotes in SQL string literals.
fn escape_sql(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn list_invocations_query_contains_projection() {
        let query = SqlQueries::list_invocations(InvocationFilter::ActiveOnly);
        assert!(query.contains("SELECT"));
        assert!(query.contains("target_service_name"));
        assert!(query.contains("last_failure_error_code"));
    }

    #[test]
    fn invocation_query_uses_explicit_projection() {
        let query = SqlQueries::invocation("inv_123");
        assert!(query.contains("target_handler_name"));
        assert!(!query.contains("SELECT *"));
    }

    #[test]
    fn invocations_by_status_uses_typed_literal() {
        let query = SqlQueries::invocations_by_status(InvocationStatus::Running);
        assert!(query.contains("status = 'running'"));
        assert!(!query.contains("DROP TABLE"));
    }

    #[test]
    fn journal_query_includes_entry_lite_json() {
        let query = SqlQueries::journal("inv_123");
        assert!(query.contains("entry_lite_json"));
    }

    #[test]
    fn journal_query_escapes_id() {
        let query = SqlQueries::journal("inv'id");
        assert!(query.contains("inv''id"));
    }

    #[test]
    fn service_state_sql_injection_attempt() {
        let query = SqlQueries::service_state("'; DROP TABLE state; --");
        assert!(query.contains("''; DROP TABLE state; --"));
        assert!(!query.contains("' DROP TABLE"));
    }

    #[test]
    fn keyed_state_escapes_both_params() {
        let query = SqlQueries::keyed_state("MyService", "my'key");
        assert!(query.contains("MyService"));
        assert!(query.contains("my''key"));
    }

    #[test]
    fn stuck_invocations_uses_absolute_cutoff() {
        let query = SqlQueries::stuck_invocations(1_700_000_000_000);
        assert!(query.contains("modified_at <= 1700000000000"));
        assert!(query.contains("ORDER BY modified_at, created_at"));
    }

    #[test]
    fn invocation_queries_share_projection_columns() {
        let queries = [
            SqlQueries::list_invocations(InvocationFilter::All),
            SqlQueries::invocations_by_status(InvocationStatus::Pending),
            SqlQueries::invocations_for_service("svc"),
            SqlQueries::retrying_invocations(),
            SqlQueries::stuck_invocations(0),
        ];

        for query in queries {
            assert!(query.contains("target_service_name"));
            assert!(query.contains("invoked_by_service_name"));
            assert!(query.contains("last_failure_error_code"));
        }
    }

    #[test]
    fn journal_events_since_max_index() {
        let query = SqlQueries::journal_events_since("inv_123", u32::MAX);
        assert!(query.contains(&format!("after_journal_entry_index > {}", u32::MAX)));
    }

    // --- Promise query tests (oya-frontend-8t3) ---

    #[test]
    fn promises_query_contains_all_columns() {
        let query = SqlQueries::promises("MySvc", "key1");
        assert!(query.contains("service_name"), "missing service_name");
        assert!(query.contains("service_key"), "missing service_key");
        assert!(query.contains("completed"), "missing completed");
        assert!(query.contains("completion_success_value"), "missing completion_success_value");
        assert!(query.contains("completion_failure"), "missing completion_failure");
        assert!(query.contains("sys_promise"), "missing sys_promise table");
    }

    #[test]
    fn promises_query_escapes_service_name() {
        let query = SqlQueries::promises("Svc'Name", "key");
        assert!(query.contains("Svc''Name"), "service_name not escaped");
        assert!(!query.contains("Svc'Name"), "raw quote found in query");
    }

    #[test]
    fn promises_query_escapes_service_key() {
        let query = SqlQueries::promises("Svc", "key'value");
        assert!(query.contains("key''value"), "service_key not escaped");
    }
}
