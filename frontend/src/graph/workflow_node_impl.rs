//! Workflow node implementations.
//!
//! This module contains the implementation blocks for the WorkflowNode enum.

use super::{NodeCategory, WorkflowNode};
use crate::graph::restate_types::PortType;
use crate::graph::workflow_node::configs::*;
use crate::graph::workflow_node::UnknownNodeTypeError;
use std::fmt;
use std::str::FromStr;

// ===========================================================================
// WorkflowNode Implementation
// ===========================================================================

impl WorkflowNode {
    /// Get the category for this node type.
    #[must_use]
    pub fn category(&self) -> NodeCategory {
        match self {
            Self::HttpHandler(_) | Self::HttpCall(_) => NodeCategory::Entry,
            Self::KafkaHandler(_) => NodeCategory::Entry,
            Self::CronTrigger(_) => NodeCategory::Entry,
            Self::WorkflowSubmit(_) => NodeCategory::Entry,
            Self::Run(_) => NodeCategory::Durable,
            Self::ServiceCall(_) => NodeCategory::Durable,
            Self::ObjectCall(_) => NodeCategory::Durable,
            Self::WorkflowCall(_) => NodeCategory::Durable,
            Self::SendMessage(_) => NodeCategory::Durable,
            Self::DelayedSend(_) => NodeCategory::Durable,
            Self::GetState(_) => NodeCategory::State,
            Self::SetState(_) => NodeCategory::State,
            Self::ClearState(_) => NodeCategory::State,
            Self::ClearAll(_) => NodeCategory::State,
            Self::Condition(_) => NodeCategory::Flow,
            Self::Switch(_) => NodeCategory::Flow,
            Self::Loop(_) => NodeCategory::Flow,
            Self::Parallel(_) => NodeCategory::Flow,
            Self::Compensate(_) => NodeCategory::Flow,
            Self::Sleep(_) => NodeCategory::Timing,
            Self::Timeout(_) => NodeCategory::Timing,
            Self::DurablePromise(_) => NodeCategory::Signal,
            Self::Awakeable(_) => NodeCategory::Signal,
            Self::ResolvePromise(_) => NodeCategory::Signal,
            Self::SignalHandler(_) => NodeCategory::Signal,
        }
    }

    /// Get the icon for this node type.
    #[must_use]
    pub fn icon(&self) -> super::NodeIcon {
        use super::NodeIcon;

        match self {
            Self::HttpHandler(_) => NodeIcon::Globe,
            Self::HttpCall(_) => NodeIcon::Send,
            Self::KafkaHandler(_) => NodeIcon::Kafka,
            Self::CronTrigger(_) => NodeIcon::Clock,
            Self::WorkflowSubmit(_) => NodeIcon::PlayCircle,
            Self::Run(_) => NodeIcon::Shield,
            Self::ServiceCall(_) => NodeIcon::ArrowRight,
            Self::ObjectCall(_) => NodeIcon::Box,
            Self::WorkflowCall(_) => NodeIcon::Workflow,
            Self::SendMessage(_) => NodeIcon::Send,
            Self::DelayedSend(_) => NodeIcon::ClockSend,
            Self::GetState(_) => NodeIcon::Download,
            Self::SetState(_) => NodeIcon::Upload,
            Self::ClearState(_) => NodeIcon::Eraser,
            Self::ClearAll(_) => NodeIcon::Trash,
            Self::Condition(_) => NodeIcon::GitBranch,
            Self::Switch(_) => NodeIcon::GitMerge,
            Self::Loop(_) => NodeIcon::Repeat,
            Self::Parallel(_) => NodeIcon::Layers,
            Self::Compensate(_) => NodeIcon::Undo,
            Self::Sleep(_) => NodeIcon::Timer,
            Self::Timeout(_) => NodeIcon::Alarm,
            Self::DurablePromise(_) => NodeIcon::Sparkles,
            Self::Awakeable(_) => NodeIcon::Bell,
            Self::ResolvePromise(_) => NodeIcon::CheckCircle,
            Self::SignalHandler(_) => NodeIcon::Radio,
        }
    }

    /// Get the description for this node type.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::HttpHandler(_) => "HTTP handler node for receiving webhooks",
            Self::HttpCall(_) => "HTTP call node for making outbound requests",
            Self::KafkaHandler(_) => "Kafka consumer node for event streaming",
            Self::CronTrigger(_) => "Cron trigger node for scheduled execution",
            Self::WorkflowSubmit(_) => "Workflow submit node for starting new workflows",
            Self::Run(_) => "Durable step node for processing data",
            Self::ServiceCall(_) => "Service call node for REST API calls",
            Self::ObjectCall(_) => "Object call node for virtual object operations",
            Self::WorkflowCall(_) => "Workflow call node for nested workflow execution",
            Self::SendMessage(_) => "Send message node for message queuing",
            Self::DelayedSend(_) => "Delayed send node for scheduled messaging",
            Self::GetState(_) => "Get state node for reading workflow state",
            Self::SetState(_) => "Set state node for writing workflow state",
            Self::ClearState(_) => "Clear state node for removing workflow state",
            Self::ClearAll(_) => "Clear all state node for removing all workflow state",
            Self::Condition(_) => "Condition node for branching logic",
            Self::Switch(_) => "Switch node for multi-way branching",
            Self::Loop(_) => "Loop node for iteration",
            Self::Parallel(_) => "Parallel node for concurrent execution",
            Self::Compensate(_) => "Compensate node for rollback handling",
            Self::Sleep(_) => "Sleep node for delaying execution",
            Self::Timeout(_) => "Timeout node for deadline handling",
            Self::DurablePromise(_) => "Durable promise node for async operations",
            Self::Awakeable(_) => "Awakeable node for external signals",
            Self::ResolvePromise(_) => "Resolve promise node for signaling completion",
            Self::SignalHandler(_) => "Signal handler node for event processing",
        }
    }

    /// Get the output port type for this node.
    #[must_use]
    pub fn output_port_type(&self) -> PortType {
        match self {
            Self::HttpHandler(_) => PortType::Json,
            Self::HttpCall(_) => PortType::Json,
            Self::KafkaHandler(_) => PortType::Event,
            Self::CronTrigger(_) => PortType::Event,
            Self::WorkflowSubmit(_) => PortType::Json,
            Self::Run(_) => PortType::Json,
            Self::ServiceCall(_) => PortType::Json,
            Self::ObjectCall(_) => PortType::Json,
            Self::WorkflowCall(_) => PortType::Json,
            Self::SendMessage(_) => PortType::Json,
            Self::DelayedSend(_) => PortType::Json,
            Self::GetState(_) => PortType::State,
            Self::SetState(_) => PortType::State,
            Self::ClearState(_) => PortType::State,
            Self::ClearAll(_) => PortType::State,
            Self::Condition(_) => PortType::FlowControl,
            Self::Switch(_) => PortType::FlowControl,
            Self::Loop(_) => PortType::Json,
            Self::Parallel(_) => PortType::Json,
            Self::Compensate(_) => PortType::Json,
            Self::Sleep(_) => PortType::Json,
            Self::Timeout(_) => PortType::Json,
            Self::DurablePromise(_) => PortType::Signal,
            Self::Awakeable(_) => PortType::Signal,
            Self::ResolvePromise(_) => PortType::Signal,
            Self::SignalHandler(_) => PortType::Signal,
        }
    }

    /// Get the input port type for this node.
    #[must_use]
    pub fn input_port_type(&self) -> PortType {
        match self {
            Self::HttpHandler(_) => PortType::Any,
            Self::HttpCall(_) => PortType::Any,
            Self::KafkaHandler(_) => PortType::Any,
            Self::CronTrigger(_) => PortType::Any,
            Self::WorkflowSubmit(_) => PortType::Any,
            Self::Run(_) => PortType::Any,
            Self::ServiceCall(_) => PortType::Any,
            Self::ObjectCall(_) => PortType::Any,
            Self::WorkflowCall(_) => PortType::Any,
            Self::SendMessage(_) => PortType::Any,
            Self::DelayedSend(_) => PortType::Any,
            Self::GetState(_) => PortType::State,
            Self::SetState(_) => PortType::State,
            Self::ClearState(_) => PortType::State,
            Self::ClearAll(_) => PortType::State,
            Self::Condition(_) => PortType::Json,
            Self::Switch(_) => PortType::Json,
            Self::Loop(_) => PortType::Json,
            Self::Parallel(_) => PortType::Json,
            Self::Compensate(_) => PortType::Json,
            Self::Sleep(_) => PortType::Json,
            Self::Timeout(_) => PortType::Json,
            Self::DurablePromise(_) => PortType::Signal,
            Self::Awakeable(_) => PortType::Signal,
            Self::ResolvePromise(_) => PortType::Signal,
            Self::SignalHandler(_) => PortType::Signal,
        }
    }
}

impl fmt::Display for WorkflowNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for WorkflowNode {
    type Err = UnknownNodeTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http-handler" => Ok(Self::HttpHandler(HttpHandlerConfig::default())),
            "http-call" => Ok(Self::HttpCall(HttpCallConfig::default())),
            "kafka-handler" => Ok(Self::KafkaHandler(KafkaHandlerConfig::default())),
            "cron-trigger" => Ok(Self::CronTrigger(CronTriggerConfig::default())),
            "workflow-submit" => Ok(Self::WorkflowSubmit(WorkflowSubmitConfig::default())),
            "run" => Ok(Self::Run(RunConfig::default())),
            "service-call" => Ok(Self::ServiceCall(ServiceCallConfig::default())),
            "object-call" => Ok(Self::ObjectCall(ObjectCallConfig::default())),
            "workflow-call" => Ok(Self::WorkflowCall(WorkflowCallConfig::default())),
            "send-message" => Ok(Self::SendMessage(SendMessageConfig::default())),
            "delayed-send" => Ok(Self::DelayedSend(DelayedSendConfig::default())),
            "get-state" => Ok(Self::GetState(GetStateConfig::default())),
            "set-state" => Ok(Self::SetState(SetStateConfig::default())),
            "clear-state" => Ok(Self::ClearState(ClearStateConfig::default())),
            "clear-all" => Ok(Self::ClearAll(ClearAllConfig::default())),
            "condition" => Ok(Self::Condition(ConditionConfig::default())),
            "switch" => Ok(Self::Switch(SwitchConfig::default())),
            "loop" => Ok(Self::Loop(LoopConfig::default())),
            "parallel" => Ok(Self::Parallel(ParallelConfig::default())),
            "compensate" => Ok(Self::Compensate(CompensateConfig::default())),
            "sleep" => Ok(Self::Sleep(SleepConfig::default())),
            "timeout" => Ok(Self::Timeout(TimeoutConfig::default())),
            "durable-promise" => Ok(Self::DurablePromise(DurablePromiseConfig::default())),
            "awakeable" => Ok(Self::Awakeable(AwakeableConfig::default())),
            "resolve-promise" => Ok(Self::ResolvePromise(ResolvePromiseConfig::default())),
            "signal-handler" => Ok(Self::SignalHandler(SignalHandlerConfig::default())),
            _ => Err(UnknownNodeTypeError(s.to_string())),
        }
    }
}
