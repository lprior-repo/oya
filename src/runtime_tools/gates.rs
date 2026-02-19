use super::super::*;
use super::command_exec::{combine_command_output, run_command_with_timeout_with_exit};
use oya::beads::moon_command::generate_moon_command;
use std::path::PathBuf;

const MOON_TIMEOUT_SECONDS: u64 = 900;
const ZJJ_TIMEOUT_SECONDS: u64 = 60;

#[derive(Clone)]
pub(crate) struct GateEvidence {
    pub(crate) command: String,
    pub(crate) passed: bool,
    pub(crate) exit_code: i32,
    pub(crate) output: String,
}

pub(crate) fn execute_gate(gate: Gate, repo_root: &PathBuf) -> Result<GateEvidence, OyaError> {
    let command = generate_moon_command(&gate).command;
    let timeout_seconds = match gate {
        Gate::ZjjMergeQueue => ZJJ_TIMEOUT_SECONDS,
        _ => MOON_TIMEOUT_SECONDS,
    };
    let parsed_command = parse_gate_command(command.as_str())?;
    let (program, args) = parsed_command.command_parts();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
        program.as_str(),
        &arg_refs,
        timeout_seconds,
        repo_root,
    )?;
    let passed = if gate == Gate::AcceptanceTestsAreRed { !passed } else { passed };
    let output = combine_command_output(stdout, stderr);
    Ok(GateEvidence { command, passed, exit_code, output })
}

pub(crate) enum GateCommand {
    Moon { task: MoonTask, passthrough: Vec<String> },
    ZjjSyncStatus,
}

pub(crate) struct ParsedCommandParts {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum MoonTask {
    Check,
    Test,
    Clippy,
    Security,
    Ci,
}

pub(crate) fn parse_gate_command(command: &str) -> Result<GateCommand, OyaError> {
    let parsed = parse_command_parts(command)?;
    parse_gate_command_parts(parsed)
}

fn parse_gate_command_parts(command: ParsedCommandParts) -> Result<GateCommand, OyaError> {
    match (command.program.as_str(), command.args.as_slice()) {
        ("moon", moon_args) => parse_moon_gate_command(moon_args),
        ("zjj", zjj_args) if zjj_args == ["sync", "--status"] => Ok(GateCommand::ZjjSyncStatus),
        _ => Err(OyaError(format!(
            "unsupported gate command: {} {}",
            command.program,
            command.args.join(" ")
        ))),
    }
}

fn parse_moon_gate_command(args: &[String]) -> Result<GateCommand, OyaError> {
    let (task, passthrough) = match args {
        [run, task_name, rest @ ..] if run == "run" => {
            MoonTask::from_task_name(task_name).map(|task| (task, rest.to_vec())).ok_or_else(
                || OyaError(format!("unsupported moon gate command args: {}", args.join(" "))),
            )?
        }
        _ => {
            return Err(OyaError(format!("unsupported moon gate command args: {}", args.join(" "))))
        }
    };
    Ok(GateCommand::Moon { task, passthrough })
}

impl GateCommand {
    fn command_parts(self) -> (String, Vec<String>) {
        match self {
            GateCommand::Moon { task, passthrough } => {
                let mut args = vec!["run".to_string(), task.as_task_name().to_string()];
                args.extend(passthrough);
                ("moon".to_string(), args)
            }
            GateCommand::ZjjSyncStatus => {
                ("zjj".to_string(), vec!["sync".to_string(), "--status".to_string()])
            }
        }
    }
}

impl MoonTask {
    fn from_task_name(value: &str) -> Option<Self> {
        match value {
            ":check" => Some(Self::Check),
            ":test" => Some(Self::Test),
            ":clippy" => Some(Self::Clippy),
            ":security" => Some(Self::Security),
            ":ci" => Some(Self::Ci),
            _ => None,
        }
    }

    fn as_task_name(&self) -> &'static str {
        match self {
            MoonTask::Check => ":check",
            MoonTask::Test => ":test",
            MoonTask::Clippy => ":clippy",
            MoonTask::Security => ":security",
            MoonTask::Ci => ":ci",
        }
    }
}

pub(crate) fn parse_command_parts(command: &str) -> Result<ParsedCommandParts, OyaError> {
    let parts: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return Err(OyaError("gate command cannot be empty".to_string()));
    }
    let program = parts
        .first()
        .cloned()
        .ok_or_else(|| OyaError("gate command program missing".to_string()))?;
    let args = parts.iter().skip(1).cloned().collect();
    Ok(ParsedCommandParts { program, args })
}

pub(crate) fn gate_failure_outcome(stage: &Stage, gate: &Gate) -> (FailureCategory, Stage) {
    gate_failure_mapping(stage, gate)
        .unwrap_or_else(|| (FailureCategory::TestFailed, stage.clone()))
}

fn gate_failure_mapping(stage: &Stage, gate: &Gate) -> Option<(FailureCategory, Stage)> {
    match (stage, gate) {
        (&Stage::Plan, &Gate::Compiles) => Some((FailureCategory::CompileFailed, Stage::Plan)),
        (&Stage::Contract, &Gate::Compiles) => {
            Some((FailureCategory::CompileFailed, Stage::Contract))
        }
        (&Stage::AcceptanceTest, &Gate::Compiles) => {
            Some((FailureCategory::CompileFailed, Stage::AcceptanceTest))
        }
        (&Stage::AcceptanceTest, &Gate::AcceptanceTestsAreRed) => {
            Some((FailureCategory::TestsUnexpectedlyGreen, Stage::AcceptanceTest))
        }
        (&Stage::Implementation, &Gate::Compiles) => {
            Some((FailureCategory::CompileFailed, Stage::Implementation))
        }
        (&Stage::Implementation, &Gate::TestsPass) => {
            Some((FailureCategory::TestFailed, Stage::Implementation))
        }
        (&Stage::Tdd15, &Gate::Compiles) => Some((FailureCategory::CompileFailed, Stage::Tdd15)),
        (&Stage::Tdd15, &Gate::TestsPass) => Some((FailureCategory::TestFailed, Stage::Tdd15)),
        (&Stage::Qa, &Gate::TestsPass) | (&Stage::Qa, &Gate::EdgeCases) => {
            Some((FailureCategory::TestFailed, Stage::Implementation))
        }
        (&Stage::RedQueen, &Gate::NoVulnerabilities) => {
            Some((FailureCategory::TestFailed, Stage::Implementation))
        }
        (&Stage::GptReview, &Gate::ClippyClean) => {
            Some((FailureCategory::LintFailed, Stage::GptReview))
        }
        (&Stage::GptReview, &Gate::Security) => {
            Some((FailureCategory::TestFailed, Stage::Implementation))
        }
        (&Stage::ShipGate, &Gate::MoonCi) => {
            Some((FailureCategory::TestFailed, Stage::Implementation))
        }
        (&Stage::ShipGate, &Gate::ZjjMergeQueue) => {
            Some((FailureCategory::MergeConflict, Stage::GptReview))
        }
        _ => None,
    }
}
