//! Gleam language stage execution.

use std::path::Path;

use crate::{
    error::{Error, Result},
    process::{command_exists, run_command},
};

/// Execute a Gleam pipeline stage.
pub fn execute_gleam_stage(stage_name: &str, cwd: &Path) -> Result<()> {
    match stage_name {
        "implement" => gleam_implement(cwd),
        "unit-test" => gleam_unit_test(cwd),
        "coverage" => gleam_coverage(cwd),
        "lint" => gleam_lint(cwd),
        "static" => gleam_static(cwd),
        "integration" => gleam_integration(cwd),
        "security" => gleam_security(cwd),
        "review" => gleam_review(cwd),
        "accept" => gleam_accept(cwd),
        other => Err(Error::UnknownStage {
            name: other.to_string(),
        }),
    }
}

/// Get preview for Gleam stage.
pub fn get_gleam_preview(stage_name: &str) -> (String, u64) {
    match stage_name {
        "implement" => ("gleam build".to_string(), 5000),
        "unit-test" => ("gleam test".to_string(), 10000),
        "coverage" => ("find . -name '*_test.gleam'".to_string(), 2000),
        "lint" => ("gleam format --check .".to_string(), 3000),
        "static" => ("gleam check".to_string(), 5000),
        "integration" => ("gleam test".to_string(), 15000),
        "security" => ("gleam deps download".to_string(), 5000),
        "review" => ("grep -r TODO/FIXME".to_string(), 2000),
        "accept" => (
            "gleam build && gleam test && gleam format --check".to_string(),
            20000,
        ),
        _ => ("unknown".to_string(), 0),
    }
}

fn gleam_implement(cwd: &Path) -> Result<()> {
    check_gleam_exists()?;
    run_command("gleam", &["build"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "implement", "Code does not compile"))
}

fn gleam_unit_test(cwd: &Path) -> Result<()> {
    check_gleam_exists()?;
    run_command("gleam", &["test"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "unit-test", "Tests failed"))
}

fn gleam_coverage(cwd: &Path) -> Result<()> {
    let result = run_command(
        "find",
        &[".", "-name", "*_test.gleam", "-o", "-name", "test_*.gleam"],
        cwd,
    )?;

    if result.exit_code == 1 {
        return Err(Error::stage_failed(
            "Gleam",
            "coverage",
            "No test files found",
        ));
    }

    result.check_success().map_err(|_| {
        Error::stage_failed(
            "Gleam",
            "coverage",
            format!("find failed with code: {}", result.exit_code),
        )
    })
}

fn gleam_lint(cwd: &Path) -> Result<()> {
    run_command("gleam", &["format", "--check", "."], cwd)?
        .check_success()
        .map_err(|_| {
            Error::stage_failed(
                "Gleam",
                "lint",
                "Code formatting issues. Run: gleam format .",
            )
        })
}

fn gleam_static(cwd: &Path) -> Result<()> {
    run_command("gleam", &["check"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "static", "Type checking failed"))
}

fn gleam_integration(cwd: &Path) -> Result<()> {
    run_command("gleam", &["test"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "integration", "Integration tests failed"))
}

fn gleam_security(cwd: &Path) -> Result<()> {
    run_command("gleam", &["deps", "download"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "security", "Dependency validation failed"))
}

fn gleam_review(cwd: &Path) -> Result<()> {
    let result = run_command(
        "grep",
        &["-r", r"TODO\|FIXME\|XXX\|HACK", "--include=*.gleam", "."],
        cwd,
    )?;

    match result.exit_code {
        0 => Err(Error::stage_failed(
            "Gleam",
            "review",
            "TODO/FIXME/XXX/HACK markers found",
        )),
        1 => Ok(()), // No matches found - good
        code => Err(Error::stage_failed(
            "Gleam",
            "review",
            format!("grep failed with code: {code}"),
        )),
    }
}

fn gleam_accept(cwd: &Path) -> Result<()> {
    gleam_implement(cwd)?;
    gleam_unit_test(cwd)?;
    gleam_lint(cwd)
}

fn check_gleam_exists() -> Result<()> {
    if !command_exists("gleam")? {
        return Err(Error::CommandNotFound {
            cmd: "gleam".to_string(),
        });
    }
    Ok(())
}
