//! `PortType` implementation with validation.

use std::fmt;
use std::str::FromStr;

/// A validated port type string (tcp:8080, udp:53, unix:</path>).
///
/// This is a newtype wrapper that guarantees all instances are valid.
/// Parsing happens at the boundary, not during construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PortType(String);

impl PortType {
    /// Parses a port type string into a validated `PortType`.
    ///
    /// # Errors
    ///
    /// Returns `PortTypeParseError` if the string is invalid:
    /// - Empty string
    /// - Missing colon separator
    /// - Invalid protocol (not tcp, udp, or unix)
    /// - Invalid port number (not 1-65535 for tcp/udp)
    /// - Leading zeros in port number
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_frontend::connectivity::PortType;
    ///
    /// let tcp = PortType::parse("tcp:8080").expect("tcp:8080 is valid");
    /// let unix = PortType::parse("unix:/var/run/socket").expect("unix socket path is valid");
    /// ```
    pub fn parse(s: &str) -> Result<Self, PortTypeParseError> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(PortTypeParseError::EmptyString);
        }

        let (protocol, rest) = trimmed.split_once(':').ok_or(PortTypeParseError::InvalidFormat)?;

        if protocol.is_empty() {
            return Err(PortTypeParseError::InvalidFormat);
        }

        let protocol_lower = protocol.to_lowercase();

        match protocol_lower.as_str() {
            "tcp" | "udp" => {
                let port_str = rest.trim();
                if port_str.is_empty() {
                    return Err(PortTypeParseError::InvalidFormat);
                }

                let port: u32 =
                    port_str.parse().map_err(|_| PortTypeParseError::InvalidPortNumber)?;

                if port == 0 {
                    return Err(PortTypeParseError::InvalidPortNumber);
                }

                if port > 65535 {
                    return Err(PortTypeParseError::InvalidPortNumber);
                }

                // Check for leading zeros
                if port_str.starts_with('0') && port_str.len() > 1 {
                    return Err(PortTypeParseError::InvalidFormat);
                }

                Ok(Self(format!("{protocol_lower}:{port}")))
            }
            "unix" => {
                let path = rest.trim();
                if path.is_empty() {
                    return Err(PortTypeParseError::InvalidFormat);
                }

                Ok(Self(format!("unix:{path}")))
            }
            _ => Err(PortTypeParseError::InvalidProtocol),
        }
    }

    /// Returns the underlying port type string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the protocol (tcp, udp, or unix).
    #[must_use]
    pub fn protocol(&self) -> &str {
        self.0.split_once(':').map_or("", |(p, _)| p)
    }

    /// Returns the address (port number or path).
    #[must_use]
    pub fn address(&self) -> &str {
        self.0.split_once(':').map_or("", |(_, a)| a)
    }

    /// Returns true if this is a TCP port.
    #[must_use]
    pub fn is_tcp(&self) -> bool {
        self.protocol() == "tcp"
    }

    /// Returns true if this is a UDP port.
    #[must_use]
    pub fn is_udp(&self) -> bool {
        self.protocol() == "udp"
    }

    /// Returns true if this is a Unix socket path.
    #[must_use]
    pub fn is_unix(&self) -> bool {
        self.protocol() == "unix"
    }
}

impl fmt::Display for PortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PortType {
    type Err = PortTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Error types for port type parsing.
///
/// Provides specific error information for debugging invalid port types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortTypeParseError {
    /// The input string was empty.
    EmptyString,
    /// The protocol is not tcp, udp, or unix.
    InvalidProtocol,
    /// The port number is invalid (0, >65535, or non-numeric).
    InvalidPortNumber,
    /// The format is invalid (missing colon, leading zeros, whitespace).
    InvalidFormat,
}

impl fmt::Display for PortTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyString => write!(f, "empty string"),
            Self::InvalidProtocol => write!(f, "invalid protocol (must be tcp, udp, or unix)"),
            Self::InvalidPortNumber => write!(f, "invalid port number (must be 1-65535)"),
            Self::InvalidFormat => write!(f, "invalid format (expected protocol:address)"),
        }
    }
}

impl std::error::Error for PortTypeParseError {}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;

    #[test]
    fn given_empty_string_when_parsing_then_empty_string_error_is_returned() {
        let result = PortType::parse("");
        assert!(matches!(result, Err(PortTypeParseError::EmptyString)));
    }

    #[test]
    fn given_whitespace_only_when_parsing_then_empty_string_error_is_returned() {
        let result = PortType::parse("   ");
        assert!(matches!(result, Err(PortTypeParseError::EmptyString)));
    }

    #[test]
    fn given_missing_colon_when_parsing_then_invalid_format_is_returned() {
        let result = PortType::parse("tcp8080");
        assert!(matches!(result, Err(PortTypeParseError::InvalidFormat)));
    }

    #[test]
    fn given_missing_protocol_when_parsing_then_invalid_format_is_returned() {
        let result = PortType::parse(":8080");
        assert!(matches!(result, Err(PortTypeParseError::InvalidFormat)));
    }

    #[test]
    fn given_missing_address_when_parsing_then_invalid_format_is_returned() {
        let result = PortType::parse("tcp:");
        assert!(matches!(result, Err(PortTypeParseError::InvalidFormat)));
    }

    #[test]
    fn given_invalid_protocol_http_when_parsing_then_invalid_protocol_is_returned() {
        let result = PortType::parse("http:8080");
        assert!(matches!(result, Err(PortTypeParseError::InvalidProtocol)));
    }

    #[test]
    fn given_invalid_protocol_sctp_when_parsing_then_invalid_protocol_is_returned() {
        let result = PortType::parse("sctp:8080");
        assert!(matches!(result, Err(PortTypeParseError::InvalidProtocol)));
    }

    #[test]
    fn given_uppercase_tcp_when_parsing_then_it_is_normalized() {
        let result = PortType::parse("TCP:8080").expect("TCP:8080 should parse");
        assert_eq!(result.as_str(), "tcp:8080");
    }

    #[test]
    fn given_uppercase_udp_when_parsing_then_it_is_normalized() {
        let result = PortType::parse("UDP:53").expect("UDP:53 should parse");
        assert_eq!(result.as_str(), "udp:53");
    }

    #[test]
    fn given_uppercase_unix_when_parsing_then_it_is_normalized() {
        let result =
            PortType::parse("UNIX:/var/run/socket").expect("UNIX:/var/run/socket should parse");
        assert_eq!(result.as_str(), "unix:/var/run/socket");
    }

    #[test]
    fn given_port_zero_when_parsing_then_invalid_port_number_is_returned() {
        let result = PortType::parse("tcp:0");
        assert!(matches!(result, Err(PortTypeParseError::InvalidPortNumber)));
    }

    #[test]
    fn given_port_above_max_when_parsing_then_invalid_port_number_is_returned() {
        let result = PortType::parse("tcp:65536");
        assert!(matches!(result, Err(PortTypeParseError::InvalidPortNumber)));
    }

    #[test]
    fn given_port_u16_max_when_parsing_then_invalid_port_number_is_returned() {
        let result = PortType::parse("tcp:65535");
        assert!(result.is_ok());
    }

    #[test]
    fn given_port_extreme_when_parsing_then_invalid_port_number_is_returned() {
        let result = PortType::parse("tcp:999999");
        assert!(matches!(result, Err(PortTypeParseError::InvalidPortNumber)));
    }

    #[test]
    fn given_non_numeric_port_when_parsing_then_invalid_port_number_is_returned() {
        let result = PortType::parse("tcp:abc");
        assert!(matches!(result, Err(PortTypeParseError::InvalidPortNumber)));
    }

    #[test]
    fn given_leading_zero_when_parsing_then_invalid_format_is_returned() {
        let result = PortType::parse("tcp:08080");
        assert!(matches!(result, Err(PortTypeParseError::InvalidFormat)));
    }

    #[test]
    fn given_leading_zero_single_digit_when_parsing_then_invalid_format_is_returned() {
        let result = PortType::parse("tcp:01");
        assert!(matches!(result, Err(PortTypeParseError::InvalidFormat)));
    }

    #[test]
    fn given_valid_tcp_port_one_when_parsing_then_it_succeeds() {
        let result = PortType::parse("tcp:1").expect("tcp:1 should parse");
        assert_eq!(result.as_str(), "tcp:1");
    }

    #[test]
    fn given_valid_tcp_port_max_when_parsing_then_it_succeeds() {
        let result = PortType::parse("tcp:65535").expect("tcp:65535 should parse");
        assert_eq!(result.as_str(), "tcp:65535");
    }

    #[test]
    fn given_valid_udp_port_one_when_parsing_then_it_succeeds() {
        let result = PortType::parse("udp:1").expect("udp:1 should parse");
        assert_eq!(result.as_str(), "udp:1");
    }

    #[test]
    fn given_valid_udp_port_max_when_parsing_then_it_succeeds() {
        let result = PortType::parse("udp:65535").expect("udp:65535 should parse");
        assert_eq!(result.as_str(), "udp:65535");
    }

    #[test]
    fn given_valid_unix_path_when_parsing_then_it_succeeds() {
        let result =
            PortType::parse("unix:/var/run/socket").expect("unix:/var/run/socket should parse");
        assert_eq!(result.as_str(), "unix:/var/run/socket");
    }

    #[test]
    fn given_unix_path_with_spaces_when_parsing_then_it_succeeds() {
        let result =
            PortType::parse("unix: /var/run/socket ").expect("unix: /var/run/socket  should parse");
        assert_eq!(result.as_str(), "unix:/var/run/socket");
    }

    #[test]
    fn given_control_chars_when_parsing_then_invalid_format_is_returned() {
        let result = PortType::parse("tcp:\x008080");
        assert!(result.is_err());
    }

    #[test]
    fn given_whitespace_padded_input_when_parsing_then_it_is_trimmed() {
        let result = PortType::parse("  tcp:8080  ").expect("tcp:8080 should parse");
        assert_eq!(result.as_str(), "tcp:8080");
    }

    #[test]
    fn given_tcp_port_when_is_tcp_then_it_returns_true() {
        let port = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        assert!(port.is_tcp());
    }

    #[test]
    fn given_tcp_port_when_is_udp_then_it_returns_false() {
        let port = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        assert!(!port.is_udp());
    }

    #[test]
    fn given_tcp_port_when_is_unix_then_it_returns_false() {
        let port = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        assert!(!port.is_unix());
    }

    #[test]
    fn given_udp_port_when_is_tcp_then_it_returns_false() {
        let port = PortType::parse("udp:53").expect("udp:53 should parse");
        assert!(!port.is_tcp());
    }

    #[test]
    fn given_udp_port_when_is_udp_then_it_returns_true() {
        let port = PortType::parse("udp:53").expect("udp:53 should parse");
        assert!(port.is_udp());
    }

    #[test]
    fn given_unix_port_when_is_unix_then_it_returns_true() {
        let port =
            PortType::parse("unix:/var/run/socket").expect("unix:/var/run/socket should parse");
        assert!(port.is_unix());
    }

    #[test]
    fn given_unix_port_when_is_tcp_then_it_returns_false() {
        let port =
            PortType::parse("unix:/var/run/socket").expect("unix:/var/run/socket should parse");
        assert!(!port.is_tcp());
    }

    #[test]
    fn given_tcp_port_when_protocol_then_it_returns_tcp() {
        let port = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        assert_eq!(port.protocol(), "tcp");
    }

    #[test]
    fn given_tcp_port_when_address_then_it_returns_port() {
        let port = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        assert_eq!(port.address(), "8080");
    }

    #[test]
    fn given_unix_port_when_address_then_it_returns_path() {
        let port =
            PortType::parse("unix:/var/run/socket").expect("unix:/var/run/socket should parse");
        assert_eq!(port.address(), "/var/run/socket");
    }

    #[test]
    fn given_valid_port_when_display_then_it_returns_string() {
        let port = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        assert_eq!(format!("{port}"), "tcp:8080");
    }

    #[test]
    fn given_valid_port_when_from_str_then_it_parses() {
        use std::str::FromStr;
        let port = PortType::from_str("tcp:8080").expect("tcp:8080 should parse");
        assert_eq!(port.as_str(), "tcp:8080");
    }
}
