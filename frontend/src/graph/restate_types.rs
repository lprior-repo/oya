#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Re-exports for Restate types for backward compatibility.
//!
//! These types have been split into focused modules:
//! - `service_kinds` - `ServiceKind` and `ContextType`
//! - `port_types` - `PortType` and compatibility checks
//! - `node_icon` - `NodeIcon` enum
//! - `node_ui_state` - `NodeUiState` and value objects

pub use super::node_icon::{NodeIcon, UnknownIconError};
pub use super::node_ui_state::{
    EmptyStringError, NodeLabel, NodeUiState, NonEmptyString, ServiceName, StateKey,
};
pub use super::port_types::{types_compatible, ParsePortTypeError, PortType};
pub use super::service_kinds::{
    ContextType, ParseContextTypeError, ParseServiceKindError, ServiceKind,
};
