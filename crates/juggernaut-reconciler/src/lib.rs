//! # juggernaut-reconciler
//!
//! K8s-style reconciliation loop for bead management.

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

pub use juggernaut_core::{JuggernautError, Result};

// TODO: Implement DesiredState, ActualState, Reconciler, ReconciliationLoop
