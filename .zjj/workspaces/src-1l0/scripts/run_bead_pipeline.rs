use std::env;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;

const PHASES: [&str; 7] = ["SCOUT", "ATDD", "RED", "IMPLEMENT", "REVIEW", "JUDGE", "COMMIT"];
const REVIEW_COMMANDS: [&str; 4] =
    ["moon run :build", "moon run :test", "moon run :clippy", "moon run :ci"];
const FORBIDDEN_SNIPPETS: [&str; 2] = ["cargo ", "git "];
const RESTATE_MARKERS: [&str; 3] =
    ["\"transport\":\"restate\"", "\"runtime\":\"restate\"", "\"source\":\"restate\""];
const MAX_RETRIES: usize = 5;

#[derive(Debug, Clone)]
struct CliConfig {
    run_id: String,
    beads: Vec<String>,
    parallel: usize,
}

#[derive(Debug, Clone)]
struct BeadResult {
    bead: String,
    status: String,
    retries: usize,
    commit: Option<String>,
    error: Option<String>,
}

#[derive(Debug)]
struct AppError(String);

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("error: {err}");
            1
        }
    };
    std::process::exit(exit_code);
}

fn run() -> Result<(), AppError> {
    let cfg = parse_args(env::args().collect())?;
    preflight_checks()?;

    let run_root = build_run_root(&cfg.run_id)?;
    let results = run_beads_parallel(&cfg, &run_root)?;
    write_summary(&run_root, &cfg.run_id, &results)?;

    let failed_count = results.iter().filter(|r| r.status != "PASS").count();
    print_summary(&cfg.run_id, &results, failed_count);

    if failed_count == 0 {
        Ok(())
    } else {
        Err(AppError("one or more beads failed validation".to_string()))
    }
}

fn parse_args(args: Vec<String>) -> Result<CliConfig, AppError> {
    let mut run_id = String::new();
    let mut beads = Vec::new();
    let mut parallel = 6usize;

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--run-id" => {
                run_id = read_next_arg(&args, i, "--run-id")?;
                i += 2;
            }
            "--parallel" => {
                let value = read_next_arg(&args, i, "--parallel")?;
                parallel = value
                    .parse::<usize>()
                    .map_err(|_| AppError("--parallel must be a positive integer".to_string()))?;
                i += 2;
            }
            "--beads" => {
                i += 1;
                while i < args.len() && !args[i].starts_with("--") {
                    beads.push(args[i].clone());
                    i += 1;
                }
            }
            _ => {
                return Err(AppError(format!("unknown argument: {}", args[i])));
            }
        }
    }

    if run_id.trim().is_empty() {
        return Err(AppError("missing required --run-id".to_string()));
    }
    if beads.is_empty() {
        return Err(AppError("missing required --beads".to_string()));
    }
    if parallel == 0 {
        return Err(AppError("--parallel must be >= 1".to_string()));
    }

    Ok(CliConfig { run_id, beads, parallel })
}

fn read_next_arg(args: &[String], i: usize, flag: &str) -> Result<String, AppError> {
    args.get(i + 1).cloned().ok_or_else(|| AppError(format!("missing value for {flag}")))
}

fn preflight_checks() -> Result<(), AppError> {
    ensure_agent_runner()?;
    ensure_main_clean()?;
    ensure_pending_merges_zero()?;
    Ok(())
}

fn ensure_agent_runner() -> Result<(), AppError> {
    match env::var("AGENT_RUNNER") {
        Ok(value) if !value.trim().is_empty() => {
            if value.contains("mock_phase_agent.sh") {
                return Err(AppError(
                    "AGENT_RUNNER cannot use mock phase agent; Restate-backed execution is required"
                        .to_string(),
                ));
            }
            Ok(())
        }
        _ => Err(AppError(
            "AGENT_RUNNER env var is required. Example: oya agent run --phase {phase} --bead {bead} --run {run_id} --output {out}".to_string(),
        )),
    }
}

fn ensure_main_clean() -> Result<(), AppError> {
    let out = run_shell("jj status")?;
    if out.to_lowercase().contains("conflict") {
        return Err(AppError("jj status reports unresolved conflict".to_string()));
    }
    Ok(())
}

fn ensure_pending_merges_zero() -> Result<(), AppError> {
    let out = run_shell("zjj query pending-merges")?;
    if pending_merge_count_is_zero(&out) {
        return Ok(());
    }
    Err(AppError(format!("pending merges are not zero: {out}")))
}

fn pending_merge_count_is_zero(output: &str) -> bool {
    let compact = output.replace(char::is_whitespace, "");
    compact.contains("\"count\":0") || compact == "0"
}

fn run_beads_parallel(cfg: &CliConfig, run_root: &Path) -> Result<Vec<BeadResult>, AppError> {
    let (tx, rx) = mpsc::channel::<BeadResult>();
    let mut workers = Vec::new();
    let chunks = split_chunks(cfg.beads.clone(), cfg.parallel);

    for chunk in chunks {
        let thread_tx = tx.clone();
        let run_root = run_root.to_path_buf();
        let run_id = cfg.run_id.clone();
        workers.push(thread::spawn(move || {
            for bead in chunk {
                let result = run_bead(&run_id, &bead, &run_root);
                let _ = thread_tx.send(result);
            }
        }));
    }
    drop(tx);

    let mut results = Vec::new();
    for result in rx {
        results.push(result);
    }
    for worker in workers {
        let _ = worker.join();
    }

    Ok(results)
}

fn split_chunks(mut beads: Vec<String>, workers: usize) -> Vec<Vec<String>> {
    let worker_count = workers.min(beads.len()).max(1);
    let mut chunks = vec![Vec::new(); worker_count];
    for (idx, bead) in beads.drain(..).enumerate() {
        let slot = idx % worker_count;
        chunks[slot].push(bead);
    }
    chunks
}

fn run_bead(run_id: &str, bead: &str, run_root: &Path) -> BeadResult {
    let bead_dir = run_root.join(bead);
    let _ = fs::create_dir_all(&bead_dir);

    let mut retries = 0usize;
    while retries < MAX_RETRIES {
        retries += 1;
        match run_bead_once(run_id, bead, &bead_dir) {
            Ok(commit_hash) => {
                return BeadResult {
                    bead: bead.to_string(),
                    status: "PASS".to_string(),
                    retries,
                    commit: Some(commit_hash),
                    error: None,
                };
            }
            Err(err) => {
                let err_file = bead_dir.join(format!("retry_{}_error.txt", retries));
                let _ = fs::write(err_file, err.to_string());
            }
        }
    }

    BeadResult {
        bead: bead.to_string(),
        status: "FAIL".to_string(),
        retries,
        commit: None,
        error: Some("max retries exhausted".to_string()),
    }
}

fn run_bead_once(run_id: &str, bead: &str, bead_dir: &Path) -> Result<String, AppError> {
    for phase in PHASES {
        let output_file = bead_dir.join(format!("{}.json", phase.to_lowercase()));
        invoke_phase_agent(phase, bead, run_id, &output_file)?;
        validate_phase_output(&output_file, phase, bead)?;

        if phase == "REVIEW" {
            run_review_gates()?;
        }
        if phase == "JUDGE" {
            ensure_judge_passed(&output_file)?;
        }
    }

    ensure_atdd_promoted(bead_dir, bead)?;
    run_review_gates()?;
    ensure_pending_merges_zero()?;
    ensure_main_clean()?;

    let commit_hash = extract_commit_hash(&bead_dir.join("commit.json"))?;
    ensure_commit_on_main(&commit_hash)?;
    Ok(commit_hash)
}

fn invoke_phase_agent(phase: &str, bead: &str, run_id: &str, out: &Path) -> Result<(), AppError> {
    let template =
        env::var("AGENT_RUNNER").map_err(|_| AppError("AGENT_RUNNER missing".to_string()))?;
    let command = template
        .replace("{phase}", phase)
        .replace("{bead}", bead)
        .replace("{run_id}", run_id)
        .replace("{out}", out.to_string_lossy().as_ref());

    run_shell(&command)?;
    Ok(())
}

fn validate_phase_output(path: &Path, phase: &str, bead: &str) -> Result<(), AppError> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError(format!("missing phase output {}: {e}", path.display())))?;

    ensure_contains(&content, &format!("\"bead_id\":\"{}\"", bead), phase, "bead_id")?;
    ensure_contains(&content, &format!("\"phase\":\"{}\"", phase), phase, "phase")?;
    ensure_contains(&content, "\"gate_result\"", phase, "gate_result")?;
    ensure_contains(&content, "\"passed\":true", phase, "gate_result.passed")?;

    for forbidden in FORBIDDEN_SNIPPETS {
        if content.contains(forbidden) {
            return Err(AppError(format!(
                "{} output contains forbidden command snippet: {}",
                phase, forbidden
            )));
        }
    }

    let compact = content.replace(' ', "");
    let restate_verified = RESTATE_MARKERS.iter().any(|marker| compact.contains(marker));
    if !restate_verified {
        return Err(AppError(format!("{} output missing Restate execution marker", phase)));
    }

    Ok(())
}

fn ensure_contains(content: &str, needle: &str, phase: &str, field: &str) -> Result<(), AppError> {
    if content.replace(' ', "").contains(needle) {
        Ok(())
    } else {
        Err(AppError(format!("{} output missing {}", phase, field)))
    }
}

fn run_review_gates() -> Result<(), AppError> {
    for cmd in REVIEW_COMMANDS {
        run_shell(cmd)?;
    }
    Ok(())
}

fn ensure_judge_passed(path: &Path) -> Result<(), AppError> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError(format!("unable to read judge output {}: {e}", path.display())))?;
    if content.contains("\"status\":\"PASS\"") {
        Ok(())
    } else {
        Err(AppError("judge phase did not return PASS".to_string()))
    }
}

fn ensure_atdd_promoted(bead_dir: &Path, bead: &str) -> Result<(), AppError> {
    let marker = bead_dir.join("atdd_promoted.json");
    let content = fs::read_to_string(&marker).map_err(|e| {
        AppError(format!(
            "missing ATDD promotion marker for bead {} at {}: {}",
            bead,
            marker.display(),
            e
        ))
    })?;
    if content.replace(' ', "").contains("\"promoted\":true") {
        Ok(())
    } else {
        Err(AppError(format!("ATDD promotion marker for bead {} does not confirm promotion", bead)))
    }
}

fn extract_commit_hash(commit_json: &Path) -> Result<String, AppError> {
    let content = fs::read_to_string(commit_json)
        .map_err(|e| AppError(format!("missing commit output {}: {e}", commit_json.display())))?;
    let compact = content.replace(' ', "");
    let key = "\"commit_hash\":\"";
    let start = compact
        .find(key)
        .ok_or_else(|| AppError("commit output missing commit_hash".to_string()))?
        + key.len();
    let remaining = &compact[start..];
    let end = remaining.find('"').ok_or_else(|| AppError("commit_hash parse error".to_string()))?;
    let commit = remaining[..end].to_string();
    if commit.is_empty() {
        Err(AppError("empty commit_hash in commit output".to_string()))
    } else {
        Ok(commit)
    }
}

fn ensure_commit_on_main(commit: &str) -> Result<(), AppError> {
    let cmd = format!("jj log -r 'ancestors(main) & {}' --no-graph", commit);
    let out = run_shell(&cmd)?;
    if out.trim().is_empty() {
        Err(AppError(format!("commit {} is not on main", commit)))
    } else {
        Ok(())
    }
}

fn run_shell(command: &str) -> Result<String, AppError> {
    let output = Command::new(OsStr::new("sh"))
        .arg(OsStr::new("-lc"))
        .arg(command)
        .output()
        .map_err(|e| AppError(format!("failed to execute '{}': {}", command, e)))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| AppError(format!("stdout utf8 decode failed for '{}': {}", command, e)))
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AppError(format!(
            "command failed: {}\nstdout:\n{}\nstderr:\n{}",
            command, stdout, stderr
        )))
    }
}

fn build_run_root(run_id: &str) -> Result<PathBuf, AppError> {
    let path = PathBuf::from(".orchestrator").join("runs").join(run_id);
    fs::create_dir_all(&path)
        .map_err(|e| AppError(format!("failed to create run root {}: {}", path.display(), e)))?;
    Ok(path)
}

fn write_summary(run_root: &Path, run_id: &str, results: &[BeadResult]) -> Result<(), AppError> {
    let body = build_summary_json(run_id, results);
    let path = run_root.join("summary.json");
    fs::write(&path, body)
        .map_err(|e| AppError(format!("failed to write summary {}: {}", path.display(), e)))
}

fn build_summary_json(run_id: &str, results: &[BeadResult]) -> String {
    let items = results.iter().map(result_to_json).collect::<Vec<String>>().join(",");
    format!("{{\"run_id\":\"{}\",\"results\":[{}]}}", escape(run_id), items)
}

fn result_to_json(result: &BeadResult) -> String {
    let commit = result
        .commit
        .as_ref()
        .map(|v| format!("\"{}\"", escape(v)))
        .unwrap_or_else(|| "null".to_string());
    let error = result
        .error
        .as_ref()
        .map(|v| format!("\"{}\"", escape(v)))
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{{\"bead\":\"{}\",\"status\":\"{}\",\"retries\":{},\"commit\":{},\"error\":{}}}",
        escape(&result.bead),
        escape(&result.status),
        result.retries,
        commit,
        error
    )
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn print_summary(run_id: &str, results: &[BeadResult], failed: usize) {
    println!("run_id: {}", run_id);
    for result in results {
        println!(
            "bead={} status={} retries={} commit={} error={}",
            result.bead,
            result.status,
            result.retries,
            result.commit.clone().unwrap_or_else(|| "-".to_string()),
            result.error.clone().unwrap_or_else(|| "-".to_string())
        );
    }
    println!("failed={}", failed);
}
