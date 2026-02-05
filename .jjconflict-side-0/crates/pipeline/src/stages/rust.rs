//! Rust language stage execution.

use std::path::Path;

use crate::{
    error::{Error, Result},
    process::{command_exists, run_command},
};

/// Execute a Rust pipeline stage.
pub fn execute_rust_stage(stage_name: &str, cwd: &Path) -> Result<()> {
    match stage_name {
        "implement" => rust_implement(cwd),
        "unit-test" => rust_unit_test(cwd),
        "coverage" => rust_coverage(cwd),
        "lint" => rust_lint(cwd),
        "static" => rust_static(cwd),
        "integration" => rust_integration(cwd),
        "security" => rust_security(cwd),
        "review" => rust_review(cwd),
        "accept" => rust_accept(cwd),
        other => Err(Error::UnknownStage {
            name: other.to_string(),
        }),
    }
}

/// Get preview for Rust stage.
pub fn get_rust_preview(stage_name: &str) -> (String, u64) {
    match stage_name {
        "implement" => ("cargo build".to_string(), 30000),
        "unit-test" => ("cargo test".to_string(), 60000),
        "coverage" => ("cargo tarpaulin --out Xml".to_string(), 90000),
        "lint" => ("cargo fmt --check".to_string(), 5000),
        "static" => ("cargo clippy --all-targets".to_string(), 45000),
        "integration" => ("cargo test --all".to_string(), 90000),
        "security" => ("cargo audit".to_string(), 10000),
        "review" => ("grep -r TODO/FIXME".to_string(), 2000),
        "accept" => (
            "cargo build && cargo test && cargo fmt --check".to_string(),
            120_000,
        ),
        _ => ("unknown".to_string(), 0),
    }
}

fn rust_implement(cwd: &Path) -> Result<()> {
    check_cargo_exists()?;
    run_command("cargo", &["build"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Rust", "implement", "Code does not compile"))
}

fn rust_unit_test(cwd: &Path) -> Result<()> {
    run_command("cargo", &["test"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Rust", "unit-test", "Tests failed"))
}

fn rust_coverage(cwd: &Path) -> Result<()> {
    let result = run_command("cargo", &["tarpaulin", "--out", "Xml"], cwd)?;

    if result.is_success() {
        Ok(())
    } else {
        Err(Error::stage_failed(
            "Rust",
            "coverage",
            format!("Coverage check failed with code: {}", result.exit_code),
        ))
    }
}

fn rust_lint(cwd: &Path) -> Result<()> {
    run_command("cargo", &["fmt", "--check"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Rust", "lint", "Code formatting issues"))
}

fn rust_static(cwd: &Path) -> Result<()> {
    run_command("cargo", &["clippy", "--all-targets"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Rust", "static", "Clippy failed"))
}

fn rust_integration(cwd: &Path) -> Result<()> {
    run_command("cargo", &["test", "--all"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Rust", "integration", "Integration tests failed"))
}

fn rust_security(cwd: &Path) -> Result<()> {
    run_command("cargo", &["audit"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Rust", "security", "Security audit failed"))
}

fn rust_review(cwd: &Path) -> Result<()> {
    let result = run_command(
        "grep",
        &["-r", r"TODO\|FIXME\|XXX\|HACK", "--include=*.rs", "."],
        cwd,
    )?;

    match result.exit_code {
        0 => Err(Error::stage_failed(
            "Rust",
            "review",
            "TODO/FIXME/XXX/HACK markers found",
        )),
        1 => Ok(()), // No matches found - good
        code => Err(Error::stage_failed(
            "Rust",
            "review",
            format!("grep failed with code: {code}"),
        )),
    }
}

fn rust_accept(cwd: &Path) -> Result<()> {
    rust_implement(cwd)?;
    rust_unit_test(cwd)?;
    rust_lint(cwd)
}

fn check_cargo_exists() -> Result<()> {
    if !command_exists("cargo")? {
        return Err(Error::CommandNotFound {
            cmd: "cargo".to_string(),
        });
    }
    Ok(())
}
