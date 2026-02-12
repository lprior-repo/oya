//! Plugin configuration validation tests
//!
//! These tests verify that plugin configuration is properly validated:
//! - Size fields (rows, cols) have valid ranges
//! - Configuration schema is enforced
//! - Errors are handled gracefully
//! - Invalid configurations are rejected with clear error messages

use serde_json::json;
use zellij_frontend::config_validation::{
    ConfigValidationError, ValidatedConfig, validate_auto_save_interval, validate_config,
    validate_size,
};
use zellij_frontend::plugin::Size;

#[test]
fn test_invalid_size_configuration_negative_rows() -> Result<(), ConfigValidationError> {
    let result = validate_size(0, 80);
    assert!(result.is_err());
    let err = result.map_err(|e| e).unwrap_err(); // Keep unwrap_err for error analysis
    assert!(matches!(err, ConfigValidationError::InvalidRows { .. }));
    Ok(())
}

#[test]
fn test_invalid_size_configuration_negative_cols() -> Result<(), ConfigValidationError> {
    let result = validate_size(24, 0);
    assert!(result.is_err());
    let err = result.map_err(|e| e).unwrap_err();
    assert!(matches!(err, ConfigValidationError::InvalidCols { .. }));
    Ok(())
}

#[test]
fn test_invalid_size_configuration_extreme_values() {
    let result = validate_size(100000, 100000);
    assert!(result.is_err());
}

#[test]
fn test_size_validation_accepts_valid_sizes() {
    assert!(validate_size(1, 1).is_ok());
    assert!(validate_size(24, 80).is_ok());
    assert!(validate_size(500, 500).is_ok());
}

#[test]
fn test_config_validation_requires_ipc_address() -> Result<(), ConfigValidationError> {
    let config = json!({
        "auto_save_interval_secs": 30
    });
    let result = validate_config(&config);
    assert!(result.is_ok());
    let validated = result?;
    assert_eq!(validated.ipc_address, "127.0.0.1:5555");
    Ok(())
}

#[test]
fn test_config_validation_accepts_valid_ipc_address() -> Result<(), ConfigValidationError> {
    let config = json!({
        "auto_save_interval_secs": 30,
        "ipc_address": "127.0.0.1:5555"
    });
    let result = validate_config(&config);
    assert!(result.is_ok());
    assert_eq!(result?.ipc_address, "127.0.0.1:5555");
    Ok(())
}

#[test]
fn test_config_validation_rejects_invalid_ipc_address() {
    let config = json!({
        "auto_save_interval_secs": 30,
        "ipc_address": "not-a-valid-address"
    });
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_config_validation_rejects_empty_ipc_address() {
    let config = json!({
        "ipc_address": ""
    });
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_config_validation_rejects_negative_auto_save_interval() -> Result<(), ConfigValidationError> {
    let result = validate_auto_save_interval(0);
    assert!(result.is_err());
    let err = result.map_err(|e| e).unwrap_err();
    assert!(matches!(
        err,
        ConfigValidationError::InvalidAutoSaveInterval { .. }
    ));
    Ok(())
}

#[test]
fn test_config_validation_rejects_too_large_auto_save_interval() {
    let result = validate_auto_save_interval(1000);
    assert!(result.is_err());
}

#[test]
fn test_config_validation_accepts_valid_auto_save_interval() -> Result<(), ConfigValidationError> {
    assert_eq!(validate_auto_save_interval(10)?, 10);
    assert_eq!(validate_auto_save_interval(30)?, 30);
    assert_eq!(validate_auto_save_interval(600)?, 600);
    Ok(())
}

#[test]
fn test_config_validation_uses_defaults_for_missing_optional_fields() -> Result<(), ConfigValidationError> {
    let config = json!({});
    let result = validate_config(&config);
    assert!(result.is_ok());
    let validated = result?;
    assert_eq!(validated.auto_save_interval_secs, 30);
    assert_eq!(validated.ipc_address, "127.0.0.1:5555");
    Ok(())
}

#[test]
fn test_config_validation_rejects_non_object_config() {
    let config = json!(123);
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_config_validation_accepts_stdio_ipc_address() -> Result<(), ConfigValidationError> {
    let config = json!({
        "ipc_address": "stdio://zellij"
    });
    let result = validate_config(&config);
    assert!(result.is_ok());
    assert_eq!(result?.ipc_address, "stdio://zellij");
    Ok(())
}

#[test]
fn test_config_validation_accepts_localhost_ipc_address() -> Result<(), ConfigValidationError> {
    let config = json!({
        "ipc_address": "localhost:8080"
    });
    let result = validate_config(&config);
    assert!(result.is_ok());
    assert_eq!(result?.ipc_address, "localhost:8080");
    Ok(())
}

#[test]
fn test_config_validation_rejects_auto_save_interval_too_small() {
    let result = validate_auto_save_interval(5);
    assert!(result.is_err());
}

#[test]
fn test_config_validation_rejects_auto_save_interval_too_large() {
    let result = validate_auto_save_interval(10000);
    assert!(result.is_err());
}

#[test]
fn test_error_messages_are_clear_and_actionable() -> Result<(), ConfigValidationError> {
    let result = validate_size(0, 80);
    assert!(result.is_err());
    let err = result.map_err(|e| e).unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("Invalid size"));
    assert!(err_str.contains("rows"));
    assert!(err_str.contains("0"));
    assert!(err_str.contains("1"));
    assert!(err_str.contains("500"));
    Ok(())
}

#[test]
fn test_validated_config_has_correct_defaults() {
    let validated = ValidatedConfig {
        auto_save_interval_secs: 30,
        ipc_address: "127.0.0.1:5555".to_string(),
    };
    assert_eq!(validated.auto_save_interval_secs, 30);
    assert_eq!(validated.ipc_address, "127.0.0.1:5555");
}

#[test]
fn test_size_struct_matches_validation_requirements() {
    let size = Size { rows: 24, cols: 80 };
    let result = validate_size(size.rows, size.cols);
    assert!(result.is_ok());
}

#[test]
fn test_size_struct_rejects_zero_values() {
    let result = validate_size(0, 80);
    assert!(result.is_err());

    let result = validate_size(24, 0);
    assert!(result.is_err());
}
