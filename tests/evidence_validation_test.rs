use oya::domain::{GateResult, ValidationError};

fn sample_gate_result() -> GateResult {
    GateResult {
        run_id: "run-1".to_string(),
        gate_name: "contract:001:moon_ci".to_string(),
        command: Some("moon run :ci".to_string()),
        passed: true,
        exit_code: 0,
        log_ref: None,
    }
}

#[test]
fn valid_evidence_passes_validation() {
    let result = sample_gate_result();
    assert!(result.validate().is_ok());
}

#[test]
fn missing_command_field_fails() {
    let mut result = sample_gate_result();
    result.command = Some("".to_string());
    assert!(result.validate().is_err());
}

#[test]
fn placeholder_command_fails() {
    let mut result = sample_gate_result();
    result.command = Some("TODO: implement moon ci".to_string());
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::PlaceholderValue(field, value)) = validation_result {
        assert_eq!(field, "command");
        assert_eq!(value, "TODO: implement moon ci");
    } else {
        panic!("Expected PlaceholderValue error");
    }
}

#[test]
fn placeholder_command_lowercase_fails() {
    let mut result = sample_gate_result();
    result.command = Some("placeholder command".to_string());
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::PlaceholderValue(_, _)) = validation_result {
        // Expected
    } else {
        panic!("Expected PlaceholderValue error");
    }
}

#[test]
fn empty_gate_name_fails() {
    let mut result = sample_gate_result();
    result.gate_name = "".to_string();
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::MissingField(field)) = validation_result {
        assert_eq!(field, "gate_name");
    } else {
        panic!("Expected MissingField error");
    }
}

#[test]
fn invalid_exit_code_negative_fails() {
    let mut result = sample_gate_result();
    result.exit_code = -1;
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::InvalidExitCode(code)) = validation_result {
        assert_eq!(code, -1);
    } else {
        panic!("Expected InvalidExitCode error");
    }
}

#[test]
fn invalid_exit_code_too_large_fails() {
    let mut result = sample_gate_result();
    result.exit_code = 256;
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::InvalidExitCode(code)) = validation_result {
        assert_eq!(code, 256);
    } else {
        panic!("Expected InvalidExitCode error");
    }
}

#[test]
fn inconsistent_passed_true_exit_code_nonzero_fails() {
    let mut result = sample_gate_result();
    result.passed = true;
    result.exit_code = 1;
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::InconsistentEvidence(msg)) = validation_result {
        assert!(msg.contains("passed=true"));
        assert!(msg.contains("exit_code≠0"));
    } else {
        panic!("Expected InconsistentEvidence error");
    }
}

#[test]
fn inconsistent_passed_false_exit_code_zero_fails() {
    let mut result = sample_gate_result();
    result.passed = false;
    result.exit_code = 0;
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::InconsistentEvidence(msg)) = validation_result {
        assert!(msg.contains("passed=false"));
        assert!(msg.contains("exit_code=0"));
    } else {
        panic!("Expected InconsistentEvidence error");
    }
}

#[test]
fn placeholder_in_log_ref_fails() {
    let mut result = sample_gate_result();
    result.log_ref = Some("TBD: will add log reference later".to_string());
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::PlaceholderValue(field, value)) = validation_result {
        assert_eq!(field, "log_ref");
        assert!(value.contains("TBD"));
    } else {
        panic!("Expected PlaceholderValue error");
    }
}

#[test]
fn all_placeholder_variants_detected() {
    let placeholders = vec![
        "TODO",
        "placeholder",
        "not implemented",
        "TBD",
        "TBC",
        "todo",
        "PLACEHOLDER",
        "Not Implemented",
    ];

    for placeholder in placeholders {
        let mut result = sample_gate_result();
        result.command = Some(format!("moon run :ci - {}", placeholder));
        assert!(
            result.validate().is_err(),
            "Expected validation to fail for placeholder: {}",
            placeholder
        );
    }
}

#[test]
fn empty_run_id_fails() {
    let mut result = sample_gate_result();
    result.run_id = "".to_string();
    let validation_result = result.validate();
    assert!(validation_result.is_err());
    if let Err(ValidationError::MissingField(field)) = validation_result {
        assert_eq!(field, "run_id");
    } else {
        panic!("Expected MissingField error");
    }
}

#[test]
fn none_log_ref_passes_validation() {
    let result = sample_gate_result();
    assert!(result.log_ref.is_none());
    assert!(result.validate().is_ok());
}

#[test]
fn valid_log_ref_passes_validation() {
    let mut result = sample_gate_result();
    result.log_ref = Some("/var/log/oya/contract-001.log".to_string());
    assert!(result.validate().is_ok());
}

#[test]
fn exit_code_boundary_values_work() {
    // Exit code 0 with passed=true
    let mut result = sample_gate_result();
    result.passed = true;
    result.exit_code = 0;
    assert!(result.validate().is_ok());

    // Exit code 1 with passed=false
    result.passed = false;
    result.exit_code = 1;
    assert!(result.validate().is_ok());

    // Exit code 255 with passed=false
    result.passed = false;
    result.exit_code = 255;
    assert!(result.validate().is_ok());
}
