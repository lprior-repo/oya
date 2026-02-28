use crate::cli::args::parse_repo_slug;
use crate::cli::args::{parse_admin_url, parse_ingress_url, parse_service_url};
use crate::cli::commands::{decode_bead_entries, is_uninitialized_snapshot, BeadEntry};
use crate::cli::doctor::{has_required_services, parse_host_port};
use crate::cli::init::{
    extract_exec_binary, extract_exec_start, is_missing_container_error, is_valid_oya_exec_start,
};
use crate::cli::repo::{
    ensure_repo_matches_jj_origin, extract_repo_slug_from_gh_output,
    extract_repo_slug_from_jj_remote_output, format_repo_lookup_error_json, gh_repo_view_args,
    is_retryable_repo_lookup_stderr, normalize_error_message, parse_repo_slug_from_remote_url,
};
use crate::cli::restate::{format_http_error, normalize_http_error_body, parse_json_payload};
use crate::restate_oya::types::LifecycleStatusSnapshot;
use serde_json::json;

#[test]
fn parse_repo_slug_accepts_valid_owner_repo() {
    let parsed = parse_repo_slug("lprior-repo/oya");
    assert_eq!(parsed, Ok("lprior-repo/oya".to_owned()));
}

#[test]
fn parse_repo_slug_rejects_missing_separator() {
    let parsed = parse_repo_slug("lprior-repo");
    assert!(parsed.is_err());
}

#[test]
fn parse_repo_slug_rejects_extra_path_segments() {
    let parsed = parse_repo_slug("owner/repo/extra");
    assert!(parsed.is_err());
}

#[test]
fn parse_repo_slug_rejects_invalid_chars() {
    let parsed = parse_repo_slug("owner/repo with spaces");
    assert!(parsed.is_err());
}

#[test]
fn has_required_services_accepts_expected_service_set() {
    let output = "Oya\nOyaMemory\nOyaService\n";
    assert!(has_required_services(output));
}

#[test]
fn has_required_services_rejects_missing_service() {
    let output = "Oya\nOyaMemory\n";
    assert!(!has_required_services(output));
}

#[test]
fn parse_ingress_url_accepts_port_909() {
    let parsed = parse_ingress_url("http://127.0.0.1:909");
    assert_eq!(parsed, Ok("http://127.0.0.1:909".to_owned()));
}

#[test]
fn parse_ingress_url_rejects_non_909_port() {
    let parsed = parse_ingress_url("http://127.0.0.1:9080");
    assert!(parsed.is_err());
}

#[test]
fn parse_ingress_url_rejects_non_http_scheme() {
    let parsed = parse_ingress_url("ftp://127.0.0.1:909");
    assert!(parsed.is_err());
}

#[test]
fn parse_ingress_url_rejects_path_suffix() {
    let parsed = parse_ingress_url("http://127.0.0.1:909/extra");
    assert!(parsed.is_err());
}

#[test]
fn parse_service_url_accepts_container_endpoint() {
    let parsed = parse_service_url("http://127.0.0.1:9180/");
    assert_eq!(parsed, Ok("http://127.0.0.1:9180/".to_owned()));
}

#[test]
fn parse_admin_url_accepts_port_9070() {
    let parsed = parse_admin_url("http://127.0.0.1:9070");
    assert_eq!(parsed, Ok("http://127.0.0.1:9070".to_owned()));
}

#[test]
fn extract_exec_start_reads_service_line() {
    let unit = "[Service]\nExecStart=/home/lewis/.local/bin/oya serve --bind 127.0.0.1:9180\n";
    let parsed = extract_exec_start(unit);
    assert_eq!(parsed, Some("/home/lewis/.local/bin/oya serve --bind 127.0.0.1:9180"));
}

#[test]
fn extract_exec_binary_returns_command_path() {
    let parsed = extract_exec_binary("/home/lewis/.local/bin/oya serve --bind 127.0.0.1:9180");
    assert_eq!(parsed, Some("/home/lewis/.local/bin/oya"));
}

#[test]
fn parse_host_port_accepts_remote_host_with_expected_port() {
    let parsed = parse_host_port("http://example.internal:9070", 9070);
    assert_eq!(parsed, Ok(("example.internal".to_owned(), 9070)));
}

#[test]
fn parse_host_port_rejects_unexpected_port() {
    let parsed = parse_host_port("http://127.0.0.1:9180", 9070);
    assert!(parsed.is_err());
}

#[test]
fn is_valid_oya_exec_start_checks_port_and_command() {
    let valid = is_valid_oya_exec_start("/home/lewis/.local/bin/oya serve --bind 127.0.0.1:9180");
    let invalid = is_valid_oya_exec_start("/home/lewis/.local/bin/oya serve --bind 127.0.0.1:9080");
    assert!(valid);
    assert!(!invalid);
}

#[test]
fn extract_repo_slug_from_gh_output_accepts_name_with_owner() {
    let raw = r#"{"nameWithOwner":"lprior-repo/oya"}"#;
    let parsed = extract_repo_slug_from_gh_output(raw);
    assert!(matches!(parsed, Ok(value) if value == "lprior-repo/oya"));
}

#[test]
fn extract_repo_slug_from_gh_output_rejects_invalid_repo_name() {
    let raw = r#"{"nameWithOwner":"bad repo/oya"}"#;
    let parsed = extract_repo_slug_from_gh_output(raw);
    assert!(parsed.is_err());
}

#[test]
fn extract_repo_slug_from_jj_remote_output_reads_origin_slug() {
    let raw =
        "origin https://github.com/lprior-repo/oya.git\nbackup https://github.com/other/repo.git\n";
    let parsed = extract_repo_slug_from_jj_remote_output(raw);
    assert!(matches!(parsed, Ok(Some(value)) if value == "lprior-repo/oya"));
}

#[test]
fn extract_repo_slug_from_jj_remote_output_accepts_ssh_origin_slug() {
    let raw = "origin git@github.com:lprior-repo/oya.git\n";
    let parsed = extract_repo_slug_from_jj_remote_output(raw);
    assert!(matches!(parsed, Ok(Some(value)) if value == "lprior-repo/oya"));
}

#[test]
fn extract_repo_slug_from_jj_remote_output_returns_none_without_origin() {
    let raw = "upstream https://github.com/lprior-repo/oya.git\n";
    let parsed = extract_repo_slug_from_jj_remote_output(raw);
    assert!(matches!(parsed, Ok(None)));
}

#[test]
fn parse_repo_slug_from_remote_url_rejects_non_github_remote() {
    let parsed = parse_repo_slug_from_remote_url("https://gitlab.com/lprior-repo/oya.git");
    assert!(parsed.is_err());
}

#[test]
fn parse_repo_slug_from_remote_url_accepts_https_without_dot_git() {
    let parsed = parse_repo_slug_from_remote_url("https://github.com/lprior-repo/oya");
    assert!(matches!(parsed, Ok(value) if value == "lprior-repo/oya"));
}

#[test]
fn parse_repo_slug_from_remote_url_accepts_ssh_scheme_form() {
    let parsed = parse_repo_slug_from_remote_url("ssh://git@github.com/lprior-repo/oya.git");
    assert!(matches!(parsed, Ok(value) if value == "lprior-repo/oya"));
}

#[test]
fn parse_repo_slug_from_remote_url_rejects_missing_repo_segment() {
    let parsed = parse_repo_slug_from_remote_url("https://github.com/lprior-repo");
    assert!(parsed.is_err());
}

#[test]
fn ensure_repo_matches_jj_origin_accepts_matching_values() {
    let result = ensure_repo_matches_jj_origin("lprior-repo/oya", Some("lprior-repo/oya"));
    assert!(result.is_ok());
}

#[test]
fn ensure_repo_matches_jj_origin_rejects_mismatch() {
    let result =
        ensure_repo_matches_jj_origin("lprior-repo/claude-skills", Some("lprior-repo/oya"));
    assert!(result.is_err());
}

#[test]
fn is_retryable_repo_lookup_stderr_accepts_transient_errors() {
    let transient = "HTTP 503 Service Unavailable: please try again";
    assert!(is_retryable_repo_lookup_stderr(transient));
}

#[test]
fn is_retryable_repo_lookup_stderr_rejects_not_found() {
    let not_found = "GraphQL: Could not resolve to a Repository with the name";
    assert!(!is_retryable_repo_lookup_stderr(not_found));
}

#[test]
fn normalize_error_message_collapses_whitespace() {
    let normalized = normalize_error_message("bad\n  request\t from   gh");
    assert_eq!(normalized, "bad request from gh");
}

#[test]
fn format_repo_lookup_error_json_emits_pretty_json_payload() {
    let payload = format_repo_lookup_error_json("lprior-repo/oya", 2, 3, true, "  timeout\nretry ");
    let parsed: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
    assert_eq!(parsed["category"], "repo_lookup");
    assert_eq!(parsed["repo"], "lprior-repo/oya");
    assert_eq!(parsed["attempt"], 2);
    assert_eq!(parsed["max_retries"], 3);
    assert_eq!(parsed["retryable"], true);
    assert_eq!(parsed["message"], "timeout retry");
}

#[test]
fn gh_repo_view_args_uses_positional_repo_argument() {
    let args = gh_repo_view_args("lprior-repo/opencode-hypr-notifier");
    assert_eq!(
        args,
        ["repo", "view", "lprior-repo/opencode-hypr-notifier", "--json", "nameWithOwner",]
    );
}

#[test]
fn format_http_error_includes_response_body_when_present() {
    let url = reqwest::Url::parse("http://127.0.0.1:909/Oya/test/run").expect("valid URL");
    let message = format_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, &url, " boom ");
    assert_eq!(
        message,
        "HTTP status 500 Internal Server Error for url (http://127.0.0.1:909/Oya/test/run): boom"
    );
}

#[test]
fn is_missing_container_error_detects_docker_message() {
    assert!(is_missing_container_error(
        "Error response from daemon: No such container: oya-restate"
    ));
    assert!(!is_missing_container_error("permission denied"));
}

#[test]
fn parse_json_payload_skips_non_json_prefix_with_braces() {
    let raw = "warn: metadata {not json}\n{\"items\":[{\"id\":\"src-1\"}]}";
    let parsed = parse_json_payload(raw).expect("should parse trailing JSON");
    assert_eq!(parsed["items"][0]["id"], "src-1");
}

#[test]
fn normalize_http_error_body_extracts_nested_terminal_message() {
    let raw = r#"{"message":"{\"error\":{\"Terminal\":{\"message\":\"invalid model: empty\"}}}"}"#;
    let normalized = normalize_http_error_body(raw);
    assert_eq!(normalized, "invalid model: empty");
}

#[test]
fn normalize_http_error_body_simplifies_terminal_json_payload() {
    let raw = r#"{"message":"Br { args: [\"update\"] } exited with Some(3): {\"error\":{\"message\":\"Issue not found: src-x\"}}"}"#;
    let normalized = normalize_http_error_body(raw);
    assert_eq!(normalized, "Issue not found: src-x");
}

#[test]
fn normalize_http_error_body_simplifies_raw_terminal_payload() {
    let raw = r#"Br { args: [\"update\"] } exited with Some(3): {"error":{"message":"Issue not found: src-y"}}"#;
    let normalized = normalize_http_error_body(raw);
    assert_eq!(normalized, "Issue not found: src-y");
}

#[test]
fn bead_entry_parses_valid_json() {
    let raw =
        r#"{"id":"src-abc","title":"test bead","status":"ready","priority":1,"issue_type":"task"}"#;
    let entry: BeadEntry = serde_json::from_str(raw).expect("should parse");
    assert_eq!(entry.id, "src-abc");
    assert_eq!(entry.title, "test bead");
    assert_eq!(entry.status, "ready");
    assert_eq!(entry.priority, 1);
    assert_eq!(entry.issue_type, "task");
}

#[test]
fn bead_entry_serializes_to_json() {
    let entry = BeadEntry {
        id: "src-xyz".to_owned(),
        title: "sample".to_owned(),
        status: "blocked".to_owned(),
        priority: 2,
        issue_type: "feature".to_owned(),
    };
    let json = serde_json::to_string(&entry).expect("should serialize");
    assert!(json.contains("src-xyz"));
    assert!(json.contains("sample"));
}

#[test]
fn bead_entry_parses_ready_payload_with_type_alias() {
    let payload = json!([
        {
            "id": "src-abc",
            "title": "test bead",
            "status": "open",
            "priority": 0,
            "type": "bug"
        }
    ]);
    let parsed = decode_bead_entries(payload).expect("should parse ready payload");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].issue_type, "bug");
}

#[test]
fn decode_bead_entries_rejects_object_without_items() {
    let payload = json!({"unexpected": []});
    let parsed = decode_bead_entries(payload);
    assert!(parsed.is_err());
}

#[test]
fn is_uninitialized_snapshot_detects_empty_status_payload() {
    let snapshot = LifecycleStatusSnapshot {
        bead_id: None,
        steps: Vec::new(),
        gates: Vec::new(),
        discipline_gates: Vec::new(),
        state: None,
        pr_url: None,
        done: false,
        success: None,
        message: None,
        compensation_diagnostics: Vec::new(),
    };
    assert!(is_uninitialized_snapshot(&snapshot));
}
