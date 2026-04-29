#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Core workflow node types.
//!
//! Contains the `WorkflowNode` enum and related types (`ConditionResult`, `HttpMethod`).
//! Config structs are defined in the `configs` submodule.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::NodeCategory;
use crate::graph::{restate_types::PortType, service_kinds::ServiceKind};

pub mod configs;
pub use configs::*;

// ============================================================================
// WorkflowNode Enum
// ============================================================================

/// The workflow node types in the OYA graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum WorkflowNode {
    Awakeable(AwakeableConfig),
    ClearAll(ClearAllConfig),
    ClearState(ClearStateConfig),
    Compensate(CompensateConfig),
    Condition(ConditionConfig),
    CronTrigger(CronTriggerConfig),
    DelayedSend(DelayedSendConfig),
    DurablePromise(DurablePromiseConfig),
    GetState(GetStateConfig),
    HttpCall(HttpCallConfig),
    HttpHandler(HttpHandlerConfig),
    KafkaConsumer(KafkaHandlerConfig),
    KafkaHandler(KafkaHandlerConfig),
    LoadFromMemory(ObjectCallConfig),
    Loop(LoopConfig),
    LoopIterate(LoopConfig),
    ObjectCall(ObjectCallConfig),
    Parallel(ParallelConfig),
    PeekPromise(PeekPromiseConfig),
    ResolvePromise(ResolvePromiseConfig),
    Run(RunConfig),
    SaveToMemory(SetStateConfig),
    SendMessage(SendMessageConfig),
    ServiceCall(ServiceCallConfig),
    SetState(SetStateConfig),
    SignalHandler(SignalHandlerConfig),
    Sleep(SleepConfig),
    Switch(SwitchConfig),
    Timeout(TimeoutConfig),
    TimeoutGuard(TimeoutConfig),
    WaitForWebhook(AwakeableConfig),
    WorkflowCall(WorkflowCallConfig),
    WorkflowSubmit(WorkflowSubmitConfig),
}

impl Default for WorkflowNode {
    fn default() -> Self {
        Self::Run(RunConfig::default())
    }
}

// ============================================================================
// FromStr Implementation for WorkflowNode
// ============================================================================

impl FromStr for WorkflowNode {
    type Err = UnknownNodeTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_lowercase();
        parse_workflow_node(&normalized).ok_or_else(|| UnknownNodeTypeError(s.to_string()))
    }
}

fn parse_workflow_node(name: &str) -> Option<WorkflowNode> {
    parse_core_node(name)
        .or_else(|| parse_state_node(name))
        .or_else(|| parse_io_node(name))
        .or_else(|| parse_control_node(name))
        .or_else(|| parse_invocation_node(name))
}

fn parse_core_node(name: &str) -> Option<WorkflowNode> {
    match name {
        "run" | "run-code" => Some(WorkflowNode::Run(RunConfig::default())),
        "condition" => Some(WorkflowNode::Condition(ConditionConfig::default())),
        "compensate" => Some(WorkflowNode::Compensate(CompensateConfig::default())),
        "parallel" => Some(WorkflowNode::Parallel(ParallelConfig::default())),
        "switch" | "router" => Some(WorkflowNode::Switch(SwitchConfig::default())),
        _ => None,
    }
}

fn parse_state_node(name: &str) -> Option<WorkflowNode> {
    match name {
        "clear-all" => Some(WorkflowNode::ClearAll(ClearAllConfig::default())),
        "clear-state" => Some(WorkflowNode::ClearState(ClearStateConfig::default())),
        "get-state" => Some(WorkflowNode::GetState(GetStateConfig::default())),
        "load-from-memory" => Some(WorkflowNode::LoadFromMemory(ObjectCallConfig::default())),
        "save-to-memory" => Some(WorkflowNode::SaveToMemory(SetStateConfig::default())),
        "set-state" => Some(WorkflowNode::SetState(SetStateConfig::default())),
        _ => None,
    }
}

fn parse_io_node(name: &str) -> Option<WorkflowNode> {
    match name {
        "http-call" | "http-request" => Some(WorkflowNode::HttpCall(HttpCallConfig::default())),
        "http-handler" | "http-trigger" => {
            Some(WorkflowNode::HttpHandler(HttpHandlerConfig::default()))
        }
        "kafka-consumer" => Some(WorkflowNode::KafkaConsumer(KafkaHandlerConfig::default())),
        "kafka-handler" => Some(WorkflowNode::KafkaHandler(KafkaHandlerConfig::default())),
        "wait-for-webhook" => Some(WorkflowNode::WaitForWebhook(AwakeableConfig::default())),
        _ => None,
    }
}

fn parse_control_node(name: &str) -> Option<WorkflowNode> {
    match name {
        "cron-trigger" | "schedule-trigger" => {
            Some(WorkflowNode::CronTrigger(CronTriggerConfig::default()))
        }
        "delayed-send" | "delayed-message" => {
            Some(WorkflowNode::DelayedSend(DelayedSendConfig::default()))
        }
        "loop" => Some(WorkflowNode::Loop(LoopConfig::default())),
        "loop-iterate" => Some(WorkflowNode::LoopIterate(LoopConfig::default())),
        "sleep" | "delay" => Some(WorkflowNode::Sleep(SleepConfig::default())),
        "timeout" => Some(WorkflowNode::Timeout(TimeoutConfig::default())),
        "timeout-guard" => Some(WorkflowNode::TimeoutGuard(TimeoutConfig::default())),
        _ => None,
    }
}

fn parse_invocation_node(name: &str) -> Option<WorkflowNode> {
    match name {
        "awakeable" => Some(WorkflowNode::Awakeable(AwakeableConfig::default())),
        "durable-promise" | "promise" => {
            Some(WorkflowNode::DurablePromise(DurablePromiseConfig::default()))
        }
        "object-call" => Some(WorkflowNode::ObjectCall(ObjectCallConfig::default())),
        "peek-promise" | "peek" => Some(WorkflowNode::PeekPromise(PeekPromiseConfig::default())),
        "resolve-promise" | "resolve" => {
            Some(WorkflowNode::ResolvePromise(ResolvePromiseConfig::default()))
        }
        "send-message" => Some(WorkflowNode::SendMessage(SendMessageConfig::default())),
        "service-call" => Some(WorkflowNode::ServiceCall(ServiceCallConfig::default())),
        "signal-handler" | "wait-for-signal" => {
            Some(WorkflowNode::SignalHandler(SignalHandlerConfig::default()))
        }
        "workflow-call" => Some(WorkflowNode::WorkflowCall(WorkflowCallConfig::default())),
        "workflow-submit" => Some(WorkflowNode::WorkflowSubmit(WorkflowSubmitConfig::default())),
        _ => None,
    }
}

// ============================================================================
// Display Implementation for WorkflowNode
// ============================================================================

impl fmt::Display for WorkflowNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Awakeable(_) => write!(f, "awakeable"),
            Self::ClearAll(_) => write!(f, "clear-all"),
            Self::ClearState(_) => write!(f, "clear-state"),
            Self::Compensate(_) => write!(f, "compensate"),
            Self::Condition(_) => write!(f, "condition"),
            Self::CronTrigger(_) => write!(f, "cron-trigger"),
            Self::DelayedSend(_) => write!(f, "delayed-send"),
            Self::DurablePromise(_) => write!(f, "durable-promise"),
            Self::GetState(_) => write!(f, "get-state"),
            Self::HttpCall(_) => write!(f, "http-call"),
            Self::HttpHandler(_) => write!(f, "http-handler"),
            Self::KafkaConsumer(_) => write!(f, "kafka-consumer"),
            Self::KafkaHandler(_) => write!(f, "kafka-handler"),
            Self::LoadFromMemory(_) => write!(f, "load-from-memory"),
            Self::Loop(_) => write!(f, "loop"),
            Self::LoopIterate(_) => write!(f, "loop-iterate"),
            Self::ObjectCall(_) => write!(f, "object-call"),
            Self::Parallel(_) => write!(f, "parallel"),
            Self::PeekPromise(_) => write!(f, "peek-promise"),
            Self::ResolvePromise(_) => write!(f, "resolve-promise"),
            Self::Run(_) => write!(f, "run"),
            Self::SaveToMemory(_) => write!(f, "save-to-memory"),
            Self::SendMessage(_) => write!(f, "send-message"),
            Self::ServiceCall(_) => write!(f, "service-call"),
            Self::SetState(_) => write!(f, "set-state"),
            Self::SignalHandler(_) => write!(f, "signal-handler"),
            Self::Sleep(_) => write!(f, "sleep"),
            Self::Switch(_) => write!(f, "switch"),
            Self::Timeout(_) => write!(f, "timeout"),
            Self::TimeoutGuard(_) => write!(f, "timeout-guard"),
            Self::WaitForWebhook(_) => write!(f, "wait-for-webhook"),
            Self::WorkflowCall(_) => write!(f, "workflow-call"),
            Self::WorkflowSubmit(_) => write!(f, "workflow-submit"),
        }
    }
}

// ============================================================================
// WorkflowNode Methods
// ============================================================================

impl WorkflowNode {
    #[must_use]
    pub const fn category(&self) -> NodeCategory {
        match self {
            Self::CronTrigger(_)
            | Self::HttpHandler(_)
            | Self::KafkaConsumer(_)
            | Self::KafkaHandler(_) => NodeCategory::Entry,
            Self::Awakeable(_)
            | Self::DelayedSend(_)
            | Self::DurablePromise(_)
            | Self::HttpCall(_)
            | Self::ObjectCall(_)
            | Self::PeekPromise(_)
            | Self::ResolvePromise(_)
            | Self::Run(_)
            | Self::SendMessage(_)
            | Self::ServiceCall(_)
            | Self::WaitForWebhook(_)
            | Self::WorkflowCall(_) => NodeCategory::Durable,
            Self::ClearAll(_)
            | Self::ClearState(_)
            | Self::GetState(_)
            | Self::LoadFromMemory(_)
            | Self::SaveToMemory(_)
            | Self::SetState(_) => NodeCategory::State,
            Self::Compensate(_)
            | Self::Condition(_)
            | Self::Loop(_)
            | Self::LoopIterate(_)
            | Self::Parallel(_)
            | Self::Switch(_)
            | Self::WorkflowSubmit(_) => NodeCategory::Flow,
            Self::Sleep(_) | Self::Timeout(_) | Self::TimeoutGuard(_) => NodeCategory::Timing,
            Self::SignalHandler(_) => NodeCategory::Signal,
        }
    }

    #[must_use]
    pub const fn icon(&self) -> super::NodeIcon {
        match self {
            Self::Awakeable(_) | Self::WaitForWebhook(_) => super::NodeIcon::Radio,
            Self::ClearAll(_) | Self::ClearState(_) => super::NodeIcon::Trash,
            Self::Compensate(_) => super::NodeIcon::Undo,
            Self::Condition(_) => super::NodeIcon::GitBranch,
            Self::CronTrigger(_) => super::NodeIcon::Clock,
            Self::DelayedSend(_) => super::NodeIcon::ClockSend,
            Self::DurablePromise(_) => super::NodeIcon::Shield,
            Self::GetState(_) | Self::LoadFromMemory(_) => super::NodeIcon::Database,
            Self::HttpCall(_) | Self::ServiceCall(_) => super::NodeIcon::Call,
            Self::HttpHandler(_) => super::NodeIcon::Globe,
            Self::KafkaConsumer(_) | Self::KafkaHandler(_) => super::NodeIcon::Kafka,
            Self::Loop(_) | Self::LoopIterate(_) => super::NodeIcon::Repeat,
            Self::ObjectCall(_) => super::NodeIcon::Box,
            Self::Parallel(_) => super::NodeIcon::Layers,
            Self::PeekPromise(_) => super::NodeIcon::Eye,
            Self::ResolvePromise(_) => super::NodeIcon::CheckCircle,
            Self::Run(_) => super::NodeIcon::Play,
            Self::SaveToMemory(_) | Self::SetState(_) => super::NodeIcon::Save,
            Self::SendMessage(_) => super::NodeIcon::Send,
            Self::SignalHandler(_) => super::NodeIcon::Bell,
            Self::Sleep(_) | Self::Timeout(_) | Self::TimeoutGuard(_) => super::NodeIcon::Timer,
            Self::Switch(_) => super::NodeIcon::GitMerge,
            Self::WorkflowCall(_) | Self::WorkflowSubmit(_) => super::NodeIcon::Workflow,
        }
    }

    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Awakeable(_) => "Awakeable callback",
            Self::ClearAll(_) => "Clear all state values",
            Self::ClearState(_) => "Clear state value",
            Self::Compensate(_) => "Compensating transaction",
            Self::Condition(_) => "Conditional branch",
            Self::CronTrigger(_) => "Scheduled cron trigger",
            Self::DelayedSend(_) => "Send delayed message",
            Self::DurablePromise(_) => "Durable promise",
            Self::GetState(_) => "Get state value",
            Self::HttpCall(_) => "Call external HTTP API",
            Self::HttpHandler(_) => "HTTP request handler",
            Self::KafkaConsumer(_) => "Kafka message consumer",
            Self::KafkaHandler(_) => "Kafka message handler",
            Self::LoadFromMemory(_) => "Load from memory",
            Self::Loop(_) | Self::LoopIterate(_) => "Iterate over collection",
            Self::ObjectCall(_) => "Call Restate object",
            Self::Parallel(_) => "Execute in parallel",
            Self::PeekPromise(_) => "Non-blocking promise inspection",
            Self::ResolvePromise(_) => "Resolve promise",
            Self::Run(_) => "Run arbitrary code",
            Self::SaveToMemory(_) => "Save to memory",
            Self::SendMessage(_) => "Send message to queue",
            Self::ServiceCall(_) => "Call Restate service",
            Self::SetState(_) => "Set state value",
            Self::SignalHandler(_) => "Signal handler",
            Self::Sleep(_) => "Wait for duration",
            Self::Switch(_) => "Multi-way branch",
            Self::Timeout(_) | Self::TimeoutGuard(_) => "Operation timeout",
            Self::WaitForWebhook(_) => "Wait for webhook",
            Self::WorkflowCall(_) => "Call Restate workflow",
            Self::WorkflowSubmit(_) => "Submit nested workflow",
        }
    }

    #[must_use]
    pub const fn output_port_type(&self) -> PortType {
        match self {
            Self::Awakeable(_)
            | Self::ClearAll(_)
            | Self::ClearState(_)
            | Self::Compensate(_)
            | Self::Condition(_)
            | Self::DelayedSend(_)
            | Self::DurablePromise(_)
            | Self::GetState(_)
            | Self::LoadFromMemory(_)
            | Self::Loop(_)
            | Self::LoopIterate(_)
            | Self::ObjectCall(_)
            | Self::Parallel(_)
            | Self::PeekPromise(_)
            | Self::ResolvePromise(_)
            | Self::Run(_)
            | Self::SaveToMemory(_)
            | Self::SendMessage(_)
            | Self::ServiceCall(_)
            | Self::SetState(_)
            | Self::Sleep(_)
            | Self::Switch(_)
            | Self::Timeout(_)
            | Self::TimeoutGuard(_)
            | Self::WaitForWebhook(_)
            | Self::WorkflowCall(_)
            | Self::WorkflowSubmit(_) => PortType::FlowControl,
            Self::CronTrigger(_) => PortType::Event,
            Self::HttpCall(_)
            | Self::HttpHandler(_)
            | Self::KafkaConsumer(_)
            | Self::KafkaHandler(_) => PortType::Json,
            Self::SignalHandler(_) => PortType::Signal,
        }
    }

    #[must_use]
    pub const fn input_port_type(&self) -> PortType {
        match self {
            Self::Awakeable(_)
            | Self::ClearAll(_)
            | Self::ClearState(_)
            | Self::Compensate(_)
            | Self::Condition(_)
            | Self::DelayedSend(_)
            | Self::DurablePromise(_)
            | Self::GetState(_)
            | Self::HttpCall(_)
            | Self::LoadFromMemory(_)
            | Self::Loop(_)
            | Self::LoopIterate(_)
            | Self::ObjectCall(_)
            | Self::Parallel(_)
            | Self::PeekPromise(_)
            | Self::ResolvePromise(_)
            | Self::Run(_)
            | Self::SaveToMemory(_)
            | Self::SendMessage(_)
            | Self::ServiceCall(_)
            | Self::SetState(_)
            | Self::Sleep(_)
            | Self::Switch(_)
            | Self::Timeout(_)
            | Self::TimeoutGuard(_)
            | Self::WaitForWebhook(_)
            | Self::WorkflowCall(_)
            | Self::WorkflowSubmit(_) => PortType::FlowControl,
            Self::CronTrigger(_) => PortType::Event,
            Self::HttpHandler(_) | Self::KafkaConsumer(_) | Self::KafkaHandler(_) => PortType::Json,
            Self::SignalHandler(_) => PortType::Signal,
        }
    }

    /// Returns the `ServiceKind` for this workflow node.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::{WorkflowNode, service_kinds::ServiceKind};
    /// assert_eq!(WorkflowNode::default().service_kind(), ServiceKind::Handler);
    /// ```
    #[must_use]
    pub const fn service_kind(&self) -> ServiceKind {
        match self {
            // Stateless services - Handler context
            Self::Compensate(_)
            | Self::Condition(_)
            | Self::CronTrigger(_)
            | Self::DelayedSend(_)
            | Self::HttpCall(_)
            | Self::HttpHandler(_)
            | Self::KafkaConsumer(_)
            | Self::KafkaHandler(_)
            | Self::Loop(_)
            | Self::LoopIterate(_)
            | Self::Parallel(_)
            | Self::Run(_)
            | Self::SendMessage(_)
            | Self::ServiceCall(_)
            | Self::SignalHandler(_)
            | Self::Sleep(_)
            | Self::Switch(_)
            | Self::Timeout(_)
            | Self::TimeoutGuard(_) => ServiceKind::Handler,

            // Stateful operations - Actor context
            Self::ClearAll(_)
            | Self::ClearState(_)
            | Self::GetState(_)
            | Self::LoadFromMemory(_)
            | Self::ObjectCall(_)
            | Self::SaveToMemory(_)
            | Self::SetState(_) => ServiceKind::Actor,

            // Workflow operations - Workflow context
            Self::Awakeable(_)
            | Self::DurablePromise(_)
            | Self::PeekPromise(_)
            | Self::ResolvePromise(_)
            | Self::WaitForWebhook(_)
            | Self::WorkflowCall(_)
            | Self::WorkflowSubmit(_) => ServiceKind::Workflow,
        }
    }

    /// Returns the `ContextType` for this workflow node.
    ///
    /// This is derived from the node's `ServiceKind`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::oya_frontend::graph::{WorkflowNode, service_kinds::ContextType};
    /// assert_eq!(WorkflowNode::default().context_type(), ContextType::Synchronous);
    /// ```
    #[must_use]
    pub const fn context_type(&self) -> crate::graph::service_kinds::ContextType {
        self.service_kind().context_type()
    }
}

// ============================================================================
// ConditionResult
// ============================================================================

/// Result of a condition evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionResult {
    True,
    False,
}

impl ConditionResult {
    #[must_use]
    pub const fn is_true(self) -> bool {
        matches!(self, Self::True)
    }

    #[must_use]
    pub const fn is_false(self) -> bool {
        matches!(self, Self::False)
    }

    #[must_use]
    pub const fn branch_port(self) -> &'static str {
        match self {
            Self::True => "true",
            Self::False => "false",
        }
    }

    #[must_use]
    pub const fn opposite_port(self) -> &'static str {
        match self {
            Self::True => "false",
            Self::False => "true",
        }
    }
}

impl From<bool> for ConditionResult {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }
}

impl From<ConditionResult> for bool {
    fn from(value: ConditionResult) -> Self {
        value.is_true()
    }
}

impl fmt::Display for ConditionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
        }
    }
}

// ============================================================================
// HttpMethod
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for HttpMethod {
    type Err = UnknownHttpMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "DELETE" => Ok(Self::Delete),
            "PATCH" => Ok(Self::Patch),
            "HEAD" => Ok(Self::Head),
            "OPTIONS" => Ok(Self::Options),
            _ => Err(UnknownHttpMethodError(s.to_string())),
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownHttpMethodError(pub String);

impl fmt::Display for UnknownHttpMethodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown HTTP method: {}", self.0)
    }
}

impl std::error::Error for UnknownHttpMethodError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownNodeTypeError(pub String);

impl std::fmt::Display for UnknownNodeTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown node type: {}", self.0)
    }
}

impl std::error::Error for UnknownNodeTypeError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn http_method_from_str_parses_get() {
        assert_eq!(HttpMethod::from_str("GET").unwrap(), HttpMethod::Get);
    }

    #[test]
    fn http_method_from_str_parses_lowercase_post() {
        assert_eq!(HttpMethod::from_str("post").unwrap(), HttpMethod::Post);
    }

    #[test]
    fn condition_result_from_bool_true() {
        let result: ConditionResult = true.into();
        assert!(result.is_true());
    }

    #[test]
    fn condition_result_into_bool_true() {
        let bool: bool = ConditionResult::True.into();
        assert!(bool);
    }
}
