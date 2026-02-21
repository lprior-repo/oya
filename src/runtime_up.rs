use anyhow::{anyhow, bail, Context, Result};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const RESTATE_ADMIN: &str = "http://127.0.0.1:9070";
const RESTATE_INGRESS: &str = "http://127.0.0.1:8080";
const RESTATE_UNIT: &str = "restate-manual";
const OPENCODE_UNIT: &str = "opencode-manual";
const OYA_UNIT: &str = "oya-manual";

pub(crate) fn run_local_up() -> Result<()> {
    let repo_root = std::env::current_dir().context("resolve repo root")?;
    ensure_restate_running(&repo_root)?;
    let opencode_bin = resolve_opencode_bin()?;
    let opencode_port = resolve_opencode_port()?;
    start_opencode_service(&repo_root, &opencode_bin, opencode_port)?;
    start_oya_service(&repo_root, &opencode_bin, opencode_port)?;
    register_deployment(RESTATE_ADMIN, "http://localhost:9080")?;
    print_ready(opencode_port);
    Ok(())
}

fn ensure_restate_running(repo_root: &Path) -> Result<()> {
    let restate_bin = resolve_restate_bin()?;
    let restate_data_dir = repo_root.join(".oya/restate-data");
    std::fs::create_dir_all(&restate_data_dir).context("create repo restate data directory")?;

    run_best_effort("systemctl", &["--user", "stop", "restate.service"], repo_root);
    run_best_effort(
        "systemctl",
        &["--user", "stop", &format!("{}.service", RESTATE_UNIT)],
        repo_root,
    );
    run_best_effort(
        "systemctl",
        &["--user", "reset-failed", &format!("{}.service", RESTATE_UNIT)],
        repo_root,
    );

    let args = vec![
        "--user".to_string(),
        "--unit".to_string(),
        RESTATE_UNIT.to_string(),
        "-E".to_string(),
        format!("RESTATE_BASE_DIR={}", restate_data_dir.display()),
        "-E".to_string(),
        "RESTATE_ADMIN__BIND_PORT=9070".to_string(),
        "-E".to_string(),
        "RESTATE_INGRESS__BIND_PORT=8080".to_string(),
        "-E".to_string(),
        "RESTATE_ADMIN__BIND_ADDRESS=127.0.0.1".to_string(),
        "-E".to_string(),
        "RESTATE_INGRESS__BIND_ADDRESS=127.0.0.1".to_string(),
        "--working-directory".to_string(),
        repo_root.display().to_string(),
        restate_bin.display().to_string(),
        "dev".to_string(),
        "--retain".to_string(),
    ];
    run_checked_owned("systemd-run", &args, repo_root)?;
    wait_for_http_ready(&format!("{RESTATE_ADMIN}/health"), 30, 1)
}

fn resolve_restate_bin() -> Result<PathBuf> {
    let from_env = std::env::var("OYA_RESTATE_BIN").ok().map(PathBuf::from);
    let from_path = lookup_binary("restate");
    let fallback = std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home).join(".local/share/mise/installs/ubi-restatedev-restate/latest/restate")
    });

    from_env
        .into_iter()
        .chain(from_path)
        .chain(fallback)
        .find(|candidate| candidate.exists())
        .ok_or_else(|| anyhow!("restate binary not found. Install restate CLI or set PATH"))
}

fn resolve_opencode_bin() -> Result<PathBuf> {
    let from_env = std::env::var("OPENCODE_PATH").ok().map(PathBuf::from);
    let from_path = lookup_binary("opencode");
    let fallback = std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home).join(".local/share/mise/installs/github-sst-opencode/latest/opencode")
    });

    [from_env, from_path, fallback]
        .into_iter()
        .flatten()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| anyhow!("OpenCode binary not found. Set OPENCODE_PATH or install opencode"))
}

fn resolve_opencode_port() -> Result<u16> {
    let preferred = std::env::var("OPENCODE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(4097);

    (preferred..=preferred.saturating_add(3))
        .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .ok_or_else(|| anyhow!("no available OpenCode port near {}", preferred))
}

fn start_opencode_service(repo_root: &Path, opencode_bin: &Path, port: u16) -> Result<()> {
    run_best_effort(
        "systemctl",
        &["--user", "stop", &format!("{}.service", OPENCODE_UNIT)],
        repo_root,
    );
    run_best_effort(
        "systemctl",
        &["--user", "reset-failed", &format!("{}.service", OPENCODE_UNIT)],
        repo_root,
    );

    let args = vec![
        "--user".to_string(),
        "--unit".to_string(),
        OPENCODE_UNIT.to_string(),
        "-E".to_string(),
        "OPENCODE_SERVER_PASSWORD=".to_string(),
        "--working-directory".to_string(),
        repo_root.display().to_string(),
        opencode_bin.display().to_string(),
        "serve".to_string(),
        "--port".to_string(),
        port.to_string(),
        "--hostname".to_string(),
        "127.0.0.1".to_string(),
        "--print-logs".to_string(),
    ];
    run_checked_owned("systemd-run", &args, repo_root)?;
    wait_for_http_ready(&format!("http://127.0.0.1:{port}/global/health"), 25, 1)
}

fn start_oya_service(repo_root: &Path, opencode_bin: &Path, port: u16) -> Result<()> {
    let oya_bin = std::env::current_exe().context("resolve running oya binary path")?;
    let moon_bin = resolve_moon_bin()?;
    let skip_gate =
        std::env::var("OYA_SKIP_ZJJ_GATE").map_or_else(|_| "1".to_string(), std::convert::identity);
    let env_path = std::env::var("PATH").map_or_else(|_| String::new(), std::convert::identity);

    run_best_effort("systemctl", &["--user", "stop", &format!("{}.service", OYA_UNIT)], repo_root);
    run_best_effort(
        "systemctl",
        &["--user", "reset-failed", &format!("{}.service", OYA_UNIT)],
        repo_root,
    );

    let args = vec![
        "--user".to_string(),
        "--unit".to_string(),
        OYA_UNIT.to_string(),
        "-E".to_string(),
        format!("PATH={}", env_path),
        "-E".to_string(),
        format!("MOON_PATH={}", moon_bin.display()),
        "-E".to_string(),
        format!("OPENCODE_PATH={}", opencode_bin.display()),
        "-E".to_string(),
        format!("OYA_RESTATE_ADMIN_URL={RESTATE_ADMIN}"),
        "-E".to_string(),
        format!("OYA_OPENCODE_BASE_URL=http://127.0.0.1:{port}"),
        "-E".to_string(),
        "OYA_OPENCODE_PASSWORD=".to_string(),
        "-E".to_string(),
        format!("OYA_SKIP_ZJJ_GATE={skip_gate}"),
        "--working-directory".to_string(),
        repo_root.display().to_string(),
        oya_bin.display().to_string(),
        "serve".to_string(),
    ];
    run_checked_owned("systemd-run", &args, repo_root).map(|_| ())
}

fn resolve_moon_bin() -> Result<PathBuf> {
    let from_env = std::env::var("MOON_PATH").ok().map(PathBuf::from);
    let from_path = lookup_binary("moon");

    from_env
        .into_iter()
        .chain(from_path)
        .find(|candidate| candidate.exists())
        .ok_or_else(|| anyhow!("moon binary not found. Set MOON_PATH or install moon"))
}

fn lookup_binary(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then(|| PathBuf::from(value))
}

fn wait_for_http_ready(url: &str, attempts: u32, sleep_seconds: u64) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("build readiness HTTP client")?;

    for _ in 0..attempts {
        if client.get(url).send().is_ok_and(|res| res.status().is_success()) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(sleep_seconds));
    }

    bail!("service readiness check failed for {}", url)
}

fn register_deployment(admin_url: &str, service_url: &str) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(format!("{admin_url}/deployments"))
        .json(&serde_json::json!({ "uri": service_url }))
        .send()
        .context("register deployment")?;

    let payload = response
        .error_for_status()
        .context("deployment registration failed")?
        .json::<serde_json::Value>()
        .context("decode deployment registration response")?;

    let deployment_id = payload
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("deployment register response missing id"))?;

    client
        .patch(format!("{admin_url}/deployments/{deployment_id}"))
        .json(&serde_json::json!({ "uri": service_url, "overwrite": true }))
        .send()
        .context("patch deployment")?
        .error_for_status()
        .context("deployment patch failed")?;
    Ok(())
}

fn run_checked(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to execute {} {}", program, args.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    bail!("command failed: {} {}\n{}\n{}", program, args.join(" "), stdout, stderr)
}

fn run_checked_owned(program: &str, args: &[String], cwd: &Path) -> Result<()> {
    let arg_refs = args.iter().map(std::string::String::as_str).collect::<Vec<_>>();
    run_checked(program, &arg_refs, cwd)
}

fn run_best_effort(program: &str, args: &[&str], cwd: &Path) {
    let _status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn print_ready(opencode_port: u16) {
    println!("[oya] Runtime ready");
    println!("  Admin:   {RESTATE_ADMIN}");
    println!("  Ingress: {RESTATE_INGRESS}");
    println!("  Service: http://127.0.0.1:9080");
    println!("  OpenCode: http://127.0.0.1:{opencode_port}");
}
