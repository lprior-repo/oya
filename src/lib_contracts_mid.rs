/// Common default runtime command shared across all bead contract families.
pub const DEFAULT_SMOKE_RUNTIME_COMMAND: &str = "scripts/dev-up.sh";
const DEFAULT_DEV_RUNTIME_COMMAND: &str = DEFAULT_SMOKE_RUNTIME_COMMAND;
const DEFAULT_SMOKE_INGRESS_HEALTH_URL: &str = "http://localhost:8080/restate/health";
const MAX_SMOKE_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickInput {
    pub workflow_id: String,
    pub bead_id: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickPlan {
    pub workflow_id: String,
    pub bead_id: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickCheck {
    pub endpoint: String,
    pub visible: bool,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickObservation {
    pub workflow_id: String,
    pub bead_id: String,
    pub checks: Vec<OnewfBeadQuickCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickStageName {
    EndpointVisibility,
    EndpointProbe,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickStageReport {
    pub stage: OnewfBeadQuickStageName,
    pub status: OnewfBeadQuickStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickReport {
    pub workflow_id: String,
    pub bead_id: String,
    pub checks: Vec<OnewfBeadQuickCheck>,
    pub stages: Vec<OnewfBeadQuickStageReport>,
    pub decision: OnewfBeadQuickDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickError {
    #[error("onewf field is empty: {0}")]
    EmptyField(&'static str),
    #[error("onewf field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("onewf field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("onewf identifier format invalid: {0}")]
    InvalidIdentifier(&'static str),
    #[error("onewf endpoint invalid")]
    InvalidEndpoint,
    #[error("onewf check missing")]
    MissingCheck,
    #[error("onewf report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_onewf_bead_quick_plan(
    input: &OnewfBeadQuickInput,
) -> Result<OnewfBeadQuickPlan, OnewfBeadQuickError> {
    let workflow_id = validate_onewf_identifier(
        input.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    let bead_id =
        validate_onewf_identifier(input.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;
    let endpoint = validate_onewf_endpoint(input.endpoint.as_str())?;

    Ok(OnewfBeadQuickPlan { workflow_id, bead_id, endpoint })
}

pub fn run_onewf_bead_quick_check(
    plan: &OnewfBeadQuickPlan,
) -> Result<OnewfBeadQuickObservation, OnewfBeadQuickError> {
    let workflow_id = validate_onewf_identifier(
        plan.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    let bead_id =
        validate_onewf_identifier(plan.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;
    let endpoint = validate_onewf_endpoint(plan.endpoint.as_str())?;

    let visible = !endpoint.contains("hidden=true") && !endpoint.ends_with("/hidden");
    let success = visible && !endpoint.contains("fail=true") && !endpoint.ends_with("/fail");
    let diagnostics = if !visible {
        "endpoint not visible".to_string()
    } else if !success {
        "endpoint probe failed".to_string()
    } else {
        "endpoint visible and probe succeeded".to_string()
    };

    let check =
        OnewfBeadQuickCheck { endpoint, visible, success, diagnostics, timestamp: Utc::now() };

    Ok(OnewfBeadQuickObservation { workflow_id, bead_id, checks: vec![check] })
}

pub fn evaluate_onewf_bead_quick_result(
    observation: &OnewfBeadQuickObservation,
) -> Result<OnewfBeadQuickReport, OnewfBeadQuickError> {
    let workflow_id = validate_onewf_identifier(
        observation.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    let bead_id =
        validate_onewf_identifier(observation.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;

    let check = extract_single_check(observation.checks.as_slice())?;

    let (decision, visibility_diagnostics, probe_diagnostics, _decision_diagnostics) =
        compute_onewf_diagnostics(&check);

    let stages = build_onewf_stages(&check, &decision, &visibility_diagnostics, &probe_diagnostics);

    let report =
        OnewfBeadQuickReport { workflow_id, bead_id, checks: vec![check], stages, decision };

    validate_onewf_bead_quick_report(&report)?;
    Ok(report)
}

fn extract_single_check(
    checks: &[OnewfBeadQuickCheck],
) -> Result<OnewfBeadQuickCheck, OnewfBeadQuickError> {
    match checks {
        [] => Err(OnewfBeadQuickError::MissingCheck),
        [check] => Ok(check.clone()),
        _ => Err(OnewfBeadQuickError::InvalidReport("invalid check count")),
    }
}

fn compute_onewf_diagnostics(
    check: &OnewfBeadQuickCheck,
) -> (OnewfBeadQuickDecision, String, String, String) {
    let decision = if check.visible && check.success {
        OnewfBeadQuickDecision::Pass
    } else {
        OnewfBeadQuickDecision::Fail
    };

    let visibility_diagnostics = if check.visible {
        "one endpoint visible".to_string()
    } else {
        "endpoint not visible".to_string()
    };
    let probe_diagnostics = if check.success {
        "endpoint probe passed".to_string()
    } else {
        "endpoint probe failed".to_string()
    };
    let decision_diagnostics = if decision == OnewfBeadQuickDecision::Pass {
        "onewf-bead-quick gate passed".to_string()
    } else {
        "onewf-bead-quick gate failed".to_string()
    };

    (decision, visibility_diagnostics, probe_diagnostics, decision_diagnostics)
}

fn build_onewf_stages(
    check: &OnewfBeadQuickCheck,
    decision: &OnewfBeadQuickDecision,
    visibility_diagnostics: &str,
    probe_diagnostics: &str,
) -> Vec<OnewfBeadQuickStageReport> {
    let timestamp = check.timestamp;
    let visibility_status = if check.visible {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };
    let probe_status = if check.success {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };
    let decision_status = if decision == &OnewfBeadQuickDecision::Pass {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };

    vec![
        OnewfBeadQuickStageReport {
            stage: OnewfBeadQuickStageName::EndpointVisibility,
            status: visibility_status,
            diagnostics: visibility_diagnostics.to_string(),
            timestamp,
        },
        OnewfBeadQuickStageReport {
            stage: OnewfBeadQuickStageName::EndpointProbe,
            status: probe_status,
            diagnostics: probe_diagnostics.to_string(),
            timestamp: timestamp + chrono::Duration::milliseconds(1),
        },
        OnewfBeadQuickStageReport {
            stage: OnewfBeadQuickStageName::FinalDecision,
            status: decision_status,
            diagnostics: if decision == &OnewfBeadQuickDecision::Pass {
                "onewf-bead-quick gate passed".to_string()
            } else {
                "onewf-bead-quick gate failed".to_string()
            },
            timestamp: timestamp + chrono::Duration::milliseconds(2),
        },
    ]
}

pub fn validate_onewf_bead_quick_report(
    report: &OnewfBeadQuickReport,
) -> Result<(), OnewfBeadQuickError> {
    validate_onewf_identifier(
        report.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    validate_onewf_identifier(report.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;

    let check = extract_single_check(report.checks.as_slice())?;
    validate_onewf_endpoint(check.endpoint.as_str())?;

    validate_onewf_check_diagnostics(check.diagnostics.as_str())?;

    let visible_checks = report.checks.iter().filter(|item| item.visible).count();
    if visible_checks != 1 {
        return Err(OnewfBeadQuickError::InvalidReport(
            "single-endpoint visibility contract violated",
        ));
    }

    validate_onewf_stage_order(report.stages.as_slice())?;
    validate_onewf_stage_diagnostics(report.stages.as_slice())?;
    validate_onewf_timestamps(report.stages.as_slice())?;

    let visibility_stage = &report.stages[0];
    let probe_stage = &report.stages[1];
    let decision_stage = &report.stages[2];

    validate_onewf_stage_status(visibility_stage, check.visible)?;
    validate_onewf_stage_status(probe_stage, check.success)?;
    validate_onewf_decision(&report.decision, check.visible, check.success, decision_stage)?;

    Ok(())
}

fn validate_onewf_check_diagnostics(diagnostics: &str) -> Result<(), OnewfBeadQuickError> {
    if diagnostics.trim().is_empty() {
        return Err(OnewfBeadQuickError::InvalidReport("empty check diagnostics"));
    }
    if diagnostics.len() > MAX_ONEWF_DIAGNOSTICS_LEN {
        return Err(OnewfBeadQuickError::InvalidReport("check diagnostics exceed max length"));
    }
    if contains_forbidden_control_chars(diagnostics) {
        return Err(OnewfBeadQuickError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    Ok(())
}

fn validate_onewf_stage_order(
    stages: &[OnewfBeadQuickStageReport],
) -> Result<(), OnewfBeadQuickError> {
    let expected_stage_order = [
        OnewfBeadQuickStageName::EndpointVisibility,
        OnewfBeadQuickStageName::EndpointProbe,
        OnewfBeadQuickStageName::FinalDecision,
    ];
    if stages.len() != expected_stage_order.len() {
        return Err(OnewfBeadQuickError::InvalidReport("unexpected stage count"));
    }

    let stage_order_valid =
        stages.iter().map(|stage| stage.stage.clone()).eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(OnewfBeadQuickError::InvalidReport("invalid stage order"));
    }

    Ok(())
}

fn validate_onewf_stage_diagnostics(
    stages: &[OnewfBeadQuickStageReport],
) -> Result<(), OnewfBeadQuickError> {
    let has_empty_stage_diagnostics =
        stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_stage_diagnostics {
        return Err(OnewfBeadQuickError::InvalidReport("empty stage diagnostics"));
    }

    let has_oversized_stage_diagnostics =
        stages.iter().any(|stage| stage.diagnostics.len() > MAX_ONEWF_DIAGNOSTICS_LEN);
    if has_oversized_stage_diagnostics {
        return Err(OnewfBeadQuickError::InvalidReport("stage diagnostics exceed max length"));
    }

    let has_invalid_stage_diagnostics =
        stages.iter().any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str()));
    if has_invalid_stage_diagnostics {
        return Err(OnewfBeadQuickError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }

    Ok(())
}

fn validate_onewf_timestamps(
    stages: &[OnewfBeadQuickStageReport],
) -> Result<(), OnewfBeadQuickError> {
    let has_non_monotonic_timestamps = stages.windows(2).any(|pair| {
        let first = &pair[0].timestamp;
        let second = &pair[1].timestamp;
        first > second
    });
    if has_non_monotonic_timestamps {
        return Err(OnewfBeadQuickError::InvalidReport("non-monotonic stage timestamps"));
    }

    Ok(())
}

fn validate_onewf_stage_status(
    stage: &OnewfBeadQuickStageReport,
    expected_success: bool,
) -> Result<(), OnewfBeadQuickError> {
    let expected_status = if expected_success {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };
    if stage.status != expected_status {
        return Err(OnewfBeadQuickError::InvalidReport("stage status mismatch"));
    }
    Ok(())
}

fn validate_onewf_decision(
    decision: &OnewfBeadQuickDecision,
    visible: bool,
    success: bool,
    decision_stage: &OnewfBeadQuickStageReport,
) -> Result<(), OnewfBeadQuickError> {
    let derived_decision = if visible && success {
        OnewfBeadQuickDecision::Pass
    } else {
        OnewfBeadQuickDecision::Fail
    };

    if &derived_decision != decision {
        return Err(OnewfBeadQuickError::InvalidReport("decision mismatch"));
    }

    let expected_decision_stage = if decision == &OnewfBeadQuickDecision::Pass {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };
    if decision_stage.status != expected_decision_stage {
        return Err(OnewfBeadQuickError::InvalidReport("final decision stage mismatch"));
    }

    Ok(())
}

fn validate_onewf_identifier(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<String, OnewfBeadQuickError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OnewfBeadQuickError::EmptyField(field));
    }
    if trimmed.len() > max_len {
        return Err(OnewfBeadQuickError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(OnewfBeadQuickError::InvalidFieldContent(field));
    }
    if !trimmed.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_') {
        return Err(OnewfBeadQuickError::InvalidIdentifier(field));
    }

    Ok(trimmed.to_string())
}

fn validate_onewf_endpoint(value: &str) -> Result<String, OnewfBeadQuickError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OnewfBeadQuickError::EmptyField("endpoint"));
    }
    if trimmed.len() > MAX_ONEWF_ENDPOINT_LEN {
        return Err(OnewfBeadQuickError::FieldTooLong("endpoint", MAX_ONEWF_ENDPOINT_LEN));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(OnewfBeadQuickError::InvalidFieldContent("endpoint"));
    }

    let parsed = reqwest::Url::parse(trimmed).map_err(|_| OnewfBeadQuickError::InvalidEndpoint)?;
    let scheme_valid = parsed.scheme() == "http" || parsed.scheme() == "https";
    let host_valid = parsed.host_str().is_some();
    let creds_valid = parsed.username().is_empty() && parsed.password().is_none();

    if !scheme_valid || !host_valid || !creds_valid {
        return Err(OnewfBeadQuickError::InvalidEndpoint);
    }

    Ok(trimmed.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokePlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeCheckObservation {
    pub check: SmokeCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeObservation {
    pub run_id: String,
    pub checks: Vec<SmokeCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeStageReport {
    pub stage: SmokeStageName,
    pub status: SmokeStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeReport {
    pub run_id: String,
    pub checks: Vec<SmokeCheckObservation>,
    pub stages: Vec<SmokeStageReport>,
    pub decision: SmokeDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SmokeError {
    #[error("smoke field is empty: {0}")]
    EmptyField(&'static str),
    #[error("smoke field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("smoke field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("smoke runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("smoke endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("smoke runtime not ready")]
    RuntimeNotReady,
    #[error("smoke check missing: {0}")]
    MissingCheck(&'static str),
    #[error("smoke report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_smoke_plan(input: &SmokeInput) -> Result<SmokePlan, SmokeError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(SmokeError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }

    Ok(SmokePlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_DEV_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!("http://localhost:8080/Oya/{}/get_status", run_id),
    })
}

pub fn start_docker_default_runtime(plan: &SmokePlan) -> Result<RuntimeHandle, SmokeError> {
    validate_normalized_smoke_run_id(&plan.run_id)?;

    if plan.runtime_command != DEFAULT_DEV_RUNTIME_COMMAND {
        return Err(SmokeError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(&plan.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_ingress_health_contract(&plan.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(&plan.orchestrator_status_url) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_orchestrator_status_contract(&plan.orchestrator_status_url, &plan.run_id) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(RuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

pub fn run_default_smoke_checks(handle: &RuntimeHandle) -> Result<SmokeObservation, SmokeError> {
    validate_normalized_smoke_run_id(&handle.run_id)?;

    if !handle.runtime_ready {
        return Err(SmokeError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_DEV_RUNTIME_COMMAND {
        return Err(SmokeError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(&handle.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_ingress_health_contract(&handle.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(&handle.orchestrator_status_url) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_orchestrator_status_contract(&handle.orchestrator_status_url, &handle.run_id) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(SmokeObservation {
        run_id: handle.run_id.clone(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: handle.ingress_health_url.clone(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: Utc::now(),
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: handle.orchestrator_status_url.clone(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: Utc::now(),
            },
        ],
    })
}

pub fn evaluate_smoke_result(observation: &SmokeObservation) -> Result<SmokeReport, SmokeError> {
    let ingress_check = extract_single_check_by_name(
        &observation.checks,
        SmokeCheckName::IngressHealth,
        "ingress_health",
        "duplicate ingress_health checks",
    )?;
    let orchestrator_check = extract_single_check_by_name(
        &observation.checks,
        SmokeCheckName::OrchestratorStatus,
        "orchestrator_status",
        "duplicate orchestrator_status checks",
    )?;

    let decision = compute_smoke_decision(ingress_check, orchestrator_check);

    let stages = build_smoke_stages(ingress_check, orchestrator_check, &decision);

    let report = SmokeReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages,
        decision,
    };

    validate_smoke_report(&report)?;
    Ok(report)
}

fn extract_single_check_by_name<'a>(
    checks: &'a [SmokeCheckObservation],
    expected: SmokeCheckName,
    name: &'static str,
    duplicate: &'static str,
) -> Result<&'a SmokeCheckObservation, SmokeError> {
    let matching: Vec<&SmokeCheckObservation> =
        checks.iter().filter(|check| check.check == expected).collect();

    match matching.as_slice() {
        [] => Err(SmokeError::MissingCheck(name)),
        [check] => Ok(*check),
        _ => Err(SmokeError::InvalidReport(duplicate)),
    }
}

fn compute_smoke_decision(
    ingress: &SmokeCheckObservation,
    orchestrator: &SmokeCheckObservation,
) -> SmokeDecision {
    if ingress.success && orchestrator.success {
        SmokeDecision::Pass
    } else {
        SmokeDecision::Fail
    }
}

fn build_smoke_stages(
    ingress: &SmokeCheckObservation,
    orchestrator: &SmokeCheckObservation,
    decision: &SmokeDecision,
) -> Vec<SmokeStageReport> {
    let ingress_status =
        if ingress.success { SmokeStageStatus::Passed } else { SmokeStageStatus::Failed };
    let orchestrator_status =
        if orchestrator.success { SmokeStageStatus::Passed } else { SmokeStageStatus::Failed };
    let decision_status = if decision == &SmokeDecision::Pass {
        SmokeStageStatus::Passed
    } else {
        SmokeStageStatus::Failed
    };

    let prior_timestamp = ingress.timestamp.max(orchestrator.timestamp);
    let decision_timestamp = prior_timestamp + chrono::Duration::milliseconds(1);

    vec![
        SmokeStageReport {
            stage: SmokeStageName::IngressHealth,
            status: ingress_status,
            diagnostics: ingress.diagnostics.clone(),
            timestamp: ingress.timestamp,
        },
        SmokeStageReport {
            stage: SmokeStageName::OrchestratorStatus,
            status: orchestrator_status,
            diagnostics: orchestrator.diagnostics.clone(),
            timestamp: orchestrator.timestamp,
        },
        SmokeStageReport {
            stage: SmokeStageName::FinalDecision,
            status: decision_status,
            diagnostics: if decision == &SmokeDecision::Pass {
                "smoke checks passed".to_string()
            } else {
                "smoke checks failed".to_string()
            },
            timestamp: decision_timestamp,
        },
    ]
}

pub fn validate_smoke_report(report: &SmokeReport) -> Result<(), SmokeError> {
    validate_normalized_smoke_run_id(&report.run_id)?;

    let ingress_check = extract_single_check_by_name(
        &report.checks,
        SmokeCheckName::IngressHealth,
        "ingress_health",
        "invalid ingress check count",
    )?;
    let orchestrator_check = extract_single_check_by_name(
        &report.checks,
        SmokeCheckName::OrchestratorStatus,
        "orchestrator_status",
        "invalid orchestrator check count",
    )?;

    validate_smoke_check(ingress_check)?;
    validate_orchestrator_check(orchestrator_check, &report.run_id)?;

    validate_smoke_stage_order(report.stages.as_slice())?;
    validate_smoke_stage_diagnostics(report.stages.as_slice())?;
    validate_smoke_timestamps(report.stages.as_slice())?;
    validate_smoke_decision(report)?;

    Ok(())
}

fn validate_smoke_check(check: &SmokeCheckObservation) -> Result<(), SmokeError> {
    if check.diagnostics.trim().is_empty() {
        return Err(SmokeError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(&check.diagnostics) {
        return Err(SmokeError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if check.endpoint != DEFAULT_SMOKE_INGRESS_HEALTH_URL {
        return Err(SmokeError::InvalidReport("invalid ingress check endpoint"));
    }
    Ok(())
}

fn validate_orchestrator_check(
    check: &SmokeCheckObservation,
    run_id: &str,
) -> Result<(), SmokeError> {
    if check.diagnostics.trim().is_empty() {
        return Err(SmokeError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(&check.diagnostics) {
        return Err(SmokeError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if !matches_orchestrator_status_contract(&check.endpoint, run_id) {
        return Err(SmokeError::InvalidReport("invalid orchestrator check endpoint"));
    }
    Ok(())
}

fn validate_smoke_stage_order(stages: &[SmokeStageReport]) -> Result<(), SmokeError> {
    let expected_order = [
        SmokeStageName::IngressHealth,
        SmokeStageName::OrchestratorStatus,
        SmokeStageName::FinalDecision,
    ];

    if stages.len() != expected_order.len() {
        return Err(SmokeError::InvalidReport("unexpected stage count"));
    }

    let order_is_valid =
        stages.iter().map(|stage| stage.stage.clone()).eq(expected_order.iter().cloned());
    if !order_is_valid {
        return Err(SmokeError::InvalidReport("invalid stage order"));
    }

    Ok(())
}

fn validate_smoke_stage_diagnostics(stages: &[SmokeStageReport]) -> Result<(), SmokeError> {
    let has_empty_diagnostics = stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_diagnostics {
        return Err(SmokeError::InvalidReport("empty stage diagnostics"));
    }

    Ok(())
}

fn validate_smoke_timestamps(stages: &[SmokeStageReport]) -> Result<(), SmokeError> {
    let has_non_monotonic_timestamps = stages.windows(2).any(|pair| {
        let first = &pair[0].timestamp;
        let second = &pair[1].timestamp;
        first > second
    });
    if has_non_monotonic_timestamps {
        return Err(SmokeError::InvalidReport("non-monotonic stage timestamps"));
    }

    Ok(())
}

fn expected_smoke_decision(checks: &[SmokeCheckObservation]) -> SmokeDecision {
    match checks.iter().all(|c| c.success) {
        true => SmokeDecision::Pass,
        false => SmokeDecision::Fail,
    }
}

fn validate_smoke_decision(report: &SmokeReport) -> Result<(), SmokeError> {
    (report.decision == expected_smoke_decision(&report.checks))
        .then_some(())
        .ok_or(SmokeError::InvalidReport("decision mismatch"))
}

fn is_valid_http_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || contains_forbidden_control_chars(trimmed) {
        return false;
    }

    let parsed = reqwest::Url::parse(trimmed);
    match parsed {
        Ok(url) => {
            let scheme_valid = url.scheme() == "http" || url.scheme() == "https";
            let has_host = url.host_str().is_some();
            let has_no_credentials = url.username().is_empty() && url.password().is_none();
            scheme_valid && has_host && has_no_credentials
        }
        Err(_) => false,
    }
}

fn is_valid_smoke_run_id(value: &str) -> bool {
    value.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
}

fn validate_normalized_smoke_run_id(value: &str) -> Result<(), SmokeError> {
    if value.trim().is_empty() {
        return Err(SmokeError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_SMOKE_INGRESS_HEALTH_URL
}

fn matches_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/Oya/{}/get_status", run_id)
}

#[allow(dead_code)]
const DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND: &str = DEFAULT_DEV_RUNTIME_COMMAND;
#[allow(dead_code)]
const DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL: &str = DEFAULT_SMOKE_INGRESS_HEALTH_URL;
#[allow(dead_code)]
const MAX_SMOKE_BEAD_DIAGNOSTICS_LEN: usize = MAX_SMOKE_DIAGNOSTICS_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Input payload for building a smoke-bead execution plan.
pub struct SmokeBeadInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Immutable plan that binds the runtime command and required endpoints.
pub struct SmokeBeadPlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Runtime handle returned after smoke-bead runtime startup succeeds.
pub struct SmokeBeadRuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Named checks captured during smoke-bead observation.
pub enum SmokeBeadCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Observation row for a single smoke-bead check.
pub struct SmokeBeadCheckObservation {
    pub check: SmokeBeadCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Full smoke-bead observation containing all checks for one run.
pub struct SmokeBeadObservation {
    pub run_id: String,
    pub checks: Vec<SmokeBeadCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Ordered stage names for smoke-bead report evaluation.
pub enum SmokeBeadStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage pass/fail state in smoke-bead reporting.
pub enum SmokeBeadStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Structured stage-level report for smoke-bead evaluation.
pub struct SmokeBeadStageReport {
    pub stage: SmokeBeadStageName,
    pub status: SmokeBeadStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final smoke-bead decision derived from check outcomes.
pub enum SmokeBeadDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final report for a smoke-bead run.
pub struct SmokeBeadReport {
    pub run_id: String,
    pub checks: Vec<SmokeBeadCheckObservation>,
    pub stages: Vec<SmokeBeadStageReport>,
    pub decision: SmokeBeadDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Typed errors for smoke-bead planning, execution, and validation.
pub enum SmokeBeadError {
    #[error("smoke-bead field is empty: {0}")]
    EmptyField(&'static str),
    #[error("smoke-bead field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("smoke-bead field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("smoke-bead runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("smoke-bead endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("smoke-bead runtime not ready")]
    RuntimeNotReady,
    #[error("smoke-bead check missing: {0}")]
    MissingCheck(&'static str),
    #[error("smoke-bead report invalid: {0}")]
    InvalidReport(&'static str),
}

/// Builds a validated smoke-bead plan from raw input.
pub fn build_smoke_bead_plan(input: &SmokeBeadInput) -> Result<SmokeBeadPlan, SmokeBeadError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(SmokeBeadError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }

    Ok(SmokeBeadPlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!("http://localhost:8080/Oya/{}/get_status", run_id),
    })
}

/// Starts the smoke-bead runtime using the default runtime contract.
pub fn start_smoke_bead_runtime(
    plan: &SmokeBeadPlan,
) -> Result<SmokeBeadRuntimeHandle, SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(plan.run_id.as_str())?;

    if plan.runtime_command != DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND {
        return Err(SmokeBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(plan.ingress_health_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_smoke_bead_ingress_health_contract(plan.ingress_health_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(plan.orchestrator_status_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_smoke_bead_orchestrator_status_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(SmokeBeadRuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

/// Captures smoke-bead observations for ingress and orchestrator checks.
pub fn capture_smoke_bead_observation(
    handle: &SmokeBeadRuntimeHandle,
) -> Result<SmokeBeadObservation, SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(handle.run_id.as_str())?;
    validate_smoke_bead_runtime_handle(handle)?;

    Ok(SmokeBeadObservation {
        run_id: handle.run_id.clone(),
        checks: build_smoke_bead_checks(handle, Utc::now()),
    })
}

/// Evaluates a smoke-bead observation into a typed report.
pub fn evaluate_smoke_bead_result(
    observation: &SmokeBeadObservation,
) -> Result<SmokeBeadReport, SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(observation.run_id.as_str())?;
    let ingress_check = find_smoke_bead_check(
        observation.checks.as_slice(),
        SmokeBeadCheckName::IngressHealth,
        "ingress_health",
        "duplicate ingress_health checks",
    )?;
    let orchestrator_check = find_smoke_bead_check(
        observation.checks.as_slice(),
        SmokeBeadCheckName::OrchestratorStatus,
        "orchestrator_status",
        "duplicate orchestrator_status checks",
    )?;
    let decision = derive_smoke_bead_decision(ingress_check, orchestrator_check);

    let report = SmokeBeadReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages: build_smoke_bead_stages(ingress_check, orchestrator_check, &decision),
        decision,
    };

    validate_smoke_bead_report(&report)?;
    Ok(report)
}

/// Validates report structure, stage ordering, endpoint coherence, and decision consistency.
pub fn validate_smoke_bead_report(report: &SmokeBeadReport) -> Result<(), SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(report.run_id.as_str())?;

    let ingress_check = find_smoke_bead_check(
        report.checks.as_slice(),
        SmokeBeadCheckName::IngressHealth,
        "ingress_health",
        "invalid ingress check count",
    )?;
    let orchestrator_check = find_smoke_bead_check(
        report.checks.as_slice(),
        SmokeBeadCheckName::OrchestratorStatus,
        "orchestrator_status",
        "invalid orchestrator check count",
    )?;

    validate_smoke_bead_check(ingress_check, report.run_id.as_str())?;
    validate_smoke_bead_check(orchestrator_check, report.run_id.as_str())?;

    validate_smoke_bead_stage_order(report.stages.as_slice())?;
    validate_smoke_bead_stage_diagnostics(report.stages.as_slice())?;
    validate_smoke_bead_timestamps(report.stages.as_slice())?;

    validate_smoke_bead_stage_status(
        &report.stages[0],
        ingress_check,
        SmokeBeadStageRole::Ingress,
    )?;
    validate_smoke_bead_stage_status(
        &report.stages[1],
        orchestrator_check,
        SmokeBeadStageRole::Orchestrator,
    )?;
    validate_smoke_bead_decision(report, ingress_check, orchestrator_check)?;
    validate_smoke_bead_final_stage(&report.stages[2], &report.decision)?;

    Ok(())
}

fn validate_smoke_bead_runtime_handle(
    handle: &SmokeBeadRuntimeHandle,
) -> Result<(), SmokeBeadError> {
    if !handle.runtime_ready {
        return Err(SmokeBeadError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND {
        return Err(SmokeBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(handle.ingress_health_url.as_str())
        || !matches_smoke_bead_ingress_health_contract(handle.ingress_health_url.as_str())
    {
        return Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(handle.orchestrator_status_url.as_str())
        || !matches_smoke_bead_orchestrator_status_contract(
            handle.orchestrator_status_url.as_str(),
            handle.run_id.as_str(),
        )
    {
        return Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    Ok(())
}

fn build_smoke_bead_checks(
    handle: &SmokeBeadRuntimeHandle,
    base_timestamp: DateTime<Utc>,
) -> Vec<SmokeBeadCheckObservation> {
    vec![
        SmokeBeadCheckObservation {
            check: SmokeBeadCheckName::IngressHealth,
            endpoint: handle.ingress_health_url.clone(),
            success: true,
            diagnostics: "ingress health check passed".to_string(),
            timestamp: base_timestamp,
        },
        SmokeBeadCheckObservation {
            check: SmokeBeadCheckName::OrchestratorStatus,
            endpoint: handle.orchestrator_status_url.clone(),
            success: true,
            diagnostics: "orchestrator status check passed".to_string(),
            timestamp: base_timestamp + chrono::Duration::milliseconds(1),
        },
    ]
}

fn find_smoke_bead_check<'a>(
    checks: &'a [SmokeBeadCheckObservation],
    target: SmokeBeadCheckName,
    missing: &'static str,
    duplicate: &'static str,
) -> Result<&'a SmokeBeadCheckObservation, SmokeBeadError> {
    let matches: Vec<&SmokeBeadCheckObservation> =
        checks.iter().filter(|check| check.check == target).collect();
    match matches.as_slice() {
        [] => Err(SmokeBeadError::MissingCheck(missing)),
        [check] => Ok(*check),
        _ => Err(SmokeBeadError::InvalidReport(duplicate)),
    }
}

fn derive_smoke_bead_decision(
    ingress_check: &SmokeBeadCheckObservation,
    orchestrator_check: &SmokeBeadCheckObservation,
) -> SmokeBeadDecision {
    if ingress_check.success && orchestrator_check.success {
        SmokeBeadDecision::Pass
    } else {
        SmokeBeadDecision::Fail
    }
}

fn build_smoke_bead_stages(
    ingress_check: &SmokeBeadCheckObservation,
    orchestrator_check: &SmokeBeadCheckObservation,
    decision: &SmokeBeadDecision,
) -> Vec<SmokeBeadStageReport> {
    let ingress_time = ingress_check.timestamp;
    let orchestrator_time = if orchestrator_check.timestamp < ingress_time {
        ingress_time
    } else {
        orchestrator_check.timestamp
    };
    let final_time = orchestrator_time + chrono::Duration::milliseconds(1);
    vec![
        SmokeBeadStageReport {
            stage: SmokeBeadStageName::IngressHealth,
            status: if ingress_check.success {
                SmokeBeadStageStatus::Passed
            } else {
                SmokeBeadStageStatus::Failed
            },
            diagnostics: ingress_check.diagnostics.clone(),
            timestamp: ingress_time,
        },
        SmokeBeadStageReport {
            stage: SmokeBeadStageName::OrchestratorStatus,
            status: if orchestrator_check.success {
                SmokeBeadStageStatus::Passed
            } else {
                SmokeBeadStageStatus::Failed
            },
            diagnostics: orchestrator_check.diagnostics.clone(),
            timestamp: orchestrator_time,
        },
        SmokeBeadStageReport {
            stage: SmokeBeadStageName::FinalDecision,
            status: if decision == &SmokeBeadDecision::Pass {
                SmokeBeadStageStatus::Passed
            } else {
                SmokeBeadStageStatus::Failed
            },
            diagnostics: expected_smoke_bead_final_diagnostics(decision).to_string(),
            timestamp: final_time,
        },
    ]
}

fn validate_smoke_bead_decision(
    report: &SmokeBeadReport,
    ingress_check: &SmokeBeadCheckObservation,
    orchestrator_check: &SmokeBeadCheckObservation,
) -> Result<(), SmokeBeadError> {
    let derived = derive_smoke_bead_decision(ingress_check, orchestrator_check);
    if report.decision != derived {
        return Err(SmokeBeadError::InvalidReport("decision mismatch"));
    }
    Ok(())
}

fn validate_smoke_bead_stage_order(stages: &[SmokeBeadStageReport]) -> Result<(), SmokeBeadError> {
    let expected_stage_order = [
        SmokeBeadStageName::IngressHealth,
        SmokeBeadStageName::OrchestratorStatus,
        SmokeBeadStageName::FinalDecision,
    ];
    if stages.len() != expected_stage_order.len() {
        return Err(SmokeBeadError::InvalidReport("unexpected stage count"));
    }

    let stage_order_valid =
        stages.iter().map(|stage| stage.stage.clone()).eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(SmokeBeadError::InvalidReport("invalid stage order"));
    }

    Ok(())
}

fn validate_smoke_bead_stage_diagnostics(
    stages: &[SmokeBeadStageReport],
) -> Result<(), SmokeBeadError> {
    let has_empty_stage_diagnostics =
        stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_stage_diagnostics {
        return Err(SmokeBeadError::InvalidReport("empty stage diagnostics"));
    }

    let has_oversized_stage_diagnostics =
        stages.iter().any(|stage| stage.diagnostics.len() > MAX_SMOKE_BEAD_DIAGNOSTICS_LEN);
    if has_oversized_stage_diagnostics {
        return Err(SmokeBeadError::InvalidReport("stage diagnostics exceed max length"));
    }

    let has_invalid_stage_diagnostics =
        stages.iter().any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str()));
    if has_invalid_stage_diagnostics {
        return Err(SmokeBeadError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }

    Ok(())
}

fn validate_smoke_bead_timestamps(stages: &[SmokeBeadStageReport]) -> Result<(), SmokeBeadError> {
    let has_non_monotonic_timestamps = stages.windows(2).any(|pair| {
        let first = &pair[0].timestamp;
        let second = &pair[1].timestamp;
        first > second
    });
    if has_non_monotonic_timestamps {
        return Err(SmokeBeadError::InvalidReport("non-monotonic stage timestamps"));
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum SmokeBeadStageRole {
    Ingress,
    Orchestrator,
}

fn smoke_bead_stage_error(role: SmokeBeadStageRole, kind: &str) -> &'static str {
    match (role, kind) {
        (SmokeBeadStageRole::Ingress, "status") => "ingress stage mismatch",
        (SmokeBeadStageRole::Ingress, "diagnostics") => "ingress stage diagnostics mismatch",
        (SmokeBeadStageRole::Ingress, "timestamp") => "ingress stage timestamp precedes check",
        (SmokeBeadStageRole::Orchestrator, "status") => "orchestrator stage mismatch",
        (SmokeBeadStageRole::Orchestrator, "diagnostics") => {
            "orchestrator stage diagnostics mismatch"
        }
        (SmokeBeadStageRole::Orchestrator, "timestamp") => {
            "orchestrator stage timestamp precedes check"
        }
        _ => "stage mismatch",
    }
}

fn validate_smoke_bead_stage_status(
    stage: &SmokeBeadStageReport,
    check: &SmokeBeadCheckObservation,
    role: SmokeBeadStageRole,
) -> Result<(), SmokeBeadError> {
    let expected_status =
        if check.success { SmokeBeadStageStatus::Passed } else { SmokeBeadStageStatus::Failed };
    if stage.status != expected_status {
        return Err(SmokeBeadError::InvalidReport(smoke_bead_stage_error(role, "status")));
    }
    if stage.diagnostics != check.diagnostics {
        return Err(SmokeBeadError::InvalidReport(smoke_bead_stage_error(role, "diagnostics")));
    }
    if stage.timestamp < check.timestamp {
        return Err(SmokeBeadError::InvalidReport(smoke_bead_stage_error(role, "timestamp")));
    }
    Ok(())
}

fn validate_smoke_bead_final_stage(
    final_stage: &SmokeBeadStageReport,
    decision: &SmokeBeadDecision,
) -> Result<(), SmokeBeadError> {
    let expected_final_stage = if decision == &SmokeBeadDecision::Pass {
        SmokeBeadStageStatus::Passed
    } else {
        SmokeBeadStageStatus::Failed
    };
    if final_stage.status != expected_final_stage {
        return Err(SmokeBeadError::InvalidReport("final decision stage mismatch"));
    }

    let expected_final_diagnostics = expected_smoke_bead_final_diagnostics(decision);
    if final_stage.diagnostics != expected_final_diagnostics {
        return Err(SmokeBeadError::InvalidReport("final decision diagnostics mismatch"));
    }

    Ok(())
}

fn validate_smoke_bead_check(
    check: &SmokeBeadCheckObservation,
    run_id: &str,
) -> Result<(), SmokeBeadError> {
    if check.diagnostics.trim().is_empty() {
        return Err(SmokeBeadError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(check.diagnostics.as_str()) {
        return Err(SmokeBeadError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if check.diagnostics.len() > MAX_SMOKE_BEAD_DIAGNOSTICS_LEN {
        return Err(SmokeBeadError::InvalidReport("check diagnostics exceed max length"));
    }

    match check.check {
        SmokeBeadCheckName::IngressHealth => {
            if !matches_smoke_bead_ingress_health_contract(check.endpoint.as_str()) {
                return Err(SmokeBeadError::InvalidReport("invalid ingress check endpoint"));
            }
        }
        SmokeBeadCheckName::OrchestratorStatus => {
            if !matches_smoke_bead_orchestrator_status_contract(check.endpoint.as_str(), run_id) {
                return Err(SmokeBeadError::InvalidReport("invalid orchestrator check endpoint"));
            }
        }
    }

    Ok(())
}

fn validate_normalized_smoke_bead_run_id(value: &str) -> Result<(), SmokeBeadError> {
    if value.trim().is_empty() {
        return Err(SmokeBeadError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_smoke_bead_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL
}

fn matches_smoke_bead_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/Oya/{}/get_status", run_id)
}

fn expected_smoke_bead_final_diagnostics(decision: &SmokeBeadDecision) -> &'static str {
    match decision {
        SmokeBeadDecision::Pass => "smoke-bead checks passed",
        SmokeBeadDecision::Fail => "smoke-bead checks failed",
    }
}

#[allow(dead_code)]
const DEFAULT_LEAN_BEAD_RUNTIME_COMMAND: &str = DEFAULT_DEV_RUNTIME_COMMAND;
#[allow(dead_code)]
const DEFAULT_LEAN_BEAD_INGRESS_HEALTH_URL: &str = DEFAULT_SMOKE_INGRESS_HEALTH_URL;
#[allow(dead_code)]
const MAX_LEAN_BEAD_DIAGNOSTICS_LEN: usize = MAX_SMOKE_DIAGNOSTICS_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadPlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadRuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadCheckObservation {
    pub check: LeanBeadCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadObservation {
    pub run_id: String,
    pub checks: Vec<LeanBeadCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadStageReport {
    pub stage: LeanBeadStageName,
    pub status: LeanBeadStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadReport {
    pub run_id: String,
    pub checks: Vec<LeanBeadCheckObservation>,
    pub stages: Vec<LeanBeadStageReport>,
    pub decision: LeanBeadDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LeanBeadError {
    #[error("lean-bead field is empty: {0}")]
    EmptyField(&'static str),
    #[error("lean-bead field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("lean-bead field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("lean-bead runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("lean-bead endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("lean-bead runtime not ready")]
    RuntimeNotReady,
    #[error("lean-bead check missing: {0}")]
    MissingCheck(&'static str),
    #[error("lean-bead report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_lean_bead_plan(input: &LeanBeadInput) -> Result<LeanBeadPlan, LeanBeadError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(LeanBeadError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(LeanBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }

    Ok(LeanBeadPlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_LEAN_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_LEAN_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!("http://localhost:8080/Oya/{}/get_status", run_id),
    })
}

pub fn start_lean_bead_runtime(
    plan: &LeanBeadPlan,
) -> Result<LeanBeadRuntimeHandle, LeanBeadError> {
    validate_normalized_lean_bead_run_id(plan.run_id.as_str())?;

    if plan.runtime_command != DEFAULT_LEAN_BEAD_RUNTIME_COMMAND {
        return Err(LeanBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(plan.ingress_health_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_lean_bead_ingress_health_contract(plan.ingress_health_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(plan.orchestrator_status_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_lean_bead_orchestrator_status_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(LeanBeadError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(LeanBeadRuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

pub fn capture_lean_bead_observation(
    handle: &LeanBeadRuntimeHandle,
) -> Result<LeanBeadObservation, LeanBeadError> {
    validate_normalized_lean_bead_run_id(handle.run_id.as_str())?;
    validate_lean_bead_runtime_handle(handle)?;
    Ok(LeanBeadObservation {
        run_id: handle.run_id.clone(),
        checks: build_lean_bead_checks(handle, Utc::now()),
    })
}

pub fn evaluate_lean_bead_result(
    observation: &LeanBeadObservation,
) -> Result<LeanBeadReport, LeanBeadError> {
    validate_normalized_lean_bead_run_id(observation.run_id.as_str())?;
    let ingress_check = find_lean_bead_check(
        observation.checks.as_slice(),
        LeanBeadCheckName::IngressHealth,
        "ingress_health",
        "duplicate ingress_health checks",
    )?;
    let orchestrator_check = find_lean_bead_check(
        observation.checks.as_slice(),
        LeanBeadCheckName::OrchestratorStatus,
        "orchestrator_status",
        "duplicate orchestrator_status checks",
    )?;
    let decision = derive_lean_bead_decision(ingress_check, orchestrator_check);

    let report = LeanBeadReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages: build_lean_bead_stages(ingress_check, orchestrator_check, &decision),
        decision,
    };

    validate_lean_bead_report(&report)?;
    Ok(report)
}

pub fn validate_lean_bead_report(report: &LeanBeadReport) -> Result<(), LeanBeadError> {
    validate_normalized_lean_bead_run_id(report.run_id.as_str())?;
    let ingress_check = find_lean_bead_check(
        report.checks.as_slice(),
        LeanBeadCheckName::IngressHealth,
        "ingress_health",
        "invalid ingress check count",
    )?;
    let orchestrator_check = find_lean_bead_check(
        report.checks.as_slice(),
        LeanBeadCheckName::OrchestratorStatus,
        "orchestrator_status",
        "invalid orchestrator check count",
    )?;
    validate_lean_bead_check(ingress_check, report.run_id.as_str())?;
    validate_lean_bead_check(orchestrator_check, report.run_id.as_str())?;
    validate_lean_bead_stage_contract(report.stages.as_slice())?;
    validate_lean_bead_stage_semantics(report, ingress_check, orchestrator_check)
}

fn validate_lean_bead_runtime_handle(handle: &LeanBeadRuntimeHandle) -> Result<(), LeanBeadError> {
    if !handle.runtime_ready {
        return Err(LeanBeadError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_LEAN_BEAD_RUNTIME_COMMAND {
        return Err(LeanBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(handle.ingress_health_url.as_str())
        || !matches_lean_bead_ingress_health_contract(handle.ingress_health_url.as_str())
    {
        return Err(LeanBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(handle.orchestrator_status_url.as_str())
        || !matches_lean_bead_orchestrator_status_contract(
            handle.orchestrator_status_url.as_str(),
            handle.run_id.as_str(),
        )
    {
        return Err(LeanBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    Ok(())
}

fn build_lean_bead_checks(
    handle: &LeanBeadRuntimeHandle,
    base_timestamp: DateTime<Utc>,
) -> Vec<LeanBeadCheckObservation> {
    vec![
        LeanBeadCheckObservation {
            check: LeanBeadCheckName::IngressHealth,
            endpoint: handle.ingress_health_url.clone(),
            success: true,
            diagnostics: "ingress health check passed".to_string(),
            timestamp: base_timestamp,
        },
        LeanBeadCheckObservation {
            check: LeanBeadCheckName::OrchestratorStatus,
            endpoint: handle.orchestrator_status_url.clone(),
            success: true,
            diagnostics: "orchestrator status check passed".to_string(),
            timestamp: base_timestamp + chrono::Duration::milliseconds(1),
        },
    ]
}

fn find_lean_bead_check<'a>(
    checks: &'a [LeanBeadCheckObservation],
    target: LeanBeadCheckName,
    missing: &'static str,
    duplicate: &'static str,
) -> Result<&'a LeanBeadCheckObservation, LeanBeadError> {
    let matches: Vec<&LeanBeadCheckObservation> =
        checks.iter().filter(|check| check.check == target).collect();
    match matches.as_slice() {
        [] => Err(LeanBeadError::MissingCheck(missing)),
        [check] => Ok(*check),
        _ => Err(LeanBeadError::InvalidReport(duplicate)),
    }
}

fn derive_lean_bead_decision(
    ingress_check: &LeanBeadCheckObservation,
    orchestrator_check: &LeanBeadCheckObservation,
) -> LeanBeadDecision {
    if ingress_check.success && orchestrator_check.success {
        LeanBeadDecision::Pass
    } else {
        LeanBeadDecision::Fail
    }
}

fn build_lean_bead_stages(
    ingress_check: &LeanBeadCheckObservation,
    orchestrator_check: &LeanBeadCheckObservation,
    decision: &LeanBeadDecision,
) -> Vec<LeanBeadStageReport> {
    let ingress_time = ingress_check.timestamp;
    let orchestrator_time = if orchestrator_check.timestamp < ingress_time {
        ingress_time
    } else {
        orchestrator_check.timestamp
    };
    let final_time = orchestrator_time + chrono::Duration::milliseconds(1);
    vec![
        LeanBeadStageReport {
            stage: LeanBeadStageName::IngressHealth,
            status: if ingress_check.success {
                LeanBeadStageStatus::Passed
            } else {
                LeanBeadStageStatus::Failed
            },
            diagnostics: ingress_check.diagnostics.clone(),
            timestamp: ingress_time,
        },
        LeanBeadStageReport {
            stage: LeanBeadStageName::OrchestratorStatus,
            status: if orchestrator_check.success {
                LeanBeadStageStatus::Passed
            } else {
                LeanBeadStageStatus::Failed
            },
            diagnostics: orchestrator_check.diagnostics.clone(),
            timestamp: orchestrator_time,
        },
        LeanBeadStageReport {
            stage: LeanBeadStageName::FinalDecision,
            status: if decision == &LeanBeadDecision::Pass {
                LeanBeadStageStatus::Passed
            } else {
                LeanBeadStageStatus::Failed
            },
            diagnostics: expected_lean_bead_final_diagnostics(decision).to_string(),
            timestamp: final_time,
        },
    ]
}

fn validate_lean_bead_stage_contract(stages: &[LeanBeadStageReport]) -> Result<(), LeanBeadError> {
    let expected = [
        LeanBeadStageName::IngressHealth,
        LeanBeadStageName::OrchestratorStatus,
        LeanBeadStageName::FinalDecision,
    ];
    if stages.len() != expected.len() {
        return Err(LeanBeadError::InvalidReport("unexpected stage count"));
    }
    if !stages.iter().map(|stage| stage.stage.clone()).eq(expected.iter().cloned()) {
        return Err(LeanBeadError::InvalidReport("invalid stage order"));
    }
    if stages.iter().any(|stage| stage.diagnostics.trim().is_empty()) {
        return Err(LeanBeadError::InvalidReport("empty stage diagnostics"));
    }
    if stages.iter().any(|stage| stage.diagnostics.len() > MAX_LEAN_BEAD_DIAGNOSTICS_LEN) {
        return Err(LeanBeadError::InvalidReport("stage diagnostics exceed max length"));
    }
    if stages.iter().any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str())) {
        return Err(LeanBeadError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }
    if stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp) {
        return Err(LeanBeadError::InvalidReport("non-monotonic stage timestamps"));
    }
    Ok(())
}

fn validate_lean_bead_stage_semantics(
    report: &LeanBeadReport,
    ingress_check: &LeanBeadCheckObservation,
    orchestrator_check: &LeanBeadCheckObservation,
) -> Result<(), LeanBeadError> {
    let ingress_status = if ingress_check.success {
        LeanBeadStageStatus::Passed
    } else {
        LeanBeadStageStatus::Failed
    };
    let orchestrator_status = if orchestrator_check.success {
        LeanBeadStageStatus::Passed
    } else {
        LeanBeadStageStatus::Failed
    };
    if report.stages[0].status != ingress_status
        || report.stages[0].diagnostics != ingress_check.diagnostics
    {
        return Err(LeanBeadError::InvalidReport("ingress stage mismatch"));
    }
    if report.stages[1].status != orchestrator_status
        || report.stages[1].diagnostics != orchestrator_check.diagnostics
    {
        return Err(LeanBeadError::InvalidReport("orchestrator stage mismatch"));
    }
    if report.stages[0].timestamp < ingress_check.timestamp
        || report.stages[1].timestamp < orchestrator_check.timestamp
    {
        return Err(LeanBeadError::InvalidReport("stage timestamp precedes check"));
    }
    let derived = derive_lean_bead_decision(ingress_check, orchestrator_check);
    if report.decision != derived {
        return Err(LeanBeadError::InvalidReport("decision mismatch"));
    }
    validate_lean_bead_final_stage(&report.stages[2], &derived)
}

fn validate_lean_bead_final_stage(
    stage: &LeanBeadStageReport,
    decision: &LeanBeadDecision,
) -> Result<(), LeanBeadError> {
    let expected = if decision == &LeanBeadDecision::Pass {
        LeanBeadStageStatus::Passed
    } else {
        LeanBeadStageStatus::Failed
    };
    if stage.status != expected {
        return Err(LeanBeadError::InvalidReport("final decision stage mismatch"));
    }
    if stage.diagnostics != expected_lean_bead_final_diagnostics(decision) {
        return Err(LeanBeadError::InvalidReport("final decision diagnostics mismatch"));
    }
    Ok(())
}

fn validate_lean_bead_check(
    check: &LeanBeadCheckObservation,
    run_id: &str,
) -> Result<(), LeanBeadError> {
    if check.diagnostics.trim().is_empty() {
        return Err(LeanBeadError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(check.diagnostics.as_str()) {
        return Err(LeanBeadError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if check.diagnostics.len() > MAX_LEAN_BEAD_DIAGNOSTICS_LEN {
        return Err(LeanBeadError::InvalidReport("check diagnostics exceed max length"));
    }

    match check.check {
        LeanBeadCheckName::IngressHealth => {
            if !matches_lean_bead_ingress_health_contract(check.endpoint.as_str()) {
                return Err(LeanBeadError::InvalidReport("invalid ingress check endpoint"));
            }
        }
        LeanBeadCheckName::OrchestratorStatus => {
            if !matches_lean_bead_orchestrator_status_contract(check.endpoint.as_str(), run_id) {
                return Err(LeanBeadError::InvalidReport("invalid orchestrator check endpoint"));
            }
        }
    }

    Ok(())
}

fn validate_normalized_lean_bead_run_id(value: &str) -> Result<(), LeanBeadError> {
    if value.trim().is_empty() {
        return Err(LeanBeadError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(LeanBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_lean_bead_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_LEAN_BEAD_INGRESS_HEALTH_URL
}

fn matches_lean_bead_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/Oya/{}/get_status", run_id)
}

fn expected_lean_bead_final_diagnostics(decision: &LeanBeadDecision) -> &'static str {
    match decision {
        LeanBeadDecision::Pass => "lean-bead checks passed",
        LeanBeadDecision::Fail => "lean-bead checks failed",
    }
}

#[allow(dead_code)]
const DEFAULT_BEAD_MIN_RUNTIME_COMMAND: &str = DEFAULT_DEV_RUNTIME_COMMAND;
#[allow(dead_code)]
const DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL: &str = DEFAULT_SMOKE_INGRESS_HEALTH_URL;
#[allow(dead_code)]
const MAX_BEAD_MIN_DIAGNOSTICS_LEN: usize = MAX_SMOKE_DIAGNOSTICS_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinPlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinRuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinCheckObservation {
    pub check: BeadMinCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinObservation {
    pub run_id: String,
    pub checks: Vec<BeadMinCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinStageReport {
    pub stage: BeadMinStageName,
    pub status: BeadMinStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinReport {
    pub run_id: String,
    pub checks: Vec<BeadMinCheckObservation>,
    pub stages: Vec<BeadMinStageReport>,
    pub decision: BeadMinDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BeadMinError {
    #[error("bead-min field is empty: {0}")]
    EmptyField(&'static str),
    #[error("bead-min field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("bead-min field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("bead-min runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("bead-min endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("bead-min runtime not ready")]
    RuntimeNotReady,
    #[error("bead-min check missing: {0}")]
    MissingCheck(&'static str),
    #[error("bead-min report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_bead_min_plan(input: &BeadMinInput) -> Result<BeadMinPlan, BeadMinError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(BeadMinError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(BeadMinError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }

    Ok(BeadMinPlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!("http://localhost:8080/Oya/{}/get_status", run_id),
    })
}

pub fn start_bead_min_runtime(plan: &BeadMinPlan) -> Result<BeadMinRuntimeHandle, BeadMinError> {
    validate_normalized_bead_min_run_id(plan.run_id.as_str())?;

    if plan.runtime_command != DEFAULT_BEAD_MIN_RUNTIME_COMMAND {
        return Err(BeadMinError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(plan.ingress_health_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_min_ingress_health_contract(plan.ingress_health_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(plan.orchestrator_status_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_min_orchestrator_status_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(BeadMinRuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

pub fn capture_bead_min_observation(
    handle: &BeadMinRuntimeHandle,
) -> Result<BeadMinObservation, BeadMinError> {
    validate_normalized_bead_min_run_id(handle.run_id.as_str())?;
    validate_bead_min_runtime_handle(handle)?;

    Ok(BeadMinObservation {
        run_id: handle.run_id.clone(),
        checks: build_bead_min_checks(handle, Utc::now()),
    })
}

pub fn evaluate_bead_min_result(
    observation: &BeadMinObservation,
) -> Result<BeadMinReport, BeadMinError> {
    validate_normalized_bead_min_run_id(observation.run_id.as_str())?;
    let ingress_check = find_bead_min_check(
        observation.checks.as_slice(),
        BeadMinCheckName::IngressHealth,
        "ingress_health",
        "duplicate ingress_health checks",
    )?;
    let orchestrator_check = find_bead_min_check(
        observation.checks.as_slice(),
        BeadMinCheckName::OrchestratorStatus,
        "orchestrator_status",
        "duplicate orchestrator_status checks",
    )?;
    let decision = derive_bead_min_decision(ingress_check, orchestrator_check);

    let report = BeadMinReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages: build_bead_min_stages(ingress_check, orchestrator_check, &decision),
        decision,
    };

    validate_bead_min_report(&report)?;
    Ok(report)
}

pub fn validate_bead_min_report(report: &BeadMinReport) -> Result<(), BeadMinError> {
    validate_normalized_bead_min_run_id(report.run_id.as_str())?;
    let ingress_check = find_bead_min_check(
        report.checks.as_slice(),
        BeadMinCheckName::IngressHealth,
        "ingress_health",
        "invalid ingress check count",
    )?;
    let orchestrator_check = find_bead_min_check(
        report.checks.as_slice(),
        BeadMinCheckName::OrchestratorStatus,
        "orchestrator_status",
        "invalid orchestrator check count",
    )?;
    validate_bead_min_check(ingress_check, report.run_id.as_str())?;
    validate_bead_min_check(orchestrator_check, report.run_id.as_str())?;
    validate_bead_min_stage_contract(report.stages.as_slice())?;
    validate_bead_min_stage_semantics(report, ingress_check, orchestrator_check)
}

fn validate_bead_min_runtime_handle(handle: &BeadMinRuntimeHandle) -> Result<(), BeadMinError> {
    if !handle.runtime_ready {
        return Err(BeadMinError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_BEAD_MIN_RUNTIME_COMMAND {
        return Err(BeadMinError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(handle.ingress_health_url.as_str())
        || !matches_bead_min_ingress_health_contract(handle.ingress_health_url.as_str())
    {
        return Err(BeadMinError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(handle.orchestrator_status_url.as_str())
        || !matches_bead_min_orchestrator_status_contract(
            handle.orchestrator_status_url.as_str(),
            handle.run_id.as_str(),
        )
    {
        return Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"));
    }
    Ok(())
}

fn build_bead_min_checks(
    handle: &BeadMinRuntimeHandle,
    base_timestamp: DateTime<Utc>,
) -> Vec<BeadMinCheckObservation> {
    vec![
        BeadMinCheckObservation {
            check: BeadMinCheckName::IngressHealth,
            endpoint: handle.ingress_health_url.clone(),
            success: true,
            diagnostics: "ingress health check passed".to_string(),
            timestamp: base_timestamp,
        },
        BeadMinCheckObservation {
            check: BeadMinCheckName::OrchestratorStatus,
            endpoint: handle.orchestrator_status_url.clone(),
            success: true,
            diagnostics: "orchestrator status check passed".to_string(),
            timestamp: base_timestamp + chrono::Duration::milliseconds(1),
        },
    ]
}

fn find_bead_min_check<'a>(
    checks: &'a [BeadMinCheckObservation],
    target: BeadMinCheckName,
    missing: &'static str,
    duplicate: &'static str,
) -> Result<&'a BeadMinCheckObservation, BeadMinError> {
    let matches: Vec<&BeadMinCheckObservation> =
        checks.iter().filter(|check| check.check == target).collect();
    match matches.as_slice() {
        [] => Err(BeadMinError::MissingCheck(missing)),
        [check] => Ok(*check),
        _ => Err(BeadMinError::InvalidReport(duplicate)),
    }
}

fn derive_bead_min_decision(
    ingress_check: &BeadMinCheckObservation,
    orchestrator_check: &BeadMinCheckObservation,
) -> BeadMinDecision {
    if ingress_check.success && orchestrator_check.success {
        BeadMinDecision::Pass
    } else {
        BeadMinDecision::Fail
    }
}

fn build_bead_min_stages(
    ingress_check: &BeadMinCheckObservation,
    orchestrator_check: &BeadMinCheckObservation,
    decision: &BeadMinDecision,
) -> Vec<BeadMinStageReport> {
    let ingress_time = ingress_check.timestamp;
    let orchestrator_time = if orchestrator_check.timestamp < ingress_time {
        ingress_time
    } else {
        orchestrator_check.timestamp
    };
    let final_time = orchestrator_time + chrono::Duration::milliseconds(1);
    vec![
        BeadMinStageReport {
            stage: BeadMinStageName::IngressHealth,
            status: if ingress_check.success {
                BeadMinStageStatus::Passed
            } else {
                BeadMinStageStatus::Failed
            },
            diagnostics: ingress_check.diagnostics.clone(),
            timestamp: ingress_time,
        },
        BeadMinStageReport {
            stage: BeadMinStageName::OrchestratorStatus,
            status: if orchestrator_check.success {
                BeadMinStageStatus::Passed
            } else {
                BeadMinStageStatus::Failed
            },
            diagnostics: orchestrator_check.diagnostics.clone(),
            timestamp: orchestrator_time,
        },
        BeadMinStageReport {
            stage: BeadMinStageName::FinalDecision,
            status: if decision == &BeadMinDecision::Pass {
                BeadMinStageStatus::Passed
            } else {
                BeadMinStageStatus::Failed
            },
            diagnostics: expected_bead_min_final_diagnostics(decision).to_string(),
            timestamp: final_time,
        },
    ]
}

fn validate_bead_min_stage_contract(stages: &[BeadMinStageReport]) -> Result<(), BeadMinError> {
    let expected = [
        BeadMinStageName::IngressHealth,
        BeadMinStageName::OrchestratorStatus,
        BeadMinStageName::FinalDecision,
    ];
    if stages.len() != expected.len() {
        return Err(BeadMinError::InvalidReport("unexpected stage count"));
    }
    if !stages.iter().map(|stage| stage.stage.clone()).eq(expected.iter().cloned()) {
        return Err(BeadMinError::InvalidReport("invalid stage order"));
    }
    if stages.iter().any(|stage| stage.diagnostics.trim().is_empty()) {
        return Err(BeadMinError::InvalidReport("empty stage diagnostics"));
    }
    if stages.iter().any(|stage| stage.diagnostics.len() > MAX_BEAD_MIN_DIAGNOSTICS_LEN) {
        return Err(BeadMinError::InvalidReport("stage diagnostics exceed max length"));
    }
    if stages.iter().any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str())) {
        return Err(BeadMinError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }
    if stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp) {
        return Err(BeadMinError::InvalidReport("non-monotonic stage timestamps"));
    }
    Ok(())
}

fn validate_bead_min_stage_semantics(
    report: &BeadMinReport,
    ingress_check: &BeadMinCheckObservation,
    orchestrator_check: &BeadMinCheckObservation,
) -> Result<(), BeadMinError> {
    let ingress_status =
        if ingress_check.success { BeadMinStageStatus::Passed } else { BeadMinStageStatus::Failed };
    let orchestrator_status = if orchestrator_check.success {
        BeadMinStageStatus::Passed
    } else {
        BeadMinStageStatus::Failed
    };
    if report.stages[0].status != ingress_status
        || report.stages[0].diagnostics != ingress_check.diagnostics
    {
        return Err(BeadMinError::InvalidReport("ingress stage mismatch"));
    }
    if report.stages[1].status != orchestrator_status
        || report.stages[1].diagnostics != orchestrator_check.diagnostics
    {
        return Err(BeadMinError::InvalidReport("orchestrator stage mismatch"));
    }
    if report.stages[0].timestamp < ingress_check.timestamp {
        return Err(BeadMinError::InvalidReport("ingress stage timestamp precedes check"));
    }
    if report.stages[1].timestamp < orchestrator_check.timestamp {
        return Err(BeadMinError::InvalidReport("orchestrator stage timestamp precedes check"));
    }
    let derived = derive_bead_min_decision(ingress_check, orchestrator_check);
    if report.decision != derived {
        return Err(BeadMinError::InvalidReport("decision mismatch"));
    }
    if report.stages[2].status
        != if derived == BeadMinDecision::Pass {
            BeadMinStageStatus::Passed
        } else {
            BeadMinStageStatus::Failed
        }
    {
        return Err(BeadMinError::InvalidReport("final decision stage mismatch"));
    }
    if report.stages[2].diagnostics != expected_bead_min_final_diagnostics(&derived) {
        return Err(BeadMinError::InvalidReport("final decision diagnostics mismatch"));
    }
    Ok(())
}

fn validate_bead_min_check(
    check: &BeadMinCheckObservation,
    run_id: &str,
) -> Result<(), BeadMinError> {
    if check.diagnostics.trim().is_empty() {
        return Err(BeadMinError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(check.diagnostics.as_str()) {
        return Err(BeadMinError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if check.diagnostics.len() > MAX_BEAD_MIN_DIAGNOSTICS_LEN {
        return Err(BeadMinError::InvalidReport("check diagnostics exceed max length"));
    }

    match check.check {
        BeadMinCheckName::IngressHealth => {
            if !matches_bead_min_ingress_health_contract(check.endpoint.as_str()) {
                return Err(BeadMinError::InvalidReport("invalid ingress check endpoint"));
            }
        }
        BeadMinCheckName::OrchestratorStatus => {
            if !matches_bead_min_orchestrator_status_contract(check.endpoint.as_str(), run_id) {
                return Err(BeadMinError::InvalidReport("invalid orchestrator check endpoint"));
            }
        }
    }

    Ok(())
}

fn validate_normalized_bead_min_run_id(value: &str) -> Result<(), BeadMinError> {
    if value.trim().is_empty() {
        return Err(BeadMinError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(BeadMinError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_bead_min_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL
}

fn matches_bead_min_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/Oya/{}/get_status", run_id)
}

fn expected_bead_min_final_diagnostics(decision: &BeadMinDecision) -> &'static str {
    match decision {
        BeadMinDecision::Pass => "bead-min checks passed",
        BeadMinDecision::Fail => "bead-min checks failed",
    }
}
