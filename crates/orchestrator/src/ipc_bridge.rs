//! IPC Bridge: Converts EventBus events to IPC messages for Zellij UI.
//!
//! This module bridges the gap between the orchestrator's EventBus and the
//! Zellij plugin IPC protocol. It listens for stage lifecycle events and
//! converts them to HostMessage variants for transmission to the UI.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         EventBus                                │
//! │  (BeadEvent::StageStarted, StageCompleted, StageFailed, etc.)   │
//! └────────────────────────┬────────────────────────────────────────┘
//!                          │
//!                          ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      IpcBridge                                  │
//! │  - Subscribes to stage lifecycle events                         │
//! │  - Converts BeadEvent → HostMessage                             │
//! │  - Handles event routing and batching                           │
//! │  - Tracks event-to-message mappings                             │
//! └────────────────────────┬────────────────────────────────────────┘
//!                          │
//!                          ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    IPC Protocol                                 │
//! │  (HostMessage::StageStarted, StageCompleted, etc.)              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Supported Events
//!
//! - `StageStarted` → `HostMessage::StageStarted`
//! - `StageCompleted` → `HostMessage::StageCompleted`
//! - `StageFailed` → `HostMessage::StageFailed`
//! - `StageReentry` → `HostMessage::StageReentry`
//! - `ValidationRan` → `HostMessage::ValidationRan`
//! - `RecursionExhausted` → `HostMessage::RecursionExhausted`
//!
//! # Quality Gates
//!
//! - Zero panics, zero unwrap
//! - Pure functional conversions (no mutation)
//! - Railway-Oriented Programming with Result types
//! - Comprehensive test coverage (>10 tests)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::ipc_messages::HostMessage;
use oya_events::{BeadEvent, Severity, StageKind};
use thiserror::Error;

/// Errors that can occur during IPC bridging.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IpcBridgeError {
    /// Event type not supported for IPC conversion.
    #[error("event type not supported: {event_type}")]
    UnsupportedEventType { event_type: String },

    /// Missing required field in event.
    #[error("missing required field: {field}")]
    MissingField { field: String },

    /// Invalid timestamp in event.
    #[error("invalid timestamp: {timestamp}")]
    InvalidTimestamp { timestamp: i64 },

    /// Serialization error for IPC message.
    #[error("serialization failed: {message}")]
    SerializationError { message: String },
}

/// Result type for IPC bridge operations.
pub type IpcBridgeResult<T> = Result<T, IpcBridgeError>;

/// IPC Bridge for converting `EventBus` events to IPC messages.
///
/// This struct provides pure functional conversion methods with no internal
/// state. All conversions are deterministic and side-effect free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcBridge;

impl IpcBridge {
    /// Create a new IPC bridge (zero-sized, no state).
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Convert a `BeadEvent` to a `HostMessage` for IPC transmission.
    ///
    /// This method handles all stage lifecycle events and returns the
    /// corresponding IPC message variant. Non-stage events return an error.
    ///
    /// # Arguments
    ///
    /// * `event` - The `BeadEvent` from `EventBus` to convert
    ///
    /// # Returns
    ///
    /// * `Ok(HostMessage)` - The converted IPC message
    /// * `Err(IpcBridgeError)` - If conversion fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let bridge = IpcBridge::new();
    /// let event = BeadEvent::stage_started(bead_id, StageKind::Implement, 1);
    /// let msg = bridge.convert_event(event)?;
    /// assert!(matches!(msg, HostMessage::StageStarted { .. }));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `IpcBridgeError::UnsupportedEventType` if the event is not a stage lifecycle event.
    /// Returns `IpcBridgeError::MissingField` if required fields are missing.
    /// Returns `IpcBridgeError::InvalidTimestamp` if the timestamp is invalid.
    pub fn convert_event(&self, event: &BeadEvent) -> IpcBridgeResult<HostMessage> {
        match event {
            // Stage lifecycle events
            BeadEvent::StageStarted {
                bead_id,
                stage,
                attempt,
                timestamp,
                event_id: _,
            } => self.convert_stage_started(bead_id, *stage, *attempt, timestamp),

            BeadEvent::StageCompleted {
                bead_id,
                stage,
                artifact_ref,
                timestamp,
                event_id: _,
            } => self.convert_stage_completed(bead_id, *stage, artifact_ref, timestamp),

            BeadEvent::StageFailed {
                bead_id,
                stage,
                feedback,
                severity,
                timestamp,
                event_id: _,
            } => self.convert_stage_failed(bead_id, *stage, feedback, *severity, timestamp),

            BeadEvent::StageReentry {
                bead_id,
                from_stage,
                to_stage,
                reason,
                attempt,
                timestamp,
                event_id: _,
            } => self.convert_stage_reentry(
                bead_id,
                *from_stage,
                *to_stage,
                reason,
                *attempt,
                timestamp,
            ),

            BeadEvent::ValidationRan {
                bead_id,
                passed,
                output,
                command,
                exit_code,
                timestamp,
                event_id: _,
            } => self
                .convert_validation_ran(bead_id, *passed, output, command, *exit_code, timestamp),

            BeadEvent::RecursionExhausted {
                bead_id,
                total_attempts,
                last_stage,
                timestamp,
                event_id: _,
            } => self.convert_recursion_exhausted(bead_id, *total_attempts, *last_stage, timestamp),

            // Non-stage events are not supported
            _ => Err(IpcBridgeError::UnsupportedEventType {
                event_type: event.event_type().to_string(),
            }),
        }
    }

    /// Convert multiple events to IPC messages, filtering out errors.
    ///
    /// This is useful for batch processing where you want to skip unsupported
    /// events rather than failing the entire batch.
    ///
    /// # Arguments
    ///
    /// * `events` - Slice of `BeadEvents` to convert
    ///
    /// # Returns
    ///
    /// Vector of successfully converted `HostMessages`
    #[must_use]
    pub fn convert_events_batch(&self, events: &[BeadEvent]) -> Vec<HostMessage> {
        events
            .iter()
            .filter_map(|event| self.convert_event(event).ok())
            .collect()
    }

    /// Convert `StageStarted` event to IPC message.
    fn convert_stage_started(
        &self,
        bead_id: &oya_events::BeadId,
        stage: StageKind,
        attempt: u32,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> IpcBridgeResult<HostMessage> {
        let ts = self.datetime_to_timestamp(timestamp)?;
        Ok(HostMessage::StageStarted {
            bead_id: bead_id.to_string(),
            stage: self.stage_kind_to_string(stage),
            attempt,
            timestamp: ts,
        })
    }

    /// Convert `StageCompleted` event to IPC message.
    fn convert_stage_completed(
        &self,
        bead_id: &oya_events::BeadId,
        stage: StageKind,
        artifact_ref: &Option<String>,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> IpcBridgeResult<HostMessage> {
        let ts = self.datetime_to_timestamp(timestamp)?;
        Ok(HostMessage::StageCompleted {
            bead_id: bead_id.to_string(),
            stage: self.stage_kind_to_string(stage),
            artifact_ref: artifact_ref.clone(),
            timestamp: ts,
        })
    }

    /// Convert `StageFailed` event to IPC message.
    fn convert_stage_failed(
        &self,
        bead_id: &oya_events::BeadId,
        stage: StageKind,
        feedback: &str,
        severity: Severity,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> IpcBridgeResult<HostMessage> {
        let ts = self.datetime_to_timestamp(timestamp)?;
        Ok(HostMessage::StageFailed {
            bead_id: bead_id.to_string(),
            stage: self.stage_kind_to_string(stage),
            feedback: feedback.to_string(),
            severity: self.severity_to_string(severity),
            timestamp: ts,
        })
    }

    /// Convert `StageReentry` event to IPC message.
    fn convert_stage_reentry(
        &self,
        bead_id: &oya_events::BeadId,
        from_stage: StageKind,
        to_stage: StageKind,
        reason: &str,
        attempt: u32,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> IpcBridgeResult<HostMessage> {
        let ts = self.datetime_to_timestamp(timestamp)?;
        Ok(HostMessage::StageReentry {
            bead_id: bead_id.to_string(),
            from_stage: self.stage_kind_to_string(from_stage),
            to_stage: self.stage_kind_to_string(to_stage),
            reason: reason.to_string(),
            attempt,
            timestamp: ts,
        })
    }

    /// Convert `ValidationRan` event to IPC message.
    fn convert_validation_ran(
        &self,
        bead_id: &oya_events::BeadId,
        passed: bool,
        output: &str,
        command: &str,
        exit_code: i32,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> IpcBridgeResult<HostMessage> {
        let ts = self.datetime_to_timestamp(timestamp)?;
        Ok(HostMessage::ValidationRan {
            bead_id: bead_id.to_string(),
            passed,
            output: output.to_string(),
            command: command.to_string(),
            exit_code,
            timestamp: ts,
        })
    }

    /// Convert `RecursionExhausted` event to IPC message.
    fn convert_recursion_exhausted(
        &self,
        bead_id: &oya_events::BeadId,
        total_attempts: u32,
        last_stage: StageKind,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> IpcBridgeResult<HostMessage> {
        let ts = self.datetime_to_timestamp(timestamp)?;
        Ok(HostMessage::RecursionExhausted {
            bead_id: bead_id.to_string(),
            total_attempts,
            last_stage: self.stage_kind_to_string(last_stage),
            timestamp: ts,
        })
    }

    /// Convert `StageKind` to IPC string representation.
    fn stage_kind_to_string(&self, stage: StageKind) -> String {
        match stage {
            StageKind::Research => "research",
            StageKind::Plan => "plan",
            StageKind::Implement => "implement",
            StageKind::Review => "review",
            StageKind::Validate => "validate",
            StageKind::Accept => "accept",
        }
        .to_string()
    }

    /// Convert Severity to IPC string representation.
    fn severity_to_string(&self, severity: Severity) -> String {
        match severity {
            Severity::Minor => "minor",
            Severity::Major => "major",
            Severity::Fundamental => "fundamental",
        }
        .to_string()
    }

    /// Convert chrono `DateTime` to Unix timestamp (seconds).
    fn datetime_to_timestamp(&self, dt: &chrono::DateTime<chrono::Utc>) -> IpcBridgeResult<u64> {
        dt.timestamp()
            .try_into()
            .map_err(|_| IpcBridgeError::InvalidTimestamp {
                timestamp: dt.timestamp(),
            })
    }
}

impl Default for IpcBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use oya_events::BeadId;
    use std::str::FromStr;

    // Helper to create a test bead ID
    fn test_bead_id() -> BeadId {
        BeadId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").map_or_else(|_| BeadId::new(), |id| id)
    }

    // Helper to create a test timestamp
    fn test_timestamp() -> chrono::DateTime<Utc> {
        Utc::now()
    }

    // ========================================================================
    // STAGE STARTED CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_convert_stage_started_returns_ipc_message() {
        // GIVEN: A StageStarted event
        let bridge = IpcBridge::new();
        let event = BeadEvent::stage_started(test_bead_id(), StageKind::Implement, 2);

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should return StageStarted HostMessage
        assert!(result.is_ok(), "conversion should succeed");
        let is_correct = match result {
            Ok(HostMessage::StageStarted {
                bead_id,
                stage,
                attempt,
                ..
            }) => bead_id == test_bead_id().to_string() && stage == "implement" && attempt == 2,
            _ => false,
        };
        assert!(is_correct, "Expected StageStarted with correct fields");
    }

    #[test]
    fn test_convert_stage_started_research_stage() {
        // GIVEN: A StageStarted event for Research stage
        let bridge = IpcBridge::new();
        let event = BeadEvent::stage_started(test_bead_id(), StageKind::Research, 1);

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should have correct stage name
        let stage_name = match result {
            Ok(HostMessage::StageStarted { stage, .. }) => stage,
            _ => String::from("wrong"),
        };
        assert_eq!(stage_name, "research");
    }

    #[test]
    fn test_convert_stage_started_all_stages() {
        // GIVEN: An IPC bridge
        let bridge = IpcBridge::new();
        let stages = [
            (StageKind::Research, "research"),
            (StageKind::Plan, "plan"),
            (StageKind::Implement, "implement"),
            (StageKind::Review, "review"),
            (StageKind::Validate, "validate"),
            (StageKind::Accept, "accept"),
        ];

        // THEN: All stages should convert correctly
        for (stage_kind, expected_name) in stages {
            let event = BeadEvent::stage_started(test_bead_id(), stage_kind, 1);
            let result = bridge.convert_event(&event);
            let stage = match result {
                Ok(HostMessage::StageStarted { stage, .. }) => stage,
                _ => String::from("error"),
            };
            assert_eq!(
                stage, expected_name,
                "stage name mismatch for {:?}",
                stage_kind
            );
        }
    }

    // ========================================================================
    // STAGE COMPLETED CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_convert_stage_completed_with_artifact() {
        // GIVEN: A StageCompleted event with artifact
        let bridge = IpcBridge::new();
        let artifact = Some("artifacts/review.txt".to_string());
        let event = BeadEvent::stage_completed(test_bead_id(), StageKind::Review, artifact.clone());

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should include artifact reference
        let artifact_ref = match result {
            Ok(HostMessage::StageCompleted { artifact_ref, .. }) => artifact_ref,
            _ => None,
        };
        assert_eq!(artifact_ref, artifact);
    }

    #[test]
    fn test_convert_stage_completed_without_artifact() {
        // GIVEN: A StageCompleted event without artifact
        let bridge = IpcBridge::new();
        let event = BeadEvent::stage_completed(test_bead_id(), StageKind::Validate, None);

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should have None for artifact_ref
        let has_none = match result {
            Ok(HostMessage::StageCompleted { artifact_ref, .. }) => artifact_ref.is_none(),
            _ => false,
        };
        assert!(has_none);
    }

    // ========================================================================
    // STAGE FAILED CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_convert_stage_failed_minor_severity() {
        // GIVEN: A StageFailed event with minor severity
        let bridge = IpcBridge::new();
        let event = BeadEvent::stage_failed(
            test_bead_id(),
            StageKind::Review,
            "nitpick issues",
            Severity::Minor,
        );

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should include feedback and severity
        let (feedback, severity) = match result {
            Ok(HostMessage::StageFailed {
                feedback, severity, ..
            }) => (feedback, severity),
            _ => (String::new(), String::new()),
        };
        assert_eq!(feedback, "nitpick issues");
        assert_eq!(severity, "minor");
    }

    #[test]
    fn test_convert_stage_failed_major_severity() {
        // GIVEN: A StageFailed event with major severity
        let bridge = IpcBridge::new();
        let event = BeadEvent::stage_failed(
            test_bead_id(),
            StageKind::Implement,
            "design flaws",
            Severity::Major,
        );

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should have major severity
        let severity = match result {
            Ok(HostMessage::StageFailed { severity, .. }) => severity,
            _ => String::new(),
        };
        assert_eq!(severity, "major");
    }

    #[test]
    fn test_convert_stage_failed_fundamental_severity() {
        // GIVEN: A StageFailed event with fundamental severity
        let bridge = IpcBridge::new();
        let event = BeadEvent::stage_failed(
            test_bead_id(),
            StageKind::Research,
            "wrong approach entirely",
            Severity::Fundamental,
        );

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should have fundamental severity
        let severity = match result {
            Ok(HostMessage::StageFailed { severity, .. }) => severity,
            _ => String::new(),
        };
        assert_eq!(severity, "fundamental");
    }

    // ========================================================================
    // STAGE REENTRY CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_convert_stage_reentry_valid_transition() {
        // GIVEN: A StageReentry event
        let bridge = IpcBridge::new();
        let event = BeadEvent::stage_reentry(
            test_bead_id(),
            StageKind::Review,
            StageKind::Plan,
            "major redesign needed",
            3,
        );

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should include all reentry details
        let details = match result {
            Ok(HostMessage::StageReentry {
                from_stage,
                to_stage,
                reason,
                attempt,
                ..
            }) => Some((from_stage, to_stage, reason, attempt)),
            _ => None,
        };
        assert!(details.is_some(), "Expected StageReentry message");
        if let Some((from_stage, to_stage, reason, attempt)) = details {
            assert_eq!(from_stage, "review");
            assert_eq!(to_stage, "plan");
            assert_eq!(reason, "major redesign needed");
            assert_eq!(attempt, 3);
        }
    }

    // ========================================================================
    // VALIDATION RAN CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_convert_validation_ran_passed() {
        // GIVEN: A ValidationRan event that passed
        let bridge = IpcBridge::new();
        let event =
            BeadEvent::validation_ran(test_bead_id(), true, "all checks passed", "moon run :ci", 0);

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should reflect passing validation
        let validation = match result {
            Ok(HostMessage::ValidationRan {
                passed,
                output,
                command,
                exit_code,
                ..
            }) => Some((passed, output, command, exit_code)),
            _ => None,
        };
        assert!(validation.is_some());
        if let Some((passed, output, command, exit_code)) = validation {
            assert!(passed);
            assert_eq!(output, "all checks passed");
            assert_eq!(command, "moon run :ci");
            assert_eq!(exit_code, 0);
        }
    }

    #[test]
    fn test_convert_validation_ran_failed() {
        // GIVEN: A ValidationRan event that failed
        let bridge = IpcBridge::new();
        let event =
            BeadEvent::validation_ran(test_bead_id(), false, "3 tests failed", "moon run :ci", 1);

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should reflect failing validation
        let (passed, exit_code) = match result {
            Ok(HostMessage::ValidationRan {
                passed, exit_code, ..
            }) => (passed, exit_code),
            _ => (true, 0),
        };
        assert!(!passed);
        assert_eq!(exit_code, 1);
    }

    // ========================================================================
    // RECURSION EXHAUSTED CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_convert_recursion_exhausted() {
        // GIVEN: A RecursionExhausted event
        let bridge = IpcBridge::new();
        let event = BeadEvent::recursion_exhausted(test_bead_id(), 15, StageKind::Review);

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should include attempt count and last stage
        let (total_attempts, last_stage) = match result {
            Ok(HostMessage::RecursionExhausted {
                total_attempts,
                last_stage,
                ..
            }) => (total_attempts, last_stage),
            _ => (0, String::new()),
        };
        assert_eq!(total_attempts, 15);
        assert_eq!(last_stage, "review");
    }

    // ========================================================================
    // UNSUPPORTED EVENT TESTS
    // ========================================================================

    #[test]
    fn test_convert_unsupported_event_returns_error() {
        // GIVEN: An unsupported event type
        let bridge = IpcBridge::new();
        let event = BeadEvent::created(test_bead_id(), oya_events::BeadSpec::new("test bead"));

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should return UnsupportedEventType error
        assert!(result.is_err(), "should return error for unsupported event");
        let event_type = match result {
            Err(IpcBridgeError::UnsupportedEventType { event_type }) => event_type,
            _ => String::new(),
        };
        assert_eq!(event_type, "created");
    }

    #[test]
    fn test_convert_state_changed_returns_error() {
        // GIVEN: A StateChanged event (not a stage event)
        let bridge = IpcBridge::new();
        let event = BeadEvent::state_changed(
            test_bead_id(),
            oya_events::BeadState::Pending,
            oya_events::BeadState::Running,
        );

        // WHEN: Converting to IPC message
        let result = bridge.convert_event(&event);

        // THEN: Should return UnsupportedEventType error
        assert!(result.is_err());
        let event_type = match result {
            Err(IpcBridgeError::UnsupportedEventType { event_type }) => event_type,
            _ => String::new(),
        };
        assert_eq!(event_type, "state_changed");
    }

    // ========================================================================
    // BATCH CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_convert_events_batch_filters_errors() {
        // GIVEN: A mix of supported and unsupported events
        let bridge = IpcBridge::new();
        let events = vec![
            BeadEvent::stage_started(test_bead_id(), StageKind::Implement, 1),
            BeadEvent::created(test_bead_id(), oya_events::BeadSpec::new("test")),
            BeadEvent::stage_completed(test_bead_id(), StageKind::Validate, None),
        ];

        // WHEN: Converting batch
        let results = bridge.convert_events_batch(&events);

        // THEN: Should only include supported events
        assert_eq!(results.len(), 2, "should filter out unsupported event");
        assert!(
            results
                .iter()
                .any(|m| matches!(m, HostMessage::StageStarted { .. })),
            "should include StageStarted"
        );
        assert!(
            results
                .iter()
                .any(|m| matches!(m, HostMessage::StageCompleted { .. })),
            "should include StageCompleted"
        );
    }

    #[test]
    fn test_convert_events_batch_empty() {
        // GIVEN: An empty event list
        let bridge = IpcBridge::new();
        let events: Vec<BeadEvent> = vec![];

        // WHEN: Converting batch
        let results = bridge.convert_events_batch(&events);

        // THEN: Should return empty vector
        assert!(results.is_empty());
    }

    // ========================================================================
    // DEFAULT TEST
    // ========================================================================

    #[test]
    fn test_ipc_bridge_default() {
        // WHEN: Creating default IPC bridge
        let bridge = IpcBridge::default();

        // THEN: Should be equivalent to new()
        let expected = IpcBridge::new();
        assert_eq!(bridge, expected);
    }

    // ========================================================================
    // TIMESTAMP CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_datetime_to_timestamp_valid() {
        // GIVEN: An IPC bridge and valid datetime
        let bridge = IpcBridge::new();
        let dt = match chrono::DateTime::from_timestamp(1_234_567_890, 0) {
            Some(d) => d,
            None => chrono::Utc::now(),
        };

        // WHEN: Converting to timestamp
        let result = bridge.datetime_to_timestamp(&dt);

        // THEN: Should return correct timestamp
        assert!(result.is_ok());
        if let Ok(ts) = result {
            assert_eq!(ts, 1_234_567_890);
        }
    }
}
