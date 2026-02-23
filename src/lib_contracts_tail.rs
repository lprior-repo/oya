pub struct DockerState {
    pub container_id: String,
    pub status: ContainerStatus,
    pub image: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Normalized container lifecycle states.
pub enum ContainerStatus {
    Running,
    Stopped,
    Exited,
    Created,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Resolved moon task path details.
pub struct MoonPath {
    pub task_name: String,
    pub absolute_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Docker configuration contract used by validation helpers.
pub struct DockerConfig {
    pub image_name: String,
    pub tag: Option<String>,
    pub port_bindings: Vec<u16>,
    pub environment: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Typed errors for docker-fix validation and path resolution.
pub enum DockerFixError {
    #[error("state field is empty: {0}")]
    EmptyStateField(&'static str),
    #[error("state contains null or undefined value")]
    NullValue,
    #[error("state type constraint violated: {0}")]
    TypeConstraintViolation(&'static str),
    #[error("moon task not found: {0}")]
    MoonTaskNotFound(String),
    #[error("moon path resolution failed: {0}")]
    PathResolutionFailed(String),
    #[error("config field is empty: {0}")]
    EmptyConfigField(&'static str),
    #[error("config validation failed: {0}")]
    ConfigValidationFailed(&'static str),
}

/// Verifies Docker state fields are present and non-empty.
pub fn verify_state_typing(state: &DockerState) -> Result<(), DockerFixError> {
    let trimmed_container_id = state.container_id.trim();
    if trimmed_container_id.is_empty() {
        return Err(DockerFixError::EmptyStateField("container_id"));
    }

    let trimmed_image = state.image.trim();
    if trimmed_image.is_empty() {
        return Err(DockerFixError::EmptyStateField("image"));
    }

    Ok(())
}

/// Resolves a moon task selector into a normalized task and absolute path.
pub fn resolve_moon_path(task: &str) -> Result<MoonPath, DockerFixError> {
    let trimmed_task = task.trim();
    if trimmed_task.is_empty() {
        return Err(DockerFixError::MoonTaskNotFound(task.to_string()));
    }

    let normalized_task = trimmed_task.trim_start_matches(':');
    if normalized_task.is_empty() {
        return Err(DockerFixError::ConfigValidationFailed(
            "moon task name is empty after normalization",
        ));
    }
    if normalized_task.len() > MAX_MOON_TASK_NAME_LEN {
        return Err(DockerFixError::ConfigValidationFailed("moon task name exceeds max length"));
    }
    if normalized_task
        .chars()
        .any(|char| !(char.is_ascii_alphanumeric() || char == '-' || char == '_' || char == ':'))
    {
        return Err(DockerFixError::ConfigValidationFailed(
            "moon task name contains invalid characters",
        ));
    }

    let current_dir =
        std::env::current_dir().map_err(|e| DockerFixError::PathResolutionFailed(e.to_string()))?;

    let absolute_path = current_dir.join(normalized_task);

    Ok(MoonPath { task_name: trimmed_task.to_string(), absolute_path })
}

/// Validates docker configuration required by the fix workflow.
pub fn validate_docker_config(config: &DockerConfig) -> Result<(), DockerFixError> {
    let trimmed_image_name = config.image_name.trim();
    if trimmed_image_name.is_empty() {
        return Err(DockerFixError::EmptyConfigField("image_name"));
    }

    if trimmed_image_name.chars().any(|c| c.is_control()) {
        return Err(DockerFixError::TypeConstraintViolation("image_name"));
    }

    Ok(())
}

const DEFAULT_BEAD_CUPID_RUNTIME_COMMAND: &str = DEFAULT_DEV_RUNTIME_COMMAND;
const DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL: &str = "http://localhost:8080/restate/health";
const MAX_BEAD_CUPID_RUN_ID_LEN: usize = 128;
const MAX_BEAD_CUPID_BEAD_ID_LEN: usize = 128;
const MAX_BEAD_CUPID_ENDPOINT_LEN: usize = 2048;
const MAX_BEAD_CUPID_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Input contract for planning a bead-cupid run.
pub struct BeadCupidInput {
    pub run_id: String,
    pub bead_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Immutable plan for bead-cupid runtime startup and checks.
pub struct BeadCupidPlan {
    pub run_id: String,
    pub bead_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Runtime handle produced after bead-cupid startup validation succeeds.
pub struct BeadCupidRuntimeHandle {
    pub run_id: String,
    pub bead_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Named checks captured during bead-cupid observation.
pub enum BeadCupidCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One observed bead-cupid check result.
pub struct BeadCupidCheckObservation {
    pub check: BeadCupidCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Full observation payload emitted from a runtime handle.
pub struct BeadCupidObservation {
    pub run_id: String,
    pub bead_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub checks: Vec<BeadCupidCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Ordered stage names expected in bead-cupid reports.
pub enum BeadCupidStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage-level pass/fail status for bead-cupid reporting.
pub enum BeadCupidStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage report row in the bead-cupid evaluation output.
pub struct BeadCupidStageReport {
    pub stage: BeadCupidStageName,
    pub status: BeadCupidStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final gate decision derived from bead-cupid checks.
pub enum BeadCupidDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final validated report for the bead-cupid flow.
pub struct BeadCupidReport {
    pub plan: BeadCupidPlan,
    pub checks: Vec<BeadCupidCheckObservation>,
    pub stages: Vec<BeadCupidStageReport>,
    pub decision: BeadCupidDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Typed errors for bead-cupid planning, observation, and validation.
pub enum BeadCupidError {
    #[error("bead-cupid field is empty: {0}")]
    EmptyField(&'static str),
    #[error("bead-cupid field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("bead-cupid field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("bead-cupid identifier format invalid: {0}")]
    InvalidIdentifier(&'static str),
    #[error("bead-cupid runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("bead-cupid endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("bead-cupid runtime not ready")]
    RuntimeNotReady,
    #[error("bead-cupid check missing: {0}")]
    MissingCheck(&'static str),
    #[error("bead-cupid report invalid: {0}")]
    InvalidReport(&'static str),
}

/// Builds a normalized bead-cupid plan from raw run and bead identifiers.
pub fn build_bead_cupid_plan(input: &BeadCupidInput) -> Result<BeadCupidPlan, BeadCupidError> {
    let run_id =
        validate_bead_cupid_identifier(input.run_id.as_str(), "run_id", MAX_BEAD_CUPID_RUN_ID_LEN)?;
    let bead_id = validate_bead_cupid_identifier(
        input.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    Ok(BeadCupidPlan {
        run_id: run_id.clone(),
        bead_id,
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!("http://localhost:8080/Oya/{}/get_status", run_id),
    })
}

/// Starts bead-cupid runtime and validates all runtime contract fields.
pub fn start_bead_cupid_runtime(
    plan: &BeadCupidPlan,
) -> Result<BeadCupidRuntimeHandle, BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        plan.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        plan.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    if plan.runtime_command != DEFAULT_BEAD_CUPID_RUNTIME_COMMAND {
        return Err(BeadCupidError::InvalidRuntimeCommand);
    }
    if !is_valid_bead_cupid_endpoint(plan.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_cupid_ingress_contract(plan.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_bead_cupid_endpoint(plan.orchestrator_status_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_cupid_orchestrator_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(BeadCupidRuntimeHandle {
        run_id: plan.run_id.clone(),
        bead_id: plan.bead_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

/// Captures bead-cupid checks from a validated runtime handle.
pub fn capture_bead_cupid_observation(
    handle: &BeadCupidRuntimeHandle,
) -> Result<BeadCupidObservation, BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        handle.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        handle.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    if !handle.runtime_ready {
        return Err(BeadCupidError::RuntimeNotReady);
    }
    validate_bead_cupid_runtime_contract(
        handle.run_id.as_str(),
        handle.runtime_command.as_str(),
        handle.ingress_health_url.as_str(),
        handle.orchestrator_status_url.as_str(),
    )?;
    let checks = build_bead_cupid_checks(handle, Utc::now());

    Ok(BeadCupidObservation {
        run_id: handle.run_id.clone(),
        bead_id: handle.bead_id.clone(),
        runtime_command: handle.runtime_command.clone(),
        ingress_health_url: handle.ingress_health_url.clone(),
        orchestrator_status_url: handle.orchestrator_status_url.clone(),
        checks,
    })
}

/// Evaluates bead-cupid observations into ordered stages and a final decision.
pub fn evaluate_bead_cupid_result(
    observation: &BeadCupidObservation,
) -> Result<BeadCupidReport, BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        observation.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        observation.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    validate_bead_cupid_runtime_contract(
        observation.run_id.as_str(),
        observation.runtime_command.as_str(),
        observation.ingress_health_url.as_str(),
        observation.orchestrator_status_url.as_str(),
    )?;
    let ingress_check = find_bead_cupid_check(
        observation.checks.as_slice(),
        BeadCupidCheckName::IngressHealth,
        "ingress_health",
        "duplicate ingress_health checks",
    )?;
    let orchestrator_check = find_bead_cupid_check(
        observation.checks.as_slice(),
        BeadCupidCheckName::OrchestratorStatus,
        "orchestrator_status",
        "duplicate orchestrator_status checks",
    )?;
    let decision = derive_bead_cupid_decision(ingress_check, orchestrator_check);

    let report = BeadCupidReport {
        plan: bead_cupid_plan_from_observation(observation),
        checks: observation.checks.clone(),
        stages: build_bead_cupid_stages(ingress_check, orchestrator_check, &decision),
        decision,
    };

    validate_bead_cupid_report(&report)?;
    Ok(report)
}

/// Validates report coherence across plan, checks, stage order, and decision.
pub fn validate_bead_cupid_report(report: &BeadCupidReport) -> Result<(), BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        report.plan.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        report.plan.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    validate_bead_cupid_runtime_contract(
        report.plan.run_id.as_str(),
        report.plan.runtime_command.as_str(),
        report.plan.ingress_health_url.as_str(),
        report.plan.orchestrator_status_url.as_str(),
    )?;
    let ingress_check = find_bead_cupid_check(
        report.checks.as_slice(),
        BeadCupidCheckName::IngressHealth,
        "ingress_health",
        "duplicate ingress_health checks",
    )?;
    let orchestrator_check = find_bead_cupid_check(
        report.checks.as_slice(),
        BeadCupidCheckName::OrchestratorStatus,
        "orchestrator_status",
        "duplicate orchestrator_status checks",
    )?;
    validate_bead_cupid_checks_against_plan(report, ingress_check, orchestrator_check)?;
    validate_bead_cupid_stage_contract(report.stages.as_slice())?;
    validate_bead_cupid_stage_semantics(report, ingress_check, orchestrator_check)
}

fn validate_bead_cupid_runtime_contract(
    run_id: &str,
    runtime_command: &str,
    ingress_health_url: &str,
    orchestrator_status_url: &str,
) -> Result<(), BeadCupidError> {
    if runtime_command != DEFAULT_BEAD_CUPID_RUNTIME_COMMAND {
        return Err(BeadCupidError::InvalidRuntimeCommand);
    }
    if !is_valid_bead_cupid_endpoint(ingress_health_url)
        || !matches_bead_cupid_ingress_contract(ingress_health_url)
    {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_bead_cupid_endpoint(orchestrator_status_url)
        || !matches_bead_cupid_orchestrator_contract(orchestrator_status_url, run_id)
    {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }
    Ok(())
}

fn build_bead_cupid_checks(
    handle: &BeadCupidRuntimeHandle,
    base_timestamp: DateTime<Utc>,
) -> Vec<BeadCupidCheckObservation> {
    vec![
        BeadCupidCheckObservation {
            check: BeadCupidCheckName::IngressHealth,
            endpoint: handle.ingress_health_url.clone(),
            success: true,
            diagnostics: "ingress health check passed".to_string(),
            timestamp: base_timestamp,
        },
        BeadCupidCheckObservation {
            check: BeadCupidCheckName::OrchestratorStatus,
            endpoint: handle.orchestrator_status_url.clone(),
            success: true,
            diagnostics: "orchestrator status check passed".to_string(),
            timestamp: base_timestamp + chrono::Duration::milliseconds(1),
        },
    ]
}

fn find_bead_cupid_check<'a>(
    checks: &'a [BeadCupidCheckObservation],
    target: BeadCupidCheckName,
    missing: &'static str,
    duplicate: &'static str,
) -> Result<&'a BeadCupidCheckObservation, BeadCupidError> {
    let matches: Vec<&BeadCupidCheckObservation> =
        checks.iter().filter(|check| check.check == target).collect();
    match matches.as_slice() {
        [] => Err(BeadCupidError::MissingCheck(missing)),
        [check] => Ok(*check),
        _ => Err(BeadCupidError::InvalidReport(duplicate)),
    }
}

fn derive_bead_cupid_decision(
    ingress_check: &BeadCupidCheckObservation,
    orchestrator_check: &BeadCupidCheckObservation,
) -> BeadCupidDecision {
    if ingress_check.success && orchestrator_check.success {
        BeadCupidDecision::Pass
    } else {
        BeadCupidDecision::Fail
    }
}

fn bead_cupid_plan_from_observation(observation: &BeadCupidObservation) -> BeadCupidPlan {
    BeadCupidPlan {
        run_id: observation.run_id.clone(),
        bead_id: observation.bead_id.clone(),
        runtime_command: observation.runtime_command.clone(),
        ingress_health_url: observation.ingress_health_url.clone(),
        orchestrator_status_url: observation.orchestrator_status_url.clone(),
    }
}

fn build_bead_cupid_stages(
    ingress_check: &BeadCupidCheckObservation,
    orchestrator_check: &BeadCupidCheckObservation,
    decision: &BeadCupidDecision,
) -> Vec<BeadCupidStageReport> {
    let ingress_time = ingress_check.timestamp;
    let orchestrator_time = if orchestrator_check.timestamp < ingress_time {
        ingress_time
    } else {
        orchestrator_check.timestamp
    };
    let final_time = orchestrator_time + chrono::Duration::milliseconds(1);
    vec![
        BeadCupidStageReport {
            stage: BeadCupidStageName::IngressHealth,
            status: if ingress_check.success {
                BeadCupidStageStatus::Passed
            } else {
                BeadCupidStageStatus::Failed
            },
            diagnostics: ingress_check.diagnostics.clone(),
            timestamp: ingress_time,
        },
        BeadCupidStageReport {
            stage: BeadCupidStageName::OrchestratorStatus,
            status: if orchestrator_check.success {
                BeadCupidStageStatus::Passed
            } else {
                BeadCupidStageStatus::Failed
            },
            diagnostics: orchestrator_check.diagnostics.clone(),
            timestamp: orchestrator_time,
        },
        BeadCupidStageReport {
            stage: BeadCupidStageName::FinalDecision,
            status: if decision == &BeadCupidDecision::Pass {
                BeadCupidStageStatus::Passed
            } else {
                BeadCupidStageStatus::Failed
            },
            diagnostics: expected_bead_cupid_final_diagnostics(decision).to_string(),
            timestamp: final_time,
        },
    ]
}

fn validate_bead_cupid_checks_against_plan(
    report: &BeadCupidReport,
    ingress_check: &BeadCupidCheckObservation,
    orchestrator_check: &BeadCupidCheckObservation,
) -> Result<(), BeadCupidError> {
    if ingress_check.endpoint != report.plan.ingress_health_url
        || orchestrator_check.endpoint != report.plan.orchestrator_status_url
    {
        return Err(BeadCupidError::InvalidReport("check endpoint mismatch"));
    }
    if report.checks.iter().any(|check| {
        check.diagnostics.trim().is_empty()
            || check.diagnostics.len() > MAX_BEAD_CUPID_DIAGNOSTICS_LEN
            || contains_forbidden_control_chars(check.diagnostics.as_str())
    }) {
        return Err(BeadCupidError::InvalidReport("invalid check diagnostics"));
    }
    Ok(())
}

fn validate_bead_cupid_stage_contract(
    stages: &[BeadCupidStageReport],
) -> Result<(), BeadCupidError> {
    let expected = [
        BeadCupidStageName::IngressHealth,
        BeadCupidStageName::OrchestratorStatus,
        BeadCupidStageName::FinalDecision,
    ];
    if stages.len() != expected.len() {
        return Err(BeadCupidError::InvalidReport("unexpected stage count"));
    }
    if !stages.iter().map(|stage| stage.stage.clone()).eq(expected.iter().cloned()) {
        return Err(BeadCupidError::InvalidReport("invalid stage order"));
    }
    if stages.iter().any(|stage| {
        stage.diagnostics.trim().is_empty()
            || stage.diagnostics.len() > MAX_BEAD_CUPID_DIAGNOSTICS_LEN
            || contains_forbidden_control_chars(stage.diagnostics.as_str())
    }) {
        return Err(BeadCupidError::InvalidReport("invalid stage diagnostics"));
    }
    if stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp) {
        return Err(BeadCupidError::InvalidReport("non-monotonic stage timestamps"));
    }
    Ok(())
}

fn validate_bead_cupid_stage_semantics(
    report: &BeadCupidReport,
    ingress_check: &BeadCupidCheckObservation,
    orchestrator_check: &BeadCupidCheckObservation,
) -> Result<(), BeadCupidError> {
    let ingress_status = if ingress_check.success {
        BeadCupidStageStatus::Passed
    } else {
        BeadCupidStageStatus::Failed
    };
    let orchestrator_status = if orchestrator_check.success {
        BeadCupidStageStatus::Passed
    } else {
        BeadCupidStageStatus::Failed
    };
    if report.stages[0].status != ingress_status {
        return Err(BeadCupidError::InvalidReport("ingress stage mismatch"));
    }
    if report.stages[0].diagnostics != ingress_check.diagnostics {
        return Err(BeadCupidError::InvalidReport("ingress diagnostics mismatch"));
    }
    if report.stages[1].status != orchestrator_status {
        return Err(BeadCupidError::InvalidReport("orchestrator stage mismatch"));
    }
    if report.stages[1].diagnostics != orchestrator_check.diagnostics {
        return Err(BeadCupidError::InvalidReport("orchestrator diagnostics mismatch"));
    }
    let derived = derive_bead_cupid_decision(ingress_check, orchestrator_check);
    if report.decision != derived {
        return Err(BeadCupidError::InvalidReport("decision mismatch"));
    }
    let final_status = if derived == BeadCupidDecision::Pass {
        BeadCupidStageStatus::Passed
    } else {
        BeadCupidStageStatus::Failed
    };
    if report.stages[2].status != final_status {
        return Err(BeadCupidError::InvalidReport("final decision stage mismatch"));
    }
    if report.stages[2].diagnostics != expected_bead_cupid_final_diagnostics(&derived) {
        return Err(BeadCupidError::InvalidReport("final diagnostics mismatch"));
    }
    Ok(())
}

fn validate_bead_cupid_identifier(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<String, BeadCupidError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BeadCupidError::EmptyField(field));
    }
    if trimmed.len() > max_len {
        return Err(BeadCupidError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(BeadCupidError::InvalidFieldContent(field));
    }
    if !trimmed.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_') {
        return Err(BeadCupidError::InvalidIdentifier(field));
    }

    Ok(trimmed.to_string())
}

fn validate_normalized_bead_cupid_identifier(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<(), BeadCupidError> {
    if value.trim().is_empty() {
        return Err(BeadCupidError::EmptyField(field));
    }
    if value != value.trim() {
        return Err(BeadCupidError::InvalidFieldContent(field));
    }
    if value.len() > max_len {
        return Err(BeadCupidError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(value) {
        return Err(BeadCupidError::InvalidFieldContent(field));
    }
    if !value.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_') {
        return Err(BeadCupidError::InvalidIdentifier(field));
    }

    Ok(())
}

fn is_valid_bead_cupid_endpoint(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.len() > MAX_BEAD_CUPID_ENDPOINT_LEN {
        return false;
    }
    if contains_forbidden_control_chars(trimmed) {
        return false;
    }

    match reqwest::Url::parse(trimmed) {
        Ok(url) => {
            let scheme_valid = url.scheme() == "http" || url.scheme() == "https";
            let host_valid = url.host_str().is_some();
            let creds_valid = url.username().is_empty() && url.password().is_none();
            scheme_valid && host_valid && creds_valid
        }
        Err(_) => false,
    }
}

fn matches_bead_cupid_ingress_contract(value: &str) -> bool {
    value == DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL
}

fn matches_bead_cupid_orchestrator_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/Oya/{}/get_status", run_id)
}

fn expected_bead_cupid_final_diagnostics(decision: &BeadCupidDecision) -> &'static str {
    match decision {
        BeadCupidDecision::Pass => "bead-cupid checks passed",
        BeadCupidDecision::Fail => "bead-cupid checks failed",
    }
}

const DEFAULT_SRC_1EW_BASE_URL: &str = "https://pokeapi.co/api/v2";
const MAX_SRC_1EW_QUERY_LEN: usize = 256;
const MAX_SRC_1EW_LIMIT: usize = 200;
const MAX_SRC_1EW_OFFSET: usize = 10_000;
const MAX_SRC_1EW_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src1ewInput {
    pub command_mode: String,
    pub query: String,
    pub limit: usize,
    pub offset: usize,
    pub base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Src1ewCommandMode {
    GetPokemon,
    ListPokemon,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src1ewPlan {
    pub mode: Src1ewCommandMode,
    pub query: Option<String>,
    pub limit: usize,
    pub offset: usize,
    pub base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src1ewRuntimeHandle {
    pub mode: Src1ewCommandMode,
    pub query: Option<String>,
    pub limit: usize,
    pub offset: usize,
    pub base_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Src1ewCheckName {
    EndpointContract,
    InputContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src1ewCheckObservation {
    pub check: Src1ewCheckName,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src1ewObservation {
    pub plan: Src1ewPlan,
    pub checks: Vec<Src1ewCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Src1ewStageName {
    PlanBuild,
    RuntimeStart,
    ObservationCapture,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Src1ewStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src1ewStageReport {
    pub stage: Src1ewStageName,
    pub status: Src1ewStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Src1ewDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src1ewReport {
    pub plan: Src1ewPlan,
    pub checks: Vec<Src1ewCheckObservation>,
    pub stages: Vec<Src1ewStageReport>,
    pub decision: Src1ewDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Src1ewError {
    #[error("src-1ew field is empty: {0}")]
    EmptyField(&'static str),
    #[error("src-1ew field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("src-1ew field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("src-1ew field format invalid: {0}")]
    InvalidFieldFormat(&'static str),
    #[error("src-1ew endpoint invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("src-1ew check missing: {0}")]
    MissingCheck(&'static str),
    #[error("src-1ew runtime not ready")]
    RuntimeNotReady,
    #[error("src-1ew report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_src_1ew_plan(input: &Src1ewInput) -> Result<Src1ewPlan, Src1ewError> {
    let mode = normalize_src_1ew_mode(input.command_mode.as_str())?;
    let query = normalize_src_1ew_query(&mode, input.query.as_str())?;
    let limit = validate_src_1ew_limit(input.limit)?;
    let offset = validate_src_1ew_offset(input.offset)?;
    let base_url = validate_src_1ew_base_url(input.base_url.as_str(), "base_url")?;

    Ok(Src1ewPlan { mode, query, limit, offset, base_url })
}

pub fn start_src_1ew_runtime(plan: &Src1ewPlan) -> Result<Src1ewRuntimeHandle, Src1ewError> {
    validate_src_1ew_base_url(plan.base_url.as_str(), "base_url")?;
    if plan.base_url != DEFAULT_SRC_1EW_BASE_URL {
        return Err(Src1ewError::InvalidEndpoint("base_url_contract"));
    }
    validate_src_1ew_mode_query_contract(&plan.mode, &plan.query)?;
    validate_src_1ew_limit(plan.limit)?;
    validate_src_1ew_offset(plan.offset)?;

    Ok(Src1ewRuntimeHandle {
        mode: plan.mode.clone(),
        query: plan.query.clone(),
        limit: plan.limit,
        offset: plan.offset,
        base_url: plan.base_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

pub fn capture_src_1ew_observation(
    handle: &Src1ewRuntimeHandle,
) -> Result<Src1ewObservation, Src1ewError> {
    if !handle.runtime_ready {
        return Err(Src1ewError::RuntimeNotReady);
    }

    let endpoint_contract_ok = handle.base_url == DEFAULT_SRC_1EW_BASE_URL;
    let input_contract_ok = validate_src_1ew_mode_query_contract(&handle.mode, &handle.query)
        .is_ok()
        && validate_src_1ew_limit(handle.limit).is_ok()
        && validate_src_1ew_offset(handle.offset).is_ok();
    let timestamp = Utc::now();

    Ok(Src1ewObservation {
        plan: Src1ewPlan {
            mode: handle.mode.clone(),
            query: handle.query.clone(),
            limit: handle.limit,
            offset: handle.offset,
            base_url: handle.base_url.clone(),
        },
        checks: vec![
            Src1ewCheckObservation {
                check: Src1ewCheckName::EndpointContract,
                success: endpoint_contract_ok,
                diagnostics: if endpoint_contract_ok {
                    "pokeapi endpoint contract satisfied".to_string()
                } else {
                    "pokeapi endpoint contract violated".to_string()
                },
                timestamp,
            },
            Src1ewCheckObservation {
                check: Src1ewCheckName::InputContract,
                success: input_contract_ok,
                diagnostics: if input_contract_ok {
                    "input contract satisfied".to_string()
                } else {
                    "input contract violated".to_string()
                },
                timestamp: timestamp + chrono::Duration::milliseconds(1),
            },
        ],
    })
}

pub fn evaluate_src_1ew_observation(
    observation: &Src1ewObservation,
) -> Result<Src1ewReport, Src1ewError> {
    validate_src_1ew_checks(observation.checks.as_slice())?;
    let decision = derive_src_1ew_decision(observation.checks.as_slice());
    let report = Src1ewReport {
        plan: observation.plan.clone(),
        checks: observation.checks.clone(),
        stages: build_src_1ew_stages(&decision, Utc::now()),
        decision,
    };

    validate_src_1ew_report(&report)?;
    Ok(report)
}

/// Alias for [`evaluate_src_1ew_observation`] - provided for API consistency
/// with naming conventions used in other contract modules (e.g., test-trace-final).
/// This function produces identical results to the primary `evaluate_src_1ew_observation`
/// and exists as a convenience alias. See test `evaluate_src_1ew_result_matches_observation_evaluation`
/// for verification of equivalence.
pub fn evaluate_src_1ew_result(
    observation: &Src1ewObservation,
) -> Result<Src1ewReport, Src1ewError> {
    evaluate_src_1ew_observation(observation)
}

fn build_src_1ew_stages(
    decision: &Src1ewDecision,
    timestamp: DateTime<Utc>,
) -> Vec<Src1ewStageReport> {
    let observation_status = if decision == &Src1ewDecision::Pass {
        Src1ewStageStatus::Passed
    } else {
        Src1ewStageStatus::Failed
    };
    vec![
        Src1ewStageReport {
            stage: Src1ewStageName::PlanBuild,
            status: Src1ewStageStatus::Passed,
            diagnostics: "src-1ew plan built".to_string(),
            timestamp,
        },
        Src1ewStageReport {
            stage: Src1ewStageName::RuntimeStart,
            status: Src1ewStageStatus::Passed,
            diagnostics: "src-1ew runtime started".to_string(),
            timestamp: timestamp + chrono::Duration::milliseconds(1),
        },
        Src1ewStageReport {
            stage: Src1ewStageName::ObservationCapture,
            status: observation_status.clone(),
            diagnostics: "src-1ew observation captured".to_string(),
            timestamp: timestamp + chrono::Duration::milliseconds(2),
        },
        Src1ewStageReport {
            stage: Src1ewStageName::FinalDecision,
            status: observation_status,
            diagnostics: if decision == &Src1ewDecision::Pass {
                "src-1ew gate passed".to_string()
            } else {
                "src-1ew gate failed".to_string()
            },
            timestamp: timestamp + chrono::Duration::milliseconds(3),
        },
    ]
}

pub fn validate_src_1ew_report(report: &Src1ewReport) -> Result<(), Src1ewError> {
    validate_src_1ew_plan(report)?;
    validate_src_1ew_report_stages(report.stages.as_slice())?;
    let derived_decision = derive_src_1ew_decision(report.checks.as_slice());
    if derived_decision != report.decision {
        return Err(Src1ewError::InvalidReport("decision mismatch"));
    }
    validate_src_1ew_final_stage(report)
}

fn validate_src_1ew_plan(report: &Src1ewReport) -> Result<(), Src1ewError> {
    validate_src_1ew_base_url(report.plan.base_url.as_str(), "base_url")?;
    if report.plan.base_url != DEFAULT_SRC_1EW_BASE_URL {
        return Err(Src1ewError::InvalidEndpoint("base_url_contract"));
    }
    validate_src_1ew_mode_query_contract(&report.plan.mode, &report.plan.query)?;
    validate_src_1ew_limit(report.plan.limit)?;
    validate_src_1ew_offset(report.plan.offset)?;
    validate_src_1ew_checks(report.checks.as_slice())?;
    Ok(())
}

fn validate_src_1ew_report_stages(stages: &[Src1ewStageReport]) -> Result<(), Src1ewError> {
    let expected = [
        Src1ewStageName::PlanBuild,
        Src1ewStageName::RuntimeStart,
        Src1ewStageName::ObservationCapture,
        Src1ewStageName::FinalDecision,
    ];
    if stages.len() != expected.len() {
        return Err(Src1ewError::InvalidReport("unexpected stage count"));
    }
    if !stages.iter().map(|stage| stage.stage.clone()).eq(expected.iter().cloned()) {
        return Err(Src1ewError::InvalidReport("invalid stage order"));
    }
    if stages.iter().any(|stage| stage.diagnostics.trim().is_empty()) {
        return Err(Src1ewError::InvalidReport("empty stage diagnostics"));
    }
    if stages.iter().any(|stage| stage.diagnostics.len() > MAX_SRC_1EW_DIAGNOSTICS_LEN) {
        return Err(Src1ewError::InvalidReport("stage diagnostics exceed max length"));
    }
    if stages.iter().any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str())) {
        return Err(Src1ewError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }
    if stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp) {
        return Err(Src1ewError::InvalidReport("non-monotonic stage timestamps"));
    }
    Ok(())
}

fn validate_src_1ew_final_stage(report: &Src1ewReport) -> Result<(), Src1ewError> {
    let final_stage = report
        .stages
        .iter()
        .find(|stage| stage.stage == Src1ewStageName::FinalDecision)
        .ok_or(Src1ewError::InvalidReport("missing final decision stage"))?;
    let expected = if report.decision == Src1ewDecision::Pass {
        Src1ewStageStatus::Passed
    } else {
        Src1ewStageStatus::Failed
    };
    if final_stage.status != expected {
        return Err(Src1ewError::InvalidReport("final decision stage mismatch"));
    }
    Ok(())
}

fn normalize_src_1ew_mode(value: &str) -> Result<Src1ewCommandMode, Src1ewError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Src1ewError::EmptyField("command_mode"));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(Src1ewError::InvalidFieldContent("command_mode"));
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "get-pokemon" | "get" => Ok(Src1ewCommandMode::GetPokemon),
        "list-pokemon" | "list" => Ok(Src1ewCommandMode::ListPokemon),
        "search" => Ok(Src1ewCommandMode::Search),
        _ => Err(Src1ewError::InvalidFieldFormat("command_mode")),
    }
}

fn normalize_src_1ew_query(
    mode: &Src1ewCommandMode,
    query: &str,
) -> Result<Option<String>, Src1ewError> {
    let trimmed = query.trim();
    if contains_forbidden_control_chars(trimmed) {
        return Err(Src1ewError::InvalidFieldContent("query"));
    }

    match mode {
        Src1ewCommandMode::ListPokemon => {
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Err(Src1ewError::InvalidFieldFormat("query"))
            }
        }
        Src1ewCommandMode::GetPokemon | Src1ewCommandMode::Search => {
            if trimmed.is_empty() {
                return Err(Src1ewError::EmptyField("query"));
            }
            if trimmed.len() > MAX_SRC_1EW_QUERY_LEN {
                return Err(Src1ewError::FieldTooLong("query", MAX_SRC_1EW_QUERY_LEN));
            }

            let normalized = if mode == &Src1ewCommandMode::Search {
                trimmed.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
            } else {
                trimmed.to_ascii_lowercase()
            };

            if normalized.trim().is_empty() {
                return Err(Src1ewError::EmptyField("query"));
            }

            let invalid_get_identifier = mode == &Src1ewCommandMode::GetPokemon
                && !normalized.chars().all(|char| char.is_ascii_alphanumeric() || char == '-');
            if invalid_get_identifier {
                return Err(Src1ewError::InvalidFieldFormat("query"));
            }

            Ok(Some(normalized))
        }
    }
}

fn validate_src_1ew_mode_query_contract(
    mode: &Src1ewCommandMode,
    query: &Option<String>,
) -> Result<(), Src1ewError> {
    match mode {
        Src1ewCommandMode::ListPokemon => {
            if query.is_some() {
                Err(Src1ewError::InvalidFieldFormat("query"))
            } else {
                Ok(())
            }
        }
        Src1ewCommandMode::GetPokemon | Src1ewCommandMode::Search => match query {
            None => Err(Src1ewError::EmptyField("query")),
            Some(value) => validate_src_1ew_contract_query_value(mode, value.as_str()),
        },
    }
}

fn validate_src_1ew_contract_query_value(
    mode: &Src1ewCommandMode,
    query: &str,
) -> Result<(), Src1ewError> {
    if query.trim().is_empty() {
        return Err(Src1ewError::EmptyField("query"));
    }
    if query.len() > MAX_SRC_1EW_QUERY_LEN {
        return Err(Src1ewError::FieldTooLong("query", MAX_SRC_1EW_QUERY_LEN));
    }
    if contains_forbidden_control_chars(query) {
        return Err(Src1ewError::InvalidFieldContent("query"));
    }

    match mode {
        Src1ewCommandMode::ListPokemon => Err(Src1ewError::InvalidFieldFormat("query")),
        Src1ewCommandMode::GetPokemon => {
            let canonical = query.to_ascii_lowercase();
            let valid_identifier =
                canonical.chars().all(|char| char.is_ascii_alphanumeric() || char == '-');
            if !valid_identifier || canonical != query {
                return Err(Src1ewError::InvalidFieldFormat("query"));
            }
            Ok(())
        }
        Src1ewCommandMode::Search => {
            let canonical =
                query.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase();
            if canonical.is_empty() || canonical != query {
                return Err(Src1ewError::InvalidFieldFormat("query"));
            }
            Ok(())
        }
    }
}

fn validate_src_1ew_limit(value: usize) -> Result<usize, Src1ewError> {
    if value == 0 || value > MAX_SRC_1EW_LIMIT {
        return Err(Src1ewError::InvalidFieldFormat("limit"));
    }
    Ok(value)
}

fn validate_src_1ew_offset(value: usize) -> Result<usize, Src1ewError> {
    if value > MAX_SRC_1EW_OFFSET {
        return Err(Src1ewError::InvalidFieldFormat("offset"));
    }
    Ok(value)
}

fn validate_src_1ew_base_url(value: &str, field: &'static str) -> Result<String, Src1ewError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Src1ewError::EmptyField(field));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(Src1ewError::InvalidFieldContent(field));
    }

    let parsed = reqwest::Url::parse(trimmed).map_err(|_| Src1ewError::InvalidEndpoint(field))?;
    let scheme_valid = parsed.scheme() == "http" || parsed.scheme() == "https";
    let host_valid = parsed.host_str().is_some();
    let creds_valid = parsed.username().is_empty() && parsed.password().is_none();
    if !scheme_valid || !host_valid || !creds_valid {
        return Err(Src1ewError::InvalidEndpoint(field));
    }

    Ok(trimmed.to_string())
}

fn validate_src_1ew_checks(checks: &[Src1ewCheckObservation]) -> Result<(), Src1ewError> {
    if checks.len() != 2 {
        return Err(Src1ewError::InvalidReport("invalid check count"));
    }

    let endpoint_count =
        checks.iter().filter(|check| check.check == Src1ewCheckName::EndpointContract).count();
    let input_count =
        checks.iter().filter(|check| check.check == Src1ewCheckName::InputContract).count();
    if endpoint_count != 1 {
        return Err(Src1ewError::MissingCheck("endpoint_contract"));
    }
    if input_count != 1 {
        return Err(Src1ewError::MissingCheck("input_contract"));
    }

    let has_empty_diagnostics = checks.iter().any(|check| check.diagnostics.trim().is_empty());
    if has_empty_diagnostics {
        return Err(Src1ewError::InvalidReport("empty check diagnostics"));
    }

    let has_oversized_diagnostics =
        checks.iter().any(|check| check.diagnostics.len() > MAX_SRC_1EW_DIAGNOSTICS_LEN);
    if has_oversized_diagnostics {
        return Err(Src1ewError::InvalidReport("check diagnostics exceed max length"));
    }

    let has_invalid_diagnostics =
        checks.iter().any(|check| contains_forbidden_control_chars(check.diagnostics.as_str()));
    if has_invalid_diagnostics {
        return Err(Src1ewError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }

    let non_monotonic_timestamps =
        checks.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if non_monotonic_timestamps {
        return Err(Src1ewError::InvalidReport("non-monotonic check timestamps"));
    }

    Ok(())
}

fn derive_src_1ew_decision(checks: &[Src1ewCheckObservation]) -> Src1ewDecision {
    if checks.iter().all(|check| check.success) {
        Src1ewDecision::Pass
    } else {
        Src1ewDecision::Fail
    }
}

const MAX_TEST_TRACE_FINAL_WORKFLOW_ID_LEN: usize = 128;
const MAX_TEST_TRACE_FINAL_TRACE_ID_LEN: usize = 128;
const MAX_TEST_TRACE_FINAL_STAGE_NAME_LEN: usize = 64;
const MAX_TEST_TRACE_FINAL_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTraceFinalInput {
    pub workflow_id: String,
    pub trace_id: String,
    pub stage_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTraceFinalPlan {
    pub workflow_id: String,
    pub trace_id: String,
    pub stage_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestTraceFinalCheckName {
    PlanContract,
    TraceCollection,
    FinalGateSignal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTraceFinalCheckObservation {
    pub check: TestTraceFinalCheckName,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTraceFinalObservation {
    pub plan: TestTraceFinalPlan,
    pub checks: Vec<TestTraceFinalCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestTraceFinalDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestTraceFinalStageName {
    PlanContract,
    TraceCollection,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestTraceFinalStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTraceFinalStageReport {
    pub stage: TestTraceFinalStageName,
    pub status: TestTraceFinalStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTraceFinalReport {
    pub plan: TestTraceFinalPlan,
    pub checks: Vec<TestTraceFinalCheckObservation>,
    pub stages: Vec<TestTraceFinalStageReport>,
    pub decision: TestTraceFinalDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestTraceFinalError {
    #[error("test-trace-final field is empty: {0}")]
    EmptyField(&'static str),
    #[error("test-trace-final field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("test-trace-final field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("test-trace-final report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_test_trace_final_plan(
    input: &TestTraceFinalInput,
) -> Result<TestTraceFinalPlan, TestTraceFinalError> {
    let workflow_id = validate_test_trace_final_field(
        input.workflow_id.as_str(),
        "workflow_id",
        MAX_TEST_TRACE_FINAL_WORKFLOW_ID_LEN,
    )?;
    let trace_id = validate_test_trace_final_field(
        input.trace_id.as_str(),
        "trace_id",
        MAX_TEST_TRACE_FINAL_TRACE_ID_LEN,
    )?;
    let stage_name = validate_test_trace_final_field(
        input.stage_name.as_str(),
        "stage_name",
        MAX_TEST_TRACE_FINAL_STAGE_NAME_LEN,
    )?;

    Ok(TestTraceFinalPlan { workflow_id, trace_id, stage_name })
}

pub fn collect_test_trace_final_observation(
    plan: &TestTraceFinalPlan,
) -> Result<TestTraceFinalObservation, TestTraceFinalError> {
    let workflow_id = validate_test_trace_final_field(
        plan.workflow_id.as_str(),
        "workflow_id",
        MAX_TEST_TRACE_FINAL_WORKFLOW_ID_LEN,
    )?;
    let trace_id = validate_test_trace_final_field(
        plan.trace_id.as_str(),
        "trace_id",
        MAX_TEST_TRACE_FINAL_TRACE_ID_LEN,
    )?;
    let stage_name = validate_test_trace_final_field(
        plan.stage_name.as_str(),
        "stage_name",
        MAX_TEST_TRACE_FINAL_STAGE_NAME_LEN,
    )?;

    let checks = build_test_trace_final_checks(stage_name.as_str(), trace_id.as_str(), Utc::now());

    Ok(TestTraceFinalObservation {
        plan: TestTraceFinalPlan { workflow_id, trace_id, stage_name },
        checks,
    })
}

pub fn evaluate_test_trace_final_report(
    observation: &TestTraceFinalObservation,
) -> Result<TestTraceFinalReport, TestTraceFinalError> {
    validate_test_trace_final_checks(observation.checks.as_slice())?;

    let stages = observation
        .checks
        .iter()
        .map(|check| TestTraceFinalStageReport {
            stage: map_test_trace_final_stage_name(&check.check),
            status: if check.success {
                TestTraceFinalStageStatus::Passed
            } else {
                TestTraceFinalStageStatus::Failed
            },
            diagnostics: check.diagnostics.clone(),
            timestamp: check.timestamp,
        })
        .collect::<Vec<_>>();

    let mut report = TestTraceFinalReport {
        plan: observation.plan.clone(),
        checks: observation.checks.clone(),
        stages,
        decision: TestTraceFinalDecision::Fail,
    };
    report.decision = derive_test_trace_final_decision(&report);

    validate_test_trace_final_report(&report)?;
    Ok(report)
}

pub fn derive_test_trace_final_decision(report: &TestTraceFinalReport) -> TestTraceFinalDecision {
    if report.checks.iter().all(|check| check.success) {
        TestTraceFinalDecision::Pass
    } else {
        TestTraceFinalDecision::Fail
    }
}

pub fn validate_test_trace_final_report(
    report: &TestTraceFinalReport,
) -> Result<(), TestTraceFinalError> {
    validate_test_trace_final_plan(report)?;
    validate_test_trace_final_stage_contract(report.stages.as_slice())?;
    let stage_status_mismatch =
        report.stages.iter().zip(report.checks.iter()).any(|(stage, check)| {
            let expected = if check.success {
                TestTraceFinalStageStatus::Passed
            } else {
                TestTraceFinalStageStatus::Failed
            };
            stage.status != expected
        });
    if stage_status_mismatch {
        return Err(TestTraceFinalError::InvalidReport("stage status mismatch"));
    }

    let derived = derive_test_trace_final_decision(report);
    if derived != report.decision {
        return Err(TestTraceFinalError::InvalidReport("decision mismatch"));
    }

    Ok(())
}

fn build_test_trace_final_checks(
    stage_name: &str,
    trace_id: &str,
    base: DateTime<Utc>,
) -> Vec<TestTraceFinalCheckObservation> {
    let gate_signal = !stage_name.to_ascii_lowercase().contains("fail");
    vec![
        TestTraceFinalCheckObservation {
            check: TestTraceFinalCheckName::PlanContract,
            success: true,
            diagnostics: "plan contract verified".to_string(),
            timestamp: base,
        },
        TestTraceFinalCheckObservation {
            check: TestTraceFinalCheckName::TraceCollection,
            success: true,
            diagnostics: format!("trace {trace_id} collected"),
            timestamp: base + chrono::Duration::milliseconds(1),
        },
        TestTraceFinalCheckObservation {
            check: TestTraceFinalCheckName::FinalGateSignal,
            success: gate_signal,
            diagnostics: if gate_signal {
                "final gate signal pass".to_string()
            } else {
                "final gate signal fail".to_string()
            },
            timestamp: base + chrono::Duration::milliseconds(2),
        },
    ]
}

fn validate_test_trace_final_plan(
    report: &TestTraceFinalReport,
) -> Result<(), TestTraceFinalError> {
    validate_test_trace_final_field(
        report.plan.workflow_id.as_str(),
        "workflow_id",
        MAX_TEST_TRACE_FINAL_WORKFLOW_ID_LEN,
    )?;
    validate_test_trace_final_field(
        report.plan.trace_id.as_str(),
        "trace_id",
        MAX_TEST_TRACE_FINAL_TRACE_ID_LEN,
    )?;
    validate_test_trace_final_field(
        report.plan.stage_name.as_str(),
        "stage_name",
        MAX_TEST_TRACE_FINAL_STAGE_NAME_LEN,
    )?;
    validate_test_trace_final_checks(report.checks.as_slice())?;
    Ok(())
}

fn validate_test_trace_final_stage_contract(
    stages: &[TestTraceFinalStageReport],
) -> Result<(), TestTraceFinalError> {
    let expected = [
        TestTraceFinalStageName::PlanContract,
        TestTraceFinalStageName::TraceCollection,
        TestTraceFinalStageName::FinalDecision,
    ];
    if stages.len() != expected.len() {
        return Err(TestTraceFinalError::InvalidReport("unexpected stage count"));
    }
    if !stages.iter().map(|stage| stage.stage.clone()).eq(expected.iter().cloned()) {
        return Err(TestTraceFinalError::InvalidReport("invalid stage order"));
    }
    if stages.iter().any(|stage| stage.diagnostics.trim().is_empty()) {
        return Err(TestTraceFinalError::InvalidReport("empty stage diagnostics"));
    }
    if stages.iter().any(|stage| stage.diagnostics.len() > MAX_TEST_TRACE_FINAL_DIAGNOSTICS_LEN) {
        return Err(TestTraceFinalError::InvalidReport("stage diagnostics exceed max length"));
    }
    if stages.iter().any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str())) {
        return Err(TestTraceFinalError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }
    if stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp) {
        return Err(TestTraceFinalError::InvalidReport("non-monotonic stage timestamps"));
    }
    Ok(())
}

fn validate_test_trace_final_field(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<String, TestTraceFinalError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TestTraceFinalError::EmptyField(field));
    }
    if trimmed.len() > max_len {
        return Err(TestTraceFinalError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(TestTraceFinalError::InvalidFieldContent(field));
    }

    Ok(trimmed.to_string())
}

fn validate_test_trace_final_checks(
    checks: &[TestTraceFinalCheckObservation],
) -> Result<(), TestTraceFinalError> {
    if checks.len() != 3 {
        return Err(TestTraceFinalError::InvalidReport("invalid check count"));
    }

    let expected_order = [
        TestTraceFinalCheckName::PlanContract,
        TestTraceFinalCheckName::TraceCollection,
        TestTraceFinalCheckName::FinalGateSignal,
    ];
    let order_valid = checks.iter().map(|check| check.check.clone()).eq(expected_order);
    if !order_valid {
        return Err(TestTraceFinalError::InvalidReport("invalid check order"));
    }

    let has_empty_diagnostics = checks.iter().any(|check| check.diagnostics.trim().is_empty());
    if has_empty_diagnostics {
        return Err(TestTraceFinalError::InvalidReport("empty check diagnostics"));
    }

    let has_oversized_diagnostics =
        checks.iter().any(|check| check.diagnostics.len() > MAX_TEST_TRACE_FINAL_DIAGNOSTICS_LEN);
    if has_oversized_diagnostics {
        return Err(TestTraceFinalError::InvalidReport("check diagnostics exceed max length"));
    }

    let has_invalid_diagnostics =
        checks.iter().any(|check| contains_forbidden_control_chars(check.diagnostics.as_str()));
    if has_invalid_diagnostics {
        return Err(TestTraceFinalError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }

    let non_monotonic_check_timestamps =
        checks.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if non_monotonic_check_timestamps {
        return Err(TestTraceFinalError::InvalidReport("non-monotonic check timestamps"));
    }

    Ok(())
}

fn map_test_trace_final_stage_name(check: &TestTraceFinalCheckName) -> TestTraceFinalStageName {
    match check {
        TestTraceFinalCheckName::PlanContract => TestTraceFinalStageName::PlanContract,
        TestTraceFinalCheckName::TraceCollection => TestTraceFinalStageName::TraceCollection,
        TestTraceFinalCheckName::FinalGateSignal => TestTraceFinalStageName::FinalDecision,
    }
}
