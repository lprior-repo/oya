#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::doctor::{run_command_capture, run_command_outcome};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;

pub async fn init_command(ingress: &str, service_url: &str, down: bool) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    disable_systemd_restate().await?;
    if down {
        stop_docker_restate(&repo_root).await?;
        println!("[oya] Docker Restate stopped");
        return Ok(());
    }
    start_fresh_docker_restate(&repo_root).await?;
    restart_oya_service().await?;
    verify_oya_service_unit().await?;
    wait_for_health(ingress, 30).await?;
    wait_for_tcp_port("127.0.0.1", 9180, 30).await?;
    register_services(service_url).await?;
    validate_registered_services().await?;
    println!("[oya] Runtime ready (fresh Restate + handlers registered)");
    println!("  Admin:   http://127.0.0.1:9070");
    println!("  Ingress: {ingress}");
    println!("  Service: {service_url}");
    Ok(())
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

async fn stop_docker_restate(repo_root: &Path) -> anyhow::Result<()> {
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "stop", "restate"],
        Some(repo_root),
    )
    .await
    .map(|_| ())
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
    run_command_capture("systemctl", &["--user", "restart", "oya.service"], None).await?;
    let status =
        run_command_capture("systemctl", &["--user", "is-active", "oya.service"], None).await?;
    if status.trim() == "active" {
        Ok(())
    } else {
        Err(anyhow::anyhow!("oya.service is not active after restart"))
    }
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

async fn register_services(service_url: &str) -> anyhow::Result<()> {
    run_command_capture("restate", &["deployments", "register", "--force", "-y", service_url], None)
        .await
        .map(|_| ())
}

async fn validate_registered_services() -> anyhow::Result<()> {
    let output = run_command_outcome("restate", &["services", "list"], None).await?;
    if super::doctor::has_required_services(&output.stdout) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "missing required services in Restate registry (expected Oya/OyaMemory/OyaService)"
        ))
    }
}
