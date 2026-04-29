#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

// ===========================================================================
// Execution State Machine
// ===========================================================================

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionState {
    #[default]
    Idle,
    Queued,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

impl ExecutionState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }

    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Queued)
    }

    #[must_use]
    pub const fn is_idle(self) -> bool {
        matches!(self, Self::Idle)
    }
}

// ===========================================================================
// Invalid Transition Error
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTransition {
    from: ExecutionState,
    to: ExecutionState,
}

impl InvalidTransition {
    #[must_use]
    pub const fn new(from: ExecutionState, to: ExecutionState) -> Self {
        Self { from, to }
    }

    #[must_use]
    pub const fn from_state(self) -> ExecutionState {
        self.from
    }

    #[must_use]
    pub const fn to_state(self) -> ExecutionState {
        self.to
    }
}

impl fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid state transition: {} -> {}", self.from, self.to)
    }
}

impl std::error::Error for InvalidTransition {}

// ===========================================================================
// Type-State Pattern (Makes illegal states unrepresentable)
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IdleState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuedState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunningState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletedState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailedState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkippedState;

impl IdleState {
    #[must_use]
    pub const fn queue(self) -> QueuedState {
        QueuedState
    }

    #[must_use]
    pub const fn skip(self) -> SkippedState {
        SkippedState
    }
}

impl QueuedState {
    #[must_use]
    pub const fn start(self) -> RunningState {
        RunningState
    }

    #[must_use]
    pub const fn skip(self) -> SkippedState {
        SkippedState
    }
}

impl RunningState {
    #[must_use]
    pub const fn complete(self) -> CompletedState {
        CompletedState
    }

    #[must_use]
    pub const fn fail(self) -> FailedState {
        FailedState
    }
}

pub trait TerminalState: sealed::TerminalSealed {}

impl TerminalState for CompletedState {}
impl TerminalState for FailedState {}
impl TerminalState for SkippedState {}

mod sealed {
    pub trait TerminalSealed {}
    impl TerminalSealed for super::CompletedState {}
    impl TerminalSealed for super::FailedState {}
    impl TerminalSealed for super::SkippedState {}
}

// ===========================================================================
// Explicit State Transitions
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTransition {
    IdleToQueued,
    IdleToSkipped,
    QueuedToRunning,
    QueuedToSkipped,
    RunningToCompleted,
    RunningToFailed,
}

impl StateTransition {
    #[must_use]
    pub const fn apply(self) -> ExecutionState {
        match self {
            Self::IdleToQueued => ExecutionState::Queued,
            Self::IdleToSkipped | Self::QueuedToSkipped => ExecutionState::Skipped,
            Self::QueuedToRunning => ExecutionState::Running,
            Self::RunningToCompleted => ExecutionState::Completed,
            Self::RunningToFailed => ExecutionState::Failed,
        }
    }

    #[must_use]
    pub const fn from_states(self) -> (ExecutionState, ExecutionState) {
        match self {
            Self::IdleToQueued => (ExecutionState::Idle, ExecutionState::Queued),
            Self::IdleToSkipped => (ExecutionState::Idle, ExecutionState::Skipped),
            Self::QueuedToRunning => (ExecutionState::Queued, ExecutionState::Running),
            Self::QueuedToSkipped => (ExecutionState::Queued, ExecutionState::Skipped),
            Self::RunningToCompleted => (ExecutionState::Running, ExecutionState::Completed),
            Self::RunningToFailed => (ExecutionState::Running, ExecutionState::Failed),
        }
    }
}

#[must_use]
pub const fn try_transition(from: ExecutionState, to: ExecutionState) -> Option<StateTransition> {
    match (from, to) {
        (ExecutionState::Idle, ExecutionState::Queued) => Some(StateTransition::IdleToQueued),
        (ExecutionState::Idle, ExecutionState::Skipped) => Some(StateTransition::IdleToSkipped),
        (ExecutionState::Queued, ExecutionState::Running) => Some(StateTransition::QueuedToRunning),
        (ExecutionState::Queued, ExecutionState::Skipped) => Some(StateTransition::QueuedToSkipped),
        (ExecutionState::Running, ExecutionState::Completed) => {
            Some(StateTransition::RunningToCompleted)
        }
        (ExecutionState::Running, ExecutionState::Failed) => Some(StateTransition::RunningToFailed),
        _ => None,
    }
}

#[must_use]
pub const fn can_transition(from: ExecutionState, to: ExecutionState) -> bool {
    try_transition(from, to).is_some()
}

// ===========================================================================
// State Machine Conversions
// ===========================================================================

impl From<IdleState> for ExecutionState {
    fn from(_: IdleState) -> Self {
        Self::Idle
    }
}

impl From<QueuedState> for ExecutionState {
    fn from(_: QueuedState) -> Self {
        Self::Queued
    }
}

impl From<RunningState> for ExecutionState {
    fn from(_: RunningState) -> Self {
        Self::Running
    }
}

impl From<CompletedState> for ExecutionState {
    fn from(_: CompletedState) -> Self {
        Self::Completed
    }
}

impl From<FailedState> for ExecutionState {
    fn from(_: FailedState) -> Self {
        Self::Failed
    }
}

impl From<SkippedState> for ExecutionState {
    fn from(_: SkippedState) -> Self {
        Self::Skipped
    }
}

// ===========================================================================
// TESTS
// ===========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests;
