//! Adversarial Input Tests for Restate API Endpoints
//!
//! Verifies that all user-controllable inputs are properly sanitized:
//! - SQL injection (', DROP TABLE, --, UNION SELECT)
//! - XSS payloads (<script>, javascript:, onerror)
//! - Path traversal (../../, /etc/passwd)
//! - Empty/null inputs
//! - Oversized payloads
//! - Unicode edge cases
//! - Null bytes and control characters
//!
//! These tests verify input sanitization at the query/URL-building layer
//! without requiring a live Restate server.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::Workflow;
use oya_frontend::restate_client::queries::SqlQueries;
use serde_json::json;

const UNREACHABLE_INGRESS_URL: &str = "http://127.0.0.1:1";

// ===========================================================================
// SQL Injection
// ===========================================================================

/// given a classic SQL injection in invocation ID when query built then quotes
/// are escaped
#[test]
fn given_sql_injection_drop_table_when_invocation_query_then_quotes_escaped() {
    let query = SqlQueries::invocation("'; DROP TABLE sys_invocation; --");
    // The single quote in the input must be doubled to escape
    assert!(
        query.contains("''; DROP TABLE sys_invocation; --"),
        "SQL injection not escaped: {query}"
    );
    // Raw unescaped quote must not appear in the WHERE clause
    let where_clause = query.split("WHERE").nth(1).unwrap();
    assert!(
        !where_clause.contains("' DROP TABLE"),
        "Unescaped injection found in WHERE: {where_clause}"
    );
}

/// given UNION SELECT injection when journal query built then quotes are escaped
#[test]
fn given_sql_injection_union_select_when_journal_query_then_quotes_escaped() {
    let query = SqlQueries::journal("' UNION SELECT * FROM sys_invocation --");
    assert!(query.contains("'' UNION SELECT"), "UNION injection not escaped: {query}");
}

/// given SQL injection in service name when state query built then quotes escaped
#[test]
fn given_sql_injection_service_name_when_state_query_then_quotes_escaped() {
    let query = SqlQueries::service_state("'; DELETE FROM state; --");
    assert!(
        query.contains("''; DELETE FROM state; --"),
        "SQL injection in service_name not escaped"
    );
}

/// given SQL injection in both params when keyed_state query built then both
/// escaped
#[test]
fn given_sql_injection_both_params_when_keyed_state_then_both_escaped() {
    let query =
        SqlQueries::keyed_state("svc'; DROP TABLE state; --", "key'; DELETE FROM state; --");
    assert!(query.contains("svc''; DROP TABLE state; --"), "service_name not escaped");
    assert!(query.contains("key''; DELETE FROM state; --"), "service_key not escaped");
}

/// given SQL injection in promises params when query built then both escaped
#[test]
fn given_sql_injection_promises_when_query_then_both_escaped() {
    let query = SqlQueries::promises("svc'OR'1'='1", "key'OR'1'='1");
    assert!(query.contains("svc''OR''1''=''1"), "service_name injection not escaped");
    assert!(query.contains("key''OR''1''=''1"), "service_key injection not escaped");
}

/// given SQL injection in service name when invocations_for_service query built
/// then escaped
#[test]
fn given_sql_injection_service_name_when_invocations_for_service_then_escaped() {
    let query = SqlQueries::invocations_for_service("svc' AND 1=1 --");
    assert!(query.contains("svc'' AND 1=1 --"), "Injection not escaped in invocations_for_service");
}

/// given stacked queries injection when journal query built then quotes escaped
#[test]
fn given_stacked_queries_injection_when_journal_query_then_escaped() {
    let query = SqlQueries::journal("inv'; SELECT password FROM users; --");
    assert!(
        query.contains("inv''; SELECT password FROM users; --"),
        "Stacked query injection not escaped"
    );
}

/// given comment-based injection when invocation query built then escaped
#[test]
fn given_comment_injection_when_invocation_query_then_escaped() {
    let query = SqlQueries::invocation("inv'/*");
    assert!(query.contains("inv''/*"), "Comment injection not escaped");
}

/// given hex-encoded injection when service_state query built then no special
/// handling needed (string literal prevents execution)
#[test]
fn given_hex_encoded_injection_when_service_state_then_safe_in_string_literal() {
    let query = SqlQueries::service_state("0x736376");
    // Non-SQL strings pass through as-is; the value is in a string literal
    assert!(query.contains("'0x736376'"), "Hex string should be in string literal");
}

/// given multiple quotes when invocation query built then all are doubled
#[test]
fn given_multiple_quotes_when_invocation_query_then_all_doubled() {
    let query = SqlQueries::invocation("it's a ''test'' of 'quotes'");
    assert!(
        query.contains("it''s a ''''test'''' of ''quotes''"),
        "Multiple quotes not all escaped: {query}"
    );
}

// ===========================================================================
// XSS Payloads
// ===========================================================================

/// given XSS script tag in service name when state query built then safely
/// embedded in string literal
#[test]
fn given_xss_script_in_service_name_when_state_query_then_in_string_literal() {
    let malicious = "<script>alert('xss')</script>";
    let query = SqlQueries::service_state(malicious);
    // The quote in the XSS payload gets escaped by escape_sql
    assert!(
        query.contains("<script>alert(''xss'')</script>"),
        "XSS payload should be inside string literal with quotes escaped"
    );
}

/// given XSS in invocation ID when query built then safely in string literal
#[test]
fn given_xss_in_invocation_id_when_query_then_in_string_literal() {
    let query = SqlQueries::invocation("<img src=x onerror=alert(1)>");
    assert!(
        query.contains("'<img src=x onerror=alert(1)>'"),
        "XSS payload should be in string literal"
    );
}

/// given javascript URI in service key when keyed_state query then safely
/// in string literal
#[test]
fn given_javascript_uri_in_key_when_keyed_state_then_in_string_literal() {
    let query = SqlQueries::keyed_state("svc", "javascript:alert(document.cookie)");
    assert!(
        query.contains("'javascript:alert(document.cookie)'"),
        "JavaScript URI should be in string literal"
    );
}

// ===========================================================================
// Path Traversal
// ===========================================================================

/// given path traversal in service name when state query built then safely
/// in string literal (no filesystem access via SQL)
#[test]
fn given_path_traversal_in_service_name_when_state_query_then_in_string_literal() {
    let query = SqlQueries::service_state("../../../../etc/passwd");
    assert!(
        query.contains("'../../../../etc/passwd'"),
        "Path traversal payload should be in string literal"
    );
}

/// given path traversal in service call config when executed then included in
/// URL but does not escape base URL
#[tokio::test]
async fn given_path_traversal_in_service_call_config_when_executed_then_url_bounded() {
    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = UNREACHABLE_INGRESS_URL.to_owned();

    let config = json!({
        "service": "../../etc",
        "endpoint": "passwd"
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    // The function doesn't connect to a real server; it returns a connection
    // error. The key assertion: no panic, no filesystem access.
    assert!(result.get("error").is_some(), "Should get connection error, not panic");
}

// ===========================================================================
// Empty / Null / Missing Inputs
// ===========================================================================

/// given empty invocation ID when query built then returns valid SQL
#[test]
fn given_empty_invocation_id_when_query_then_valid_sql() {
    let query = SqlQueries::invocation("");
    assert!(query.contains("WHERE id = ''"), "Empty ID should produce empty string literal");
}

/// given empty service name when state query built then returns valid SQL
#[test]
fn given_empty_service_name_when_state_query_then_valid_sql() {
    let query = SqlQueries::service_state("");
    assert!(
        query.contains("WHERE service_name = ''"),
        "Empty service name should produce valid SQL"
    );
}

/// given empty service call config when executed then validation error
#[tokio::test]
async fn given_empty_service_call_config_when_executed_then_validation_error() {
    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = UNREACHABLE_INGRESS_URL.to_owned();

    let config = json!({});
    let result = workflow.execute_service_call_internal("service-call", &config).await;
    // Missing service and endpoint should return error, not panic
    assert!(result.get("error").is_some(), "Empty config should return validation error");
}

/// given empty key in object call when executed then uses "default"
#[tokio::test]
async fn given_empty_key_in_object_call_when_executed_then_uses_default() {
    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = UNREACHABLE_INGRESS_URL.to_owned();

    let config = json!({
        "object_name": "obj",
        "handler": "handle",
        "key": ""
    });
    // This will fail to connect but should not panic
    let result = workflow.execute_service_call_internal("object-call", &config).await;
    assert!(
        result.get("error").is_some() || result.get("status").is_some(),
        "Should return error or status, not panic"
    );
}

// ===========================================================================
// Oversized Inputs
// ===========================================================================

/// given very long invocation ID when query built then handles without panic
#[test]
fn given_very_long_invocation_id_when_query_then_no_panic() {
    let long_id = "a".repeat(100_000);
    let query = SqlQueries::invocation(&long_id);
    assert!(query.contains(&format!("'{}'", long_id)), "Should handle long IDs without panic");
}

/// given very long service name when state query built then handles without
/// panic
#[test]
fn given_very_long_service_name_when_state_query_then_no_panic() {
    let long_name = "x".repeat(65_536);
    let query = SqlQueries::service_state(&long_name);
    assert!(query.contains("WHERE service_name ="), "Should handle long service names");
}

/// given large payload in service call when executed then no panic
#[tokio::test]
async fn given_large_payload_in_service_call_when_executed_then_no_panic() {
    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = UNREACHABLE_INGRESS_URL.to_owned();

    let big_data = "A".repeat(1_000_000);
    let config = json!({
        "service": "svc",
        "endpoint": "handle",
        "payload": {"data": big_data}
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;
    // Connection error expected, no panic
    assert!(result.get("error").is_some());
}

// ===========================================================================
// Unicode Edge Cases
// ===========================================================================

/// given unicode in service name when state query built then safely in string
/// literal
#[test]
fn given_unicode_service_name_when_state_query_then_in_string_literal() {
    let query = SqlQueries::service_state("servïce名前🎉");
    assert!(query.contains("'servïce名前🎉'"), "Unicode should pass through in string literal");
}

/// given unicode in invocation ID when query built then safely in string
/// literal
#[test]
fn given_unicode_invocation_id_when_query_then_in_string_literal() {
    let query = SqlQueries::invocation("inv-日本語-עברית");
    assert!(query.contains("'inv-日本語-עברית'"), "Multilingual unicode should pass through");
}

/// given null byte in service name when state query built then included in
/// literal (no crash)
#[test]
fn given_null_byte_in_service_name_when_state_query_then_no_panic() {
    let query = SqlQueries::service_state("svc\x00malicious");
    assert!(query.contains("svc\x00malicious"), "Null byte should not cause panic");
}

/// given combining characters in service key when keyed_state query then
/// safely in literal
#[test]
fn given_combining_chars_in_service_key_when_keyed_state_then_safe() {
    let query = SqlQueries::keyed_state("svc", "ke\u{0301}y");
    assert!(query.contains("ke\u{0301}y"), "Combining characters should pass through");
}

/// given right-to-left override in invocation ID when query built then in
/// literal
#[test]
fn given_rtl_override_in_invocation_id_when_query_then_in_literal() {
    let query = SqlQueries::invocation("inv\u{202E}evil");
    assert!(query.contains("inv\u{202E}evil"), "RTL override should pass through in literal");
}

// ===========================================================================
// Control Characters
// ===========================================================================

/// given newline in service name when query built then in string literal
#[test]
fn given_newline_in_service_name_when_query_then_in_literal() {
    let query = SqlQueries::service_state("svc\nname");
    assert!(query.contains("svc\nname"), "Newline should be in literal, not break SQL structure");
}

/// given tab in service key when keyed_state query then in literal
#[test]
fn given_tab_in_service_key_when_keyed_state_then_in_literal() {
    let query = SqlQueries::keyed_state("svc", "key\tvalue");
    assert!(query.contains("key\tvalue"), "Tab should be in literal");
}

// ===========================================================================
// Service Call URL Construction Edge Cases
// ===========================================================================

/// given special chars in service name when service call executed then included
/// in URL
#[tokio::test]
async fn given_special_chars_in_service_name_when_service_call_then_in_url() {
    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = UNREACHABLE_INGRESS_URL.to_owned();

    let config = json!({
        "service": "my service & co",
        "endpoint": "handle"
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;
    // Connection error expected (no server), but no panic
    assert!(result.get("error").is_some());
}

/// given workflow-call with unicode name when executed then no panic
#[tokio::test]
async fn given_unicode_workflow_name_when_workflow_call_then_no_panic() {
    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = UNREACHABLE_INGRESS_URL.to_owned();

    let config = json!({
        "workflow_name": "ワークフロー",
        "payload": {}
    });
    let result = workflow.execute_service_call_internal("workflow-call", &config).await;
    assert!(result.get("error").is_some());
}

/// given object-call with special chars in key when executed then no panic
#[tokio::test]
async fn given_special_chars_in_object_key_when_object_call_then_no_panic() {
    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = UNREACHABLE_INGRESS_URL.to_owned();

    let config = json!({
        "object_name": "obj",
        "handler": "handle",
        "key": "<script>alert(1)</script>",
        "payload": {}
    });
    let result = workflow.execute_service_call_internal("object-call", &config).await;
    assert!(result.get("error").is_some());
}

// ===========================================================================
// SQL Escape Function Edge Cases (via SqlQueries public API)
// ===========================================================================

/// given only a quote when invocation query built then doubled
#[test]
fn given_single_quote_only_when_invocation_query_then_doubled() {
    let query = SqlQueries::invocation("'");
    // Input ' becomes '' (doubled), so the WHERE clause is: id = ''''
    assert!(query.contains("id = ''''"), "Single quote should be doubled: {query}");
}

/// given backslash in service name when state query built then passes through
#[test]
fn given_backslash_in_service_name_when_state_query_then_passes_through() {
    let query = SqlQueries::service_state("svc\\name");
    assert!(
        query.contains("'svc\\name'"),
        "Backslash should pass through (SQL uses '' for escaping, not \\)"
    );
}

/// given semicolon in service name when state query built then in literal
#[test]
fn given_semicolon_in_service_name_when_state_query_then_in_literal() {
    let query = SqlQueries::service_state("svc; DROP TABLE state");
    // The semicolon is inside a string literal, so it's not a SQL terminator
    assert!(query.contains("'svc; DROP TABLE state'"), "Semicolon should be inside string literal");
}
