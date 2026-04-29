#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Pure business logic for Restate invocation actions.
//!
//! This module contains types and functions that determine which actions
//! are available for a given invocation status, independent of the UI layer.

use crate::restate_client::types::{InvocationAction, InvocationStatus};

/// Whether a given action is available for a given invocation status.
///
/// Rules:
/// - Cancel: All active statuses except Paused.
/// - Kill: All statuses except Completed.
/// - Pause: Only `Running` and `BackingOff`.
/// - Resume: Only `Paused`.
/// - Purge: Only `Completed`.
#[must_use]
pub const fn is_action_available(action: InvocationAction, status: InvocationStatus) -> bool {
    match action {
        InvocationAction::Cancel => matches!(
            status,
            InvocationStatus::Pending
                | InvocationStatus::Scheduled
                | InvocationStatus::Ready
                | InvocationStatus::Running
                | InvocationStatus::BackingOff
                | InvocationStatus::Suspended
        ),
        InvocationAction::Kill => !matches!(status, InvocationStatus::Completed),
        InvocationAction::Pause => {
            matches!(status, InvocationStatus::Running | InvocationStatus::BackingOff)
        }
        InvocationAction::Resume => matches!(status, InvocationStatus::Paused),
        InvocationAction::Purge => matches!(status, InvocationStatus::Completed),
    }
}

/// Feedback state for an invocation action displayed in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActionFeedback {
    /// No action in progress.
    #[default]
    Idle,
    /// Action request in flight.
    Loading,
    /// Action succeeded.
    Success { action: InvocationAction },
    /// Action failed.
    Error { action: InvocationAction, message: String },
}

/// Tracks whether a destructive-action confirmation dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmDialog {
    /// No dialog open.
    #[default]
    None,
    /// Kill confirmation shown.
    Kill,
    /// Purge confirmation shown.
    Purge,
}

/// Returns the human-readable label for an invocation action.
#[must_use]
pub const fn action_label(action: InvocationAction) -> &'static str {
    match action {
        InvocationAction::Cancel => "Cancel",
        InvocationAction::Kill => "Kill",
        InvocationAction::Pause => "Pause",
        InvocationAction::Resume => "Resume",
        InvocationAction::Purge => "Purge",
    }
}

/// Returns whether an action is destructive and requires confirmation.
#[must_use]
pub const fn is_destructive(action: InvocationAction) -> bool {
    matches!(action, InvocationAction::Kill | InvocationAction::Purge)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // --- is_action_available: Cancel ---

    #[test]
    fn cancel_available_for_pending() {
        assert!(is_action_available(InvocationAction::Cancel, InvocationStatus::Pending));
    }

    #[test]
    fn cancel_available_for_scheduled() {
        assert!(is_action_available(InvocationAction::Cancel, InvocationStatus::Scheduled));
    }

    #[test]
    fn cancel_available_for_ready() {
        assert!(is_action_available(InvocationAction::Cancel, InvocationStatus::Ready));
    }

    #[test]
    fn cancel_available_for_running() {
        assert!(is_action_available(InvocationAction::Cancel, InvocationStatus::Running));
    }

    #[test]
    fn cancel_available_for_backing_off() {
        assert!(is_action_available(InvocationAction::Cancel, InvocationStatus::BackingOff));
    }

    #[test]
    fn cancel_available_for_suspended() {
        assert!(is_action_available(InvocationAction::Cancel, InvocationStatus::Suspended));
    }

    #[test]
    fn cancel_not_available_for_paused() {
        assert!(!is_action_available(InvocationAction::Cancel, InvocationStatus::Paused));
    }

    #[test]
    fn cancel_not_available_for_completed() {
        assert!(!is_action_available(InvocationAction::Cancel, InvocationStatus::Completed));
    }

    // --- is_action_available: Kill ---

    #[test]
    fn kill_available_for_pending() {
        assert!(is_action_available(InvocationAction::Kill, InvocationStatus::Pending));
    }

    #[test]
    fn kill_available_for_running() {
        assert!(is_action_available(InvocationAction::Kill, InvocationStatus::Running));
    }

    #[test]
    fn kill_available_for_paused() {
        assert!(is_action_available(InvocationAction::Kill, InvocationStatus::Paused));
    }

    #[test]
    fn kill_not_available_for_completed() {
        assert!(!is_action_available(InvocationAction::Kill, InvocationStatus::Completed));
    }

    // --- is_action_available: Pause ---

    #[test]
    fn pause_available_for_running() {
        assert!(is_action_available(InvocationAction::Pause, InvocationStatus::Running));
    }

    #[test]
    fn pause_available_for_backing_off() {
        assert!(is_action_available(InvocationAction::Pause, InvocationStatus::BackingOff));
    }

    #[test]
    fn pause_not_available_for_pending() {
        assert!(!is_action_available(InvocationAction::Pause, InvocationStatus::Pending));
    }

    #[test]
    fn pause_not_available_for_paused() {
        assert!(!is_action_available(InvocationAction::Pause, InvocationStatus::Paused));
    }

    #[test]
    fn pause_not_available_for_completed() {
        assert!(!is_action_available(InvocationAction::Pause, InvocationStatus::Completed));
    }

    // --- is_action_available: Resume ---

    #[test]
    fn resume_available_only_for_paused() {
        assert!(is_action_available(InvocationAction::Resume, InvocationStatus::Paused));
    }

    #[test]
    fn resume_not_available_for_running() {
        assert!(!is_action_available(InvocationAction::Resume, InvocationStatus::Running));
    }

    #[test]
    fn resume_not_available_for_completed() {
        assert!(!is_action_available(InvocationAction::Resume, InvocationStatus::Completed));
    }

    // --- is_action_available: Purge ---

    #[test]
    fn purge_available_only_for_completed() {
        assert!(is_action_available(InvocationAction::Purge, InvocationStatus::Completed));
    }

    #[test]
    fn purge_not_available_for_running() {
        assert!(!is_action_available(InvocationAction::Purge, InvocationStatus::Running));
    }

    #[test]
    fn purge_not_available_for_paused() {
        assert!(!is_action_available(InvocationAction::Purge, InvocationStatus::Paused));
    }

    // --- ActionFeedback ---

    #[test]
    fn action_feedback_default_is_idle() {
        assert_eq!(ActionFeedback::default(), ActionFeedback::Idle);
    }

    #[test]
    fn action_feedback_clone() {
        let fb = ActionFeedback::Error {
            action: InvocationAction::Kill,
            message: "timeout".to_string(),
        };
        assert_eq!(fb.clone(), fb);
    }

    #[test]
    fn action_feedback_partial_eq() {
        let a = ActionFeedback::Success { action: InvocationAction::Cancel };
        let b = ActionFeedback::Success { action: InvocationAction::Cancel };
        assert_eq!(a, b);
    }

    // --- ConfirmDialog ---

    #[test]
    fn confirm_dialog_default_is_none() {
        assert_eq!(ConfirmDialog::default(), ConfirmDialog::None);
    }

    #[test]
    fn confirm_dialog_copy() {
        let d = ConfirmDialog::Kill;
        let copy = d;
        assert_eq!(copy, d);
    }

    #[test]
    fn confirm_dialog_all_variants() {
        let _none = ConfirmDialog::None;
        let _kill = ConfirmDialog::Kill;
        let _purge = ConfirmDialog::Purge;
        // Compile-time check: exhaustive match
        match ConfirmDialog::None {
            ConfirmDialog::None | ConfirmDialog::Kill | ConfirmDialog::Purge => {}
        }
    }

    // --- action_label ---

    #[test]
    fn action_label_returns_correct_strings() {
        assert_eq!(action_label(InvocationAction::Cancel), "Cancel");
        assert_eq!(action_label(InvocationAction::Kill), "Kill");
        assert_eq!(action_label(InvocationAction::Pause), "Pause");
        assert_eq!(action_label(InvocationAction::Resume), "Resume");
        assert_eq!(action_label(InvocationAction::Purge), "Purge");
    }

    // --- is_destructive ---

    #[test]
    fn is_destructive_kill_and_purge_only() {
        assert!(is_destructive(InvocationAction::Kill));
        assert!(is_destructive(InvocationAction::Purge));
        assert!(!is_destructive(InvocationAction::Cancel));
        assert!(!is_destructive(InvocationAction::Pause));
        assert!(!is_destructive(InvocationAction::Resume));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod proptests {
    use super::*;
    use crate::restate_client::types::{InvocationAction, InvocationStatus};

    fn all_statuses() -> Vec<InvocationStatus> {
        vec![
            InvocationStatus::Pending,
            InvocationStatus::Scheduled,
            InvocationStatus::Ready,
            InvocationStatus::Running,
            InvocationStatus::Paused,
            InvocationStatus::BackingOff,
            InvocationStatus::Suspended,
            InvocationStatus::Completed,
        ]
    }

    fn all_actions() -> Vec<InvocationAction> {
        vec![
            InvocationAction::Cancel,
            InvocationAction::Kill,
            InvocationAction::Pause,
            InvocationAction::Resume,
            InvocationAction::Purge,
        ]
    }

    #[test]
    fn is_action_available_never_panics() {
        for action in all_actions() {
            for status in all_statuses() {
                let _ = is_action_available(action, status);
            }
        }
    }

    #[test]
    fn cancel_never_for_completed_or_paused() {
        for status in [InvocationStatus::Completed, InvocationStatus::Paused] {
            assert!(
                !is_action_available(InvocationAction::Cancel, status),
                "Cancel should not be available for {status:?}"
            );
        }
    }

    #[test]
    fn kill_never_for_completed() {
        assert!(!is_action_available(InvocationAction::Kill, InvocationStatus::Completed));
    }

    #[test]
    fn pause_only_running_or_backing_off() {
        let disallowed: Vec<InvocationStatus> = all_statuses()
            .into_iter()
            .filter(|s| !matches!(s, InvocationStatus::Running | InvocationStatus::BackingOff))
            .collect();
        for status in disallowed {
            assert!(
                !is_action_available(InvocationAction::Pause, status),
                "Pause should not be available for {status:?}"
            );
        }
    }

    #[test]
    fn resume_only_paused() {
        let disallowed: Vec<InvocationStatus> =
            all_statuses().into_iter().filter(|s| !matches!(s, InvocationStatus::Paused)).collect();
        for status in disallowed {
            assert!(
                !is_action_available(InvocationAction::Resume, status),
                "Resume should not be available for {status:?}"
            );
        }
    }

    #[test]
    fn purge_only_completed() {
        let disallowed: Vec<InvocationStatus> = all_statuses()
            .into_iter()
            .filter(|s| !matches!(s, InvocationStatus::Completed))
            .collect();
        for status in disallowed {
            assert!(
                !is_action_available(InvocationAction::Purge, status),
                "Purge should not be available for {status:?}"
            );
        }
    }

    #[test]
    fn feedback_clone_identity() {
        let variants: Vec<ActionFeedback> = vec![
            ActionFeedback::Idle,
            ActionFeedback::Loading,
            ActionFeedback::Success { action: InvocationAction::Cancel },
            ActionFeedback::Error { action: InvocationAction::Kill, message: "err".to_string() },
        ];
        for fb in variants {
            assert_eq!(fb.clone(), fb);
        }
    }

    #[test]
    fn confirm_dialog_copy_identity() {
        let variants: Vec<ConfirmDialog> =
            vec![ConfirmDialog::None, ConfirmDialog::Kill, ConfirmDialog::Purge];
        for d in variants {
            let copy = d;
            assert_eq!(copy, d);
        }
    }
}
