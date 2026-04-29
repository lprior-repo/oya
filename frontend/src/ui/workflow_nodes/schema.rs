use serde::{Deserialize, Serialize};
use std::fmt;

fn deserialize_empty_to_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: From<String> + Deserialize<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|s| !s.is_empty()).map(T::from))
}

// --- Basic Config Types (Newtypes) ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct MemoryKey(pub String);
impl MemoryKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for MemoryKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for MemoryKey {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ServiceName(pub String);
impl ServiceName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for ServiceName {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for ServiceName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HandlerName(pub String);
impl HandlerName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for HandlerName {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for HandlerName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ObjectKey(pub String);
impl ObjectKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for ObjectKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for ObjectKey {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct PromiseName(pub String);
impl PromiseName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for PromiseName {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for PromiseName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HttpPath(pub String);
impl HttpPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for HttpPath {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for HttpPath {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct CronExpression(pub String);
impl CronExpression {
    pub fn new(expr: impl Into<String>) -> Self {
        Self(expr.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for CronExpression {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for CronExpression {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct CodeContent(pub String);
impl CodeContent {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for CodeContent {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for CodeContent {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Condition(pub String);
impl Condition {
    pub fn new(expr: impl Into<String>) -> Self {
        Self(expr.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for Condition {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for Condition {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct BranchName(pub String);
impl BranchName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for BranchName {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl std::ops::Deref for BranchName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct NodeId(pub uuid::Uuid);
impl NodeId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct RouterBranchId(pub String);
impl RouterBranchId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

// --- Workflow Node Enum (Unified) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: HttpPath::default(),
            method: HttpMethod::POST,
        })
    }
}

// --- Configuration Structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwakeableConfig {
    pub awakeable_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearStateConfig {
    pub key: ObjectKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClearAllConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensateConfig {
    pub handler_name: HandlerName,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConditionConfig {
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub expression: Option<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronTriggerConfig {
    pub schedule: CronExpression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayedSendConfig {
    pub target_type: TargetType,
    pub service_name: ServiceName,
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub key: Option<ObjectKey>,
    pub handler_name: HandlerName,
    pub input: serde_json::Value,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurablePromiseConfig {
    pub promise_name: PromiseName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStateConfig {
    pub key: ObjectKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpCallConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHandlerConfig {
    pub path: HttpPath,
    pub method: HttpMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KafkaHandlerConfig {
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub iterator: String,
    pub collection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectCallConfig {
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub object_name: Option<ServiceName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    pub branches: Vec<ParallelBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelBranch {
    #[serde(default)]
    pub id: RouterBranchId,
    pub name: BranchName,
    pub next_node_id: Option<crate::graph::NodeId>,
}

impl ParallelBranch {
    pub fn new(name: impl Into<String>) -> Self {
        Self { id: RouterBranchId::new(), name: BranchName::new(name), next_node_id: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvePromiseConfig {
    pub promise_name: PromiseName,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeekPromiseConfig {
    pub promise_name: PromiseName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub code: CodeContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageConfig {
    pub target_type: TargetType,
    pub service_name: ServiceName,
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub key: Option<ObjectKey>,
    pub handler_name: HandlerName,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCallConfig {
    #[serde(default)]
    pub target_type: TargetType,
    pub service_name: ServiceName,
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub key: Option<ObjectKey>,
    pub handler_name: HandlerName,
    pub input: serde_json::Value,
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStateConfig {
    pub key: ObjectKey,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalHandlerConfig {
    pub signal_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepConfig {
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SwitchConfig {
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub expression: Option<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowCallConfig {
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub workflow_name: Option<ServiceName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSubmitConfig {
    pub workflow_name: ServiceName,
    #[serde(default, deserialize_with = "deserialize_empty_to_none")]
    pub key: Option<ObjectKey>,
    pub input: serde_json::Value,
}

// --- Supporting Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TargetType {
    #[default]
    Service,
    VirtualObject,
    Workflow,
}
