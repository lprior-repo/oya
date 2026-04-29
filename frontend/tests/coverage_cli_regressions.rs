#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("oya-coverage-cli-{label}-{nanos}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, content)?;
    Ok(())
}

#[test]
fn given_json_format_when_running_coverage_cli_then_stdout_is_pure_json(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_dir("json-pure")?;
    let specs = root.join("specs");
    let scenarios = root.join("scenarios");
    fs::create_dir_all(&specs)?;
    fs::create_dir_all(&scenarios)?;

    write_file(
        &specs.join("spec.yaml"),
        r#"
specification:
  identity:
    id: spec-cli-purity
  behaviors: []
"#,
    )?;
    write_file(
        &scenarios.join("scenario.yaml"),
        r#"
scenario:
  spec_ref: cli-purity
  steps: []
"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_coverage"))
        .args([
            "--specs-dir",
            specs.to_str().ok_or_else(|| std::io::Error::other("invalid specs path"))?,
            "--scenarios-dir",
            scenarios.to_str().ok_or_else(|| std::io::Error::other("invalid scenarios path"))?,
            "--format",
            "json",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "coverage binary failed: stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let stdout = String::from_utf8(output.stdout)?;
    match serde_json::from_str::<serde_json::Value>(&stdout) {
        Ok(_) => {}
        Err(e) => panic!("Expected valid JSON, got error: {e}"),
    }
    assert!(!stdout.contains("Analyzing scenario coverage"));

    fs::remove_dir_all(root)?;
    Ok(())
}
