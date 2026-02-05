//! Tests for shared bead types.
//!
//! Validates bead status, priority, color mapping, and display behavior.

use crate::Result;
use oya_shared::{Bead, BeadFilters, BeadPriority, BeadStatus};

#[test]
fn test_bead_status_color() {
    assert_eq!(BeadStatus::Pending.color(), "#9ca3af");
    assert_eq!(BeadStatus::Ready.color(), "#3b82f6");
    assert_eq!(BeadStatus::Running.color(), "#f59e0b");
    assert_eq!(BeadStatus::Completed.color(), "#10b981");
    assert_eq!(BeadStatus::Failed.color(), "#ef4444");
    assert_eq!(BeadStatus::Cancelled.color(), "#6b7280");
}

#[test]
fn test_bead_status_label() {
    assert_eq!(BeadStatus::Pending.label(), "Pending");
    assert_eq!(BeadStatus::Ready.label(), "Ready");
    assert_eq!(BeadStatus::Running.label(), "Running");
    assert_eq!(BeadStatus::Completed.label(), "Completed");
    assert_eq!(BeadStatus::Failed.label(), "Failed");
    assert_eq!(BeadStatus::Cancelled.label(), "Cancelled");
}

#[test]
fn test_bead_status_is_terminal() {
    assert!(BeadStatus::Pending.is_terminal());
    assert!(BeadStatus::Ready.is_terminal() == false);
    assert!(BeadStatus::Running.is_terminal() == false);
    assert!(BeadStatus::Completed.is_terminal());
    assert!(BeadStatus::Failed.is_terminal());
    assert!(BeadStatus::Cancelled.is_terminal());
}

#[test]
fn test_bead_status_serialization() {
    use serde_json;

    let statuses = vec![
        BeadStatus::Pending,
        BeadStatus::Ready,
        BeadStatus::Running,
        BeadStatus::Completed,
        BeadStatus::Failed,
        BeadStatus::Cancelled,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status);
        let deserialized: BeadStatus = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized, status);
    }
}

#[test]
fn test_bead_status_display() {
    assert_eq!(format!("{}", BeadStatus::Pending), "Pending");
    assert_eq!(format!("{}", BeadStatus::Ready), "Ready");
    assert_eq!(format!("{}", BeadStatus::Running), "Running");
    assert_eq!(format!("{}", BeadStatus::Completed), "Completed");
    assert_eq!(format!("{}", BeadStatus::Failed), "Failed");
    assert_eq!(format!("{}", BeadStatus::Cancelled), "Cancelled");
}
