#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::doctor::{run_command_capture, run_command_outcome};
use reqwest::Client;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

const OYA_SERVICE_RESTART_ATTEMPTS: u8 = 4;
const OYA_SERVICE_RESTART_DELAY: Duration = Duration::from_millis(400);

pub async fn init_command(ingress: &str, service_url: &str, down: bool) -> anyhow::Result<()> {
    disable_systemd_restate().await?;
    if down {
        stop_docker_restate().await?;
        println!("[oya] Docker Restate stopped");
        return Ok(());
    }
    let repo_root = find_repo_root()?;
    let admin = admin_url_from_ingress(ingress)?;
    start_fresh_docker_restate(&repo_root).await?;
    ensure_oya_service_running(&repo_root).await?;
    wait_for_health(ingress, 30).await?;
    wait_for_tcp_port("127.0.0.1", 9180, 30).await?;
    register_services(service_url, &admin).await?;
    validate_registered_services(&admin).await?;
    println!("[oya] Runtime ready (fresh Restate + handlers registered)");
    println!("  Admin:   http://127.0.0.1:9070");
    println!("  Ingress: {ingress}");
    println!("  Service: {service_url}");
    Ok(())
}

pub(crate) fn admin_url_from_ingress(ingress: &str) -> anyhow::Result<String> {
    let mut parsed = url::Url::parse(ingress)?;
    parsed
        .set_port(Some(9070))
        .map_err(|()| anyhow::anyhow!("ingress URL cannot be mapped to admin port: {ingress}"))?;
    Ok(parsed.as_str().trim_end_matches('/').to_owned())
}

pub fn find_repo_root() -> anyhow::Result<PathBuf> {
    let current = std::env::current_dir()?;
    let root = current
        .ancestors()
        .find(|path| path.join("docker-compose.yml").is_file())
        .map(Path::to_path_buf);
    root.ok_or_else(|| anyhow::anyhow!("could not find docker-compose.yml from current directory"))
}

async fn disable_systemd_restate() -> anyhow::Result<()> {
    let _ =
        run_command_capture("systemctl", &["--user", "disable", "--now", "restate.service"], None)
            .await;
    let _ =
        run_command_capture("systemctl", &["--user", "stop", "restate-manual.service"], None).await;
    Ok(())
}

async fn stop_docker_restate() -> anyhow::Result<()> {
    if let Ok(repo_root) = find_repo_root() {
        return run_command_capture(
            "docker",
            &["compose", "-f", "docker-compose.yml", "stop", "restate"],
            Some(repo_root.as_path()),
        )
        .await
        .map(|_| ());
    }
    stop_named_restate_container().await
}

async fn stop_named_restate_container() -> anyhow::Result<()> {
    let outcome = run_command_outcome("docker", &["rm", "-f", "oya-restate"], None).await?;
    if outcome.success
        || is_missing_container_error(&outcome.stderr)
        || is_container_removal_in_progress(&outcome.stderr)
    {
        Ok(())
    } else {
        Err(anyhow::anyhow!("docker rm -f oya-restate failed: {}", outcome.stderr.trim()))
    }
}

pub(crate) fn is_missing_container_error(stderr: &str) -> bool {
    stderr.to_ascii_lowercase().contains("no such container")
}

pub(crate) fn is_container_removal_in_progress(stderr: &str) -> bool {
    let lowered = stderr.to_ascii_lowercase();
    lowered.contains("removal") && lowered.contains("already in progress")
}

async fn start_fresh_docker_restate(repo_root: &Path) -> anyhow::Result<()> {
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "down", "-v", "--remove-orphans"],
        Some(repo_root),
    )
    .await?;
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "pull", "restate"],
        Some(repo_root),
    )
    .await?;
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "up", "-d", "restate"],
        Some(repo_root),
    )
    .await
    .map(|_| ())
}

async fn restart_oya_service() -> anyhow::Result<()> {
    for attempt in 0..=OYA_SERVICE_RESTART_ATTEMPTS {
        match restart_oya_service_once().await {
            Ok(()) => return Ok(()),
            Err(error)
                if attempt < OYA_SERVICE_RESTART_ATTEMPTS
                    && is_restart_rate_limit_error(&error.to_string()) =>
            {
                let _ = run_command_capture(
                    "systemctl",
                    &["--user", "reset-failed", "oya.service"],
                    None,
                )
                .await;
                sleep(OYA_SERVICE_RESTART_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    Err(anyhow::anyhow!("oya.service restart retries exhausted"))
}

async fn ensure_oya_service_running(repo_root: &Path) -> anyhow::Result<()> {
    match restart_and_verify_oya_service().await {
        Ok(()) => Ok(()),
        Err(error) => start_local_oya_service(repo_root, &error.to_string()).await,
    }
}

async fn restart_and_verify_oya_service() -> anyhow::Result<()> {
    restart_oya_service().await?;
    verify_oya_service_unit().await
}

async fn start_local_oya_service(repo_root: &Path, reason: &str) -> anyhow::Result<()> {
    if is_tcp_port_open("127.0.0.1", 9180).await {
        println!("[oya] oya.service unavailable ({reason}); using existing local :9180 service");
        return Ok(());
    }
    let exe = std::env::current_exe()?;
    let data_dir = repo_root.join(".oya-lite");
    std::fs::create_dir_all(&data_dir)?;
    let log = File::create(data_dir.join("oya-serve.log"))?;
    let stderr = log.try_clone()?;
    Command::new(exe)
        .arg("serve")
        .current_dir(repo_root)
        .env("OYA_DATA_DIR", data_dir)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| anyhow::anyhow!("failed to start local `oya serve`: {error}"))?;
    println!("[oya] oya.service unavailable ({reason}); started local `oya serve`");
    Ok(())
}

async fn restart_oya_service_once() -> anyhow::Result<()> {
    run_command_capture("systemctl", &["--user", "restart", "oya.service"], None).await?;
    let status =
        run_command_capture("systemctl", &["--user", "is-active", "oya.service"], None).await?;
    if status.trim() == "active" {
        Ok(())
    } else {
        Err(anyhow::anyhow!("oya.service is not active after restart"))
    }
}

pub(crate) fn is_restart_rate_limit_error(stderr: &str) -> bool {
    let lowered = stderr.to_ascii_lowercase();
    lowered.contains("attempted too often")
        || lowered.contains("start request repeated too quickly")
}

async fn verify_oya_service_unit() -> anyhow::Result<()> {
    let unit = run_command_capture("systemctl", &["--user", "cat", "oya.service"], None).await?;
    let exec = extract_exec_start(&unit)
        .ok_or_else(|| anyhow::anyhow!("oya.service missing ExecStart"))?;
    let binary_ok =
        extract_exec_binary(exec).map(std::path::Path::new).is_some_and(std::path::Path::exists);
    if !binary_ok {
        return Err(anyhow::anyhow!("oya.service ExecStart binary not found: {exec}"));
    }
    if !is_valid_oya_exec_start(exec) {
        Err(anyhow::anyhow!(
            "oya.service ExecStart must run 'oya serve' on port 9180 (current: {exec})"
        ))
    } else {
        Ok(())
    }
}

pub fn extract_exec_start(unit_text: &str) -> Option<&str> {
    unit_text.lines().find_map(|line| line.strip_prefix("ExecStart="))
}

pub fn extract_exec_binary(exec_start: &str) -> Option<&str> {
    exec_start.split_whitespace().next()
}

pub fn is_valid_oya_exec_start(exec_start: &str) -> bool {
    exec_start.contains("oya") && exec_start.contains("serve") && exec_start.contains(":9180")
}

async fn wait_for_health(ingress: &str, retries: u8) -> anyhow::Result<()> {
    let url = format!("{ingress}/restate/health");
    for _ in 0..retries {
        if let Ok(response) = Client::new().get(&url).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow::anyhow!("restate health check timed out: {url}"))
}

async fn wait_for_tcp_port(host: &str, port: u16, retries: u8) -> anyhow::Result<()> {
    for _ in 0..retries {
        if tokio::net::TcpStream::connect((host, port)).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow::anyhow!("tcp port check timed out: {host}:{port}"))
}

async fn is_tcp_port_open(host: &str, port: u16) -> bool {
    tokio::net::TcpStream::connect((host, port)).await.is_ok()
}

async fn register_services(service_url: &str, admin: &str) -> anyhow::Result<()> {
    match run_command_capture(
        "restate",
        &["deployments", "register", "--force", "-y", service_url],
        None,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(cli_error) => register_services_admin(admin, service_url, &cli_error.to_string()).await,
    }
}

async fn register_services_admin(
    admin: &str,
    service_url: &str,
    cli_error: &str,
) -> anyhow::Result<()> {
    let response = Client::new()
        .post(format!("{admin}/deployments"))
        .json(&serde_json::json!({ "uri": service_url }))
        .send()
        .await?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response_body_or_status(response).await;
    Err(anyhow::anyhow!(
        "restate CLI registration failed ({cli_error}); admin registration failed ({status}): {body}"
    ))
}

async fn validate_registered_services(admin: &str) -> anyhow::Result<()> {
    match run_command_outcome("restate", &["services", "list"], None).await {
        Ok(output) if output.success && super::doctor::has_required_services(&output.stdout) => {
            Ok(())
        }
        _ => validate_registered_services_admin(admin).await,
    }
}

async fn validate_registered_services_admin(admin: &str) -> anyhow::Result<()> {
    let response = Client::new().get(format!("{admin}/deployments")).send().await?;
    let status = response.status();
    let body = response_body_or_status(response).await;
    if status.is_success() && has_required_services_admin_body(&body) {
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "missing required services in Restate registry (expected Oya/OyaMemory/OyaService); admin status {status}: {body}"
    ))
}

fn has_required_services_admin_body(body: &str) -> bool {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
        return super::doctor::has_required_services(body);
    };
    let names = json
        .get("deployments")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|deployment| deployment.get("services").and_then(serde_json::Value::as_array))
        .flatten()
        .filter_map(|service| service.get("name").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    ["Oya", "OyaMemory", "OyaService"]
        .iter()
        .all(|required| names.iter().any(|name| name == required))
}

async fn response_body_or_status(response: reqwest::Response) -> String {
    let status = response.status();
    match response.text().await {
        Ok(body) if !body.trim().is_empty() => body,
        _ => status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_url_from_ingress_maps_oya_ports() {
        let result = admin_url_from_ingress("http://127.0.0.1:909");
        assert!(matches!(result.as_deref(), Ok("http://127.0.0.1:9070")));
    }

    #[test]
    fn admin_url_from_ingress_rejects_invalid_url() {
        assert!(admin_url_from_ingress("not-a-url").is_err());
    }

    #[test]
    fn has_required_services_admin_body_reads_deployments_json() {
        let body = r#"{
            "deployments": [{
                "services": [
                    {"name": "Oya"},
                    {"name": "OyaMemory"},
                    {"name": "OyaService"}
                ]
            }]
        }"#;
        assert!(has_required_services_admin_body(body));
    }
}
