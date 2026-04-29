#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

mod commands;
mod repo;

use reqwest::Client;
use serde::Serialize;
use std::path::{Path, PathBuf};

use repo::detect_repo_slug;

pub use commands::{run_command_capture, run_command_outcome};

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub id: String,
    pub pass: bool,
    pub expected: String,
    pub actual: String,
    pub remediation: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

struct DeploymentScan {
    has_expected: bool,
    has_stale: bool,
}

struct MoonTaskRequirement {
    label: &'static str,
    accepted_names: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenCodeEnvFlags {
    server_url: bool,
    server_user: bool,
    server_password: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenCodeMode {
    Server,
    IncompleteServer,
    Subprocess,
}

const OPENCODE_SERVER_URL_ENV: &str = "OPENCODE_SERVER_URL";
const OPENCODE_SERVER_USER_ENV: &str = "OPENCODE_SERVER_USER";
const OPENCODE_SERVER_PASSWORD_ENV: &str = "OPENCODE_SERVER_PASSWORD";
const DEFAULT_OPENCODE_SERVER_URL: &str = "http://localhost:4099";

const MOON_TASK_REQUIREMENTS: &[MoonTaskRequirement] = &[
    MoonTaskRequirement { label: "fmt", accepted_names: &["fmt"] },
    MoonTaskRequirement { label: "lint", accepted_names: &["clippy", "lint"] },
    MoonTaskRequirement { label: "test", accepted_names: &["test"] },
    MoonTaskRequirement { label: "build", accepted_names: &["build", "build-oya"] },
    MoonTaskRequirement { label: "ci", accepted_names: &["ci", "root-ci"] },
];

pub async fn run_doctor_checks(ingress: &str, admin: &str, service_url: &str) -> DoctorReport {
    let checks = vec![
        check_restate_ingress(ingress).await,
        check_tcp_open(
            "restate_admin",
            admin,
            9070,
            "ensure Restate admin is running on configured host/port",
        )
        .await,
        check_tcp_open(
            "oya_service",
            service_url,
            9180,
            "ensure oya.service is running and bound to configured host/port",
        )
        .await,
        check_restate_services(admin).await,
        check_restate_deployments(admin).await,
        check_moon_tasks().await,
        check_fjall_data_dir(),
        check_opencode_config().await,
        check_repo_detection().await,
    ];
    let ok = checks.iter().all(|item| item.pass);
    DoctorReport { ok, checks }
}

/// # Errors
///
/// Returns an error when JSON serialization fails for emitted JSONL records.
pub fn print_doctor_jsonl(report: &DoctorReport) -> anyhow::Result<()> {
    doctor_jsonl_lines(report)?.iter().for_each(|line| println!("{line}"));
    Ok(())
}

fn doctor_jsonl_lines(report: &DoctorReport) -> anyhow::Result<Vec<String>> {
    let mut lines =
        report.checks.iter().map(doctor_check_jsonl_line).collect::<anyhow::Result<Vec<_>>>()?;
    lines.push(doctor_summary_jsonl_line(report)?);
    Ok(lines)
}

fn doctor_check_jsonl_line(check: &DoctorCheck) -> anyhow::Result<String> {
    let payload = serde_json::json!({
        "type": "check",
        "id": check.id,
        "name": doctor_check_name(&check.id),
        "category": doctor_check_category(&check.id),
        "status": doctor_check_status(check.pass),
        "pass": check.pass,
        "message": doctor_check_message(&check.id, check.pass, &check.remediation),
        "expected": check.expected,
        "actual": check.actual,
        "remediation": check.remediation,
    });
    serde_json::to_string(&payload).map_err(Into::into)
}

fn doctor_summary_jsonl_line(report: &DoctorReport) -> anyhow::Result<String> {
    let payload = serde_json::json!({
        "type": "summary",
        "ok": report.ok,
        "checks": report.checks.len(),
        "failed": failed_check_ids(report),
    });
    serde_json::to_string(&payload).map_err(Into::into)
}

fn doctor_check_name(id: &str) -> &'static str {
    match id {
        "restate_ingress" => "Restate ingress",
        "restate_admin" => "Restate admin",
        "oya_service" => "Oya service",
        "restate_services" => "Restate services",
        "restate_deployments" => "Restate deployments",
        "moon_tasks" => "Moon tasks",
        "fjall_data_dir" => "Fjall data dir",
        "opencode_config" => "OpenCode configuration",
        "repo_slug" => "Repository slug",
        _ => "Unknown check",
    }
}

fn doctor_check_category(id: &str) -> &'static str {
    match id {
        "restate_ingress" | "restate_admin" | "restate_services" | "restate_deployments" => {
            "restate"
        }
        "oya_service" => "oya",
        "moon_tasks" => "build",
        "fjall_data_dir" => "storage",
        "opencode_config" => "opencode",
        "repo_slug" => "repository",
        _ => "unknown",
    }
}

fn doctor_check_status(pass: bool) -> &'static str {
    if pass {
        "pass"
    } else {
        "fail"
    }
}

fn doctor_check_message(id: &str, pass: bool, remediation: &str) -> String {
    let name = doctor_check_name(id);
    if pass {
        format!("{name} is healthy")
    } else {
        format!("{name} failed: {remediation}")
    }
}

fn failed_check_ids(report: &DoctorReport) -> Vec<String> {
    report.checks.iter().filter(|item| !item.pass).map(|item| item.id.clone()).collect()
}

/// # Errors
///
/// Returns an error string when the endpoint URL is invalid or uses the wrong port.
pub fn parse_host_port(endpoint_url: &str, expected_port: u16) -> Result<(String, u16), String> {
    let parsed = url::Url::parse(endpoint_url).map_err(|error| error.to_string())?;
    let host = parsed.host_str().ok_or_else(|| "URL missing host".to_owned())?.to_owned();
    let port = parsed.port_or_known_default().ok_or_else(|| "URL missing port".to_owned())?;
    if port == expected_port {
        Ok((host, port))
    } else {
        Err(format!("expected port {expected_port}, found {port}"))
    }
}

#[must_use]
pub fn has_required_services(output: &str) -> bool {
    let tokens = output.lines().flat_map(|line| line.split_whitespace()).collect::<Vec<_>>();
    ["Oya", "OyaMemory", "OyaService"].iter().all(|name| tokens.iter().any(|token| token == name))
}

fn sample_lines(output: &str, max_lines: usize) -> String {
    output.lines().take(max_lines).collect::<Vec<_>>().join(" | ")
}

fn scan_deployments(output: &str) -> DeploymentScan {
    let has_expected = output.contains("http://127.0.0.1:9180/");
    let has_stale = ["http://oya:9180/", "http://127.0.0.1:8080/", "http://127.0.0.1:9090/"]
        .iter()
        .any(|endpoint| output.contains(endpoint));
    DeploymentScan { has_expected, has_stale }
}

fn admin_deployment_service_names(body: &str) -> Vec<String> {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(deployments) = payload.get("deployments").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    deployments
        .iter()
        .filter_map(|deployment| deployment.get("services").and_then(serde_json::Value::as_array))
        .flat_map(|services| services.iter())
        .filter_map(|service| service.get("name").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn has_required_service_names(names: &[String]) -> bool {
    ["Oya", "OyaMemory", "OyaService"]
        .iter()
        .all(|required| names.iter().any(|name| name == required))
}

fn has_required_moon_tasks(output: &str) -> bool {
    missing_required_moon_task_labels(output).is_empty()
}

fn missing_required_moon_task_labels(output: &str) -> Vec<&'static str> {
    MOON_TASK_REQUIREMENTS
        .iter()
        .filter(|requirement| !has_moon_task_requirement(output, requirement))
        .map(|requirement| requirement.label)
        .collect()
}

fn has_moon_task_requirement(output: &str, requirement: &MoonTaskRequirement) -> bool {
    requirement.accepted_names.iter().any(|task| moon_output_has_task(output, task))
}

fn moon_output_has_task(output: &str, task: &str) -> bool {
    let json_key = format!("\"{task}\"");
    output.contains(&json_key) || output.lines().any(|line| line_has_task_token(line, task))
}

fn line_has_task_token(line: &str, task: &str) -> bool {
    let project_task = format!(":{task}");
    line.split_whitespace().any(|token| token == task || token.ends_with(&project_task))
}

fn expected_moon_task_labels() -> String {
    MOON_TASK_REQUIREMENTS
        .iter()
        .map(|requirement| requirement.label)
        .collect::<Vec<_>>()
        .join(", ")
}

fn moon_task_actual(output: &str, missing: &[&str]) -> String {
    if missing.is_empty() {
        format!("found required task groups: {}", expected_moon_task_labels())
    } else {
        format!("missing task groups: {}; sample: {}", missing.join(", "), sample_lines(output, 6))
    }
}

async fn check_restate_ingress(ingress: &str) -> DoctorCheck {
    match canonical_host_port(
        "restate_ingress",
        ingress,
        8080,
        "use rootless Restate ingress http://127.0.0.1:8080 or run `oya init`",
    ) {
        Ok(_) => {
            let health_url = format!("{}/restate/health", ingress.trim_end_matches('/'));
            check_http_ok(
                "restate_ingress",
                &health_url,
                "HTTP 200 from rootless ingress port 8080",
            )
            .await
        }
        Err(check) => check,
    }
}

async fn check_http_ok(id: &str, url: &str, expected: &str) -> DoctorCheck {
    let result = Client::new().get(url).send().await;
    match result {
        Ok(response) => DoctorCheck {
            id: id.to_owned(),
            pass: response.status().is_success(),
            expected: expected.to_owned(),
            actual: response.status().to_string(),
            remediation: format!("verify service bound correctly for {url}"),
        },
        Err(error) => DoctorCheck {
            id: id.to_owned(),
            pass: false,
            expected: expected.to_owned(),
            actual: error.to_string(),
            remediation: format!("start runtime with `oya init` and recheck {url}"),
        },
    }
}

async fn check_tcp_open(
    id: &str,
    endpoint_url: &str,
    expected_port: u16,
    remediation: &str,
) -> DoctorCheck {
    match canonical_host_port(id, endpoint_url, expected_port, remediation) {
        Ok((host, port)) => {
            let result = tokio::net::TcpStream::connect((host.as_str(), port)).await;
            let pass = result.is_ok();
            DoctorCheck {
                id: id.to_owned(),
                pass,
                expected: format!("tcp:{port} open ({endpoint_url})"),
                actual: if pass {
                    format!("open on canonical port {port}")
                } else {
                    format!("closed on canonical port {port}")
                },
                remediation: remediation.to_owned(),
            }
        }
        Err(check) => check,
    }
}

fn canonical_host_port(
    id: &str,
    endpoint_url: &str,
    expected_port: u16,
    remediation: &str,
) -> Result<(String, u16), DoctorCheck> {
    parse_host_port(endpoint_url, expected_port).map_err(|error| DoctorCheck {
        id: id.to_owned(),
        pass: false,
        expected: format!("canonical URL with port {expected_port}"),
        actual: error,
        remediation: remediation.to_owned(),
    })
}

async fn check_restate_services(admin: &str) -> DoctorCheck {
    let result = run_command_outcome("restate", &["services", "list"], None).await;
    match result {
        Ok(output) if output.success && has_required_services(&output.stdout) => {
            let pass = output.success && has_required_services(&output.stdout);
            DoctorCheck {
                id: "restate_services".to_owned(),
                pass,
                expected: "Oya,OyaMemory,OyaService present".to_owned(),
                actual: output.stdout.trim().to_owned(),
                remediation: "run `oya init` to register handlers".to_owned(),
            }
        }
        Ok(output) => check_restate_services_admin(admin, &sample_lines(&output.stdout, 4)).await,
        Err(error) => check_restate_services_admin(admin, &error.to_string()).await,
    }
}

async fn check_restate_deployments(admin: &str) -> DoctorCheck {
    let result = run_command_outcome("restate", &["deployments", "list"], None).await;
    match result {
        Ok(output) if output.success && deployment_scan_passes(&output.stdout) => {
            let scan = scan_deployments(&output.stdout);
            DoctorCheck {
                id: "restate_deployments".to_owned(),
                pass: output.success && scan.has_expected && !scan.has_stale,
                expected: "single active endpoint http://127.0.0.1:9180/".to_owned(),
                actual: sample_lines(&output.stdout, 4),
                remediation:
                    "remove stale endpoints with `restate deployments remove <id> --force -y`"
                        .to_owned(),
            }
        }
        Ok(output) => {
            check_restate_deployments_admin(admin, &sample_lines(&output.stdout, 4)).await
        }
        Err(error) => check_restate_deployments_admin(admin, &error.to_string()).await,
    }
}

fn deployment_scan_passes(output: &str) -> bool {
    let scan = scan_deployments(output);
    scan.has_expected && !scan.has_stale
}

async fn fetch_admin_deployments(admin: &str) -> Result<String, String> {
    let url = format!("{}/deployments", admin.trim_end_matches('/'));
    let response = Client::new().get(&url).send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    if status.is_success() {
        Ok(body)
    } else {
        Err(format!("{status}: {}", sample_lines(&body, 4)))
    }
}

async fn check_restate_services_admin(admin: &str, cli_detail: &str) -> DoctorCheck {
    match fetch_admin_deployments(admin).await {
        Ok(body) => admin_services_check(&body, cli_detail),
        Err(error) => DoctorCheck {
            id: "restate_services".to_owned(),
            pass: false,
            expected: "Oya,OyaMemory,OyaService present".to_owned(),
            actual: format!("CLI unavailable: {cli_detail}; admin API failed: {error}"),
            remediation: "run `oya init` to register handlers".to_owned(),
        },
    }
}

fn admin_services_check(body: &str, cli_detail: &str) -> DoctorCheck {
    let names = admin_deployment_service_names(body);
    DoctorCheck {
        id: "restate_services".to_owned(),
        pass: has_required_service_names(&names),
        expected: "Oya,OyaMemory,OyaService present".to_owned(),
        actual: format!("admin API services: {}; CLI detail: {cli_detail}", names.join(",")),
        remediation: "run `oya init` to register handlers".to_owned(),
    }
}

async fn check_restate_deployments_admin(admin: &str, cli_detail: &str) -> DoctorCheck {
    match fetch_admin_deployments(admin).await {
        Ok(body) => admin_deployments_check(&body, cli_detail),
        Err(error) => DoctorCheck {
            id: "restate_deployments".to_owned(),
            pass: false,
            expected: "single active endpoint http://127.0.0.1:9180/".to_owned(),
            actual: format!("CLI unavailable: {cli_detail}; admin API failed: {error}"),
            remediation: "ensure restate admin endpoint is healthy".to_owned(),
        },
    }
}

fn admin_deployments_check(body: &str, cli_detail: &str) -> DoctorCheck {
    let scan = scan_deployments(body);
    DoctorCheck {
        id: "restate_deployments".to_owned(),
        pass: scan.has_expected && !scan.has_stale,
        expected: "single active endpoint http://127.0.0.1:9180/".to_owned(),
        actual: format!("admin API: {}; CLI detail: {cli_detail}", sample_lines(body, 4)),
        remediation: "remove stale deployments through Restate Admin API".to_owned(),
    }
}

async fn check_moon_tasks() -> DoctorCheck {
    let result = run_command_outcome("moon", &["query", "tasks"], None).await;
    match result {
        Ok(output) => {
            let missing = missing_required_moon_task_labels(&output.stdout);
            let pass = output.success && has_required_moon_tasks(&output.stdout);
            DoctorCheck {
                id: "moon_tasks".to_owned(),
                pass,
                expected: format!("moon tasks include {}", expected_moon_task_labels()),
                actual: if output.success {
                    moon_task_actual(&output.stdout, &missing)
                } else {
                    format!("moon query tasks failed: {}", sample_lines(&output.stderr, 4))
                },
                remediation:
                    "define required moon tasks in .moon/tasks/all.yml or project moon.yml"
                        .to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: "moon_tasks".to_owned(),
            pass: false,
            expected: "moon query tasks succeeds".to_owned(),
            actual: error.to_string(),
            remediation: "install moon and run from repo root".to_owned(),
        },
    }
}

fn check_fjall_data_dir() -> DoctorCheck {
    fjall_data_dir_check(&resolved_fjall_data_dir())
}

fn resolved_fjall_data_dir() -> PathBuf {
    std::env::var("OYA_DATA_DIR").map_or_else(|_| PathBuf::from(".oya-lite"), PathBuf::from)
}

fn fjall_data_dir_check(path: &Path) -> DoctorCheck {
    let expected = "existing Fjall state database openable without writes".to_owned();
    let remediation = "run `oya init` or set OYA_DATA_DIR to the active Fjall state directory";
    match validate_fjall_data_dir(path) {
        Ok(actual) => DoctorCheck {
            id: "fjall_data_dir".to_owned(),
            pass: true,
            expected,
            actual,
            remediation: "none".to_owned(),
        },
        Err(actual) => DoctorCheck {
            id: "fjall_data_dir".to_owned(),
            pass: false,
            expected,
            actual,
            remediation: remediation.to_owned(),
        },
    }
}

fn validate_fjall_data_dir(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("missing Fjall data dir: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Fjall data path is not a directory: {}", path.display()));
    }
    if !looks_like_fjall_store(path) {
        return Err(format!("not an initialized Fjall state database: {}", path.display()));
    }
    open_fjall_data_dir(path)
}

fn looks_like_fjall_store(path: &Path) -> bool {
    path.join("version").is_file() && path.join("keyspaces").is_dir()
}

fn open_fjall_data_dir(path: &Path) -> Result<String, String> {
    match crate::lifecycle::state::StateDb::open(path) {
        Ok(db) => {
            drop(db);
            Ok(format!("opened existing Fjall state database: {}", path.display()))
        }
        Err(error) if is_fjall_lock_error(&error.to_string()) => {
            Ok(format!("Fjall data dir is locked by an active process: {}", path.display()))
        }
        Err(error) => Err(format!("failed to open Fjall state database: {error}")),
    }
}

fn is_fjall_lock_error(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("lock") || lowered.contains("already in use")
}

async fn check_opencode_config() -> DoctorCheck {
    let flags = opencode_env_flags_from_process();
    match opencode_mode(flags) {
        OpenCodeMode::Server => opencode_server_check(),
        OpenCodeMode::IncompleteServer => opencode_incomplete_server_check(flags),
        OpenCodeMode::Subprocess => opencode_subprocess_check().await,
    }
}

fn opencode_env_flags_from_process() -> OpenCodeEnvFlags {
    OpenCodeEnvFlags {
        server_url: std::env::var_os(OPENCODE_SERVER_URL_ENV).is_some(),
        server_user: std::env::var_os(OPENCODE_SERVER_USER_ENV).is_some(),
        server_password: std::env::var_os(OPENCODE_SERVER_PASSWORD_ENV).is_some(),
    }
}

fn opencode_mode(flags: OpenCodeEnvFlags) -> OpenCodeMode {
    if flags.server_password {
        OpenCodeMode::Server
    } else if flags.server_url || flags.server_user {
        OpenCodeMode::IncompleteServer
    } else {
        OpenCodeMode::Subprocess
    }
}

fn opencode_server_check() -> DoctorCheck {
    match sanitized_opencode_server_url_from_env() {
        Ok(url) => opencode_config_check(
            true,
            "server mode has URL, username, and password configured",
            &format!("server mode configured; endpoint {url}; credentials present"),
            "none",
        ),
        Err(actual) => opencode_config_check(
            false,
            "valid OpenCode server URL with credentials",
            &actual,
            "fix OPENCODE_SERVER_URL or unset it to use the default OpenCode server URL",
        ),
    }
}

fn opencode_incomplete_server_check(flags: OpenCodeEnvFlags) -> DoctorCheck {
    opencode_config_check(
        false,
        "server mode has OPENCODE_SERVER_PASSWORD or subprocess mode has opencode CLI",
        &format!(
            "server mode requested without credentials; missing {}; hints present: {}",
            missing_opencode_server_credentials(flags).join(", "),
            opencode_server_hints(flags).join(", ")
        ),
        "set OPENCODE_SERVER_PASSWORD or unset OpenCode server environment variables to use subprocess mode",
    )
}

async fn opencode_subprocess_check() -> DoctorCheck {
    match run_command_outcome("opencode", &["--version"], None).await {
        Ok(output) => opencode_config_check(
            output.success,
            "subprocess mode has opencode CLI available",
            &opencode_subprocess_actual(&output),
            "install opencode or configure OPENCODE_SERVER_PASSWORD for server mode",
        ),
        Err(error) => opencode_config_check(
            false,
            "subprocess mode has opencode CLI available",
            &format!(
                "subprocess mode unavailable: {}",
                sanitize_opencode_detail(&error.to_string())
            ),
            "install opencode or configure OPENCODE_SERVER_PASSWORD for server mode",
        ),
    }
}

fn opencode_config_check(
    pass: bool,
    expected: &str,
    actual: &str,
    remediation: &str,
) -> DoctorCheck {
    DoctorCheck {
        id: "opencode_config".to_owned(),
        pass,
        expected: expected.to_owned(),
        actual: actual.to_owned(),
        remediation: remediation.to_owned(),
    }
}

fn sanitized_opencode_server_url_from_env() -> Result<String, String> {
    let raw = std::env::var(OPENCODE_SERVER_URL_ENV)
        .unwrap_or_else(|_| DEFAULT_OPENCODE_SERVER_URL.to_owned());
    sanitize_opencode_server_url(&raw)
}

fn sanitize_opencode_server_url(raw: &str) -> Result<String, String> {
    let parsed = url::Url::parse(raw)
        .map_err(|_| "server mode configured but OPENCODE_SERVER_URL is invalid".to_owned())?;
    let Some(host) = parsed.host_str() else {
        return Err("server mode configured but OPENCODE_SERVER_URL has no host".to_owned());
    };
    let port = parsed.port().map_or_else(String::new, |value| format!(":{value}"));
    Ok(format!("{}://{}{}{}", parsed.scheme(), host, port, parsed.path()))
}

fn missing_opencode_server_credentials(flags: OpenCodeEnvFlags) -> Vec<&'static str> {
    if flags.server_password {
        Vec::new()
    } else {
        vec![OPENCODE_SERVER_PASSWORD_ENV]
    }
}

fn opencode_server_hints(flags: OpenCodeEnvFlags) -> Vec<&'static str> {
    [(flags.server_url, OPENCODE_SERVER_URL_ENV), (flags.server_user, OPENCODE_SERVER_USER_ENV)]
        .into_iter()
        .filter_map(|(present, name)| present.then_some(name))
        .collect()
}

fn opencode_subprocess_actual(output: &commands::CommandOutcome) -> String {
    let detail = if output.stdout.trim().is_empty() { &output.stderr } else { &output.stdout };
    if output.success {
        format!(
            "subprocess mode configured; opencode CLI available: {}",
            sanitize_opencode_detail(detail)
        )
    } else {
        format!(
            "subprocess mode configured but opencode --version failed: {}",
            sanitize_opencode_detail(detail)
        )
    }
}

fn sanitize_opencode_detail(raw: &str) -> String {
    let sanitized = raw.lines().take(3).map(sanitize_opencode_line).collect::<Vec<_>>().join(" | ");
    if sanitized.trim().is_empty() {
        "no output".to_owned()
    } else {
        sanitized
    }
}

fn sanitize_opencode_line(line: &str) -> String {
    let lowered = line.to_ascii_lowercase();
    if ["secret", "token", "apikey", "api_key", "bearer"]
        .iter()
        .any(|needle| lowered.contains(needle))
    {
        "[redacted sensitive OpenCode detail]".to_owned()
    } else {
        line.chars().take(200).collect()
    }
}

async fn check_repo_detection() -> DoctorCheck {
    match detect_repo_slug().await {
        Ok(Some(slug)) => DoctorCheck {
            id: "repo_slug".to_owned(),
            pass: true,
            expected: "owner/repo slug".to_owned(),
            actual: slug,
            remediation: "none".to_owned(),
        },
        Ok(None) => DoctorCheck {
            id: "repo_slug".to_owned(),
            pass: false,
            expected: "owner/repo slug via git origin".to_owned(),
            actual: "not detected".to_owned(),
            remediation: "configure git origin remote or pass --repo explicitly".to_owned(),
        },
        Err(error) => DoctorCheck {
            id: "repo_slug".to_owned(),
            pass: false,
            expected: "owner/repo slug via git origin".to_owned(),
            actual: error.to_string(),
            remediation: "fix git origin remote URL".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_jsonl_lines_emit_stable_check_schema() {
        let report = DoctorReport {
            ok: false,
            checks: vec![passing_doctor_check(), failing_doctor_check()],
        };

        let Ok(lines) = doctor_jsonl_lines(&report) else {
            assert!(false, "doctor JSONL lines should serialize");
            return;
        };
        assert_eq!(lines.len(), 3);

        assert_check_line(&lines[0], "Restate ingress", "restate", "pass");
        assert_check_line(&lines[1], "Moon tasks", "build", "fail");
        assert_summary_line(&lines[2]);
    }

    #[test]
    fn admin_deployment_service_names_reads_restate_json() {
        let names = admin_deployment_service_names(
            r#"{
              "deployments": [{
                "uri": "http://127.0.0.1:9180/",
                "services": [
                  {"name": "Oya"},
                  {"name": "OyaMemory"},
                  {"name": "OyaService"}
                ]
              }]
            }"#,
        );
        assert!(has_required_service_names(&names));
    }

    #[test]
    fn admin_deployment_service_names_rejects_invalid_json() {
        let names = admin_deployment_service_names("not json");
        assert!(!has_required_service_names(&names));
    }

    #[test]
    fn canonical_host_port_accepts_oya_ports() {
        let ingress = canonical_host_port("restate_ingress", "http://127.0.0.1:8080", 8080, "fix");
        let admin = canonical_host_port("restate_admin", "http://127.0.0.1:9070", 9070, "fix");
        let service = canonical_host_port("oya_service", "http://127.0.0.1:9180/", 9180, "fix");

        assert!(ingress.is_ok());
        assert!(admin.is_ok());
        assert!(service.is_ok());
    }

    #[test]
    fn canonical_host_port_rejects_noncanonical_port_before_network_check() {
        let Err(check) = canonical_host_port(
            "restate_ingress",
            "http://127.0.0.1:909",
            8080,
            "use canonical ingress",
        ) else {
            assert!(false, "noncanonical ingress port should fail before network checks");
            return;
        };

        assert!(!check.pass);
        assert_eq!(check.id, "restate_ingress");
        assert_eq!(check.expected, "canonical URL with port 8080");
        assert_eq!(check.actual, "expected port 8080, found 909");
    }

    #[test]
    fn canonical_host_port_rejects_invalid_endpoint_url() {
        let Err(check) = canonical_host_port("oya_service", "not a url", 9180, "fix service")
        else {
            assert!(false, "invalid endpoint URL should fail before network checks");
            return;
        };

        assert!(!check.pass);
        assert_eq!(check.id, "oya_service");
        assert!(check.actual.contains("relative URL without a base"));
    }

    #[test]
    fn moon_task_check_accepts_canonical_groups_with_aliases() {
        let output = r#"{
          "tasks": {
            "oya": {
              "fmt": {},
              "clippy": {},
              "test": {},
              "build-oya": {},
              "root-ci": {}
            }
          }
        }"#;
        assert!(has_required_moon_tasks(output));
        assert!(missing_required_moon_task_labels(output).is_empty());
    }

    #[test]
    fn moon_task_check_reports_missing_canonical_groups() {
        let output = r#"{"tasks":{"oya":{"fmt":{},"test":{},"build":{}}}}"#;
        assert_eq!(missing_required_moon_task_labels(output), vec!["lint", "ci"]);
        assert!(!has_required_moon_tasks(output));
    }

    #[test]
    fn fjall_data_dir_check_opens_existing_state_db() {
        let Ok(temp) = tempfile::tempdir() else {
            assert!(false, "temp dir should be available");
            return;
        };
        let path = temp.path().join("state");
        let Ok(db) = crate::lifecycle::state::StateDb::open(&path) else {
            assert!(false, "test Fjall state DB should initialize");
            return;
        };
        drop(db);

        let check = fjall_data_dir_check(&path);

        assert!(check.pass);
        assert_eq!(check.id, "fjall_data_dir");
        assert!(check.actual.contains("opened existing Fjall state database"));
    }

    #[test]
    fn fjall_data_dir_check_does_not_create_missing_store() {
        let Ok(temp) = tempfile::tempdir() else {
            assert!(false, "temp dir should be available");
            return;
        };
        let path = temp.path().join("missing-state");

        let check = fjall_data_dir_check(&path);

        assert!(!check.pass);
        assert!(check.actual.contains("missing Fjall data dir"));
        assert!(!path.exists(), "doctor must not create a missing Fjall store");
    }

    #[test]
    fn fjall_data_dir_check_rejects_non_fjall_directory() {
        let Ok(temp) = tempfile::tempdir() else {
            assert!(false, "temp dir should be available");
            return;
        };

        let check = fjall_data_dir_check(temp.path());

        assert!(!check.pass);
        assert!(check.actual.contains("not an initialized Fjall state database"));
    }

    #[test]
    fn opencode_mode_uses_server_when_password_is_present() {
        let flags =
            OpenCodeEnvFlags { server_url: false, server_user: false, server_password: true };

        assert_eq!(opencode_mode(flags), OpenCodeMode::Server);
    }

    #[test]
    fn opencode_mode_reports_incomplete_server_without_password() {
        let flags =
            OpenCodeEnvFlags { server_url: true, server_user: true, server_password: false };

        assert_eq!(opencode_mode(flags), OpenCodeMode::IncompleteServer);
        assert_eq!(missing_opencode_server_credentials(flags), vec![OPENCODE_SERVER_PASSWORD_ENV]);
    }

    #[test]
    fn opencode_incomplete_server_check_does_not_leak_env_values() {
        let flags =
            OpenCodeEnvFlags { server_url: true, server_user: true, server_password: false };

        let check = opencode_incomplete_server_check(flags);

        assert!(!check.pass);
        assert_eq!(check.id, "opencode_config");
        assert!(check.actual.contains(OPENCODE_SERVER_PASSWORD_ENV));
        assert!(!check.actual.contains("secret-value"));
    }

    #[test]
    fn opencode_server_url_sanitizer_removes_userinfo_query_and_fragment() {
        let Ok(sanitized) = sanitize_opencode_server_url(
            "http://user:secret@localhost:4099/api?token=secret#fragment",
        ) else {
            assert!(false, "server URL should sanitize");
            return;
        };

        assert_eq!(sanitized, "http://localhost:4099/api");
        assert!(!sanitized.contains("secret"));
        assert!(!sanitized.contains("token"));
        assert!(!sanitized.contains("user"));
    }

    #[test]
    fn opencode_detail_sanitizer_redacts_sensitive_lines() {
        let detail = sanitize_opencode_detail("version 1.2\nAuthorization: Bearer secret-token");

        assert!(detail.contains("version 1.2"));
        assert!(detail.contains("[redacted sensitive OpenCode detail]"));
        assert!(!detail.contains("secret-token"));
    }

    fn passing_doctor_check() -> DoctorCheck {
        DoctorCheck {
            id: "restate_ingress".to_owned(),
            pass: true,
            expected: "200".to_owned(),
            actual: "200 OK".to_owned(),
            remediation: "verify runtime".to_owned(),
        }
    }

    fn failing_doctor_check() -> DoctorCheck {
        DoctorCheck {
            id: "moon_tasks".to_owned(),
            pass: false,
            expected: "moon tasks include fmt, lint, test, build, ci".to_owned(),
            actual: "missing ci".to_owned(),
            remediation: "define required moon tasks".to_owned(),
        }
    }

    fn assert_check_line(line: &str, name: &str, category: &str, status: &str) {
        let Some(value) = parse_jsonl_line(line) else {
            return;
        };
        assert_eq!(value.get("type").and_then(serde_json::Value::as_str), Some("check"));
        assert_eq!(value.get("name").and_then(serde_json::Value::as_str), Some(name));
        assert_eq!(value.get("category").and_then(serde_json::Value::as_str), Some(category));
        assert_eq!(value.get("status").and_then(serde_json::Value::as_str), Some(status));
        assert!(value.get("message").and_then(serde_json::Value::as_str).is_some());
        assert!(value.get("expected").and_then(serde_json::Value::as_str).is_some());
        assert!(value.get("actual").and_then(serde_json::Value::as_str).is_some());
        assert!(value.get("remediation").and_then(serde_json::Value::as_str).is_some());
    }

    fn assert_summary_line(line: &str) {
        let Some(value) = parse_jsonl_line(line) else {
            return;
        };
        assert_eq!(value.get("type").and_then(serde_json::Value::as_str), Some("summary"));
        assert_eq!(value.get("ok").and_then(serde_json::Value::as_bool), Some(false));
        assert_eq!(value.get("checks").and_then(serde_json::Value::as_u64), Some(2));
        let failed = value.get("failed").and_then(serde_json::Value::as_array);
        assert_eq!(failed.map(Vec::len), Some(1));
    }

    fn parse_jsonl_line(line: &str) -> Option<serde_json::Value> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            assert!(false, "doctor JSONL line should parse: {line}");
            return None;
        };
        Some(value)
    }
}
