use super::super::*;
use super::command_exec::{combine_command_output, run_command_with_timeout_with_exit};
use oya::beads::moon_command::generate_moon_command;
use oya::types::Gate;
use std::path::PathBuf;

const MOON_TIMEOUT_SECONDS: u64 = 900;
const GIT_TIMEOUT_SECONDS: u64 = 30;

#[derive(Clone)]
pub(crate) struct GateEvidence {
    pub(crate) command: String,
    pub(crate) passed: bool,
    pub(crate) exit_code: i32,
    pub(crate) output: String,
    pub(crate) revision: Option<String>,
    pub(crate) current_revision: Option<String>,
}

pub(crate) fn execute_gate(gate: Gate, repo_root: &PathBuf) -> Result<GateEvidence, OyaError> {
    let command = generate_moon_command(&gate).command;
    let timeout_seconds = MOON_TIMEOUT_SECONDS;
    let parsed_command = parse_gate_command(command.as_str())?;
    let pinned_revision = collect_pinned_revision(&parsed_command, repo_root)?;
    let (program, args) = parsed_command.command_parts();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (command_passed, stdout, stderr, command_exit_code) = run_command_with_timeout_with_exit(
        program.as_str(),
        &arg_refs,
        timeout_seconds,
        repo_root,
    )?;
    let output = combine_command_output(stdout, stderr);
    let current_revision = collect_current_revision(&parsed_command, repo_root)?;
    let revision_check = validate_revision_pair(&pinned_revision, &current_revision);
    let passed = command_passed && revision_check.is_ok();
    let exit_code = if revision_check.is_ok() { command_exit_code } else { 1 };
    let output = append_revision_failure(output, revision_check.err());
    Ok(GateEvidence {
        command,
        passed,
        exit_code,
        output,
        revision: pinned_revision,
        current_revision,
    })
}

#[derive(Clone)]
pub(crate) enum GateCommand {
    Moon { task: MoonTask, passthrough: Vec<String> },
}

pub(crate) struct ParsedCommandParts {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum MoonTask {
    Check,
    Test,
    Ci,
    Holdout,
    CueCheck,
}

pub(crate) fn parse_gate_command(command: &str) -> Result<GateCommand, OyaError> {
    let parsed = parse_command_parts(command)?;
    parse_gate_command_parts(parsed)
}

fn parse_gate_command_parts(command: ParsedCommandParts) -> Result<GateCommand, OyaError> {
    match (command.program.as_str(), command.args.as_slice()) {
        ("moon", moon_args) => parse_moon_gate_command(moon_args),
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
    fn command_parts(&self) -> (String, Vec<String>) {
        match self {
            GateCommand::Moon { task, passthrough } => {
                let args = std::iter::once("run".to_string())
                    .chain(std::iter::once(task.as_task_name().to_string()))
                    .chain(passthrough.iter().cloned())
                    .collect();
                ("moon".to_string(), args)
            }
        }
    }
}

impl MoonTask {
    fn from_task_name(value: &str) -> Option<Self> {
        match value {
            ":check" => Some(Self::Check),
            ":test" => Some(Self::Test),
            ":ci" => Some(Self::Ci),
            ":holdout" => Some(Self::Holdout),
            ":cue-check" => Some(Self::CueCheck),
            _ => None,
        }
    }

    fn as_task_name(&self) -> &'static str {
        match self {
            MoonTask::Check => ":check",
            MoonTask::Test => ":test",
            MoonTask::Ci => ":ci",
            MoonTask::Holdout => ":holdout",
            MoonTask::CueCheck => ":cue-check",
        }
    }
}

pub(crate) fn parse_command_parts(command: &str) -> Result<ParsedCommandParts, OyaError> {
    let parts = tokenize_command(command)?;
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

fn tokenize_command(command: &str) -> Result<Vec<String>, OyaError> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for character in command.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                push_token_if_present(&mut parts, &mut current);
            }
            c => current.push(c),
        }
    }

    if escaped {
        return Err(OyaError("gate command has trailing escape".to_string()));
    }
    if in_single || in_double {
        return Err(OyaError("gate command has unclosed quote".to_string()));
    }

    push_token_if_present(&mut parts, &mut current);
    Ok(parts)
}

fn push_token_if_present(parts: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        parts.push(std::mem::take(current));
    }
}

fn collect_pinned_revision(
    command: &GateCommand,
    repo_root: &PathBuf,
) -> Result<Option<String>, OyaError> {
    if uses_moon(command) {
        return current_head_revision(repo_root).map(Some);
    }
    Ok(None)
}

fn collect_current_revision(
    command: &GateCommand,
    repo_root: &PathBuf,
) -> Result<Option<String>, OyaError> {
    if uses_moon(command) {
        return current_head_revision(repo_root).map(Some);
    }
    Ok(None)
}

const fn uses_moon(command: &GateCommand) -> bool {
    matches!(command, GateCommand::Moon { .. })
}

fn current_head_revision(repo_root: &PathBuf) -> Result<String, OyaError> {
    let args = ["rev-parse", "HEAD"];
    let (passed, stdout, stderr, exit_code) =
        run_command_with_timeout_with_exit("git", &args, GIT_TIMEOUT_SECONDS, repo_root)?;
    if !passed {
        let output = combine_command_output(stdout, stderr);
        return Err(OyaError(format!(
            "failed to resolve git revision (exit={}): {}",
            exit_code,
            truncate_clean(output.as_str(), 2000)
        )));
    }
    parse_revision(stdout.trim())
}

fn parse_revision(value: &str) -> Result<String, OyaError> {
    if is_full_sha(value) {
        return Ok(value.to_string());
    }
    Err(OyaError(format!("invalid git revision format: {}", value)))
}

fn is_full_sha(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn validate_revision_pair(
    pinned_revision: &Option<String>,
    current_revision: &Option<String>,
) -> Result<(), String> {
    match (pinned_revision.as_deref(), current_revision.as_deref()) {
        (Some(pinned), Some(current)) if pinned != current => {
            Err(format!("stale_evidence pinned_revision={} current_head={}", pinned, current))
        }
        _ => Ok(()),
    }
}

fn append_revision_failure(output: String, failure: Option<String>) -> String {
    let Some(failure) = failure else {
        return output;
    };
    if output.is_empty() {
        return failure;
    }
    format!("{}\n{}", failure, output)
}

pub(crate) fn gate_failure_outcome(stage: &Stage, gate: &Gate) -> (FailureCategory, Stage) {
    gate_failure_mapping(stage, gate)
        .unwrap_or_else(|| (FailureCategory::TestFailed, stage.clone()))
}

fn gate_failure_mapping(stage: &Stage, gate: &Gate) -> Option<(FailureCategory, Stage)> {
    match (stage, gate) {
        (&Stage::Explore, _) => None,
        (&Stage::Contract, &Gate::Compiles) => {
            Some((FailureCategory::CompileFailed, Stage::Contract))
        }
        (&Stage::Red, &Gate::Compiles) => Some((FailureCategory::CompileFailed, Stage::Red)),
        (&Stage::Implementation, &Gate::Compiles) => {
            Some((FailureCategory::CompileFailed, Stage::Implementation))
        }
        (&Stage::Implementation, &Gate::TestsPass) => {
            Some((FailureCategory::TestFailed, Stage::Implementation))
        }
        (&Stage::Witness, &Gate::HoldoutScenarios) => {
            Some((FailureCategory::TestFailed, Stage::Implementation))
        }
        (&Stage::ShipGate, &Gate::CueArtifactGenerated) => {
            Some((FailureCategory::OutputParseFailure, Stage::Implementation))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya::types::StageName;

    #[test]
    fn test_cue_artifact_gate_routes_to_implementation_on_failure() {
        let (failure, next_stage) =
            gate_failure_outcome(&StageName::ShipGate, &Gate::CueArtifactGenerated);
        assert_eq!(failure, FailureCategory::OutputParseFailure);
        assert_eq!(next_stage, StageName::Implementation);
    }

    #[test]
    fn test_parse_cue_check_command() {
        let result = parse_gate_command("moon run :cue-check");
        assert!(result.is_ok());
        if let Ok(GateCommand::Moon { task, passthrough }) = result {
            assert!(matches!(task, MoonTask::CueCheck));
            assert!(passthrough.is_empty());
        } else {
            panic!("Expected Moon command");
        }
    }

    #[test]
    fn test_moon_task_cue_check_roundtrip() {
        let task = MoonTask::CueCheck;
        assert_eq!(task.as_task_name(), ":cue-check");
        let parsed = MoonTask::from_task_name(":cue-check");
        assert!(matches!(parsed, Some(MoonTask::CueCheck)));
    }

    #[test]
    fn test_moon_task_holdout_roundtrip() {
        let task = MoonTask::Holdout;
        assert_eq!(task.as_task_name(), ":holdout");
        let parsed = MoonTask::from_task_name(":holdout");
        assert!(matches!(parsed, Some(MoonTask::Holdout)));
    }

    #[test]
    fn test_witness_holdout_failure_routes_to_implementation() {
        let (failure, next_stage) =
            gate_failure_outcome(&StageName::Witness, &Gate::HoldoutScenarios);
        assert_eq!(failure, FailureCategory::TestFailed);
        assert_eq!(next_stage, StageName::Implementation);
    }

    #[test]
    fn test_all_ship_gate_failures_route_to_implementation() {
        let ship_gates = vec![Gate::CueArtifactGenerated];
        for gate in ship_gates {
            let (_, next_stage) = gate_failure_outcome(&StageName::ShipGate, &gate);
            assert_eq!(
                next_stage,
                StageName::Implementation,
                "Gate {:?} should route to Implementation stage",
                gate
            );
        }

        let unknown_gate_outcome = gate_failure_outcome(&StageName::ShipGate, &Gate::ZjjMergeQueue);
        assert_eq!(unknown_gate_outcome, (FailureCategory::TestFailed, StageName::ShipGate));
    }

    #[test]
    fn test_parse_command_parts_handles_quoted_passthrough() {
        let parsed = parse_command_parts("moon run :test -- --filter 'retry loop'")
            .expect("quoted command should parse");
        assert_eq!(
            parsed.args,
            vec![
                "run".to_string(),
                ":test".to_string(),
                "--".to_string(),
                "--filter".to_string(),
                "retry loop".to_string(),
            ]
        );
    }

    #[test]
    fn test_parse_command_parts_handles_escaped_whitespace() {
        let parsed = parse_command_parts("moon run :test -- --name retry\\ loop")
            .expect("escaped whitespace should parse");
        assert_eq!(
            parsed.args,
            vec![
                "run".to_string(),
                ":test".to_string(),
                "--".to_string(),
                "--name".to_string(),
                "retry loop".to_string(),
            ]
        );
    }

    #[test]
    fn test_parse_command_parts_rejects_unclosed_quotes() {
        let parsed = parse_command_parts("moon run :test -- --name 'retry loop");
        assert!(parsed.is_err());
    }

    #[test]
    fn test_parse_command_parts_rejects_trailing_escape() {
        let parsed = parse_command_parts("moon run :test -- --name retry\\");
        assert!(parsed.is_err());
    }

    #[test]
    fn test_validate_revision_pair_detects_mismatch() {
        let pinned = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        let current = Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string());
        let result = validate_revision_pair(&pinned, &current);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_revision_pair_allows_match() {
        let revision = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        let result = validate_revision_pair(&revision, &revision);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_full_sha_requires_40_hex_chars() {
        assert!(is_full_sha("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(!is_full_sha("short"));
        assert!(!is_full_sha("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"));
    }
}
