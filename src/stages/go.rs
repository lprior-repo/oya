//! Go language stage execution.

use std::path::Path;

use crate::{
    error::{Error, Result},
    process::{command_exists, run_command},
};

/// Execute a Go pipeline stage.
pub fn execute_go_stage(stage_name: &str, cwd: &Path) -> Result<()> {
    match stage_name {
        "implement" => go_implement(cwd),
        "unit-test" => go_unit_test(cwd),
        "coverage" => go_coverage(cwd),
        "lint" => go_lint(cwd),
        "static" => go_static(cwd),
        "integration" => go_integration(cwd),
        "security" => go_security(cwd),
        "review" => go_review(cwd),
        "accept" => go_accept(cwd),
        other => Err(Error::UnknownStage {
            name: other.to_string(),
        }),
    }
}

/// Get preview for Go stage.
pub fn get_go_preview(stage_name: &str) -> (String, u64) {
    match stage_name {
        "implement" => ("go build ./...".to_string(), 10000),
        "unit-test" => ("go test -v -short ./...".to_string(), 30000),
        "coverage" => (
            "go test -coverprofile=/tmp/coverage.out ./...".to_string(),
            45000,
        ),
        "lint" => ("gofmt -l .".to_string(), 5000),
        "static" => ("go vet ./...".to_string(), 15000),
        "integration" => ("go test -v ./...".to_string(), 60000),
        "security" => ("gosec ./...".to_string(), 20000),
        "review" => ("grep -r TODO/FIXME".to_string(), 2000),
        "accept" => ("go build && go test && gofmt -l".to_string(), 60000),
        _ => ("unknown".to_string(), 0),
    }
}

fn go_implement(cwd: &Path) -> Result<()> {
    check_go_exists()?;
    run_command("go", &["build", "./..."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Go", "implement", "Code does not compile"))
}

fn go_unit_test(cwd: &Path) -> Result<()> {
    check_go_exists()?;
    run_command("go", &["test", "-v", "-short", "./..."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Go", "unit-test", "Tests failed"))
}

fn go_coverage(cwd: &Path) -> Result<()> {
    let result = run_command(
        "go",
        &["test", "-coverprofile=/tmp/coverage.out", "./..."],
        cwd,
    )?;

    if result.is_success() {
        Ok(())
    } else {
        Err(Error::stage_failed(
            "Go",
            "coverage",
            format!("Tests failed with exit code: {}", result.exit_code),
        ))
    }
}

fn go_lint(cwd: &Path) -> Result<()> {
    let result = run_command("gofmt", &["-l", "."], cwd)?;

    if result.stdout.trim().is_empty() {
        Ok(())
    } else {
        Err(Error::stage_failed(
            "Go",
            "lint",
            format!("Unformatted files:\n{}", result.stdout),
        ))
    }
}

fn go_static(cwd: &Path) -> Result<()> {
    run_command("go", &["vet", "./..."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Go", "static", "go vet failed"))
}

fn go_integration(cwd: &Path) -> Result<()> {
    run_command("go", &["test", "-v", "./..."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Go", "integration", "Integration tests failed"))
}

fn go_security(cwd: &Path) -> Result<()> {
    run_command("gosec", &["./..."], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Go", "security", "Security check failed"))
}

fn go_review(cwd: &Path) -> Result<()> {
    let result = run_command(
        "grep",
        &["-r", r"TODO\|FIXME\|XXX\|HACK", "--include=*.go", "."],
        cwd,
    )?;

    match result.exit_code {
        0 => Err(Error::stage_failed(
            "Go",
            "review",
            "TODO/FIXME/XXX/HACK markers found",
        )),
        1 => Ok(()), // No matches found - good
        code => Err(Error::stage_failed(
            "Go",
            "review",
            format!("grep failed with code: {code}"),
        )),
    }
}

fn go_accept(cwd: &Path) -> Result<()> {
    go_implement(cwd)?;
    go_unit_test(cwd)?;
    go_lint(cwd)
}

fn check_go_exists() -> Result<()> {
    if !command_exists("go")? {
        return Err(Error::CommandNotFound {
            cmd: "go".to_string(),
        });
    }
    Ok(())
}
