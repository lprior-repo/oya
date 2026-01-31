//! # juggernaut-events
//!
//! Inter-bead coordination via event sourcing and pub/sub.

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

pub use juggernaut_core::{JuggernautError, Result};

// TODO: Implement BeadEvent, EventStore, EventBus, Projection
