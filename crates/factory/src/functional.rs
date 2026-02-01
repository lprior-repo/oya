//! Functional Rust code generation and enforcement.
//!
//! Ensures all generated code follows strict functional programming patterns:
//! - Immutable by default
//! - Pure functions (no side effects)
//! - Railway-Oriented Programming (Result types)
//! - Zero panics
//! - Explicit error handling
//! - No unwrap(), expect(), or panic!()

use regex::Regex;
use std::collections::HashSet;
use tracing::{debug, warn};

use crate::error::{Error, Result};

/// Forbidden patterns in functional Rust code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForbiddenPattern {
    Unwrap,
    Expect,
    Panic,
    Unsafe,
    MutLocalVar,
    MutParameter,
    RcCell,
    InteriorMutability,
    StaticMut,
}

impl ForbiddenPattern {
    /// Get the regex pattern for this forbidden item.
    #[must_use]
    pub fn regex(&self) -> &'static str {
        match self {
            Self::Unwrap => r"\.unwrap\(\)",
            Self::Expect => r"\.expect\(",
            Self::Panic => r"\bpanic!\(",
            Self::Unsafe => r"\bunsafe\s+",
            Self::MutLocalVar => r"\blet\s+mut\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:",
            Self::MutParameter => r"fn\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\([^)]*mut\s+",
            Self::RcCell => r"\b(Rc|Cell|RefCell)\s*<",
            Self::InteriorMutability => r"\b(unsafe_cell|SyncUnsafeCell)\s*<",
            Self::StaticMut => r"\bstatic\s+mut\s+",
        }
    }

    /// Get the description for this forbidden pattern.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Unwrap => "Use of .unwrap() - replace with ? operator or pattern matching",
            Self::Expect => "Use of .expect() - replace with ? operator or explicit error handling",
            Self::Panic => "Use of panic!() - replace with Result or proper error handling",
            Self::Unsafe => "Use of unsafe block - avoid unless absolutely necessary",
            Self::MutLocalVar => "Mutable local variable - prefer immutable bindings with let",
            Self::MutParameter => "Mutable function parameter - prefer immutable parameters",
            Self::RcCell => "Use of Rc/Cell/RefCell - prefer immutable references",
            Self::InteriorMutability => "Use of interior mutability - prefer pure functions",
            Self::StaticMut => "Static mutable variable - avoid global mutable state",
        }
    }

    /// Get the severity of this violation.
    #[must_use]
    pub const fn severity(&self) -> ViolationSeverity {
        match self {
            Self::Unwrap | Self::Expect | Self::Panic => ViolationSeverity::Critical,
            Self::Unsafe | Self::StaticMut => ViolationSeverity::High,
            Self::MutLocalVar | Self::MutParameter | Self::RcCell | Self::InteriorMutability => {
                ViolationSeverity::Medium
            }
        }
    }
}

/// Severity level for violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// A single functional style violation.
#[derive(Debug, Clone)]
pub struct Violation {
    pub pattern: ForbiddenPattern,
    pub line: usize,
    pub column: usize,
    pub line_content: String,
    pub severity: ViolationSeverity,
}

impl Violation {
    /// Create a new violation.
    #[must_use]
    pub fn new(
        pattern: ForbiddenPattern,
        line: usize,
        column: usize,
        line_content: String,
    ) -> Self {
        let severity = pattern.severity();
        Self {
            pattern,
            line,
            column,
            line_content,
            severity,
        }
    }
}

/// Result of functional style audit.
#[derive(Debug, Clone)]
pub struct FunctionalAudit {
    pub violations: Vec<Violation>,
    pub total_lines: usize,
    pub functional_percentage: f64,
}

impl FunctionalAudit {
    /// Check if code passes functional style requirements.
    #[must_use]
    pub fn is_functional(&self) -> bool {
        // No critical or high violations allowed
        self.violations.iter().all(|v| {
            matches!(
                v.severity,
                ViolationSeverity::Low | ViolationSeverity::Medium
            )
        })
    }

    /// Get violations by severity.
    #[must_use]
    pub fn violations_by_severity(&self, severity: ViolationSeverity) -> Vec<&Violation> {
        self.violations
            .iter()
            .filter(|v| v.severity == severity)
            .collect()
    }

    /// Get critical violations.
    #[must_use]
    pub fn critical_violations(&self) -> Vec<&Violation> {
        self.violations_by_severity(ViolationSeverity::Critical)
    }

    /// Get high severity violations.
    #[must_use]
    pub fn high_violations(&self) -> Vec<&Violation> {
        self.violations_by_severity(ViolationSeverity::High)
    }

    /// Get count of violations by severity.
    #[must_use]
    pub fn violation_counts(&self) -> std::collections::HashMap<ViolationSeverity, usize> {
        let mut counts = std::collections::HashMap::new();
        for violation in &self.violations {
            *counts.entry(violation.severity).or_insert(0) += 1;
        }
        counts
    }
}

/// Audit code for functional Rust patterns.
pub fn audit_functional_style(code: &str) -> FunctionalAudit {
    let mut violations = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    let total_lines = lines.len();

    for (line_idx, line) in lines.iter().enumerate() {
        for pattern in &[
            ForbiddenPattern::Unwrap,
            ForbiddenPattern::Expect,
            ForbiddenPattern::Panic,
            ForbiddenPattern::Unsafe,
            ForbiddenPattern::StaticMut,
        ] {
            if let Some(matches) = find_matches(line, pattern) {
                for (col, _) in matches {
                    violations.push(Violation::new(
                        pattern.clone(),
                        line_idx + 1,
                        col,
                        line.to_string(),
                    ));
                }
            }
        }
    }

    // Check for mutable variables (but exclude common patterns like "let mut iter")
    let mut_regex = Regex::new(r"\blet\s+mut\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*:").unwrap();
    let mut_skip_regex =
        Regex::new(r"\blet\s+mut\s+(iter|self|this|cursor|pointer|idx|index|i|j|k|x|y|z)\s*:")
            .unwrap();

    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(captures) = mut_regex.captures(line) {
            if let Some(var_name) = captures.get(1) {
                let var = var_name.as_str();
                if !mut_skip_regex.is_match(line) {
                    violations.push(Violation::new(
                        ForbiddenPattern::MutLocalVar,
                        line_idx + 1,
                        var_name.start(),
                        line.to_string(),
                    ));
                }
            }
        }
    }

    // Calculate functional score
    let max_allowed_violations = (total_lines as f64 * 0.01).ceil() as usize; // 1% tolerance
    let violation_count = violations.len();
    let functional_percentage = if total_lines > 0 {
        let clean_lines = total_lines.saturating_sub(violation_count);
        (clean_lines as f64 / total_lines as f64) * 100.0
    } else {
        100.0
    };

    FunctionalAudit {
        violations,
        total_lines,
        functional_percentage,
    }
}

/// Find matches of a pattern in a line.
fn find_matches(line: &str, pattern: &ForbiddenPattern) -> Option<Vec<(usize, &str)>> {
    let regex = Regex::new(pattern.regex()).ok()?;
    let matches: Vec<_> = regex
        .find_iter(line)
        .map(|m| (m.start(), m.as_str()))
        .collect();

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

/// Generate functional Rust code stub for a module.
#[must_use]
pub fn generate_functional_module(module_name: &str, functions: &[FunctionStub]) -> String {
    let mut output = String::new();

    // Module header
    output.push_str(&format!("//! {module_name}\n"));
    output.push_str(&format!("//!\n"));
    output.push_str(&format!(
        "//! Functional Rust module - all functions are pure and return Result.\n"
    ));
    output.push_str(&format!("//!\n"));
    output.push_str(&format!("//! # Design Principles\n"));
    output.push_str(&format!("//!\n"));
    output.push_str(&format!("//! - Immutable by default\n"));
    output.push_str(&format!("//! - Pure functions (no side effects)\n"));
    output.push_str(&format!(
        "//! - Railway-Oriented Programming (Result types)\n"
    ));
    output.push_str(&format!("//! - Zero panics\n"));
    output.push_str(&format!("//! - Explicit error handling\n"));
    output.push_str(&format!("//!\n"));
    output.push_str(&format!("//! ## Functions\n"));
    output.push_str(&format!("//!\n"));

    for func in functions {
        output.push_str(&format_function_stub(func));
    }

    output
}

/// Generate a single function stub.
#[must_use]
pub fn format_function_stub(func: &FunctionStub) -> String {
    let mut output = String::new();

    // Documentation
    output.push_str(&format!("/// {}\n", func.description));

    // Function signature - note no "mut" parameters
    output.push_str(&format!("pub fn {}(", func.name));

    let params: Vec<String> = func
        .parameters
        .iter()
        .map(|p| {
            if p.is_result {
                format!("{}: Result<{}>", p.name, p.type_name)
            } else {
                format!("{}: {}", p.name, p.type_name)
            }
        })
        .collect();

    output.push_str(&params.join(", "));

    if func.returns_result {
        output.push_str(") -> Result<");
        output.push_str(&func.return_type);
        output.push_str("> {\n");
    } else {
        output.push_str(") -> ");
        output.push_str(&func.return_type);
        output.push_str(" {\n");
    }

    // Implementation note
    output.push_str(&format!("    // TODO: Implement {}\n", func.name));
    output.push_str(&format!(
        "    // Ensure: immutability, purity, explicit error handling\n"
    ));

    if func.returns_result {
        output.push_str(&format!(
            "    Err(Error::not_implemented(\"{}\"))\n",
            func.name
        ));
    } else {
        output.push_str(&format!("    todo!(\"implement {}\")\n", func.name));
    }

    output.push_str("}\n\n");

    output
}

/// Function stub for code generation.
#[derive(Debug, Clone)]
pub struct FunctionStub {
    pub name: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub return_type: String,
    pub returns_result: bool,
}

/// Parameter definition for function stub.
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_name: String,
    pub is_result: bool,
}

/// Convert a list of violations to a report string.
#[must_use]
pub fn format_violations_report(audit: &FunctionalAudit) -> Vec<String> {
    let mut report = Vec::new();

    if audit.violations.is_empty() {
        report.push("✓ Code passes functional style requirements".to_string());
        return report;
    }

    report.push(format!(
        "Functional style audit: {:.1}% compliance",
        audit.functional_percentage
    ));

    // Group by severity
    for severity in [
        ViolationSeverity::Critical,
        ViolationSeverity::High,
        ViolationSeverity::Medium,
        ViolationSeverity::Low,
    ] {
        let violations = audit.violations_by_severity(severity);
        if !violations.is_empty() {
            report.push(format!(
                "\n{} violations ({}):",
                format!("{:?}", severity),
                violations.len()
            ));

            for v in violations {
                report.push(format!(
                    "  Line {}: {} - {}",
                    v.line,
                    v.pattern.description(),
                    v.line_content.trim()
                ));
            }
        }
    }

    report
}

/// Check if a set of patterns contains any that would prevent code from being functional.
#[must_use]
pub fn has_critical_violations(code: &str) -> bool {
    let audit = audit_functional_style(code);
    audit.critical_violations().iter().any(|v| {
        matches!(
            v.pattern,
            ForbiddenPattern::Unwrap | ForbiddenPattern::Panic
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_functional_code() {
        let code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn divide(a: i32, b: i32) -> Result<i32> {
    if b == 0 {
        Err(Error::DivisionByZero)
    } else {
        Ok(a / b)
    }
}
"#;

        let audit = audit_functional_style(code);
        assert!(audit.is_functional());
        assert!(audit.functional_percentage >= 90.0);
    }

    #[test]
    fn test_audit_imperative_code() {
        let code = r#"
pub fn process_data(data: &mut Vec<i32>) -> i32 {
    let mut sum = 0;
    for item in data {
        sum += item.unwrap();  // Bad!
    }
    sum
}
"#;

        let audit = audit_functional_style(code);
        assert!(!audit.is_functional());
        assert!(!audit.critical_violations().is_empty());
    }

    #[test]
    fn test_find_unwrap() {
        let line = "let value = result.unwrap()";
        let pattern = ForbiddenPattern::Unwrap;
        let matches = find_matches(line, &pattern);
        assert!(matches.is_some());
        assert_eq!(matches.unwrap().len(), 1);
    }

    #[test]
    fn test_generate_functional_module() {
        let functions = vec![FunctionStub {
            name: "calculate".to_string(),
            description: "Calculate a value".to_string(),
            parameters: vec![Parameter {
                name: "input".to_string(),
                type_name: "i32".to_string(),
                is_result: false,
            }],
            return_type: "i32".to_string(),
            returns_result: true,
        }];

        let module = generate_functional_module("math", &functions);
        assert!(module.contains("pub fn calculate(input: i32) -> Result<i32>"));
        assert!(module.contains("Functional Rust module"));
    }

    #[test]
    fn test_has_critical_violations() {
        let good_code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let bad_code = "fn get_value() -> i32 { result.unwrap() }";

        assert!(!has_critical_violations(good_code));
        assert!(has_critical_violations(bad_code));
    }

    #[test]
    fn test_violation_severity() {
        assert_eq!(
            ForbiddenPattern::Unwrap.severity(),
            ViolationSeverity::Critical
        );
        assert_eq!(ForbiddenPattern::Unsafe.severity(), ViolationSeverity::High);
        assert_eq!(
            ForbiddenPattern::MutLocalVar.severity(),
            ViolationSeverity::Medium
        );
    }
}
