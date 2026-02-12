//! Plugin configuration validation module
//!
//! This module provides validation for plugin configuration including:
//! - Size field validation (rows, cols must be positive and reasonable)
//! - Configuration schema validation (required fields, data types)
//! - Error handling with clear, actionable messages
//!
//! Validation errors use thiserror for domain-specific error types.

use serde_json::Value;
use thiserror::Error;

const MIN_ROWS: usize = 1;
const MAX_ROWS: usize = 500;
const MIN_COLS: usize = 1;
const MAX_COLS: usize = 500;
const MIN_AUTO_SAVE_INTERVAL: u64 = 10;
const MAX_AUTO_SAVE_INTERVAL: u64 = 600;

/// Configuration validation errors
#[derive(Debug, Error)]
pub enum ConfigValidationError {
    #[error("Invalid size: rows must be between {min_rows} and {max_rows}, got {rows}")]
    InvalidRows {
        rows: usize,
        min_rows: usize,
        max_rows: usize,
    },

    #[error("Invalid size: cols must be between {min_cols} and {max_cols}, got {cols}")]
    InvalidCols {
        cols: usize,
        min_cols: usize,
        max_cols: usize,
    },

    #[error("Size object is missing or invalid: {0}")]
    MissingSize(String),

    #[error("Required field '{field}' is missing")]
    MissingRequiredField { field: String },

    #[error("Field '{field}' has wrong type: expected {expected}, got {actual}")]
    WrongType {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Field '{field}' has invalid value: {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("Auto-save interval must be between {min_interval} and {max_interval} seconds, got {interval}")]
    InvalidAutoSaveInterval {
        interval: u64,
        min_interval: u64,
        max_interval: u64,
    },

    #[error("IPC address format is invalid: {address}")]
    InvalidIpcAddress { address: String },

    #[error("Configuration is not an object")]
    NotAnObject,

    #[error("Unknown field '{field}' is not allowed")]
    UnknownField { field: String },

    #[error("Configuration parsing failed: {0}")]
    ParseError(String),
}

/// Validated plugin configuration
#[derive(Debug, Clone)]
pub struct ValidatedConfig {
    /// Auto-save interval in seconds
    pub auto_save_interval_secs: u64,
    /// IPC address for orchestrator connection
    pub ipc_address: String,
}

/// Validate terminal size
///
/// # Errors
///
/// Returns an error if rows or cols are outside valid range
pub fn validate_size(rows: usize, cols: usize) -> Result<(), ConfigValidationError> {
    if rows < MIN_ROWS || rows > MAX_ROWS {
        return Err(ConfigValidationError::InvalidRows {
            rows,
            min_rows: MIN_ROWS,
            max_rows: MAX_ROWS,
        });
    }

    if cols < MIN_COLS || cols > MAX_COLS {
        return Err(ConfigValidationError::InvalidCols {
            cols,
            min_cols: MIN_COLS,
            max_cols: MAX_COLS,
        });
    }

    Ok(())
}

/// Validate size from JSON value
///
/// # Errors
///
/// Returns an error if size is missing, has wrong types, or values are invalid
pub fn validate_size_from_json(
    size_value: &Value,
) -> Result<(usize, usize), ConfigValidationError> {
    let rows = size_value
        .get("rows")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ConfigValidationError::MissingRequiredField {
            field: "size.rows".to_string(),
        })? as usize;

    let cols = size_value
        .get("cols")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ConfigValidationError::MissingRequiredField {
            field: "size.cols".to_string(),
        })? as usize;

    validate_size(rows, cols).map(|_| (rows, cols))
}

/// Validate auto-save interval
///
/// # Errors
///
/// Returns an error if interval is outside valid range
pub fn validate_auto_save_interval(interval: u64) -> Result<u64, ConfigValidationError> {
    if interval < MIN_AUTO_SAVE_INTERVAL || interval > MAX_AUTO_SAVE_INTERVAL {
        return Err(ConfigValidationError::InvalidAutoSaveInterval {
            interval,
            min_interval: MIN_AUTO_SAVE_INTERVAL,
            max_interval: MAX_AUTO_SAVE_INTERVAL,
        });
    }

    Ok(interval)
}

/// Validate IPC address format
///
/// # Errors
///
/// Returns an error if address format is invalid
pub fn validate_ipc_address(address: &str) -> Result<String, ConfigValidationError> {
    if address.is_empty() {
        return Err(ConfigValidationError::InvalidIpcAddress {
            address: address.to_string(),
        });
    }

    // Accept various valid formats
    if address.starts_with("stdio://")
        || address.starts_with("tcp://")
        || address.starts_with("127.0.0.1:")
        || address.starts_with("localhost:")
        || address.starts_with('/')
    {
        return Ok(address.to_string());
    }

    // Check if it looks like host:port
    if address.contains(':') {
        let parts: Vec<&str> = address.rsplitn(2, ':').collect();
        if parts.len() == 2 {
            let host = parts[1];
            let port = parts[0];
            if host.parse::<u16>().is_ok() || port.parse::<u16>().is_ok() {
                return Ok(address.to_string());
            }
        }
    }

    Err(ConfigValidationError::InvalidIpcAddress {
        address: address.to_string(),
    })
}

/// Get a human-readable type name for a JSON value
fn json_type_name(v: &Value) -> &'static str {
    if v.is_null() {
        "null"
    } else if v.is_boolean() {
        "boolean"
    } else if v.is_number() {
        "number"
    } else if v.is_string() {
        "string"
    } else if v.is_array() {
        "array"
    } else if v.is_object() {
        "object"
    } else {
        "unknown"
    }
}

/// Validate configuration object
///
/// # Errors
///
/// Returns errors for missing required fields, wrong types, or invalid values
pub fn validate_config(config: &Value) -> Result<ValidatedConfig, ConfigValidationError> {
    // Ensure config is an object
    let obj = config
        .as_object()
        .ok_or(ConfigValidationError::NotAnObject)?;

    // Validate auto_save_interval_secs
    let auto_save_interval_secs = match obj.get("auto_save_interval_secs") {
        Some(v) => {
            let interval = v.as_u64().ok_or_else(|| ConfigValidationError::WrongType {
                field: "auto_save_interval_secs".to_string(),
                expected: "number".to_string(),
                actual: json_type_name(v).to_string(),
            })?;
            validate_auto_save_interval(interval)?
        }
        None => 30, // Default value
    };

    // Validate ipc_address
    let ipc_address = match obj.get("ipc_address") {
        Some(v) => {
            let address = v.as_str().ok_or_else(|| ConfigValidationError::WrongType {
                field: "ipc_address".to_string(),
                expected: "string".to_string(),
                actual: json_type_name(v).to_string(),
            })?;
            validate_ipc_address(address)?
        }
        None => "127.0.0.1:5555".to_string(), // Default value
    };

    Ok(ValidatedConfig {
        auto_save_interval_secs,
        ipc_address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_size_valid() {
        assert!(validate_size(24, 80).is_ok());
        assert!(validate_size(1, 1).is_ok());
        assert!(validate_size(500, 500).is_ok());
    }

    #[test]
    fn test_validate_size_invalid_rows() {
        let result = validate_size(0, 80);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid size: rows"));

        let result = validate_size(501, 80);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_size_invalid_cols() {
        let result = validate_size(24, 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid size: cols"));

        let result = validate_size(24, 501);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_auto_save_interval_valid() {
        assert_eq!(validate_auto_save_interval(10).unwrap(), 10);
        assert_eq!(validate_auto_save_interval(30).unwrap(), 30);
        assert_eq!(validate_auto_save_interval(600).unwrap(), 600);
    }

    #[test]
    fn test_validate_auto_save_interval_invalid() {
        let result = validate_auto_save_interval(9);
        assert!(result.is_err());

        let result = validate_auto_save_interval(601);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_ipc_address_valid() {
        assert!(validate_ipc_address("127.0.0.1:5555").is_ok());
        assert!(validate_ipc_address("localhost:8080").is_ok());
        assert!(validate_ipc_address("stdio://zellij").is_ok());
        assert!(validate_ipc_address("/tmp/oya-ipc").is_ok());
    }

    #[test]
    fn test_validate_ipc_address_invalid() {
        assert!(validate_ipc_address("").is_err());
        assert!(validate_ipc_address("not-a-valid-address").is_err());
    }

    #[test]
    fn test_validate_config_valid() {
        let config = json!({
            "auto_save_interval_secs": 30,
            "ipc_address": "127.0.0.1:5555"
        });
        let result = validate_config(&config);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.auto_save_interval_secs, 30);
        assert_eq!(validated.ipc_address, "127.0.0.1:5555");
    }

    #[test]
    fn test_validate_config_defaults() {
        let config = json!({});
        let result = validate_config(&config);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.auto_save_interval_secs, 30);
        assert_eq!(validated.ipc_address, "127.0.0.1:5555");
    }

    #[test]
    fn test_validate_config_not_object() {
        let config = json!(123);
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_size_from_json_valid() {
        let size = json!({"rows": 24, "cols": 80});
        let result = validate_size_from_json(&size);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (24, 80));
    }

    #[test]
    fn test_validate_size_from_json_missing_fields() {
        let size = json!({"rows": 24});
        let result = validate_size_from_json(&size);
        assert!(result.is_err());
    }
}
