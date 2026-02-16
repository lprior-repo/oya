pub fn zjj_done_has_constraint_violation(output: &str) -> bool {
    output.contains("CHECK constraint failed")
        && output.contains("closed_at")
        && output.contains("status")
}
