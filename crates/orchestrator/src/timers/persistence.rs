//! Timer persistence for durability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::scheduler::{DurableTimer, TimerId, TimerMetadata, TimerStatus};
use crate::persistence::{OrchestratorStore, PersistenceError, PersistenceResult};

/// A timer record for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerRecord {
    /// Timer ID
    pub timer_id: String,
    /// When to execute
    pub execute_at: DateTime<Utc>,
    /// Payload as JSON string
    pub payload: String,
    /// Status
    pub status: String,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
    /// Associated workflow
    pub workflow_id: Option<String>,
    /// Associated bead
    pub bead_id: Option<String>,
    /// Callback identifier
    pub callback_id: Option<String>,
}

impl TimerRecord {
    /// Create a record from a timer.
    #[must_use]
    pub fn from_timer(timer: &DurableTimer) -> Self {
        Self {
            timer_id: timer.id().as_str().to_string(),
            execute_at: timer.execute_at(),
            payload: timer.payload().to_string(),
            status: format!("{:?}", timer.status()).to_lowercase(),
            created_at: timer.created_at(),
            updated_at: timer.updated_at(),
            workflow_id: timer.workflow_id().map(String::from),
            bead_id: timer.bead_id().map(String::from),
            callback_id: timer.callback_id().map(String::from),
        }
    }

    /// Convert to a timer.
    ///
    /// # Errors
    ///
    /// Returns an error if payload or status data is invalid.
    pub fn into_timer(self) -> PersistenceResult<DurableTimer> {
        let payload: serde_json::Value = serde_json::from_str(&self.payload).map_err(|e| {
            PersistenceError::serialization_error(format!("invalid timer payload JSON: {}", e))
        })?;

        let status = match self.status.as_str() {
            "pending" => TimerStatus::Pending,
            "fired" => TimerStatus::Fired,
            "cancelled" => TimerStatus::Cancelled,
            "failed" => TimerStatus::Failed,
            other => {
                return Err(PersistenceError::serialization_error(format!(
                    "invalid timer status: {}",
                    other
                )));
            }
        };

        Ok(DurableTimer::restore(
            TimerId::from_string(self.timer_id),
            self.execute_at,
            payload,
            status,
            self.created_at,
            self.updated_at,
            TimerMetadata {
                workflow_id: self.workflow_id,
                bead_id: self.bead_id,
                callback_id: self.callback_id,
            },
        ))
    }
}

/// Persistence layer for timers.
pub struct TimerPersistence {
    store: OrchestratorStore,
}

impl TimerPersistence {
    /// Create a new timer persistence layer.
    #[must_use]
    pub fn new(store: OrchestratorStore) -> Self {
        Self { store }
    }

    /// Save a timer.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn save(&self, timer: &DurableTimer) -> PersistenceResult<()> {
        let record = TimerRecord::from_timer(timer);

        let _: Option<TimerRecord> = self
            .store
            .db()
            .create(("durable_timer", timer.id().as_str()))
            .content(record)
            .await
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(())
    }

    /// Update timer status.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn update_status(
        &self,
        timer_id: &TimerId,
        status: TimerStatus,
    ) -> PersistenceResult<()> {
        let status_str = format!("{:?}", status).to_lowercase();
        let timer_id_str = timer_id.as_str().to_string();

        let _: Option<TimerRecord> = self
            .store
            .db()
            .query(
                "UPDATE type::thing('durable_timer', $id) SET status = $status, updated_at = $now",
            )
            .bind(("id", timer_id_str))
            .bind(("status", status_str))
            .bind(("now", Utc::now()))
            .await
            .map_err(|e| PersistenceError::query_failed(e.to_string()))?
            .take(0)
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(())
    }

    /// Reschedule a timer by updating status and execution time.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn reschedule(&self, timer: &DurableTimer) -> PersistenceResult<()> {
        let status_str = format!("{:?}", timer.status()).to_lowercase();
        let timer_id_str = timer.id().as_str().to_string();

        let _: Option<TimerRecord> = self
            .store
            .db()
            .query(
                "UPDATE type::thing('durable_timer', $id) SET status = $status, execute_at = $execute_at, updated_at = $now",
            )
            .bind(("id", timer_id_str))
            .bind(("status", status_str))
            .bind(("execute_at", timer.execute_at()))
            .bind(("now", Utc::now()))
            .await
            .map_err(|e| PersistenceError::query_failed(e.to_string()))?
            .take(0)
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(())
    }

    /// Load a timer by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if not found or persistence fails.
    pub async fn load(&self, timer_id: &TimerId) -> PersistenceResult<TimerRecord> {
        let record: Option<TimerRecord> = self
            .store
            .db()
            .select(("durable_timer", timer_id.as_str()))
            .await
            .map_err(|e| PersistenceError::query_failed(e))?;

        record.ok_or_else(|| PersistenceError::not_found("durable_timer", timer_id.as_str()))
    }

    /// Load pending timers up to a time.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn load_pending_until(
        &self,
        until: DateTime<Utc>,
    ) -> PersistenceResult<Vec<TimerRecord>> {
        let records: Vec<TimerRecord> = self
            .store
            .db()
            .query(
                "SELECT * FROM durable_timer WHERE status = 'pending' AND execute_at <= $until ORDER BY execute_at ASC",
            )
            .bind(("until", until))
            .await
            .map_err(|e| PersistenceError::query_failed(e.to_string()))?
            .take(0)
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(records)
    }

    /// Load all pending timers.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn load_all_pending(&self) -> PersistenceResult<Vec<TimerRecord>> {
        let records: Vec<TimerRecord> = self
            .store
            .db()
            .query("SELECT * FROM durable_timer WHERE status = 'pending' ORDER BY execute_at ASC")
            .await
            .map_err(|e| PersistenceError::query_failed(e.to_string()))?
            .take(0)
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(records)
    }

    /// Delete a timer.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn delete(&self, timer_id: &TimerId) -> PersistenceResult<()> {
        let _: Option<TimerRecord> = self
            .store
            .db()
            .delete(("durable_timer", timer_id.as_str()))
            .await
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(())
    }

    /// Delete old fired/cancelled timers.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn prune_old(&self, older_than: DateTime<Utc>) -> PersistenceResult<u64> {
        let result: Vec<serde_json::Value> = self
            .store
            .db()
            .query(
                "DELETE FROM durable_timer WHERE (status = 'fired' OR status = 'cancelled') AND updated_at < $cutoff",
            )
            .bind(("cutoff", older_than))
            .await
            .map_err(|e| PersistenceError::query_failed(e.to_string()))?
            .take(0)
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(result.len() as u64)
    }

    /// Initialize the timer schema in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if schema initialization fails.
    pub async fn initialize_schema(store: &OrchestratorStore) -> PersistenceResult<()> {
        let schema = r"
            DEFINE TABLE IF NOT EXISTS durable_timer SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS timer_id ON durable_timer TYPE string;
            DEFINE FIELD IF NOT EXISTS execute_at ON durable_timer TYPE datetime;
            DEFINE FIELD IF NOT EXISTS payload ON durable_timer TYPE string;
            DEFINE FIELD IF NOT EXISTS status ON durable_timer TYPE string;
            DEFINE FIELD IF NOT EXISTS created_at ON durable_timer TYPE datetime;
            DEFINE FIELD IF NOT EXISTS updated_at ON durable_timer TYPE datetime;
            DEFINE FIELD IF NOT EXISTS workflow_id ON durable_timer TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS bead_id ON durable_timer TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS callback_id ON durable_timer TYPE option<string>;
            DEFINE INDEX IF NOT EXISTS timer_status ON durable_timer FIELDS status;
            DEFINE INDEX IF NOT EXISTS timer_execute_at ON durable_timer FIELDS execute_at;
            DEFINE INDEX IF NOT EXISTS timer_workflow ON durable_timer FIELDS workflow_id;
        ";

        store
            .db()
            .query(schema)
            .await
            .map_err(|e| PersistenceError::query_failed(e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_record_from_timer() {
        let timer = DurableTimer::with_delay(60, serde_json::json!({"task": "test"}))
            .with_workflow("wf-1")
            .with_bead("bead-1");

        let record = TimerRecord::from_timer(&timer);

        assert_eq!(record.timer_id, timer.id().as_str());
        assert_eq!(record.workflow_id, Some("wf-1".to_string()));
        assert_eq!(record.bead_id, Some("bead-1".to_string()));
        assert_eq!(record.status, "pending");
    }

    #[test]
    fn test_timer_record_into_timer() {
        let record = TimerRecord {
            timer_id: "timer-123".to_string(),
            execute_at: Utc::now() + chrono::Duration::seconds(60),
            payload: r#"{"key": "value"}"#.to_string(),
            status: "pending".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            workflow_id: Some("wf-1".to_string()),
            bead_id: None,
            callback_id: Some("cb-1".to_string()),
        };

        let timer = record.into_timer();
        assert!(timer.is_ok());
        if let Ok(timer) = timer {
            assert_eq!(timer.workflow_id(), Some("wf-1"));
            assert_eq!(timer.callback_id(), Some("cb-1"));
            assert!(timer.status().is_pending());
        }
    }

    #[test]
    fn test_timer_record_status_restoration() {
        let fired_record = TimerRecord {
            timer_id: "timer-1".to_string(),
            execute_at: Utc::now(),
            payload: "{}".to_string(),
            status: "fired".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            workflow_id: None,
            bead_id: None,
            callback_id: None,
        };

        let timer = fired_record.into_timer();
        assert!(timer.is_ok());
        if let Ok(timer) = timer {
            assert!(timer.status().is_fired());
        }

        let cancelled_record = TimerRecord {
            timer_id: "timer-2".to_string(),
            execute_at: Utc::now(),
            payload: "{}".to_string(),
            status: "cancelled".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            workflow_id: None,
            bead_id: None,
            callback_id: None,
        };

        let timer = cancelled_record.into_timer();
        assert!(timer.is_ok());
        if let Ok(timer) = timer {
            assert!(timer.status().is_cancelled());
        }
    }

    #[test]
    fn test_timer_record_serialization() {
        let record = TimerRecord {
            timer_id: "timer-123".to_string(),
            execute_at: Utc::now(),
            payload: r#"{"test": true}"#.to_string(),
            status: "pending".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            workflow_id: Some("wf-1".to_string()),
            bead_id: None,
            callback_id: None,
        };

        let json = serde_json::to_string(&record);
        assert!(json.is_ok());

        if let Ok(serialized) = json {
            let deserialized: Result<TimerRecord, _> = serde_json::from_str(&serialized);
            assert!(deserialized.is_ok());
        }
    }
}
