//! Connection validation module.
//!
//! Contains pure validation functions that check port type compatibility.
//! These functions have no side effects and can be used in both
//! core (calculation) and shell (action) layers.

use super::ConnectionError;
use super::PortType;

/// Validates that a connection between two endpoints is valid.
///
/// Compatibility rules:
/// - TCP can connect to TCP
/// - TCP can connect to UDP
/// - UDP can connect to TCP
/// - UDP can connect to UDP
/// - Unix can only connect to Unix
/// - TCP cannot connect to Unix (and vice versa)
///
/// # Errors
///
/// Returns `ConnectionError::TypeMismatch` if the port types are incompatible,
/// or `ConnectionError::PathNotFound` if a unix path does not exist.
///
/// # Examples
///
/// ```
/// use oya_frontend::connectivity::{validate_connection, PortType};
///
/// let tcp = PortType::parse("tcp:8080").expect("tcp:8080 is valid");
/// let udp = PortType::parse("udp:53").expect("udp:53 is valid");
///
/// assert!(validate_connection("tcp:8080", "udp:53", "", "").is_ok());
/// ```
pub fn validate_connection(
    source_port_str: &str,
    destination_port_str: &str,
    source_path: &str,
    destination_path: &str,
) -> Result<(), ConnectionError> {
    // Parse ports
    let source = PortType::parse(source_port_str)
        .map_err(|e| ConnectionError::ValidationFailed(format!("Invalid source port: {e}")))?;
    let destination = PortType::parse(destination_port_str)
        .map_err(|e| ConnectionError::ValidationFailed(format!("Invalid destination port: {e}")))?;

    // Check compatibility first (before path validation)
    check_compatibility(&source, &destination)?;

    // Check path existence for unix sockets
    if source.is_unix() {
        // Use port address if path parameter is empty
        let path = if source_path.is_empty() { source.address() } else { source_path };
        if path.is_empty() {
            return Err(ConnectionError::ValidationFailed("Source path is empty".to_string()));
        }
        if !std::path::Path::new(path).exists() {
            return Err(ConnectionError::PathNotFound(path.to_string()));
        }
    }

    if destination.is_unix() {
        // Use port address if path parameter is empty
        let path =
            if destination_path.is_empty() { destination.address() } else { destination_path };
        if path.is_empty() {
            return Err(ConnectionError::ValidationFailed("Destination path is empty".to_string()));
        }
        if !std::path::Path::new(path).exists() {
            return Err(ConnectionError::PathNotFound(path.to_string()));
        }
    }

    Ok(())
}

/// Checks if two port types are compatible.
fn check_compatibility(source: &PortType, destination: &PortType) -> Result<(), ConnectionError> {
    let source_is_tcp = source.is_tcp();
    let source_is_udp = source.is_udp();
    let source_is_unix = source.is_unix();

    let dest_is_tcp = destination.is_tcp();
    let dest_is_udp = destination.is_udp();
    let dest_is_unix = destination.is_unix();

    // TCP can connect to TCP or UDP
    if source_is_tcp && (dest_is_tcp || dest_is_udp) {
        return Ok(());
    }

    // UDP can connect to TCP or UDP
    if source_is_udp && (dest_is_tcp || dest_is_udp) {
        return Ok(());
    }

    // Unix can only connect to Unix
    if source_is_unix && dest_is_unix {
        return Ok(());
    }

    // Everything else is a mismatch
    Err(ConnectionError::TypeMismatch { source: source.clone(), destination: destination.clone() })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn given_compatibility_rule_unix_tcp_disallowed_when_validate_connection_then_type_mismatch_is_returned(
    ) {
        let result = validate_connection("unix:/var/run/socket", "tcp:8080", "", "");

        assert!(matches!(result, Err(ConnectionError::TypeMismatch { .. })));
    }

    #[test]
    fn given_compatibility_rule_unix_tcp_disallowed_with_paths_when_validate_connection_then_type_mismatch_is_returned(
    ) {
        let result = validate_connection("unix:/tmp/socket1", "tcp:8080", "", "");

        assert!(matches!(result, Err(ConnectionError::TypeMismatch { .. })));
    }

    #[test]
    fn given_compatibility_rule_unix_tcp_disallowed_swapped_when_validate_connection_then_type_mismatch_is_returned(
    ) {
        let result = validate_connection("tcp:8080", "unix:/var/run/socket", "", "");

        assert!(matches!(result, Err(ConnectionError::TypeMismatch { .. })));
    }

    #[test]
    fn given_compatibility_rule_tcp_unix_disallowed_when_validate_connection_then_type_mismatch_is_returned(
    ) {
        let result = validate_connection("tcp:8080", "unix:/var/run/socket", "", "");

        assert!(matches!(result, Err(ConnectionError::TypeMismatch { .. })));
    }

    #[test]
    fn given_compatibility_rule_tcp_unix_disallowed_with_paths_when_validate_connection_then_type_mismatch_is_returned(
    ) {
        let result = validate_connection("tcp:8080", "unix:/tmp/socket2", "", "");

        assert!(matches!(result, Err(ConnectionError::TypeMismatch { .. })));
    }

    #[test]
    fn given_compatibility_rule_unix_udp_disallowed_when_validate_connection_then_type_mismatch_is_returned(
    ) {
        let result = validate_connection("unix:/var/run/socket", "udp:53", "", "");

        assert!(matches!(result, Err(ConnectionError::TypeMismatch { .. })));
    }

    #[test]
    fn given_valid_tcp_ports_when_validate_connection_then_it_succeeds() {
        let result = validate_connection("tcp:8080", "tcp:9090", "", "");

        assert!(result.is_ok());
    }

    #[test]
    fn given_valid_tcp_to_udp_when_validate_connection_then_it_succeeds() {
        let result = validate_connection("tcp:8080", "udp:53", "", "");

        assert!(result.is_ok());
    }
}
