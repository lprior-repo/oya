#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

use crate::lifecycle::types::{BeadId, GateId, RunId};

const DEFAULT_BIND: &str = "127.0.0.1:9180";
const DEFAULT_INGRESS: &str = "http://127.0.0.1:8080";
const DEFAULT_ADMIN: &str = "http://127.0.0.1:9070";
const DEFAULT_IMPL_MODEL: &str = "zai-coding-plan/glm-5";
const DEFAULT_SERVICE_URL: &str = "http://127.0.0.1:9180/";

#[derive(Debug, Parser)]
#[command(name = "oya")]
#[command(about = "OIA -> Restate -> OpenCode bridge")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init(InitArgs),
    Doctor(DoctorArgs),
    Serve(ServeArgs),
    Invoke(InvokeArgs),
    Run(RunArgs),
    Evidence(EvidenceArgs),
    Verify(VerifyArgs),
    Explain(ExplainArgs),
    Report(ReportArgs),
    Implement(ImplementArgs),
    Lifecycle(LifecycleArgs),
    Status(StatusArgs),
    Cancel(CancelArgs),
    Beads(BeadsArgs),
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
    #[arg(long, default_value = DEFAULT_SERVICE_URL, value_parser = parse_service_url)]
    pub service_url: String,
    #[arg(long)]
    pub down: bool,
}

#[derive(Debug, clap::Args)]
pub struct DoctorArgs {
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
    #[arg(long, default_value = DEFAULT_ADMIN, value_parser = parse_admin_url)]
    pub admin: String,
    #[arg(long, default_value = DEFAULT_SERVICE_URL, value_parser = parse_service_url)]
    pub service_url: String,
}

#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    #[arg(long, default_value = DEFAULT_BIND)]
    pub bind: String,
}

#[derive(Debug, clap::Args)]
pub struct InvokeArgs {
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
    #[arg(long, default_value = "default", value_parser = parse_object_key)]
    pub id: String,
    #[arg(long, value_parser = parse_non_empty_text)]
    pub prompt: String,
    #[arg(long, value_parser = parse_model_name)]
    pub model: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    #[arg(long, value_parser = parse_bead_id)]
    pub bead_id: String,
    #[arg(long, value_parser = parse_non_empty_text)]
    pub prompt: String,
    #[arg(long, default_value = DEFAULT_IMPL_MODEL, value_parser = parse_model_name)]
    pub model: String,
}

#[derive(Debug, clap::Args)]
pub struct EvidenceArgs {
    #[command(subcommand)]
    pub command: EvidenceCommand,
}

#[derive(Debug, Subcommand)]
pub enum EvidenceCommand {
    Check(EvidenceCheckArgs),
}

#[derive(Debug, clap::Args)]
pub struct EvidenceCheckArgs {
    #[arg(long, default_value = "run-demo", value_parser = parse_run_id)]
    pub run_id: String,
}

#[derive(Debug, clap::Args)]
pub struct VerifyArgs {
    #[arg(long, value_parser = parse_bead_id)]
    pub bead_id: String,
    #[arg(long, default_value = "fmt", value_parser = parse_gate_id)]
    pub gate: String,
    #[arg(long)]
    pub repair: bool,
}

#[derive(Debug, clap::Args)]
pub struct ExplainArgs {
    #[arg(long, value_parser = parse_finding_id)]
    pub finding_id: String,
}

#[derive(Debug, clap::Args)]
pub struct ReportArgs {
    #[arg(long, value_parser = parse_report_run_id)]
    pub run_id: String,
}

#[derive(Debug, clap::Args)]
pub struct ImplementArgs {
    #[arg(long, value_parser = parse_bead_id)]
    pub bead: Option<String>,
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
    #[arg(long, default_value = DEFAULT_IMPL_MODEL, value_parser = parse_model_name)]
    pub model: String,
}

#[derive(Debug, clap::Args)]
pub struct LifecycleArgs {
    #[arg(long, value_parser = parse_bead_id)]
    pub bead: Option<String>,
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
    #[arg(long, default_value = DEFAULT_IMPL_MODEL, value_parser = parse_model_name)]
    pub model: String,
    #[arg(long, value_parser = parse_repo_slug)]
    pub repo: Option<String>,
}

#[derive(Debug, clap::Args)]
#[command(group(
    clap::ArgGroup::new("status_target")
        .required(true)
        .args(["key", "run_id"])
))]
pub struct StatusArgs {
    #[arg(long, value_parser = parse_object_key)]
    pub key: Option<String>,
    #[arg(long, value_parser = parse_run_id)]
    pub run_id: Option<String>,
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
}

#[derive(Debug, clap::Args)]
pub struct CancelArgs {
    #[arg(long, value_parser = parse_object_key)]
    pub key: String,
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
}

#[derive(Debug, clap::Args)]
pub struct BeadsArgs {
    #[arg(long)]
    pub ready: bool,
    #[arg(long)]
    pub json: bool,
}

pub fn parse_ingress_url(value: &str) -> Result<String, String> {
    parse_url_with_expected_port(value, 8080, "ingress")
}

pub fn parse_service_url(value: &str) -> Result<String, String> {
    parse_url_with_expected_port(value, 9180, "service")
}

pub fn parse_admin_url(value: &str) -> Result<String, String> {
    parse_url_with_expected_port(value, 9070, "admin")
}

fn parse_url_with_expected_port(
    value: &str,
    expected_port: u16,
    label: &str,
) -> Result<String, String> {
    let parsed = url::Url::parse(value).map_err(|error| format!("invalid {label} URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("{label} URL must use http or https"));
    }
    let port =
        parsed.port_or_known_default().ok_or_else(|| format!("{label} URL must include port"))?;
    if parsed.path() != "/" && !parsed.path().is_empty() {
        return Err(format!("{label} URL must not include a path suffix"));
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(format!("{label} URL must not include query or fragment"));
    }
    if port == expected_port {
        Ok(value.to_owned())
    } else {
        Err(format!("{label} URL must use port {expected_port} (avoid common 8080/80 ports)"))
    }
}

pub fn parse_repo_slug(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let (owner, repo) =
        trimmed.split_once('/').ok_or_else(|| "expected OWNER/REPO format".to_owned())?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return Err("expected OWNER/REPO format".to_owned());
    }
    if is_valid_repo_part(owner) && is_valid_repo_part(repo) {
        Ok(trimmed.to_owned())
    } else {
        Err("repo may contain only [A-Za-z0-9._-]".to_owned())
    }
}

pub fn parse_object_key(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("key/id must not be empty".to_owned());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err("key/id must not contain whitespace".to_owned());
    }
    if trimmed.contains('/') {
        return Err("key/id must not contain '/'".to_owned());
    }
    if !trimmed.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')) {
        return Err("key/id may contain only [A-Za-z0-9._-]".to_owned());
    }
    Ok(trimmed.to_owned())
}

pub fn parse_bead_id(value: &str) -> Result<String, String> {
    BeadId::parse(value)
        .map(|bead_id| bead_id.as_str().to_owned())
        .map_err(|error| format!("invalid bead id: {error}"))
}

pub fn parse_run_id(value: &str) -> Result<String, String> {
    RunId::parse(value)
        .map(|run_id| run_id.as_str().to_owned())
        .map_err(|error| format!("invalid run id: {error}"))
}

pub fn parse_gate_id(value: &str) -> Result<String, String> {
    GateId::parse(value)
        .map(|gate_id| gate_id.as_str().to_owned())
        .map_err(|error| format!("invalid gate id: {error}"))
}

pub fn parse_finding_id(value: &str) -> Result<String, String> {
    parse_object_key(value).map_err(|error| format!("invalid finding id: {error}"))
}

pub fn parse_report_run_id(value: &str) -> Result<String, String> {
    match RunId::parse(value) {
        Ok(run_id) => Ok(run_id.as_str().to_owned()),
        Err(_) => BeadId::parse(value)
            .map(|bead_id| RunId::from_bead_id(&bead_id).as_str().to_owned())
            .map_err(|error| format!("invalid report run id: {error}")),
    }
}

pub fn parse_non_empty_text(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

pub fn parse_model_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("model must not be empty".to_owned());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/' | ':'))
    {
        return Err("model may contain only [A-Za-z0-9._:/-]".to_owned());
    }
    Ok(trimmed.to_owned())
}

fn is_valid_repo_part(value: &str) -> bool {
    value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_repo_slug_validates_format() {
        assert!(parse_repo_slug("owner/repo").is_ok());
        assert!(parse_repo_slug("owner-name/repo_name").is_ok());
        assert!(parse_repo_slug("invalid").is_err());
        assert!(parse_repo_slug("owner/repo/extra").is_err());
    }

    #[test]
    fn parse_object_key_rejects_empty_and_slash() {
        assert!(parse_object_key("abc-123").is_ok());
        assert!(parse_object_key("  ").is_err());
        assert!(parse_object_key("abc 123").is_err());
        assert!(parse_object_key("abc/123").is_err());
        assert!(parse_object_key("semi;colon").is_err());
    }

    #[test]
    fn parse_bead_id_uses_strict_lowercase_slug_rules() {
        assert_eq!(parse_bead_id(" oya-8y3 "), Ok("oya-8y3".to_owned()));
        assert!(parse_bead_id("bad/../id").is_err());
        assert!(parse_bead_id("bad.id").is_err());
        assert!(parse_bead_id("bad_id").is_err());
        assert!(parse_bead_id("BAD-ID").is_err());
    }

    #[test]
    fn parse_non_empty_text_rejects_blank_values() {
        assert!(parse_non_empty_text("hello").is_ok());
        assert!(parse_non_empty_text(" ").is_err());
        assert_eq!(parse_non_empty_text("  hi  ").unwrap_or_else(|_| "".to_string()), "  hi  ");
    }

    #[test]
    fn parse_model_name_rejects_empty_and_invalid_chars() {
        assert!(parse_model_name("zai-coding-plan/glm-5").is_ok());
        assert!(parse_model_name("provider:model-v1").is_ok());
        assert!(parse_model_name(" ").is_err());
        assert!(parse_model_name("bad model").is_err());
    }

    #[test]
    fn no_docker_ingress_accepts_rootless_restate_port() {
        assert_eq!(
            parse_ingress_url("http://127.0.0.1:8080"),
            Ok("http://127.0.0.1:8080".to_owned())
        );
        assert!(parse_ingress_url("http://127.0.0.1:909").is_err());
    }

    #[test]
    fn cli_parses_run_command_with_demo_bead_and_prompt() {
        let parsed = Cli::try_parse_from(["oya", "run", "--bead-id", "demo", "--prompt", "noop"]);
        let Ok(cli) = parsed else {
            assert!(false, "run command should parse");
            return;
        };

        match cli.command {
            Command::Run(args) => {
                assert_eq!(args.bead_id, "demo");
                assert_eq!(args.prompt, "noop");
                assert_eq!(args.model, DEFAULT_IMPL_MODEL);
            }
            _ => assert!(false, "expected run command"),
        }
    }

    #[test]
    fn cli_parses_run_command_with_explicit_model() {
        let parsed = Cli::try_parse_from([
            "oya",
            "run",
            "--bead-id",
            "demo",
            "--prompt",
            "noop",
            "--model",
            "bad/model",
        ]);
        let Ok(cli) = parsed else {
            assert!(false, "run command should parse explicit model");
            return;
        };

        match cli.command {
            Command::Run(args) => assert_eq!(args.model, "bad/model"),
            _ => assert!(false, "expected run command"),
        }
    }

    #[test]
    fn cli_parses_status_command_by_run_id() {
        let parsed = Cli::try_parse_from(["oya", "status", "--run-id", "run-demo"]);
        let Ok(cli) = parsed else {
            assert!(false, "status by run id should parse");
            return;
        };

        match cli.command {
            Command::Status(args) => {
                assert_eq!(args.key, None);
                assert_eq!(args.run_id, Some("run-demo".to_owned()));
            }
            _ => assert!(false, "expected status command"),
        }
    }

    #[test]
    fn cli_parses_evidence_check_with_default_run_id() {
        let parsed = Cli::try_parse_from(["oya", "evidence", "check"]);
        let Ok(cli) = parsed else {
            assert!(false, "evidence check should parse");
            return;
        };

        match cli.command {
            Command::Evidence(args) => match args.command {
                EvidenceCommand::Check(check) => assert_eq!(check.run_id, "run-demo"),
            },
            _ => assert!(false, "expected evidence command"),
        }
    }

    #[test]
    fn cli_parses_verify_command_with_demo_bead_and_fmt_gate() {
        let parsed = Cli::try_parse_from(["oya", "verify", "--bead-id", "demo", "--gate", "fmt"]);
        let Ok(cli) = parsed else {
            assert!(false, "verify command should parse");
            return;
        };

        match cli.command {
            Command::Verify(args) => {
                assert_eq!(args.bead_id, "demo");
                assert_eq!(args.gate, "fmt");
            }
            _ => assert!(false, "expected verify command"),
        }
    }

    #[test]
    fn cli_parses_verify_command_with_default_fmt_gate() {
        let parsed = Cli::try_parse_from(["oya", "verify", "--bead-id", "demo"]);
        let Ok(cli) = parsed else {
            assert!(false, "verify command should parse with default gate");
            return;
        };

        match cli.command {
            Command::Verify(args) => {
                assert_eq!(args.bead_id, "demo");
                assert_eq!(args.gate, "fmt");
                assert!(!args.repair);
            }
            _ => assert!(false, "expected verify command"),
        }
    }

    #[test]
    fn cli_parses_verify_repair_command() {
        let parsed = Cli::try_parse_from(["oya", "verify", "--bead-id", "demo", "--repair"]);
        let Ok(cli) = parsed else {
            assert!(false, "verify repair command should parse");
            return;
        };

        match cli.command {
            Command::Verify(args) => {
                assert_eq!(args.bead_id, "demo");
                assert_eq!(args.gate, "fmt");
                assert!(args.repair);
            }
            _ => assert!(false, "expected verify command"),
        }
    }

    #[test]
    fn cli_parses_explain_command_with_demo_finding_id() {
        let parsed = Cli::try_parse_from(["oya", "explain", "--finding-id", "demo"]);
        let Ok(cli) = parsed else {
            assert!(false, "explain command should parse");
            return;
        };

        match cli.command {
            Command::Explain(args) => assert_eq!(args.finding_id, "demo"),
            _ => assert!(false, "expected explain command"),
        }
    }

    #[test]
    fn cli_parses_report_command_with_demo_alias() {
        let parsed = Cli::try_parse_from(["oya", "report", "--run-id", "demo"]);
        let Ok(cli) = parsed else {
            assert!(false, "report command should parse demo alias");
            return;
        };

        match cli.command {
            Command::Report(args) => assert_eq!(args.run_id, "run-demo"),
            _ => assert!(false, "expected report command"),
        }
    }
}
