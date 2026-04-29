pub mod preview_calc;

use crate::graph::workflow_node::WorkflowNode;
use crate::graph::{Node, NodeCategory, NodeId, PortName, Workflow};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionPriority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionKey {
    AddEntryTrigger,
    AddReliabilityBundle,
    AddTimeoutGuard,
    AddDurableCheckpoint,
    AddCompensationBranch,
    AddSignalResolution,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionPresetKey {
    Webhook,
    Approval,
    RetrySaga,
}

impl ExtensionPresetKey {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Webhook => "webhook",
            Self::Approval => "approval",
            Self::RetrySaga => "retry-saga",
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Webhook => "Webhook Reliability",
            Self::Approval => "Approval Gate",
            Self::RetrySaga => "Retry Saga",
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Webhook => {
                "Bootstraps webhook entry with timeout and durable checkpoint safeguards."
            }
            Self::Approval => {
                "Adds explicit approval orchestration with signal resolution and compensation fallback."
            }
            Self::RetrySaga => {
                "Builds retry-oriented saga safety with timeout, checkpointing, and compensation paths."
            }
        }
    }

    #[must_use]
    pub const fn extension_keys(self) -> &'static [ExtensionKey] {
        match self {
            Self::Webhook => &[ExtensionKey::AddTimeoutGuard, ExtensionKey::AddDurableCheckpoint],
            Self::Approval => {
                &[ExtensionKey::AddSignalResolution, ExtensionKey::AddCompensationBranch]
            }
            Self::RetrySaga => {
                &[ExtensionKey::AddDurableCheckpoint, ExtensionKey::AddCompensationBranch]
            }
        }
    }
}

impl FromStr for ExtensionPresetKey {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "webhook" => Ok(Self::Webhook),
            "approval" => Ok(Self::Approval),
            "retry-saga" => Ok(Self::RetrySaga),
            _ => Err(format!("Unknown extension preset key: {value}")),
        }
    }
}

impl ExtensionKey {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AddEntryTrigger => "add-entry-trigger",
            Self::AddReliabilityBundle => "add-reliability-bundle",
            Self::AddTimeoutGuard => "add-timeout-guard",
            Self::AddDurableCheckpoint => "add-durable-checkpoint",
            Self::AddCompensationBranch => "add-compensation-branch",
            Self::AddSignalResolution => "add-signal-resolution",
        }
    }
}

impl FromStr for ExtensionKey {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "add-entry-trigger" => Ok(Self::AddEntryTrigger),
            "add-reliability-bundle" => Ok(Self::AddReliabilityBundle),
            "add-timeout-guard" => Ok(Self::AddTimeoutGuard),
            "add-durable-checkpoint" => Ok(Self::AddDurableCheckpoint),
            "add-compensation-branch" => Ok(Self::AddCompensationBranch),
            "add-signal-resolution" => Ok(Self::AddSignalResolution),
            _ => Err(format!("Unknown extension key: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleContract {
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlowExtension {
    pub key: String,
    pub title: String,
    pub rationale: String,
    pub priority: ExtensionPriority,
    pub contract: RuleContract,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedExtension {
    pub key: String,
    pub created_nodes: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RationaleClass {
    StructuralCoverage,
    ReliabilityBundle,
    RuntimeSafety,
    StateSafety,
    FailureRecovery,
    AsyncCoordination,
}

pub use crate::graph::restate_types::ServiceKind as RestateServiceKind;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum RestateCapability {
    EntryTrigger,
    DurableExecution,
    TimerGuard,
    StateStore,
    Compensation,
    PromiseResolution,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionSemantics {
    pub compatible_service_kinds: Vec<RestateServiceKind>,
    pub requires: Vec<RestateCapability>,
    pub provides: Vec<RestateCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionSuggestionAnalysis {
    pub key: String,
    pub score: f32,
    pub rationale_class: RationaleClass,
    pub fingerprint: String,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub semantics: ExtensionSemantics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictKind {
    DuplicateKey,
    DuplicateFingerprint,
    SemanticIncompatibility,
    WorkflowSemanticMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionConflict {
    pub left_key: String,
    pub right_key: String,
    pub kind: ConflictKind,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionDependencyEdge {
    pub dependency: String,
    pub dependent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionDependencyGraph {
    pub keys: Vec<String>,
    pub edges: Vec<ExtensionDependencyEdge>,
    pub ordered_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionPreset {
    pub key: String,
    pub title: String,
    pub description: String,
    pub extension_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedExtensionPreset {
    pub preset: ExtensionPreset,
    pub ordered_keys: Vec<String>,
    pub conflicts: Vec<ExtensionConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompoundPlanStep {
    pub key: String,
    pub confidence_score: f32,
    pub rationale_class: RationaleClass,
    pub fingerprint: String,
    #[serde(default)]
    pub semantics: ExtensionSemantics,
    pub preview: ExtensionPatchPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompoundExtensionPlan {
    pub ordered_keys: Vec<String>,
    pub conflicts: Vec<ExtensionConflict>,
    pub steps: Vec<CompoundPlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionPatchPreview {
    pub key: String,
    pub nodes: Vec<PreviewNode>,
    pub connections: Vec<PreviewConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreviewNode {
    pub temp_id: String,
    pub node_type: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum PreviewEndpoint {
    Existing(NodeId),
    Proposed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewConnection {
    pub source: PreviewEndpoint,
    pub target: PreviewEndpoint,
    pub source_port: String,
    pub target_port: String,
}

#[derive(Clone)]
struct RuleDefinition {
    key: ExtensionKey,
    title: &'static str,
    priority: ExtensionPriority,
    contract: RuleContract,
    plan: fn(&Workflow) -> Option<RulePlan>,
}

#[derive(Clone)]
struct RulePlan {
    rationale: String,
    patch: PatchPlan,
}

#[derive(Clone)]
struct PatchPlan {
    nodes: Vec<PatchNode>,
    connections: Vec<PatchConnection>,
}

#[derive(Clone)]
struct PatchNode {
    node_type: &'static str,
    x: f32,
    y: f32,
}

#[derive(Clone)]
struct PatchConnection {
    source: PatchEndpoint,
    target: PatchEndpoint,
    source_port: &'static str,
    target_port: &'static str,
}

#[derive(Clone, Copy)]
enum PatchEndpoint {
    Existing(NodeId),
    Proposed(usize),
}

#[must_use]
pub fn suggest_extensions(workflow: &Workflow) -> Vec<FlowExtension> {
    hide_isolated_reliability_hints(
        rules()
            .into_iter()
            .filter_map(|rule| {
                if !key_is_compatible_with_workflow(workflow, rule.key) {
                    return None;
                }
                (rule.plan)(workflow).map(|rule_plan| FlowExtension {
                    key: rule.key.as_str().to_string(),
                    title: rule.title.to_string(),
                    rationale: rule_plan.rationale,
                    priority: rule.priority,
                    contract: rule.contract,
                })
            })
            .collect(),
    )
}

#[must_use]
pub fn suggest_extensions_with_analysis(workflow: &Workflow) -> Vec<ExtensionSuggestionAnalysis> {
    hide_isolated_reliability_analysis(
        rules()
            .into_iter()
            .filter_map(|rule| {
                if !key_is_compatible_with_workflow(workflow, rule.key) {
                    return None;
                }
                (rule.plan)(workflow).map(|plan| ExtensionSuggestionAnalysis {
                    key: rule.key.as_str().to_string(),
                    score: confidence_score_for(rule.key, workflow),
                    rationale_class: rationale_class_for(rule.key),
                    fingerprint: extension_fingerprint(rule.key, &plan.patch),
                    dependencies: extension_dependencies(rule.key)
                        .iter()
                        .map(|dependency| dependency.as_str().to_string())
                        .collect(),
                    semantics: extension_semantics(rule.key),
                })
            })
            .collect(),
    )
}

/// Preview an extension without applying it.
///
/// # Errors
///
/// Returns `String` if the key is invalid.
pub fn preview_extension(
    workflow: &Workflow,
    key: &str,
) -> Result<Option<ExtensionPatchPreview>, String> {
    let parsed_key = ExtensionKey::from_str(key)?;

    Ok(plan_for_key(workflow, parsed_key)
        .map(|plan| preview_from_patch(key.to_string(), &plan.patch)))
}

/// Apply an extension to a workflow.
///
/// # Errors
///
/// Returns `String` if the key is invalid or application fails.
pub fn apply_extension(workflow: &mut Workflow, key: &str) -> Result<AppliedExtension, String> {
    let parsed_key = ExtensionKey::from_str(key)?;
    if parsed_key == ExtensionKey::AddReliabilityBundle {
        return apply_reliability_bundle(workflow, key);
    }

    let created_nodes = plan_for_key(workflow, parsed_key)
        .map(|plan| {
            let fingerprint = extension_fingerprint(parsed_key, &plan.patch);
            if has_extension_fingerprint(workflow, &fingerprint) {
                Vec::new()
            } else {
                execute_patch(workflow, parsed_key, &fingerprint, &plan.patch)
            }
        })
        .unwrap_or_default();

    Ok(AppliedExtension { key: key.to_string(), created_nodes })
}

/// Detect conflicts between extensions.
///
/// # Errors
///
/// Returns `String` if a key is invalid.
pub fn detect_extension_conflicts(
    workflow: &Workflow,
    keys: &[String],
) -> Result<Vec<ExtensionConflict>, String> {
    let mut conflicts = Vec::new();
    let mut seen: HashSet<ExtensionKey> = HashSet::new();
    let mut fingerprint_map: HashMap<String, String> = HashMap::new();

    for raw_key in keys {
        let key = ExtensionKey::from_str(raw_key)?;
        if !seen.insert(key) {
            conflicts.push(ExtensionConflict {
                left_key: raw_key.clone(),
                right_key: raw_key.clone(),
                kind: ConflictKind::DuplicateKey,
                reason: "Extension key is duplicated in the request.".to_string(),
            });
        }

        if let Some(plan) = plan_for_key(workflow, key) {
            let fingerprint = extension_fingerprint(key, &plan.patch);
            if let Some(existing_key) = fingerprint_map.get(&fingerprint) {
                conflicts.push(ExtensionConflict {
                    left_key: existing_key.clone(),
                    right_key: raw_key.clone(),
                    kind: ConflictKind::DuplicateFingerprint,
                    reason: "Two extensions resolve to the same patch fingerprint.".to_string(),
                });
            } else {
                fingerprint_map.insert(fingerprint, raw_key.clone());
            }
        }
    }

    for left_index in 0..keys.len() {
        for right_index in (left_index + 1)..keys.len() {
            let left_key = ExtensionKey::from_str(&keys[left_index])?;
            let right_key = ExtensionKey::from_str(&keys[right_index])?;
            if !have_shared_service_kind(left_key, right_key) {
                conflicts.push(ExtensionConflict {
                    left_key: keys[left_index].clone(),
                    right_key: keys[right_index].clone(),
                    kind: ConflictKind::SemanticIncompatibility,
                    reason: "Requested extensions target incompatible Restate service kinds."
                        .to_string(),
                });
            }
        }
    }

    let inferred_kinds = infer_workflow_service_kinds(workflow);
    for raw_key in keys {
        let key = ExtensionKey::from_str(raw_key)?;
        let supported_kinds =
            extension_semantics(key).compatible_service_kinds.into_iter().collect::<HashSet<_>>();
        if inferred_kinds.is_disjoint(&supported_kinds) {
            conflicts.push(ExtensionConflict {
                left_key: raw_key.clone(),
                right_key: raw_key.clone(),
                kind: ConflictKind::WorkflowSemanticMismatch,
                reason:
                    "Extension is incompatible with inferred Restate service/object/workflow context."
                        .to_string(),
            });
        }
    }

    Ok(conflicts)
}

/// Build dependency graph for extensions.
///
/// # Errors
///
/// Returns `String` if a key is invalid.
pub fn extension_dependency_graph(keys: &[String]) -> Result<ExtensionDependencyGraph, String> {
    let parsed_keys = parse_unique_keys(keys)?;
    let edges = parsed_keys
        .iter()
        .flat_map(|key| {
            extension_dependencies(*key)
                .iter()
                .filter(|dependency| parsed_keys.contains(dependency))
                .map(|dependency| ExtensionDependencyEdge {
                    dependency: dependency.as_str().to_string(),
                    dependent: key.as_str().to_string(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let ordered_keys = order_keys_with_dependencies(&parsed_keys)?
        .into_iter()
        .map(|key| key.as_str().to_string())
        .collect::<Vec<_>>();

    Ok(ExtensionDependencyGraph {
        keys: parsed_keys.iter().map(|key| key.as_str().to_string()).collect(),
        edges,
        ordered_keys,
    })
}

/// Generate a compound plan for multiple extensions.
///
/// # Errors
///
/// Returns `String` if keys are invalid or conflicts exist.
pub fn generate_compound_plan(
    workflow: &Workflow,
    keys: &[String],
) -> Result<CompoundExtensionPlan, String> {
    let parsed_keys = parse_unique_keys(keys)?;
    let conflicts = detect_extension_conflicts(workflow, keys)?;
    let ordered = order_keys_with_dependencies(&parsed_keys)?;
    let analyses = suggest_extensions_with_analysis(workflow)
        .into_iter()
        .map(|analysis| (analysis.key.clone(), analysis))
        .collect::<HashMap<_, _>>();

    let mut simulation = workflow.clone();
    let mut steps = Vec::new();

    for key in &ordered {
        let key_str = key.as_str();
        if let Some(preview) = preview_extension(&simulation, key_str)? {
            let analysis_data = analyses
                .get(key_str)
                .cloned()
                .unwrap_or_else(|| fallback_analysis(key_str, &simulation, *key));
            apply_extension(&mut simulation, key_str)?;
            steps.push(CompoundPlanStep {
                key: key_str.to_string(),
                confidence_score: analysis_data.score,
                rationale_class: analysis_data.rationale_class,
                fingerprint: analysis_data.fingerprint,
                semantics: analysis_data.semantics,
                preview,
            });
        }
    }

    Ok(CompoundExtensionPlan {
        ordered_keys: ordered.into_iter().map(|key| key.as_str().to_string()).collect(),
        conflicts,
        steps,
    })
}

#[must_use]
pub fn extension_presets() -> Vec<ExtensionPreset> {
    [ExtensionPresetKey::Webhook, ExtensionPresetKey::Approval, ExtensionPresetKey::RetrySaga]
        .iter()
        .map(|preset_key| ExtensionPreset {
            key: preset_key.as_str().to_string(),
            title: preset_key.title().to_string(),
            description: preset_key.description().to_string(),
            extension_keys: preset_key
                .extension_keys()
                .iter()
                .map(|key| key.as_str().to_string())
                .collect(),
        })
        .collect()
}

/// Resolve an extension preset into individual extension keys.
///
/// # Errors
///
/// Returns `String` if preset key is invalid or conflicts exist.
pub fn resolve_extension_preset(
    workflow: &Workflow,
    preset_key: &str,
) -> Result<ResolvedExtensionPreset, String> {
    let parsed_preset = ExtensionPresetKey::from_str(preset_key)?;
    let expanded_keys = expand_keys_with_dependencies(parsed_preset.extension_keys());
    let ordered_keys = order_keys_with_dependencies(&expanded_keys)?
        .into_iter()
        .map(|key| key.as_str().to_string())
        .collect::<Vec<_>>();
    let conflicts = detect_extension_conflicts(workflow, &ordered_keys)?;

    Ok(ResolvedExtensionPreset {
        preset: ExtensionPreset {
            key: parsed_preset.as_str().to_string(),
            title: parsed_preset.title().to_string(),
            description: parsed_preset.description().to_string(),
            extension_keys: parsed_preset
                .extension_keys()
                .iter()
                .map(|key| key.as_str().to_string())
                .collect(),
        },
        ordered_keys,
        conflicts,
    })
}

fn rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            key: ExtensionKey::AddEntryTrigger,
            title: "Add entry trigger",
            priority: ExtensionPriority::High,
            contract: RuleContract {
                preconditions: vec!["Workflow has no Entry category node.".to_string()],
                postconditions: vec!["An HTTP entry trigger exists in workflow nodes.".to_string()],
                invariants: vec![
                    "Existing user-authored graph topology remains intact.".to_string()
                ],
            },
            plan: plan_missing_entry,
        },
        RuleDefinition {
            key: ExtensionKey::AddReliabilityBundle,
            title: "Add reliability bundle",
            priority: ExtensionPriority::High,
            contract: RuleContract {
                preconditions: vec![
                    "Workflow contains side-effecting durable steps and is missing one or more reliability protections.".to_string(),
                ],
                postconditions: vec![
                    "A coherent bundle is generated across timeout, state checkpoint, and compensation recommendations where applicable.".to_string(),
                ],
                invariants: vec![
                    "Bundle planning stays deterministic and idempotent across repeated applies.".to_string(),
                ],
            },
            plan: plan_reliability_bundle,
        },
        RuleDefinition {
            key: ExtensionKey::AddTimeoutGuard,
            title: "Add timeout guard",
            priority: ExtensionPriority::High,
            contract: RuleContract {
                preconditions: vec![
                    "Workflow has at least one Durable node and no timeout node.".to_string(),
                ],
                postconditions: vec![
                    "A timeout node is created and connected from a durable anchor.".to_string(),
                ],
                invariants: vec!["No self-connections are introduced.".to_string()],
            },
            plan: plan_missing_timeout_guard,
        },
        RuleDefinition {
            key: ExtensionKey::AddDurableCheckpoint,
            title: "Add state checkpoint",
            priority: ExtensionPriority::Medium,
            contract: RuleContract {
                preconditions: vec![
                    "Workflow has at least one Durable node and no set-state node.".to_string(),
                ],
                postconditions: vec![
                    "A set-state node is created and connected from a durable anchor.".to_string(),
                ],
                invariants: vec!["Existing nodes are not removed implicitly.".to_string()],
            },
            plan: plan_missing_checkpoint,
        },
        RuleDefinition {
            key: ExtensionKey::AddCompensationBranch,
            title: "Add compensation branch",
            priority: ExtensionPriority::Medium,
            contract: RuleContract {
                preconditions: vec![
                    "At least one condition node is missing true/false branch coverage."
                        .to_string(),
                ],
                postconditions: vec![
                    "A compensate node exists on the false branch of the condition anchor."
                        .to_string(),
                ],
                invariants: vec!["Condition branch wiring remains explicit.".to_string()],
            },
            plan: plan_unbalanced_condition,
        },
        RuleDefinition {
            key: ExtensionKey::AddSignalResolution,
            title: "Add signal resolution",
            priority: ExtensionPriority::Medium,
            contract: RuleContract {
                preconditions: vec![
                    "Workflow waits on awakeable/promise but has no resolve-promise node."
                        .to_string(),
                ],
                postconditions: vec![
                    "A resolve-promise node is created and connected to the wait anchor."
                        .to_string(),
                ],
                invariants: vec!["Signal wait path keeps deterministic node ordering.".to_string()],
            },
            plan: plan_missing_signal_resolution,
        },
    ]
}

fn plan_missing_entry(workflow: &Workflow) -> Option<RulePlan> {
    (!workflow.nodes.iter().any(|node| node.category == NodeCategory::Entry)).then(|| RulePlan {
        rationale:
            "Workflow has no entry node. Add an HTTP trigger so execution has a clear start."
                .to_string(),
        patch: PatchPlan {
            nodes: vec![PatchNode { node_type: "http-handler", x: 120.0, y: 100.0 }],
            connections: Vec::new(),
        },
    })
}

fn plan_missing_timeout_guard(workflow: &Workflow) -> Option<RulePlan> {
    let has_durable = workflow.nodes.iter().any(|node| node.category == NodeCategory::Durable);
    let has_timeout =
        workflow.nodes.iter().any(|node| matches!(node.node, WorkflowNode::Timeout(_)));
    let anchor = first_node_by_type(workflow, |node| node.category == NodeCategory::Durable)?;

    (has_durable && !has_timeout).then(|| RulePlan {
        rationale:
            "Durable calls are present without timeout nodes. Add a timeout guard for safer execution."
                .to_string(),
        patch: PatchPlan {
            nodes: vec![PatchNode {
                node_type: "timeout",
                x: anchor.x + 220.0,
                y: anchor.y,
            }],
            connections: vec![PatchConnection {
                source: PatchEndpoint::Existing(anchor.id),
                target: PatchEndpoint::Proposed(0),
                source_port: "out",
                target_port: "in",
            }],
        },
    })
}

fn plan_reliability_bundle(workflow: &Workflow) -> Option<RulePlan> {
    if !workflow.nodes.iter().any(is_side_effecting_durable) {
        return None;
    }

    let timeout = plan_missing_timeout_guard(workflow).map(|plan| ("timeout", plan));
    let checkpoint = key_is_compatible_with_workflow(workflow, ExtensionKey::AddDurableCheckpoint)
        .then(|| plan_missing_checkpoint(workflow))
        .flatten()
        .map(|plan| ("checkpoint", plan));
    let compensation = plan_unbalanced_condition(workflow).map(|plan| ("compensation", plan));

    let parts = [timeout, checkpoint, compensation].into_iter().flatten().collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }

    let mut patch = PatchPlan { nodes: Vec::new(), connections: Vec::new() };
    let mut labels = Vec::new();

    for (label, plan) in parts {
        labels.push(label);
        append_patch_plan(&mut patch, &plan.patch);
    }

    Some(RulePlan {
        rationale: format!(
            "Side-effecting ctx.run semantics are present. Recommend reliability bundle covering {}.",
            labels.join(", ")
        ),
        patch,
    })
}

fn plan_missing_checkpoint(workflow: &Workflow) -> Option<RulePlan> {
    let has_durable = workflow.nodes.iter().any(|node| node.category == NodeCategory::Durable);
    let has_state_write =
        workflow.nodes.iter().any(|node| matches!(node.node, WorkflowNode::SetState(_)));
    let anchor = first_node_by_type(workflow, |node| node.category == NodeCategory::Durable)?;

    (has_durable && !has_state_write).then(|| RulePlan {
        rationale:
            "No state write step found after durable actions. Add a checkpoint to persist progress."
                .to_string(),
        patch: PatchPlan {
            nodes: vec![PatchNode {
                node_type: "set-state",
                x: anchor.x + 220.0,
                y: anchor.y + 80.0,
            }],
            connections: vec![PatchConnection {
                source: PatchEndpoint::Existing(anchor.id),
                target: PatchEndpoint::Proposed(0),
                source_port: "out",
                target_port: "in",
            }],
        },
    })
}

fn plan_unbalanced_condition(workflow: &Workflow) -> Option<RulePlan> {
    let condition_node = workflow
        .nodes
        .iter()
        .find(|node| {
            matches!(node.node, WorkflowNode::Condition(_))
                && missing_condition_branch(workflow, node.id)
        })
        .cloned()?;

    Some(RulePlan {
        rationale:
            "At least one condition node is missing a true/false branch. Add compensation logic for failure paths.".to_string(),
        patch: PatchPlan {
            nodes: vec![PatchNode {
                node_type: "compensate",
                x: condition_node.x + 240.0,
                y: condition_node.y + 140.0,
            }],
            connections: vec![PatchConnection {
                source: PatchEndpoint::Existing(condition_node.id),
                target: PatchEndpoint::Proposed(0),
                source_port: "false",
                target_port: "in",
            }],
        },
    })
}

fn plan_missing_signal_resolution(workflow: &Workflow) -> Option<RulePlan> {
    let has_resolver =
        workflow.nodes.iter().any(|node| matches!(node.node, WorkflowNode::ResolvePromise(_)));

    let wait_node = first_node_by_type(workflow, |node| is_signal_wait_anchor(workflow, node))?;

    (!has_resolver).then(|| RulePlan {
        rationale:
            "Workflow waits on external events but has no resolve step. Add resolve-promise to close the loop.".to_string(),
        patch: PatchPlan {
            nodes: vec![PatchNode {
                node_type: "resolve-promise",
                x: wait_node.x + 220.0,
                y: wait_node.y,
            }],
            connections: vec![PatchConnection {
                source: PatchEndpoint::Existing(wait_node.id),
                target: PatchEndpoint::Proposed(0),
                source_port: "out",
                target_port: "in",
            }],
        },
    })
}

fn execute_patch(
    workflow: &mut Workflow,
    key: ExtensionKey,
    fingerprint: &str,
    patch: &PatchPlan,
) -> Vec<NodeId> {
    let created_nodes = patch
        .nodes
        .iter()
        .map(|node| workflow.add_node(node.node_type, node.x, node.y))
        .collect::<Vec<_>>();

    annotate_extension_nodes(workflow, key, fingerprint, &created_nodes);

    patch.connections.iter().for_each(|connection| {
        let source = resolve_patch_endpoint(connection.source, &created_nodes);
        let target = resolve_patch_endpoint(connection.target, &created_nodes);
        if let (Some(source_id), Some(target_id)) = (source, target) {
            let _ = workflow.add_connection_checked(
                source_id,
                target_id,
                &PortName::from(connection.source_port),
                &PortName::from(connection.target_port),
            );
        }
    });

    created_nodes
}

fn apply_reliability_bundle(
    workflow: &mut Workflow,
    key: &str,
) -> Result<AppliedExtension, String> {
    let mut created_nodes = Vec::new();
    for part in reliability_bundle_members() {
        let applied = apply_extension(workflow, part.as_str())?;
        created_nodes.extend(applied.created_nodes);
    }

    Ok(AppliedExtension { key: key.to_string(), created_nodes })
}

fn annotate_extension_nodes(
    workflow: &mut Workflow,
    key: ExtensionKey,
    fingerprint: &str,
    node_ids: &[NodeId],
) {
    for node_id in node_ids {
        if let Some(node) = workflow.nodes.iter_mut().find(|candidate| candidate.id == *node_id) {
            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "extension_key".to_string(),
                serde_json::Value::String(key.as_str().to_string()),
            );
            metadata.insert(
                "fingerprint".to_string(),
                serde_json::Value::String(fingerprint.to_string()),
            );
            metadata.insert(
                "restate_semantics".to_string(),
                serde_json::to_value(extension_semantics(key)).unwrap_or(serde_json::Value::Null),
            );
            node.metadata = serde_json::json!({ "flow_extender": metadata });
        }
    }
}

fn has_extension_fingerprint(workflow: &Workflow, fingerprint: &str) -> bool {
    workflow.nodes.iter().any(|node| {
        node.metadata
            .as_object()
            .and_then(|meta| meta.get("flow_extender"))
            .and_then(serde_json::Value::as_object)
            .and_then(|ext| ext.get("fingerprint"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value == fingerprint)
    })
}

fn append_patch_plan(target: &mut PatchPlan, patch: &PatchPlan) {
    let offset = target.nodes.len();
    target.nodes.extend(patch.nodes.iter().cloned());

    target.connections.extend(patch.connections.iter().map(|connection| PatchConnection {
        source: remap_endpoint(connection.source, offset),
        target: remap_endpoint(connection.target, offset),
        source_port: connection.source_port,
        target_port: connection.target_port,
    }));
}

const fn remap_endpoint(endpoint: PatchEndpoint, offset: usize) -> PatchEndpoint {
    match endpoint {
        PatchEndpoint::Existing(node_id) => PatchEndpoint::Existing(node_id),
        PatchEndpoint::Proposed(index) => PatchEndpoint::Proposed(index + offset),
    }
}

fn extension_fingerprint(key: ExtensionKey, patch: &PatchPlan) -> String {
    let node_parts = patch
        .nodes
        .iter()
        .map(|node| format!("{}@{:.2}:{:.2}", node.node_type, node.x, node.y))
        .collect::<Vec<_>>()
        .join("|");
    let connection_parts = patch
        .connections
        .iter()
        .map(|connection| {
            format!(
                "{}>{}:{}->{}",
                endpoint_signature(connection.source),
                endpoint_signature(connection.target),
                connection.source_port,
                connection.target_port
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    format!("{}::{node_parts}::{connection_parts}", key.as_str())
}

fn endpoint_signature(endpoint: PatchEndpoint) -> String {
    match endpoint {
        PatchEndpoint::Existing(node_id) => format!("e:{node_id}"),
        PatchEndpoint::Proposed(index) => format!("p:{index}"),
    }
}

fn plan_for_key(workflow: &Workflow, key: ExtensionKey) -> Option<RulePlan> {
    if !key_is_compatible_with_workflow(workflow, key) {
        return None;
    }

    rules()
        .into_iter()
        .find(|candidate| candidate.key == key)
        .and_then(|rule| (rule.plan)(workflow))
}

fn key_is_compatible_with_workflow(workflow: &Workflow, key: ExtensionKey) -> bool {
    let inferred_kinds = infer_workflow_service_kinds(workflow);
    let compatible_kinds =
        extension_semantics(key).compatible_service_kinds.into_iter().collect::<HashSet<_>>();
    !inferred_kinds.is_disjoint(&compatible_kinds)
}

fn infer_workflow_service_kinds(workflow: &Workflow) -> HashSet<RestateServiceKind> {
    let has_promise_semantics = workflow.nodes.iter().any(|node| {
        matches!(node.node, WorkflowNode::DurablePromise(_) | WorkflowNode::ResolvePromise(_))
            || is_signal_wait_anchor(workflow, node)
    });
    let has_state_semantics = workflow.nodes.iter().any(|node| {
        matches!(
            node.node,
            WorkflowNode::GetState(_) | WorkflowNode::SetState(_) | WorkflowNode::ClearState(_)
        )
    });

    if has_promise_semantics {
        HashSet::from([RestateServiceKind::Workflow])
    } else if has_state_semantics {
        HashSet::from([RestateServiceKind::Actor, RestateServiceKind::Workflow])
    } else if workflow.nodes.is_empty() {
        HashSet::from([
            RestateServiceKind::Handler,
            RestateServiceKind::Actor,
            RestateServiceKind::Workflow,
        ])
    } else {
        HashSet::from([RestateServiceKind::Handler])
    }
}

fn have_shared_service_kind(left: ExtensionKey, right: ExtensionKey) -> bool {
    let left_kinds =
        extension_semantics(left).compatible_service_kinds.into_iter().collect::<HashSet<_>>();
    let right_kinds =
        extension_semantics(right).compatible_service_kinds.into_iter().collect::<HashSet<_>>();
    !left_kinds.is_disjoint(&right_kinds)
}

fn extension_semantics(key: ExtensionKey) -> ExtensionSemantics {
    match key {
        ExtensionKey::AddEntryTrigger => ExtensionSemantics {
            compatible_service_kinds: vec![
                RestateServiceKind::Handler,
                RestateServiceKind::Actor,
                RestateServiceKind::Workflow,
            ],
            requires: vec![],
            provides: vec![RestateCapability::EntryTrigger],
        },
        ExtensionKey::AddReliabilityBundle => ExtensionSemantics {
            compatible_service_kinds: vec![
                RestateServiceKind::Handler,
                RestateServiceKind::Actor,
                RestateServiceKind::Workflow,
            ],
            requires: vec![RestateCapability::DurableExecution],
            provides: vec![RestateCapability::TimerGuard, RestateCapability::StateStore],
        },
        ExtensionKey::AddTimeoutGuard => ExtensionSemantics {
            compatible_service_kinds: vec![
                RestateServiceKind::Handler,
                RestateServiceKind::Actor,
                RestateServiceKind::Workflow,
            ],
            requires: vec![RestateCapability::DurableExecution],
            provides: vec![RestateCapability::TimerGuard],
        },
        ExtensionKey::AddDurableCheckpoint => ExtensionSemantics {
            compatible_service_kinds: vec![RestateServiceKind::Actor, RestateServiceKind::Workflow],
            requires: vec![RestateCapability::DurableExecution],
            provides: vec![RestateCapability::StateStore],
        },
        ExtensionKey::AddCompensationBranch => ExtensionSemantics {
            compatible_service_kinds: vec![
                RestateServiceKind::Handler,
                RestateServiceKind::Actor,
                RestateServiceKind::Workflow,
            ],
            requires: vec![RestateCapability::DurableExecution],
            provides: vec![RestateCapability::Compensation],
        },
        ExtensionKey::AddSignalResolution => ExtensionSemantics {
            compatible_service_kinds: vec![RestateServiceKind::Workflow],
            requires: vec![RestateCapability::PromiseResolution],
            provides: vec![RestateCapability::PromiseResolution],
        },
    }
}

fn parse_unique_keys(keys: &[String]) -> Result<Vec<ExtensionKey>, String> {
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();
    for raw_key in keys {
        let key = ExtensionKey::from_str(raw_key)?;
        if seen.insert(key) {
            parsed.push(key);
        }
    }
    Ok(parsed)
}

fn order_keys_with_dependencies(keys: &[ExtensionKey]) -> Result<Vec<ExtensionKey>, String> {
    let key_set = keys.iter().copied().collect::<HashSet<_>>();
    let mut indegree = keys.iter().copied().map(|key| (key, 0_u32)).collect::<HashMap<_, _>>();
    let mut adjacency = HashMap::<ExtensionKey, Vec<ExtensionKey>>::new();

    for key in keys {
        for dependency in extension_dependencies(*key) {
            if key_set.contains(dependency) {
                adjacency.entry(*dependency).or_default().push(*key);
                if let Some(count) = indegree.get_mut(key) {
                    *count += 1;
                }
            }
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(key, count)| (*count == 0).then_some(*key))
        .collect::<Vec<_>>();
    sort_keys(&mut ready);

    let mut ordered = Vec::new();
    while let Some(next_key) = ready.first().copied() {
        ready.remove(0);
        ordered.push(next_key);
        if let Some(dependents) = adjacency.get(&next_key) {
            for dependent in dependents {
                if let Some(count) = indegree.get_mut(dependent) {
                    if *count > 0 {
                        *count -= 1;
                    }
                    if *count == 0 {
                        ready.push(*dependent);
                        sort_keys(&mut ready);
                    }
                }
            }
        }
    }

    if ordered.len() != keys.len() {
        return Err("Extension dependency graph contains a cycle.".to_string());
    }

    Ok(ordered)
}

fn sort_keys(keys: &mut [ExtensionKey]) {
    keys.sort_by(|left, right| {
        priority_rank(*left)
            .cmp(&priority_rank(*right))
            .then_with(|| left.as_str().cmp(right.as_str()))
    });
}

const fn priority_rank(key: ExtensionKey) -> u8 {
    match key {
        ExtensionKey::AddEntryTrigger
        | ExtensionKey::AddReliabilityBundle
        | ExtensionKey::AddTimeoutGuard => 0,
        ExtensionKey::AddDurableCheckpoint
        | ExtensionKey::AddCompensationBranch
        | ExtensionKey::AddSignalResolution => 1,
    }
}

const fn extension_dependencies(key: ExtensionKey) -> &'static [ExtensionKey] {
    match key {
        ExtensionKey::AddEntryTrigger => &[],
        ExtensionKey::AddDurableCheckpoint => &[ExtensionKey::AddTimeoutGuard],
        ExtensionKey::AddReliabilityBundle
        | ExtensionKey::AddTimeoutGuard
        | ExtensionKey::AddCompensationBranch
        | ExtensionKey::AddSignalResolution => &[ExtensionKey::AddEntryTrigger],
    }
}

fn confidence_score_for(key: ExtensionKey, workflow: &Workflow) -> f32 {
    match key {
        ExtensionKey::AddEntryTrigger => {
            if workflow.nodes.iter().all(|node| node.category != NodeCategory::Entry) {
                0.98
            } else {
                0.0
            }
        }
        ExtensionKey::AddReliabilityBundle => {
            #[allow(clippy::cast_precision_loss)]
            let missing = [
                plan_missing_timeout_guard(workflow).is_some(),
                plan_missing_checkpoint(workflow).is_some(),
                plan_unbalanced_condition(workflow).is_some(),
            ]
            .into_iter()
            .filter(|value| *value)
            .count() as f32; // OK: small count < 100, no precision loss
            if missing == 0.0 {
                0.0
            } else {
                (0.78 + missing * 0.06).min(0.97)
            }
        }
        ExtensionKey::AddTimeoutGuard => {
            #[allow(clippy::cast_precision_loss)]
            let durable_count =
                workflow.nodes.iter().filter(|node| node.category == NodeCategory::Durable).count()
                    as f32; // OK: small count, no precision loss
            (0.75 + durable_count * 0.08).min(0.97)
        }
        ExtensionKey::AddDurableCheckpoint => {
            #[allow(clippy::cast_precision_loss)]
            let durable_count =
                workflow.nodes.iter().filter(|node| node.category == NodeCategory::Durable).count()
                    as f32; // OK: small count, no precision loss
            (0.70 + durable_count * 0.07).min(0.95)
        }
        ExtensionKey::AddCompensationBranch => {
            #[allow(clippy::cast_precision_loss)]
            let missing = workflow
                .nodes
                .iter()
                .filter(|node| {
                    matches!(node.node, WorkflowNode::Condition(_))
                        && missing_condition_branch(workflow, node.id)
                })
                .count() as f32; // OK: small count, no precision loss
            (0.72 + missing * 0.09).min(0.96)
        }
        ExtensionKey::AddSignalResolution => {
            #[allow(clippy::cast_precision_loss)]
            let waits =
                workflow.nodes.iter().filter(|node| is_signal_wait_anchor(workflow, node)).count()
                    as f32; // OK: small count, no precision loss
            (0.74 + waits * 0.08).min(0.97)
        }
    }
}

const fn rationale_class_for(key: ExtensionKey) -> RationaleClass {
    match key {
        ExtensionKey::AddEntryTrigger => RationaleClass::StructuralCoverage,
        ExtensionKey::AddReliabilityBundle => RationaleClass::ReliabilityBundle,
        ExtensionKey::AddTimeoutGuard => RationaleClass::RuntimeSafety,
        ExtensionKey::AddDurableCheckpoint => RationaleClass::StateSafety,
        ExtensionKey::AddCompensationBranch => RationaleClass::FailureRecovery,
        ExtensionKey::AddSignalResolution => RationaleClass::AsyncCoordination,
    }
}

const fn reliability_bundle_members() -> &'static [ExtensionKey] {
    &[
        ExtensionKey::AddTimeoutGuard,
        ExtensionKey::AddDurableCheckpoint,
        ExtensionKey::AddCompensationBranch,
    ]
}

fn hide_isolated_reliability_hints(suggestions: Vec<FlowExtension>) -> Vec<FlowExtension> {
    let has_bundle =
        suggestions.iter().any(|item| item.key == ExtensionKey::AddReliabilityBundle.as_str());
    if !has_bundle {
        return suggestions;
    }

    suggestions
        .into_iter()
        .filter(|item| {
            item.key == ExtensionKey::AddReliabilityBundle.as_str()
                || !reliability_bundle_members().iter().any(|member| item.key == member.as_str())
        })
        .collect()
}

fn hide_isolated_reliability_analysis(
    analyses: Vec<ExtensionSuggestionAnalysis>,
) -> Vec<ExtensionSuggestionAnalysis> {
    let has_bundle =
        analyses.iter().any(|item| item.key == ExtensionKey::AddReliabilityBundle.as_str());
    if !has_bundle {
        return analyses;
    }

    analyses
        .into_iter()
        .filter(|item| {
            item.key == ExtensionKey::AddReliabilityBundle.as_str()
                || !reliability_bundle_members().iter().any(|member| item.key == member.as_str())
        })
        .collect()
}

const fn is_side_effecting_durable(node: &Node) -> bool {
    matches!(
        node.node,
        WorkflowNode::Run(_)
            | WorkflowNode::ServiceCall(_)
            | WorkflowNode::ObjectCall(_)
            | WorkflowNode::WorkflowCall(_)
            | WorkflowNode::SendMessage(_)
            | WorkflowNode::DelayedSend(_)
    )
}

fn fallback_analysis(
    key: &str,
    workflow: &Workflow,
    parsed_key: ExtensionKey,
) -> ExtensionSuggestionAnalysis {
    let fingerprint = plan_for_key(workflow, parsed_key)
        .map(|plan| extension_fingerprint(parsed_key, &plan.patch))
        .unwrap_or_default();
    ExtensionSuggestionAnalysis {
        key: key.to_string(),
        score: confidence_score_for(parsed_key, workflow),
        rationale_class: rationale_class_for(parsed_key),
        fingerprint,
        dependencies: extension_dependencies(parsed_key)
            .iter()
            .map(|dependency| dependency.as_str().to_string())
            .collect(),
        semantics: extension_semantics(parsed_key),
    }
}

fn expand_keys_with_dependencies(keys: &[ExtensionKey]) -> Vec<ExtensionKey> {
    let mut expanded = Vec::new();
    let mut seen = HashSet::new();

    for key in keys {
        expand_key_with_dependencies(*key, &mut seen, &mut expanded);
    }

    expanded
}

fn expand_key_with_dependencies(
    key: ExtensionKey,
    seen: &mut HashSet<ExtensionKey>,
    expanded: &mut Vec<ExtensionKey>,
) {
    if !seen.insert(key) {
        return;
    }

    for dependency in extension_dependencies(key) {
        expand_key_with_dependencies(*dependency, seen, expanded);
    }

    expanded.push(key);
}

fn preview_from_patch(key: String, patch: &PatchPlan) -> ExtensionPatchPreview {
    let nodes = patch
        .nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| PreviewNode {
            temp_id: format!("new-{idx}"),
            node_type: node.node_type.to_string(),
            x: node.x,
            y: node.y,
        })
        .collect::<Vec<_>>();

    let connections = patch
        .connections
        .iter()
        .map(|connection| PreviewConnection {
            source: preview_endpoint(connection.source),
            target: preview_endpoint(connection.target),
            source_port: connection.source_port.to_string(),
            target_port: connection.target_port.to_string(),
        })
        .collect::<Vec<_>>();

    ExtensionPatchPreview { key, nodes, connections }
}

fn preview_endpoint(endpoint: PatchEndpoint) -> PreviewEndpoint {
    match endpoint {
        PatchEndpoint::Existing(node_id) => PreviewEndpoint::Existing(node_id),
        PatchEndpoint::Proposed(index) => PreviewEndpoint::Proposed(format!("new-{index}")),
    }
}

fn resolve_patch_endpoint(endpoint: PatchEndpoint, created_nodes: &[NodeId]) -> Option<NodeId> {
    match endpoint {
        PatchEndpoint::Existing(node_id) => Some(node_id),
        PatchEndpoint::Proposed(index) => created_nodes.get(index).copied(),
    }
}

fn missing_condition_branch(workflow: &Workflow, node_id: NodeId) -> bool {
    let has_true = workflow
        .connections
        .iter()
        .any(|connection| connection.source == node_id && connection.source_port.0 == "true");
    let has_false = workflow
        .connections
        .iter()
        .any(|connection| connection.source == node_id && connection.source_port.0 == "false");

    !(has_true && has_false)
}

fn is_signal_wait_anchor(workflow: &Workflow, node: &Node) -> bool {
    if matches!(node.node, WorkflowNode::DurablePromise(_)) {
        return true;
    }

    matches!(node.node, WorkflowNode::Awakeable(_))
        && workflow
            .connections
            .iter()
            .any(|connection| connection.source == node.id && connection.source_port.0 == "out")
}

fn first_node_by_type<F>(workflow: &Workflow, predicate: F) -> Option<Node>
where
    F: Fn(&Node) -> bool,
{
    workflow
        .nodes
        .iter()
        .filter(|node| predicate(node))
        .min_by(|left, right| left.y.total_cmp(&right.y).then_with(|| left.x.total_cmp(&right.x)))
        .cloned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{
        apply_extension, detect_extension_conflicts, extension_dependency_graph, extension_presets,
        generate_compound_plan, preview_extension, resolve_extension_preset, suggest_extensions,
        suggest_extensions_with_analysis, ConflictKind, ExtensionKey, PreviewEndpoint,
        RationaleClass, RestateCapability, RestateServiceKind,
    };
    use crate::graph::{workflow_node::WorkflowNode, Workflow};
    use std::collections::HashSet;

    #[test]
    fn given_empty_workflow_when_suggesting_then_entry_trigger_is_recommended() {
        let workflow = Workflow::new();

        let suggestions = suggest_extensions(&workflow);

        assert!(suggestions.iter().any(|item| item.key == "add-entry-trigger"));
    }

    #[test]
    fn given_empty_workflow_when_applying_entry_trigger_then_http_handler_is_added() {
        let mut workflow = Workflow::new();

        let applied = apply_extension(&mut workflow, "add-entry-trigger");

        assert!(applied.is_ok());
        assert!(workflow
            .nodes
            .iter()
            .any(|node| matches!(node.node, WorkflowNode::HttpHandler(_))));
    }

    #[test]
    fn given_extensions_when_suggesting_then_contracts_are_present() {
        let workflow = Workflow::new();

        let suggestions = suggest_extensions(&workflow);

        assert!(suggestions.iter().all(|suggestion| !suggestion.contract.preconditions.is_empty()));
        assert!(suggestions
            .iter()
            .all(|suggestion| !suggestion.contract.postconditions.is_empty()));
        assert!(suggestions.iter().all(|suggestion| !suggestion.contract.invariants.is_empty()));
    }

    #[test]
    fn given_known_extension_keys_when_parsing_then_all_are_unique_and_valid() {
        let keys = [
            ExtensionKey::AddEntryTrigger,
            ExtensionKey::AddReliabilityBundle,
            ExtensionKey::AddTimeoutGuard,
            ExtensionKey::AddDurableCheckpoint,
            ExtensionKey::AddCompensationBranch,
            ExtensionKey::AddSignalResolution,
        ];

        let unique: HashSet<&'static str> = keys.iter().map(|key| key.as_str()).collect();

        assert_eq!(unique.len(), keys.len());
    }

    #[test]
    fn given_unknown_key_when_applying_then_error_is_returned() {
        let mut workflow = Workflow::new();

        let result = apply_extension(&mut workflow, "not-a-valid-extension");

        assert!(result.is_err());
    }

    #[test]
    fn timeout_guard_preview_when_rule_applies_then_patch_contains_proposed_node_and_edge() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 10.0, 20.0);

        let preview = preview_extension(&workflow, "add-timeout-guard");

        assert!(preview.is_ok());
        let preview = preview.ok().flatten();
        assert!(preview.is_some());
        let preview = match preview {
            Some(value) => value,
            None => return,
        };
        assert_eq!(preview.nodes.len(), 1);
        assert_eq!(preview.connections.len(), 1);
        assert_eq!(preview.nodes[0].temp_id, "new-0");
        assert_eq!(preview.nodes[0].node_type, "timeout");
        assert_eq!(preview.connections[0].target, PreviewEndpoint::Proposed("new-0".to_string()));
    }

    #[test]
    fn given_missing_entry_when_analyzing_then_confidence_and_rationale_class_are_deterministic() {
        let workflow = Workflow::new();

        let analyses = suggest_extensions_with_analysis(&workflow);

        let entry = analyses.iter().find(|item| item.key == "add-entry-trigger");
        assert!(entry.is_some());
        let entry = match entry {
            Some(value) => value,
            None => return,
        };
        assert!(entry.score > 0.95);
        assert_eq!(entry.rationale_class, RationaleClass::StructuralCoverage);
        assert!(!entry.fingerprint.is_empty());
        assert!(entry.semantics.compatible_service_kinds.contains(&RestateServiceKind::Handler));
        assert!(entry.semantics.provides.contains(&RestateCapability::EntryTrigger));
    }

    #[test]
    fn side_effecting_durable_when_suggesting_then_bundle_replaces_isolated_hints() {
        let mut workflow = Workflow::new();
        let condition = workflow.add_node("condition", 120.0, 120.0);
        let run = workflow.add_node("run", 200.0, 120.0);
        workflow.add_node("get-state", 80.0, 120.0);
        let _ = workflow.add_connection_checked(condition, run, &"true".into(), &"in".into());

        let suggestions = suggest_extensions(&workflow);
        let keys = suggestions.iter().map(|item| item.key.as_str()).collect::<Vec<_>>();

        assert!(keys.contains(&"add-reliability-bundle"));
        assert!(!keys.contains(&"add-timeout-guard"));
        assert!(!keys.contains(&"add-durable-checkpoint"));
        assert!(!keys.contains(&"add-compensation-branch"));
    }

    #[test]
    fn workflow_without_side_effecting_durable_when_suggesting_then_bundle_is_absent() {
        let mut workflow = Workflow::new();
        workflow.add_node("condition", 120.0, 120.0);

        let suggestions = suggest_extensions(&workflow);

        assert!(!suggestions.iter().any(|item| item.key == "add-reliability-bundle"));
    }

    #[test]
    fn bundle_when_analyzing_then_confidence_and_rationale_are_reported() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 80.0, 80.0);
        workflow.add_node("get-state", 40.0, 40.0);

        let analyses = suggest_extensions_with_analysis(&workflow);
        let bundle = analyses.iter().find(|item| item.key == "add-reliability-bundle");

        assert!(bundle.is_some());
        let bundle = match bundle {
            Some(value) => value,
            None => return,
        };
        assert!(bundle.score > 0.7);
        assert_eq!(bundle.rationale_class, RationaleClass::ReliabilityBundle);
        assert!(!bundle.fingerprint.is_empty());
    }

    #[test]
    fn stamped_fingerprint_when_applying_then_extension_is_idempotent() {
        let mut workflow = Workflow::new();
        let _ = workflow.add_node("run", 20.0, 30.0);

        let initial = apply_extension(&mut workflow, "add-timeout-guard");
        assert!(initial.is_ok());
        let initial = match initial {
            Ok(value) => value,
            Err(_) => return,
        };
        assert_eq!(initial.created_nodes.len(), 1);

        let second = apply_extension(&mut workflow, "add-timeout-guard");
        assert!(second.is_ok());
        let second = match second {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(second.created_nodes.is_empty());

        let timeout =
            workflow.nodes.iter().find(|node| matches!(node.node, WorkflowNode::Timeout(_)));
        assert!(timeout.is_some());
        let timeout = match timeout {
            Some(value) => value,
            None => return,
        };
        let fingerprint = timeout
            .metadata
            .as_object()
            .and_then(|meta| meta.get("flow_extender"))
            .and_then(serde_json::Value::as_object)
            .and_then(|ext| ext.get("fingerprint"))
            .and_then(serde_json::Value::as_str);
        assert!(fingerprint.is_some());
    }

    #[test]
    fn bundle_when_applying_twice_then_second_apply_is_idempotent() {
        let mut workflow = Workflow::new();
        let condition = workflow.add_node("condition", 40.0, 40.0);
        let run = workflow.add_node("run", 120.0, 40.0);
        workflow.add_node("get-state", 20.0, 20.0);
        let _ = workflow.add_connection_checked(condition, run, &"true".into(), &"in".into());

        let initial = apply_extension(&mut workflow, "add-reliability-bundle");
        assert!(initial.is_ok());
        let initial = match initial {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(!initial.created_nodes.is_empty());

        let second = apply_extension(&mut workflow, "add-reliability-bundle");
        assert!(second.is_ok());
        let second = match second {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(second.created_nodes.is_empty());
    }

    #[test]
    fn duplicate_requested_keys_when_detecting_conflicts_then_duplicate_is_reported() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 10.0, 10.0);
        let keys = vec!["add-timeout-guard".to_string(), "add-timeout-guard".to_string()];

        let conflicts = detect_extension_conflicts(&workflow, &keys);

        assert!(conflicts.is_ok());
        let conflicts = match conflicts {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(conflicts.iter().any(|conflict| conflict.reason.contains("duplicated")));
    }

    #[test]
    fn given_service_context_when_detecting_conflicts_then_semantic_mismatch_is_reported() {
        let mut workflow = Workflow::new();
        let _ = workflow.add_node("run", 10.0, 10.0);
        let keys = vec!["add-signal-resolution".to_string()];

        let conflicts = detect_extension_conflicts(&workflow, &keys);

        assert!(conflicts.is_ok());
        let conflicts = match conflicts {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(conflicts
            .iter()
            .any(|conflict| conflict.kind == ConflictKind::WorkflowSemanticMismatch));
    }

    #[test]
    fn service_context_when_analyzing_then_workflow_only_recommendations_are_not_emitted() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 16.0, 24.0);
        workflow.add_node("condition", 40.0, 24.0);
        workflow.add_node("awakeable", 80.0, 24.0);

        let analyses = suggest_extensions_with_analysis(&workflow);

        assert!(analyses.iter().all(|item| item.key != "add-durable-checkpoint"));
        assert!(analyses.iter().all(|item| item.key != "add-compensation-branch"));
        assert!(analyses.iter().all(|item| item.key != "add-signal-resolution"));
    }

    #[test]
    fn given_dependency_graph_when_ordering_then_prerequisites_come_first() {
        let keys = vec![
            "add-durable-checkpoint".to_string(),
            "add-timeout-guard".to_string(),
            "add-entry-trigger".to_string(),
        ];

        let graph = extension_dependency_graph(&keys);

        assert!(graph.is_ok());
        let graph = match graph {
            Ok(value) => value,
            Err(_) => return,
        };
        assert_eq!(
            graph.ordered_keys,
            vec![
                "add-entry-trigger".to_string(),
                "add-timeout-guard".to_string(),
                "add-durable-checkpoint".to_string(),
            ]
        );
    }

    #[test]
    fn multi_rule_workflow_when_generating_compound_plan_then_steps_are_built_in_dependency_order()
    {
        let mut workflow = Workflow::new();
        let durable = workflow.add_node("run", 100.0, 100.0);
        let condition = workflow.add_node("condition", 180.0, 180.0);
        workflow.add_node("durable-promise", 200.0, 100.0);
        workflow.add_node("get-state", 60.0, 60.0);
        let _ = workflow.add_connection_checked(condition, durable, &"true".into(), &"in".into());

        let suggestions =
            suggest_extensions(&workflow).into_iter().map(|item| item.key).collect::<Vec<_>>();

        let plan = generate_compound_plan(&workflow, &suggestions);

        assert!(plan.is_ok());
        let plan = match plan {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(plan.steps.len() >= 3);
        assert!(plan.ordered_keys.first().is_some_and(|value| value == "add-entry-trigger"));
        assert!(plan.conflicts.is_empty());
    }

    #[test]
    fn given_retry_saga_preset_when_resolving_then_dependencies_expand_in_order() {
        let workflow = Workflow::new();

        let resolved = resolve_extension_preset(&workflow, "retry-saga");

        assert!(resolved.is_ok());
        let resolved = match resolved {
            Ok(value) => value,
            Err(_) => return,
        };
        assert_eq!(resolved.preset.key, "retry-saga");
        assert_eq!(
            resolved.ordered_keys,
            vec![
                "add-entry-trigger".to_string(),
                "add-timeout-guard".to_string(),
                "add-compensation-branch".to_string(),
                "add-durable-checkpoint".to_string(),
            ]
        );
        assert!(resolved.conflicts.is_empty());
    }

    #[test]
    fn webhook_preset_when_applying_then_guard_and_checkpoint_are_added() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 80.0, 80.0);
        workflow.add_node("get-state", 20.0, 20.0);
        let resolved = resolve_extension_preset(&workflow, "webhook");
        assert!(resolved.is_ok());
        let resolved = match resolved {
            Ok(value) => value,
            Err(_) => return,
        };

        for key in &resolved.ordered_keys {
            let result = apply_extension(&mut workflow, key);
            assert!(result.is_ok());
        }

        assert!(workflow
            .nodes
            .iter()
            .any(|node| matches!(node.node, WorkflowNode::HttpHandler(_))));
        assert!(workflow.nodes.iter().any(|node| matches!(node.node, WorkflowNode::Timeout(_))));
        assert!(workflow.nodes.iter().any(|node| matches!(node.node, WorkflowNode::SetState(_))));
    }

    #[test]
    fn given_presets_when_listing_then_expected_keys_exist() {
        let presets = extension_presets();

        assert_eq!(presets.len(), 3);
        assert!(presets.iter().any(|preset| preset.key == "webhook"));
        assert!(presets.iter().any(|preset| preset.key == "approval"));
        assert!(presets.iter().any(|preset| preset.key == "retry-saga"));
    }
}
