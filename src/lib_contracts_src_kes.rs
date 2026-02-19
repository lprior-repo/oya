const MAX_SRC_KES_SERVICE_NAME_LEN: usize = 64;
const MAX_SRC_KES_USER_NAME_LEN: usize = 128;
const MAX_SRC_KES_EMAIL_LEN: usize = 256;
const MAX_SRC_KES_USER_ID_LEN: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Input for building a `src-kes` service plan.
pub struct SrcKesInput {
    /// Logical service name used by the runtime contract.
    pub service_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Allowed HTTP methods for `src-kes` route contracts.
pub enum SrcKesRouteMethod {
    Post,
    Get,
    Put,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Immutable route contract entry for the `src-kes` CRUD surface.
pub struct SrcKesRouteContract {
    /// HTTP method used by the route.
    pub method: SrcKesRouteMethod,
    /// Path template exposed by the service.
    pub path: String,
    /// Successful status code expected from this route.
    pub success_status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Deterministic service plan for `src-kes` execution and validation.
pub struct SrcKesPlan {
    /// Normalized service name.
    pub service_name: String,
    /// Framework identifier required by contract (`scotty`).
    pub framework: String,
    /// Primary resource name required by contract (`user`).
    pub resource: String,
    /// Full CRUD route contract.
    pub routes: Vec<SrcKesRouteContract>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Runtime handle returned after successful server start checks.
pub struct SrcKesRuntimeHandle {
    /// Service name for the running handle.
    pub service_name: String,
    /// Framework used by the running handle.
    pub framework: String,
    /// Indicates whether runtime startup checks passed.
    pub running: bool,
}

/// Stable identifier used for `src-kes` users.
pub type UserId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Input payload for creating a user.
pub struct UserCreateRequest {
    /// Display name for the new user.
    pub name: String,
    /// User email address used for normalization and ID derivation.
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Input payload for updating an existing user.
pub struct UserUpdateRequest {
    /// Updated display name.
    pub name: String,
    /// Updated email address.
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Canonical user record stored in service state.
pub struct UserRecord {
    /// Normalized user ID.
    pub id: UserId,
    /// User display name.
    pub name: String,
    /// Normalized lowercase email address.
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// In-memory deterministic state for `src-kes` user CRUD operations.
pub struct SrcKesServiceState {
    /// User table keyed by normalized user ID.
    pub users: std::collections::BTreeMap<UserId, UserRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Validation stages used in `SrcKesReport`.
pub enum SrcKesStageName {
    PlanBuild,
    RuntimeStart,
    RouteContract,
    CrudContract,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage-level pass/fail status for `src-kes` verification.
pub enum SrcKesStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage report entry with diagnostics and timestamp.
pub struct SrcKesStageReport {
    /// Stage identifier.
    pub stage: SrcKesStageName,
    /// Stage status.
    pub status: SrcKesStageStatus,
    /// Human-readable diagnostics for this stage.
    pub diagnostics: String,
    /// Event timestamp for monotonic-order checks.
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final decision for `src-kes` report validation.
pub enum SrcKesDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// End-to-end execution report for `src-kes` observability runs.
pub struct SrcKesReport {
    /// Plan used for the run.
    pub plan: SrcKesPlan,
    /// Indicates runtime startup succeeded.
    pub runtime_started: bool,
    /// Indicates deterministic behavior constraints were met.
    pub deterministic_behavior: bool,
    /// Ordered stage reports.
    pub stages: Vec<SrcKesStageReport>,
    /// Derived final decision.
    pub decision: SrcKesDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Error type for `src-kes` planning, runtime checks, CRUD, and report validation.
pub enum SrcKesError {
    #[error("src-kes field is empty: {0}")]
    EmptyField(&'static str),
    #[error("src-kes field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("src-kes field contains invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("src-kes field has invalid format: {0}")]
    InvalidFieldFormat(&'static str),
    #[error("src-kes route contract invalid")]
    InvalidRouteContract,
    #[error("src-kes user already exists: {0}")]
    DuplicateUserId(String),
    #[error("src-kes user not found: {0}")]
    UserNotFound(String),
    #[error("src-kes report invalid: {0}")]
    InvalidReport(&'static str),
}

const SRC_KES_FRAMEWORK: &str = "scotty";
const SRC_KES_RESOURCE: &str = "user";

#[derive(Debug, Clone, PartialEq, Eq)]
struct SrcKesServiceName(String);

impl SrcKesServiceName {
    fn parse(value: &str) -> Result<Self, SrcKesError> {
        validate_src_kes_text_field(value, "service_name", MAX_SRC_KES_SERVICE_NAME_LEN).map(Self)
    }

    fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SrcKesDisplayName(String);

impl SrcKesDisplayName {
    fn parse(value: &str) -> Result<Self, SrcKesError> {
        validate_src_kes_text_field(value, "name", MAX_SRC_KES_USER_NAME_LEN).map(Self)
    }

    fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SrcKesEmail(String);

impl SrcKesEmail {
    fn parse(value: &str) -> Result<Self, SrcKesError> {
        normalize_src_kes_email(value).map(Self)
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SrcKesUserId(String);

impl SrcKesUserId {
    fn parse(value: &str) -> Result<Self, SrcKesError> {
        validate_src_kes_user_id(value).map(Self)
    }

    fn from_email(email: &SrcKesEmail) -> Result<Self, SrcKesError> {
        build_src_kes_user_id(email.as_str()).map(Self)
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn to_owned_string(&self) -> String {
        self.0.clone()
    }

    fn into_inner(self) -> String {
        self.0
    }
}

/// Build a normalized `src-kes` service plan and verify the route contract.
pub fn build_src_kes_plan(input: &SrcKesInput) -> Result<SrcKesPlan, SrcKesError> {
    let service_name = SrcKesServiceName::parse(input.service_name.as_str())?;
    let routes = register_user_routes();
    validate_src_kes_route_contract(routes.as_slice())?;

    Ok(SrcKesPlan {
        service_name: service_name.into_inner(),
        framework: SRC_KES_FRAMEWORK.to_string(),
        resource: SRC_KES_RESOURCE.to_string(),
        routes,
    })
}

/// Validate plan invariants and return a running `src-kes` runtime handle.
pub fn start_src_kes_server(plan: &SrcKesPlan) -> Result<SrcKesRuntimeHandle, SrcKesError> {
    if plan.framework != SRC_KES_FRAMEWORK {
        return Err(SrcKesError::InvalidFieldFormat("framework"));
    }
    if plan.resource != SRC_KES_RESOURCE {
        return Err(SrcKesError::InvalidFieldFormat("resource"));
    }
    SrcKesServiceName::parse(plan.service_name.as_str())?;
    validate_src_kes_route_contract(plan.routes.as_slice())?;

    Ok(SrcKesRuntimeHandle {
        service_name: plan.service_name.clone(),
        framework: plan.framework.clone(),
        running: true,
    })
}

/// Return the exact CRUD route contract required by `src-kes`.
pub fn register_user_routes() -> Vec<SrcKesRouteContract> {
    src_kes_route_contract_entries().to_vec()
}

/// Create a user and return updated immutable service state with the created record.
pub fn run_user_create(
    state: &SrcKesServiceState,
    request: &UserCreateRequest,
) -> Result<(SrcKesServiceState, UserRecord), SrcKesError> {
    let name = SrcKesDisplayName::parse(request.name.as_str())?;
    let email = SrcKesEmail::parse(request.email.as_str())?;
    let user_id = SrcKesUserId::from_email(&email)?;

    if state.users.contains_key(user_id.as_str()) {
        return Err(SrcKesError::DuplicateUserId(user_id.into_inner()));
    }

    let user_id_string = user_id.into_inner();
    let record = UserRecord {
        id: user_id_string.clone(),
        name: name.into_inner(),
        email: email.into_inner(),
    };
    let users = state
        .users
        .iter()
        .map(|(existing_id, existing_record)| (existing_id.clone(), existing_record.clone()))
        .chain(std::iter::once((user_id_string, record.clone())))
        .collect::<std::collections::BTreeMap<_, _>>();

    Ok((SrcKesServiceState { users }, record))
}

/// Read a user record by normalized user ID.
pub fn run_user_read(state: &SrcKesServiceState, user_id: &str) -> Result<UserRecord, SrcKesError> {
    let normalized_id = SrcKesUserId::parse(user_id)?;
    state
        .users
        .get(normalized_id.as_str())
        .cloned()
        .ok_or(SrcKesError::UserNotFound(normalized_id.to_owned_string()))
}

/// Update an existing user and return updated immutable state with the new record.
pub fn run_user_update(
    state: &SrcKesServiceState,
    user_id: &str,
    request: &UserUpdateRequest,
) -> Result<(SrcKesServiceState, UserRecord), SrcKesError> {
    let normalized_id = SrcKesUserId::parse(user_id)?;
    let existing = state
        .users
        .get(normalized_id.as_str())
        .cloned()
        .ok_or(SrcKesError::UserNotFound(normalized_id.to_owned_string()))?;

    let name = SrcKesDisplayName::parse(request.name.as_str())?;
    let email = SrcKesEmail::parse(request.email.as_str())?;
    let next_record =
        UserRecord { id: existing.id, name: name.into_inner(), email: email.into_inner() };

    let users = state
        .users
        .iter()
        .map(|(existing_id, existing_record)| {
            if existing_id == normalized_id.as_str() {
                (existing_id.clone(), next_record.clone())
            } else {
                (existing_id.clone(), existing_record.clone())
            }
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    Ok((SrcKesServiceState { users }, next_record))
}

/// Delete a user and return updated immutable service state.
pub fn run_user_delete(
    state: &SrcKesServiceState,
    user_id: &str,
) -> Result<SrcKesServiceState, SrcKesError> {
    let normalized_id = SrcKesUserId::parse(user_id)?;
    if !state.users.contains_key(normalized_id.as_str()) {
        return Err(SrcKesError::UserNotFound(normalized_id.to_owned_string()));
    }

    let users = state
        .users
        .iter()
        .filter(|(existing_id, _)| existing_id.as_str() != normalized_id.as_str())
        .map(|(existing_id, existing_record)| (existing_id.clone(), existing_record.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();

    Ok(SrcKesServiceState { users })
}

/// Validate a `src-kes` report against contract, stage order, and decision rules.
pub fn validate_src_kes_report(report: &SrcKesReport) -> Result<(), SrcKesError> {
    validate_src_kes_report_plan(report)?;
    validate_src_kes_report_stages(report)?;
    validate_src_kes_report_decision(report)
}

fn validate_src_kes_report_plan(report: &SrcKesReport) -> Result<(), SrcKesError> {
    validate_src_kes_route_contract(report.plan.routes.as_slice())?;
    if report.plan.framework != SRC_KES_FRAMEWORK {
        return Err(SrcKesError::InvalidReport("framework must be scotty"));
    }
    if report.plan.resource != SRC_KES_RESOURCE {
        return Err(SrcKesError::InvalidReport("resource must be user"));
    }
    if !report.runtime_started {
        return Err(SrcKesError::InvalidReport("runtime not started"));
    }
    if !report.deterministic_behavior {
        return Err(SrcKesError::InvalidReport("deterministic behavior violated"));
    }

    Ok(())
}

fn validate_src_kes_report_stages(report: &SrcKesReport) -> Result<(), SrcKesError> {
    let expected_stage_order = [
        SrcKesStageName::PlanBuild,
        SrcKesStageName::RuntimeStart,
        SrcKesStageName::RouteContract,
        SrcKesStageName::CrudContract,
        SrcKesStageName::FinalDecision,
    ];
    if report.stages.len() != expected_stage_order.len() {
        return Err(SrcKesError::InvalidReport("unexpected stage count"));
    }
    let valid_order = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !valid_order {
        return Err(SrcKesError::InvalidReport("invalid stage order"));
    }
    let has_empty_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_diagnostics {
        return Err(SrcKesError::InvalidReport("empty stage diagnostics"));
    }
    let has_non_monotonic_timestamps =
        report.stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if has_non_monotonic_timestamps {
        return Err(SrcKesError::InvalidReport("non-monotonic stage timestamps"));
    }

    Ok(())
}

fn validate_src_kes_report_decision(report: &SrcKesReport) -> Result<(), SrcKesError> {
    let has_failed_stage =
        report.stages.iter().any(|stage| stage.status == SrcKesStageStatus::Failed);
    let derived_decision =
        if has_failed_stage { SrcKesDecision::Fail } else { SrcKesDecision::Pass };
    if derived_decision != report.decision {
        return Err(SrcKesError::InvalidReport("decision mismatch"));
    }

    Ok(())
}

fn validate_src_kes_route_contract(routes: &[SrcKesRouteContract]) -> Result<(), SrcKesError> {
    let expected =
        src_kes_route_contract_entries().into_iter().collect::<std::collections::BTreeSet<_>>();

    let actual = routes.iter().cloned().collect::<std::collections::BTreeSet<_>>();

    if actual != expected {
        return Err(SrcKesError::InvalidRouteContract);
    }

    Ok(())
}

fn validate_src_kes_text_field(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<String, SrcKesError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SrcKesError::EmptyField(field));
    }
    if trimmed.len() > max_len {
        return Err(SrcKesError::FieldTooLong(field, max_len));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(SrcKesError::InvalidFieldContent(field));
    }
    Ok(trimmed.to_string())
}

fn normalize_src_kes_email(value: &str) -> Result<String, SrcKesError> {
    let lowered =
        validate_src_kes_text_field(value, "email", MAX_SRC_KES_EMAIL_LEN)?.to_ascii_lowercase();

    let valid_chars = lowered
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '@' | '.' | '_' | '-' | '+'));
    if !valid_chars {
        return Err(SrcKesError::InvalidFieldFormat("email"));
    }

    let segments = lowered.split('@').collect::<Vec<_>>();
    let local = if segments.is_empty() { "" } else { segments[0] };
    let domain = if segments.len() < 2 { "" } else { segments[1] };
    let no_extra_segments = segments.len() == 2;
    if local.is_empty()
        || domain.is_empty()
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || domain.contains("..")
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !no_extra_segments
    {
        return Err(SrcKesError::InvalidFieldFormat("email"));
    }

    let invalid_domain_label = domain
        .split('.')
        .any(|label| label.is_empty() || label.starts_with('-') || label.ends_with('-'));
    if invalid_domain_label {
        return Err(SrcKesError::InvalidFieldFormat("email"));
    }

    Ok(lowered)
}

fn build_src_kes_user_id(email: &str) -> Result<String, SrcKesError> {
    let normalized = email
        .chars()
        .map(|char| if char.is_ascii_alphanumeric() { char } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if normalized.is_empty() {
        return Err(SrcKesError::InvalidFieldFormat("user_id"));
    }

    let user_id = format!("user-{}", normalized);
    if user_id.len() > MAX_SRC_KES_USER_ID_LEN {
        return Err(SrcKesError::FieldTooLong("user_id", MAX_SRC_KES_USER_ID_LEN));
    }
    if !is_valid_src_kes_user_id(user_id.as_str()) {
        return Err(SrcKesError::InvalidFieldFormat("user_id"));
    }

    Ok(user_id)
}

fn validate_src_kes_user_id(value: &str) -> Result<String, SrcKesError> {
    let normalized = validate_src_kes_text_field(value, "user_id", MAX_SRC_KES_USER_ID_LEN)?;
    if !is_valid_src_kes_user_id(normalized.as_str()) {
        return Err(SrcKesError::InvalidFieldFormat("user_id"));
    }
    Ok(normalized)
}

fn is_valid_src_kes_user_id(value: &str) -> bool {
    value.chars().all(|char| char.is_ascii_alphanumeric() || char == '-')
}

fn src_kes_route_contract_entries() -> [SrcKesRouteContract; 4] {
    [
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Post,
            path: "/users".to_string(),
            success_status: 201,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Get,
            path: "/users/:id".to_string(),
            success_status: 200,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Put,
            path: "/users/:id".to_string(),
            success_status: 200,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Delete,
            path: "/users/:id".to_string(),
            success_status: 204,
        },
    ]
}
