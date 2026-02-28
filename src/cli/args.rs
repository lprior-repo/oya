#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

const DEFAULT_BIND: &str = "127.0.0.1:9180";
const DEFAULT_INGRESS: &str = "http://127.0.0.1:909";
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
    #[arg(long)]
    #[arg(long, value_parser = parse_non_empty_text)]
    pub prompt: String,
    #[arg(long)]
    pub model: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct ImplementArgs {
    #[arg(long, value_parser = parse_object_key)]
    pub bead: Option<String>,
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
    #[arg(long, default_value = DEFAULT_IMPL_MODEL)]
    pub model: String,
}

#[derive(Debug, clap::Args)]
pub struct LifecycleArgs {
    #[arg(long, value_parser = parse_object_key)]
    pub bead: Option<String>,
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    pub ingress: String,
    #[arg(long, default_value = DEFAULT_IMPL_MODEL)]
    pub model: String,
    #[arg(long, value_parser = parse_repo_slug)]
    pub repo: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    #[arg(long, value_parser = parse_object_key)]
    pub key: String,
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
    parse_url_with_expected_port(value, 909, "ingress")
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
    let port =
        parsed.port_or_known_default().ok_or_else(|| format!("{label} URL must include port"))?;
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
    if trimmed.contains('/') {
        return Err("key/id must not contain '/'".to_owned());
    }
    Ok(trimmed.to_owned())
}

pub fn parse_non_empty_text(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(trimmed.to_owned())
    }
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
        assert!(parse_object_key("abc/123").is_err());
    }

    #[test]
    fn parse_non_empty_text_rejects_blank_values() {
        assert!(parse_non_empty_text("hello").is_ok());
        assert!(parse_non_empty_text(" ").is_err());
    }
}
