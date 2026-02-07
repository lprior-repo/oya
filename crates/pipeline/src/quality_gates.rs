//! Quality gates for OYA pipeline stages.
//!
//! Enforces strict functional Rust requirements before allowing
//! code to proceed through the pipeline.

use itertools::Itertools;
use std::path::Path;
use tracing::{debug, warn};

use crate::error::{Error, Result};
use crate::functional::{FunctionalAudit, ViolationSeverity, audit_functional_style};

/// Minimum functional code compliance percentage required (95%).
const MIN_FUNCTIONAL_COMPLIANCE: f64 = 95.0;

/// Quality gate result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityGateResult {
    Passed,
    Failed {
        reason: String,
        violations: Vec<String>,
    },
    Warning {
        reason: String,
        violations: Vec<String>,
    },
}

impl QualityGateResult {
    /// Check if gate passed.
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }

    /// Get all violation messages.
    #[must_use]
    pub fn violation_messages(&self) -> Vec<&str> {
        match self {
            Self::Passed => Vec::new(),
            Self::Failed { violations, .. } | Self::Warning { violations, .. } => {
                violations.iter().map(String::as_str).collect()
            }
        }
    }
}

/// Functional Rust quality gate.
#[derive(Debug, Clone)]
pub struct FunctionalGate {
    audit: FunctionalAudit,
    rust_files: Vec<String>,
}

impl FunctionalGate {
    /// Create a new functional gate by auditing a directory.
    pub fn audit_directory(worktree_path: &Path) -> Result<Self> {
        let rust_files = find_rust_files(worktree_path)?;

        if rust_files.is_empty() {
            debug!("No Rust files found for functional audit");
            return Ok(Self {
                audit: FunctionalAudit {
                    violations: Vec::new(),
                    total_lines: 0,
                    functional_percentage: 100.0,
                },
                rust_files,
            });
        }

        let (all_violations, total_lines): (Vec<_>, usize) = rust_files
            .iter()
            .map(|file_path| {
                std::fs::read_to_string(file_path)
                    .map_err(|e| Error::file_read_failed(file_path, e))
                    .map(|content| {
                        let audit = audit_functional_style(&content);
                        (audit.violations, audit.total_lines)
                    })
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .fold(
                (Vec::new(), 0),
                |(mut violations, mut total_lines), (file_violations, file_lines)| {
                    violations.extend(file_violations);
                    total_lines += file_lines;
                    (violations, total_lines)
                },
            );

        let functional_percentage = if total_lines > 0 {
            let clean_lines = total_lines.saturating_sub(all_violations.len());
            (clean_lines as f64 / total_lines as f64) * 100.0
        } else {
            100.0
        };

        debug!(
            functional_percentage,
            total_lines,
            violations = all_violations.len(),
            "Functional audit complete"
        );

        Ok(Self {
            audit: FunctionalAudit {
                violations: all_violations,
                total_lines,
                functional_percentage,
            },
            rust_files,
        })
    }

    /// Run the quality gate and return result.
    #[must_use]
    pub fn run(&self) -> QualityGateResult {
        let audit = &self.audit;

        // Check for critical violations
        let critical_violations = audit
            .violations
            .iter()
            .filter(|v| matches!(v.severity, ViolationSeverity::Critical))
            .collect_vec();

        if !critical_violations.is_empty() {
            let violation_msgs = critical_violations
                .iter()
                .map(|v| format!("Line {}: {}", v.line, v.pattern.description()))
                .collect_vec();

            return QualityGateResult::Failed {
                reason: format!(
                    "Critical violations found ({}). Code contains panic or unsafe patterns.",
                    critical_violations.len()
                ),
                violations: violation_msgs,
            };
        }

        // Check functional compliance percentage
        if audit.functional_percentage < MIN_FUNCTIONAL_COMPLIANCE {
            let violation_msgs = audit
                .violations
                .iter()
                .map(|v| format!("Line {}: {}", v.line, v.pattern.description()))
                .collect_vec();

            return QualityGateResult::Failed {
                reason: format!(
                    "Functional compliance {:.1}% below minimum {}%",
                    audit.functional_percentage, MIN_FUNCTIONAL_COMPLIANCE
                ),
                violations: violation_msgs,
            };
        }

        // Check for warnings (high severity violations)
        let high_violations: Vec<_> = audit
            .violations
            .iter()
            .filter(|v| matches!(v.severity, ViolationSeverity::High))
            .collect();

        if !high_violations.is_empty() {
            let warning_msgs: Vec<String> = high_violations
                .iter()
                .map(|v| format!("Line {}: {}", v.line, v.pattern.description()))
                .collect();

            return QualityGateResult::Warning {
                reason: format!(
                    "High severity violations found ({}). Review recommended.",
                    high_violations.len()
                ),
                violations: warning_msgs,
            };
        }

        QualityGateResult::Passed
    }

    /// Get the audit details.
    #[must_use]
    pub const fn audit(&self) -> &FunctionalAudit {
        &self.audit
    }

    /// Get audited file paths.
    #[must_use]
    pub fn files(&self) -> &[String] {
        &self.rust_files
    }

    /// Generate a human-readable report.
    #[must_use]
    pub fn report(&self) -> String {
        let audit = &self.audit;
        let gate_result = self.run();

        let mut report = String::new();

        report.push_str("=== Functional Rust Quality Gate ===\n\n");
        report.push_str(&format!("Files audited: {}\n", self.rust_files.len()));
        report.push_str(&format!("Total lines: {}\n", audit.total_lines));
        report.push_str(&format!(
            "Functional compliance: {:.1}%\n",
            audit.functional_percentage
        ));
        report.push_str(&format!("Violations: {}\n", audit.violations.len()));

        if let QualityGateResult::Passed = gate_result {
            report.push_str("\n✓ Quality gate PASSED\n");
        } else if let QualityGateResult::Failed { reason, .. } = gate_result {
            report.push_str("\n✗ Quality gate FAILED\n");
            report.push_str(&format!("Reason: {}\n", reason));
        } else if let QualityGateResult::Warning { reason, .. } = gate_result {
            report.push_str("\n⚠ Quality gate PASSED with warnings\n");
            report.push_str(&format!("Warning: {}\n", reason));
        }

        if !audit.violations.is_empty() {
            report.push_str("\n=== Violations ===\n");

            let violations_by_severity: Vec<(ViolationSeverity, Vec<_>)> = [
                ViolationSeverity::Critical,
                ViolationSeverity::High,
                ViolationSeverity::Medium,
                ViolationSeverity::Low,
            ]
            .into_iter()
            .map(|severity| {
                let violations = audit
                    .violations
                    .iter()
                    .filter(|v| v.severity == severity)
                    .collect::<Vec<_>>();
                (severity, violations)
            })
            .filter(|(_, violations)| !violations.is_empty())
            .collect();

            for (severity, violations) in violations_by_severity {
                report.push_str(&format!("\n{:?} ({}):\n", severity, violations.len()));

                let violation_details: Vec<String> = violations
                    .iter()
                    .flat_map(|v| {
                        [
                            format!("  Line {}: {}", v.line, v.pattern.description()),
                            format!("    {}\n", v.line_content.trim()),
                        ]
                    })
                    .collect();

                report.push_str(&violation_details.join("\n"));
            }
        }

        report
    }
}

/// Find all Rust source files in a directory (memoized).
pub fn find_rust_files(worktree_path: &Path) -> Result<Vec<String>> {
    // Convert PathBuf to Path and collect the paths as strings
    crate::file_discovery::find_rust_files(worktree_path).map(|paths| {
        paths
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    })
}

/// Check if a directory should be skipped (hidden or common build dirs).
#[must_use]
fn is_hidden_dir(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        let name = name_str.as_ref();
        return name.starts_with('.')
            || matches!(
                name,
                "target" | "node_modules" | "build" | "dist" | ".git" | "vendor"
            );
    }
    false
}

/// Run functional quality gate and return Result.
pub fn enforce_functional_quality(worktree_path: &Path) -> Result<()> {
    let gate = FunctionalGate::audit_directory(worktree_path)?;
    let result = gate.run();

    match result {
        QualityGateResult::Passed => {
            debug!("Functional quality gate passed");
            Ok(())
        }
        QualityGateResult::Failed { reason, .. } => Err(Error::InvalidRecord {
            reason: format!("Functional quality gate failed: {reason}"),
        }),
        QualityGateResult::Warning { .. } => {
            warn!("Functional quality gate passed with warnings");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_find_rust_files() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("test-functional-gate");
        let _ = fs::create_dir_all(&test_dir);

        // Create test files
        fs::write(test_dir.join("main.rs"), "fn main() {}").ok();
        fs::write(test_dir.join("lib.rs"), "pub fn lib() {}").ok();
        fs::write(test_dir.join("README.md"), "# Test").ok();

        let files = find_rust_files(&test_dir);
        assert!(files.is_ok());
        if let Ok(rust_files) = files {
            assert_eq!(rust_files.len(), 2);
            assert!(rust_files.iter().any(|f| f.contains("main.rs")));
            assert!(rust_files.iter().any(|f| f.contains("lib.rs")));
            assert!(!rust_files.iter().any(|f| f.contains("README.md")));
        }

        // Cleanup
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_is_hidden_dir() {
        assert!(is_hidden_dir(&PathBuf::from(".git")));
        assert!(is_hidden_dir(&PathBuf::from("target")));
        assert!(is_hidden_dir(&PathBuf::from("node_modules")));
        assert!(!is_hidden_dir(&PathBuf::from("src")));
        assert!(!is_hidden_dir(&PathBuf::from("tests")));
    }

    #[test]
    fn test_quality_gate_result() {
        let passed = QualityGateResult::Passed;
        assert!(passed.is_passed());
        assert_eq!(passed.violation_messages().len(), 0);

        let failed = QualityGateResult::Failed {
            reason: "Test failure".to_string(),
            violations: vec!["violation 1".to_string()],
        };
        assert!(!failed.is_passed());
        assert_eq!(failed.violation_messages().len(), 1);
    }
}
