use oya::infrastructure::zjj::zjj_done_has_constraint_violation;

#[test]
fn detects_closed_at_constraint_violations_in_zjj_output() {
    let output =
        "Database error: CHECK constraint failed: (status = 'closed' AND closed_at IS NOT NULL)";
    assert!(zjj_done_has_constraint_violation(output));
}

#[test]
fn ignores_non_constraint_zjj_errors() {
    let output = "Error: network timeout while contacting remote";
    assert!(!zjj_done_has_constraint_violation(output));
}
