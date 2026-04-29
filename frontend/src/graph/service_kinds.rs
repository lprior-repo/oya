//! Service kinds for Restate workflows.
//!
//! Implements Scott Wlaschin DDD principles:
//! - Parse, don't validate
//! - Make illegal states unrepresentable
//! - Types act as documentation

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ===========================================================================
// Client Type
// ===========================================================================

/// Available Restate client types for service calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    Service,
    Object,
    Workflow,
}

// ===========================================================================
// Service Kind
// ===========================================================================

/// The Restate service type that a workflow node maps to.
///
/// This determines which Restate capabilities are available during execution:
/// - `Handler`: Stateless service (Service context)
/// - `Workflow`: Long-running workflow (`WorkflowContext`)
/// - `Actor`: Stateful virtual object (`ObjectContext`)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    /// Stateless service - no state operations available
    /// Maps to: Restate Service
    /// Context: Context (base service context)
    Handler,

    /// Long-running workflow with durable promises
    /// Maps to: Restate Workflow
    /// Context: `WorkflowContext`, `SharedWorkflowContext`
    Workflow,

    /// Stateful virtual object - key-addressable with state operations
    /// Maps to: Restate `VirtualObject`
    /// Context: `ObjectContext`, `SharedObjectContext`
    Actor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseServiceKindError {
    UnknownVariant { input: String, expected: &'static str },
}

impl std::fmt::Display for ParseServiceKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVariant { input, expected } => {
                write!(f, "Unknown ServiceKind '{input}': expected one of {expected}")
            }
        }
    }
}

impl std::error::Error for ParseServiceKindError {}

impl fmt::Display for ServiceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Handler => write!(f, "handler"),
            Self::Workflow => write!(f, "workflow"),
            Self::Actor => write!(f, "actor"),
        }
    }
}

impl FromStr for ServiceKind {
    type Err = ParseServiceKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "handler" => Ok(Self::Handler),
            "workflow" => Ok(Self::Workflow),
            "actor" => Ok(Self::Actor),
            _ => Err(ParseServiceKindError::UnknownVariant {
                input: s.to_string(),
                expected: "handler, workflow, actor",
            }),
        }
    }
}

impl ServiceKind {
    /// Returns true if this service kind supports state operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::service_kinds::ServiceKind;
    /// assert!(!ServiceKind::Handler.supports_state());
    /// assert!(ServiceKind::Workflow.supports_state());
    /// assert!(ServiceKind::Actor.supports_state());
    /// ```
    #[must_use]
    pub const fn supports_state(&self) -> bool {
        matches!(self, Self::Actor | Self::Workflow)
    }

    /// Returns true if this service kind supports promises.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::service_kinds::ServiceKind;
    /// assert!(!ServiceKind::Handler.supports_promises());
    /// assert!(ServiceKind::Workflow.supports_promises());
    /// assert!(!ServiceKind::Actor.supports_promises());
    /// ```
    #[must_use]
    pub const fn supports_promises(&self) -> bool {
        matches!(self, Self::Workflow)
    }

    /// Returns the context type for this service kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::service_kinds::{ServiceKind, ContextType};
    /// assert_eq!(ServiceKind::Handler.context_type(), ContextType::Synchronous);
    /// assert_eq!(ServiceKind::Workflow.context_type(), ContextType::Asynchronous);
    /// assert_eq!(ServiceKind::Actor.context_type(), ContextType::Synchronous);
    /// ```
    #[must_use]
    pub const fn context_type(&self) -> ContextType {
        match self {
            Self::Handler | Self::Actor => ContextType::Synchronous,
            Self::Workflow => ContextType::Asynchronous,
        }
    }

    /// Returns the set of available Restate client types for this service kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::service_kinds::{ServiceKind, ClientType};
    /// assert_eq!(
    ///     ServiceKind::Handler.available_clients(),
    ///     &[ClientType::Service]
    /// );
    /// assert_eq!(
    ///     ServiceKind::Actor.available_clients(),
    ///     &[ClientType::Service, ClientType::Object]
    /// );
    /// assert_eq!(
    ///     ServiceKind::Workflow.available_clients(),
    ///     &[ClientType::Service, ClientType::Object, ClientType::Workflow]
    /// );
    /// ```
    #[must_use]
    pub const fn available_clients(&self) -> &'static [ClientType] {
        match self {
            Self::Handler => &[ClientType::Service],
            Self::Actor => &[ClientType::Service, ClientType::Object],
            Self::Workflow => &[ClientType::Service, ClientType::Object, ClientType::Workflow],
        }
    }
}

// ===========================================================================
// Context Type
// ===========================================================================

/// The Restate context type available to a workflow node.
///
/// This determines which API methods are callable during execution:
/// - `Synchronous`: Standard service/virtual object execution
/// - `Asynchronous`: Workflow execution with promises
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ContextType {
    /// Synchronous context - standard service/virtual object execution
    /// Maps to: Context, `ObjectContext`, `SharedObjectContext`
    /// Available APIs: Client, timers, side effects, awakeables, state (object only)
    Synchronous,

    /// Asynchronous context - workflow execution with promises
    /// Maps to: `WorkflowContext`, `SharedWorkflowContext`
    /// Available APIs: All Synchronous + promises
    Asynchronous,
}

// ===========================================================================
// Context Trait
// ===========================================================================

/// Available context traits for Restate execution contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextTrait {
    ContextClient,
    ContextTimers,
    ContextSideEffects,
    ContextAwakeables,
    ContextReadState,
    ContextWriteState,
    ContextPromises,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseContextTypeError {
    UnknownVariant { input: String, expected: &'static str },
}

impl std::fmt::Display for ParseContextTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVariant { input, expected } => {
                write!(f, "Unknown ContextType '{input}': expected one of {expected}")
            }
        }
    }
}

impl std::error::Error for ParseContextTypeError {}

impl fmt::Display for ContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Synchronous => write!(f, "synchronous"),
            Self::Asynchronous => write!(f, "asynchronous"),
        }
    }
}

impl FromStr for ContextType {
    type Err = ParseContextTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "synchronous" | "sync" => Ok(Self::Synchronous),
            "asynchronous" | "async" => Ok(Self::Asynchronous),
            _ => Err(ParseContextTypeError::UnknownVariant {
                input: s.to_string(),
                expected: "synchronous, sync, asynchronous, async",
            }),
        }
    }
}

impl ContextType {
    /// Returns true if this context type is synchronous.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::service_kinds::ContextType;
    /// assert!(ContextType::Synchronous.is_synchronous());
    /// assert!(!ContextType::Asynchronous.is_synchronous());
    /// ```
    #[must_use]
    pub const fn is_synchronous(&self) -> bool {
        matches!(self, Self::Synchronous)
    }

    /// Returns true if this context type is asynchronous.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::service_kinds::ContextType;
    /// assert!(!ContextType::Synchronous.is_asynchronous());
    /// assert!(ContextType::Asynchronous.is_asynchronous());
    /// ```
    #[must_use]
    pub const fn is_asynchronous(&self) -> bool {
        matches!(self, Self::Asynchronous)
    }

    /// Returns the set of available context traits for this context type.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::service_kinds::{ContextType, ContextTrait};
    /// let sync_traits = ContextType::Synchronous.available_traits();
    /// assert_eq!(sync_traits.len(), 6);
    /// let async_traits = ContextType::Asynchronous.available_traits();
    /// assert_eq!(async_traits.len(), 7);
    /// ```
    #[must_use]
    pub const fn available_traits(&self) -> &'static [crate::graph::service_kinds::ContextTrait] {
        match self {
            Self::Synchronous => &[
                ContextTrait::ContextClient,
                ContextTrait::ContextTimers,
                ContextTrait::ContextSideEffects,
                ContextTrait::ContextAwakeables,
                ContextTrait::ContextReadState,
                ContextTrait::ContextWriteState,
            ],
            Self::Asynchronous => &[
                ContextTrait::ContextClient,
                ContextTrait::ContextTimers,
                ContextTrait::ContextSideEffects,
                ContextTrait::ContextAwakeables,
                ContextTrait::ContextReadState,
                ContextTrait::ContextWriteState,
                ContextTrait::ContextPromises,
            ],
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn service_kind_parses_handler() {
        let result: Result<ServiceKind, _> = "handler".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ServiceKind::Handler);
    }

    #[test]
    fn context_type_parses_synchronous() {
        let result: Result<ContextType, _> = "synchronous".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ContextType::Synchronous);
    }

    #[test]
    fn context_type_synchronous_is_synchronous() {
        assert!(ContextType::Synchronous.is_synchronous());
    }

    #[test]
    fn context_type_asynchronous_is_asynchronous() {
        assert!(ContextType::Asynchronous.is_asynchronous());
    }
}
