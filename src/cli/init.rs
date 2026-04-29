#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::doctor::{run_command_capture, run_command_outcome};
use reqwest::Client;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

const OYA_SERVICE_RESTART_ATTEMPTS: u8 = 4;
const OYA_SERVICE_RESTART_DELAY: Duration = Duration::from_millis(400);
const RESTATE_PID_FILE: &str = "restate.pid";

struct ManagedRestateConfig {
    binary: PathBuf,
    base_dir: PathBuf,
    pid_file: PathBuf,
    ingress_port: u16,
    admin_port: u16,
    ingress_advertised: String,
    admin_advertised: String,
}

pub async fn init_command(ingress: &str, service_url: &str, down: bool) -> anyhow::Result<()> {
    disable_systemd_restate().await?;
    let repo_root = find_repo_root()?;
    if down {
        stop_managed_restate(&repo_root).await?;
        println!("[oya] Managed Restate stopped");
        return Ok(());
    }
    let admin = admin_url_from_ingress(ingress)?;
    start_managed_restate(&repo_root, ingress, &admin).await?;
    ensure_oya_service_running(&repo_root).await?;
    wait_for_health(ingress, 30).await?;
    wait_for_tcp_port("127.0.0.1", 9180, 30).await?;
    register_services(service_url, &admin).await?;
    validate_registered_services(&admin).await?;
    println!("[oya] Runtime ready (managed Restate + handlers registered)");
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
        .find(|path| path.join("moon.yml").is_file() && path.join("Cargo.toml").is_file())
        .map(Path::to_path_buf);
    root.ok_or_else(|| anyhow::anyhow!("could not find Oya workspace root from current directory"))
}

async fn disable_systemd_restate() -> anyhow::Result<()> {
    let _ =
        run_command_capture("systemctl", &["--user", "disable", "--now", "restate.service"], None)
            .await;
    let _ =
        run_command_capture("systemctl", &["--user", "stop", "restate-manual.service"], None).await;
    Ok(())
}

async fn stop_managed_restate(repo_root: &Path) -> anyhow::Result<()> {
    let pid_file = managed_restate_pid_file(repo_root);
    let Some(pid) = read_managed_restate_pid(&pid_file)? else {
        return Ok(());
    };
    let outcome = run_command_outcome("kill", &[pid.as_str()], None).await?;
    if outcome.success || is_missing_process_error(&outcome.stderr) {
        remove_pid_file_if_present(&pid_file)?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("failed to stop managed Restate pid {pid}: {}", outcome.stderr.trim()))
    }
}

fn read_managed_restate_pid(pid_file: &Path) -> anyhow::Result<Option<String>> {
    if !pid_file.is_file() {
        return Ok(None);
    }
    let value = fs::read_to_string(pid_file)?;
    let pid = value.trim();
    if pid.chars().all(|ch| ch.is_ascii_digit()) && !pid.is_empty() {
        Ok(Some(pid.to_owned()))
    } else {
        remove_pid_file_if_present(pid_file)?;
        Ok(None)
    }
}

fn remove_pid_file_if_present(pid_file: &Path) -> anyhow::Result<()> {
    match fs::remove_file(pid_file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn is_missing_process_error(stderr: &str) -> bool {
    stderr.to_ascii_lowercase().contains("no such process")
}

async fn start_managed_restate(repo_root: &Path, ingress: &str, admin: &str) -> anyhow::Result<()> {
    stop_managed_restate(repo_root).await?;
    let config = managed_restate_config(repo_root, ingress, admin)?;
    fs::create_dir_all(&config.base_dir)?;
    fs::create_dir_all(pid_parent(&config.pid_file)?)?;
    let log = File::create(repo_root.join(".oya-lite/restate-server.log"))?;
    let stderr = log.try_clone()?;
    let child = Command::new(&config.binary)
        .arg("--base-dir")
        .arg(&config.base_dir)
        .arg("--no-logo")
        .arg("--auto-provision=true")
        .arg("--bind-ip")
        .arg("127.0.0.1")
        .current_dir(repo_root)
        .env("RESTATE_INGRESS__BIND_PORT", config.ingress_port.to_string())
        .env("RESTATE_INGRESS__ADVERTISED_ADDRESS", &config.ingress_advertised)
        .env("RESTATE_ADMIN__BIND_PORT", config.admin_port.to_string())
        .env("RESTATE_ADMIN__ADVERTISED_ADDRESS", &config.admin_advertised)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| anyhow::anyhow!("failed to start bundled restate-server: {error}"))?;
    fs::write(&config.pid_file, child.id().to_string())?;
    Ok(())
}

fn managed_restate_config(
    repo_root: &Path,
    ingress: &str,
    admin: &str,
) -> anyhow::Result<ManagedRestateConfig> {
    Ok(ManagedRestateConfig {
        binary: find_restate_server_binary(repo_root)?,
        base_dir: managed_restate_base_dir(repo_root),
        pid_file: managed_restate_pid_file(repo_root),
        ingress_port: endpoint_port(ingress)?,
        admin_port: endpoint_port(admin)?,
        ingress_advertised: advertised_address(ingress),
        admin_advertised: advertised_address(admin),
    })
}

fn managed_restate_base_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".oya-lite/restate-data")
}

fn managed_restate_pid_file(repo_root: &Path) -> PathBuf {
    repo_root.join(".oya-lite").join(RESTATE_PID_FILE)
}

fn pid_parent(pid_file: &Path) -> anyhow::Result<&Path> {
    pid_file.parent().ok_or_else(|| anyhow::anyhow!("invalid Restate pid file path"))
}

fn find_restate_server_binary(repo_root: &Path) -> anyhow::Result<PathBuf> {
    restate_server_candidates(repo_root).into_iter().find(|path| path.is_file()).ok_or_else(|| {
        anyhow::anyhow!(
            "restate-server binary not found; set OYA_RESTATE_SERVER or install bin/restate-server"
        )
    })
}

fn restate_server_candidates(repo_root: &Path) -> Vec<PathBuf> {
    std::env::var_os("OYA_RESTATE_SERVER")
        .map(PathBuf::from)
        .into_iter()
        .chain([
            repo_root.join("bin/restate-server"),
            repo_root.join("target/release/restate-server"),
        ])
        .chain(home_restate_server_candidate())
        .collect()
}

fn home_restate_server_candidate() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("bin/restate-server"))
}

fn endpoint_port(url: &str) -> anyhow::Result<u16> {
    url::Url::parse(url)?
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("endpoint URL must include a port: {url}"))
}

fn advertised_address(url: &str) -> String {
    format!("{}/", url.trim_end_matches('/'))
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
        let result = admin_url_from_ingress("http://127.0.0.1:8080");
        assert!(matches!(result.as_deref(), Ok("http://127.0.0.1:9070")));
    }

    #[test]
    fn no_docker_runtime_uses_managed_restate_paths() {
        let root = Path::new("/tmp/oya-root");

        assert_eq!(
            managed_restate_base_dir(root),
            PathBuf::from("/tmp/oya-root/.oya-lite/restate-data")
        );
        assert_eq!(
            managed_restate_pid_file(root),
            PathBuf::from("/tmp/oya-root/.oya-lite/restate.pid")
        );
    }

    #[test]
    fn no_docker_runtime_discovers_bundled_restate_first() {
        let candidates = restate_server_candidates(Path::new("/tmp/oya-root"));

        assert_eq!(candidates.first(), Some(&PathBuf::from("/tmp/oya-root/bin/restate-server")));
        assert!(candidates.contains(&PathBuf::from("/tmp/oya-root/target/release/restate-server")));
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
