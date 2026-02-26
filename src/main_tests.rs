use super::parse_repo_slug;

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
