use super::{
    extract_exec_binary, extract_exec_start, extract_repo_slug_from_gh_output,
    has_required_services, is_valid_oya_exec_start, parse_admin_url, parse_host_port,
    parse_ingress_url, parse_repo_slug, parse_service_url,
};

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
