//! JavaScript/TypeScript language stage execution.

use std::path::Path;

use crate::{
    error::{Error, Result},
    process::run_command,
};

/// Execute a JavaScript/TypeScript pipeline stage.
pub fn execute_javascript_stage(stage_name: &str, cwd: &Path) -> Result<()> {
    match stage_name {
        "implement" => javascript_implement(cwd),
        "unit-test" => javascript_unit_test(cwd),
        "coverage" => javascript_coverage(cwd),
        "lint" => javascript_lint(cwd),
        "static" => javascript_static(cwd),
        "integration" => javascript_integration(cwd),
        "security" => javascript_security(cwd),
        "review" => javascript_review(cwd),
        "accept" => javascript_accept(cwd),
        other => Err(Error::UnknownStage {
            name: other.to_string(),
        }),
    }
}

/// Get preview for JavaScript/TypeScript stage.
pub fn get_javascript_preview(stage_name: &str) -> (String, u64) {
    match stage_name {
        "implement" => ("npm run build".to_string(), 30000),
        "unit-test" => ("npm test".to_string(), 30000),
        "coverage" => ("npm run coverage".to_string(), 45000),
        "lint" => ("npm run lint".to_string(), 10000),
        "static" => ("npm run typecheck".to_string(), 30000),
        "integration" => ("npm run test:integration".to_string(), 60000),
        "security" => ("npm audit".to_string(), 10000),
        "review" => ("grep -r TODO/FIXME".to_string(), 2000),
        "accept" => (
            "npm run build && npm test && npm run lint".to_string(),
            90000,
        ),
        _ => ("unknown".to_string(), 0),
    }
}

fn javascript_implement(cwd: &Path) -> Result<()> {
    run_command("npm", &["run", "build"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("JavaScript", "implement", "Build failed"))
}

fn javascript_unit_test(cwd: &Path) -> Result<()> {
    run_command("npm", &["test"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("JavaScript", "unit-test", "Tests failed"))
}

fn javascript_coverage(cwd: &Path) -> Result<()> {
    run_command("npm", &["run", "coverage"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("JavaScript", "coverage", "Coverage check failed"))
}

fn javascript_lint(cwd: &Path) -> Result<()> {
    run_command("npm", &["run", "lint"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("JavaScript", "lint", "Linting failed"))
}

fn javascript_static(cwd: &Path) -> Result<()> {
    run_command("npm", &["run", "typecheck"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("JavaScript", "static", "Type checking failed"))
}

fn javascript_integration(cwd: &Path) -> Result<()> {
    run_command("npm", &["run", "test:integration"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("JavaScript", "integration", "Integration tests failed"))
}

fn javascript_security(cwd: &Path) -> Result<()> {
    run_command("npm", &["audit"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("JavaScript", "security", "Security audit failed"))
}

fn javascript_review(cwd: &Path) -> Result<()> {
    let result = run_command(
        "grep",
        &[
            "-r",
            r"TODO\|FIXME\|XXX\|HACK",
            "--include=*.js",
            "--include=*.ts",
            "--include=*.jsx",
            "--include=*.tsx",
            ".",
        ],
        cwd,
    )?;

    match result.exit_code {
        0 => Err(Error::stage_failed(
            "JavaScript",
            "review",
            "TODO/FIXME/XXX/HACK markers found",
        )),
        1 => Ok(()), // No matches found - good
        code => Err(Error::stage_failed(
            "JavaScript",
            "review",
            format!("grep failed with code: {code}"),
        )),
    }
}

fn javascript_accept(cwd: &Path) -> Result<()> {
    javascript_implement(cwd)?;
    javascript_unit_test(cwd)?;
    javascript_lint(cwd)
}
