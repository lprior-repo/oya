//! Python language stage execution.

use std::path::Path;

use crate::{
    error::{Error, Result},
    process::run_command,
};

/// Execute a Python pipeline stage.
pub fn execute_python_stage(stage_name: &str, cwd: &Path) -> Result<()> {
    match stage_name {
        "implement" => python_implement(cwd),
        "unit-test" => python_unit_test(cwd),
        "coverage" => python_coverage(cwd),
        "lint" => python_lint(cwd),
        "static" => python_static(cwd),
        "integration" => python_integration(cwd),
        "security" => python_security(cwd),
        "review" => python_review(cwd),
        "accept" => python_accept(cwd),
        other => Err(Error::UnknownStage {
            name: other.to_string(),
        }),
    }
}

/// Get preview for Python stage.
pub fn get_python_preview(stage_name: &str) -> (String, u64) {
    match stage_name {
        "implement" => ("python -m py_compile".to_string(), 5000),
        "unit-test" => ("pytest".to_string(), 30000),
        "coverage" => ("pytest --cov".to_string(), 45000),
        "lint" => ("ruff check .".to_string(), 5000),
        "static" => ("mypy .".to_string(), 30000),
        "integration" => ("pytest -m integration".to_string(), 60000),
        "security" => ("bandit -r .".to_string(), 15000),
        "review" => ("grep -r TODO/FIXME".to_string(), 2000),
        "accept" => ("python -m py_compile && pytest && ruff".to_string(), 60000),
        _ => ("unknown".to_string(), 0),
    }
}

fn python_implement(cwd: &Path) -> Result<()> {
    // Try to compile all Python files
    run_command("python", &["-m", "py_compile"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Python", "implement", "Code does not compile"))
}

fn python_unit_test(cwd: &Path) -> Result<()> {
    run_command("pytest", &[], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Python", "unit-test", "Tests failed"))
}

fn python_coverage(cwd: &Path) -> Result<()> {
    run_command("pytest", &["--cov"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Python", "coverage", "Coverage check failed"))
}

fn python_lint(cwd: &Path) -> Result<()> {
    run_command("ruff", &["check", "."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Python", "lint", "Code formatting issues"))
}

fn python_static(cwd: &Path) -> Result<()> {
    run_command("mypy", &["."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Python", "static", "Type checking failed"))
}

fn python_integration(cwd: &Path) -> Result<()> {
    run_command("pytest", &["-m", "integration"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Python", "integration", "Integration tests failed"))
}

fn python_security(cwd: &Path) -> Result<()> {
    run_command("bandit", &["-r", "."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Python", "security", "Security check failed"))
}

fn python_review(cwd: &Path) -> Result<()> {
    let result = run_command(
        "grep",
        &["-r", r"TODO\|FIXME\|XXX\|HACK", "--include=*.py", "."],
        cwd,
    )?;

    match result.exit_code {
        0 => Err(Error::stage_failed(
            "Python",
            "review",
            "TODO/FIXME/XXX/HACK markers found",
        )),
        1 => Ok(()), // No matches found - good
        code => Err(Error::stage_failed(
            "Python",
            "review",
            format!("grep failed with code: {code}"),
        )),
    }
}

fn python_accept(cwd: &Path) -> Result<()> {
    python_implement(cwd)?;
    python_unit_test(cwd)?;
    python_lint(cwd)
}
