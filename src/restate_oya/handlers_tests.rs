use super::handlers::is_lifecycle_not_found;

#[test]
fn test_is_lifecycle_not_found_detects_empty_response() {
    assert!(is_lifecycle_not_found(""));
    assert!(is_lifecycle_not_found("   "));
    assert!(is_lifecycle_not_found("\n\n"));
}

#[test]
fn test_is_lifecycle_not_found_returns_false_for_running_lifecycle() {
    assert!(!is_lifecycle_not_found("Status: running\nCommand: moon run :ci"));
    assert!(!is_lifecycle_not_found("Status: completed\n"));
}

#[test]
fn test_is_lifecycle_not_found_returns_false_for_backing_off_lifecycle() {
    assert!(!is_lifecycle_not_found("Status: backing-off\nError: transient failure"));
}

#[test]
fn test_is_lifecycle_not_found_returns_false_for_error_messages() {
    assert!(!is_lifecycle_not_found("No invocations matched the query"));
    assert!(!is_lifecycle_not_found("Error: not found in registry"));
    assert!(!is_lifecycle_not_found("Error: invocation not found"));
}
