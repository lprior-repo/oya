#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod commands;
mod repo;

use reqwest::Client;
use serde::Serialize;

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

pub async fn run_doctor_checks(ingress: &str, admin: &str, service_url: &str) -> DoctorReport {
    let checks = vec![
        check_http_ok("restate_ingress", &format!("{ingress}/restate/health"), "200").await,
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
        check_restate_services().await,
        check_restate_deployments().await,
        check_moon_tasks().await,
        check_repo_detection().await,
    ];
    let ok = checks.iter().all(|item| item.pass);
    DoctorReport { ok, checks }
}

pub fn print_doctor_jsonl(report: &DoctorReport) -> anyhow::Result<()> {
    for check in &report.checks {
        let payload = serde_json::json!({
            "type": "check",
            "id": check.id,
            "pass": check.pass,
            "expected": check.expected,
            "actual": check.actual,
            "remediation": check.remediation,
        });
        println!("{}", serde_json::to_string(&payload)?);
    }
    let failed = report
        .checks
        .iter()
        .filter(|item| !item.pass)
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let summary = serde_json::json!({
        "type": "summary",
        "ok": report.ok,
        "checks": report.checks.len(),
        "failed": failed,
    });
    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}

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

pub fn has_required_services(output: &str) -> bool {
    let tokens = output.lines().flat_map(|line| line.split_whitespace()).collect::<Vec<_>>();
    ["Oya", "OyaMemory", "OyaService"].iter().all(|name| tokens.iter().any(|token| token == name))
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
    match parse_host_port(endpoint_url, expected_port) {
        Ok((host, port)) => {
            let result = tokio::net::TcpStream::connect((host.as_str(), port)).await;
            let pass = result.is_ok();
            DoctorCheck {
                id: id.to_owned(),
                pass,
                expected: format!("tcp:{port} open ({endpoint_url})"),
                actual: if pass { "open".to_owned() } else { "closed".to_owned() },
                remediation: remediation.to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: id.to_owned(),
            pass: false,
            expected: format!("valid URL with port {expected_port}"),
            actual: error,
            remediation: remediation.to_owned(),
        },
    }
}

async fn check_restate_services() -> DoctorCheck {
    let result = run_command_outcome("restate", &["services", "list"], None).await;
    match result {
        Ok(output) => {
            let pass = output.success && has_required_services(&output.stdout);
            DoctorCheck {
                id: "restate_services".to_owned(),
                pass,
                expected: "Oya,OyaMemory,OyaService present".to_owned(),
                actual: output.stdout.trim().to_owned(),
                remediation: "run `oya init` to register handlers".to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: "restate_services".to_owned(),
            pass: false,
            expected: "restate services list succeeds".to_owned(),
            actual: error.to_string(),
            remediation: "install/verify `restate` CLI and runtime".to_owned(),
        },
    }
}

async fn check_restate_deployments() -> DoctorCheck {
    let result = run_command_outcome("restate", &["deployments", "list"], None).await;
    match result {
        Ok(output) => {
            let lines = output.stdout;
            let has_expected = lines.contains("http://127.0.0.1:9180/");
            let has_stale = lines.contains("http://oya:9180/")
                || lines.contains("http://127.0.0.1:8080/")
                || lines.contains("http://127.0.0.1:9090/");
            DoctorCheck {
                id: "restate_deployments".to_owned(),
                pass: output.success && has_expected && !has_stale,
                expected: "single active endpoint http://127.0.0.1:9180/".to_owned(),
                actual: lines.lines().take(4).collect::<Vec<_>>().join(" | "),
                remediation:
                    "remove stale endpoints with `restate deployments remove <id> --force -y`"
                        .to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: "restate_deployments".to_owned(),
            pass: false,
            expected: "restate deployments list succeeds".to_owned(),
            actual: error.to_string(),
            remediation: "ensure restate admin endpoint is healthy".to_owned(),
        },
    }
}

async fn check_moon_tasks() -> DoctorCheck {
    let result = run_command_outcome("moon", &["query", "tasks"], None).await;
    match result {
        Ok(output) => {
            let pass =
                output.success && ["quick", "ci", "test"].iter().all(|v| output.stdout.contains(v));
            DoctorCheck {
                id: "moon_tasks".to_owned(),
                pass,
                expected: "moon tasks include quick, ci, test".to_owned(),
                actual: output.stdout.lines().take(6).collect::<Vec<_>>().join(" | "),
                remediation: "define required moon tasks in .moon/tasks/all.yml".to_owned(),
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
            expected: "owner/repo slug via gh".to_owned(),
            actual: "not detected".to_owned(),
            remediation: "authenticate gh or pass --repo explicitly".to_owned(),
        },
        Err(error) => DoctorCheck {
            id: "repo_slug".to_owned(),
            pass: false,
            expected: "owner/repo slug via gh".to_owned(),
            actual: error.to_string(),
            remediation: "fix gh auth/config".to_owned(),
        },
    }
}
