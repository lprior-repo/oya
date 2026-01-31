//! Domain types for Intent CLI
//!
//! All types use newtype pattern for type safety and validation.
//! No public constructors - use `try_new()` for validated construction.
//!
//! # Philosophy
//!
//! - **Type-driven design**: Make invalid states unrepresentable
//! - **Validated construction**: All types validate their invariants on creation
//! - **Zero panics**: Use `Result` for fallible construction
//! - **Newtype pattern**: Wrap primitives for type safety

use std::{fmt, path::Path, str::FromStr};

use crate::error::IntentError;

// =============================================================================
// Spec Types
// =============================================================================

/// Validated specification name
///
/// Must be non-empty, alphanumeric + hyphens/underscores only, and end with .cue
///
/// # Examples
///
/// ```
/// use intent_core::types::SpecName;
///
/// let name = SpecName::try_new("user-api.cue").unwrap();
/// assert_eq!(name.as_str(), "user-api.cue");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecName(String);

impl SpecName {
    /// Create a new `SpecName` with validation
    ///
    /// # Errors
    ///
    /// Returns `IntentError::Validation` if:
    /// - Name is empty
    /// - Name doesn't end with .cue
    /// - Name contains invalid characters
    pub fn try_new(name: impl Into<String>) -> Result<Self, IntentError> {
        let name = name.into();

        if name.is_empty() {
            return Err(IntentError::validation(
                "spec_name",
                "Spec name cannot be empty",
            ));
        }

        if !Path::new(&name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cue"))
        {
            return Err(IntentError::validation(
                "spec_name",
                format!("Spec name must end with .cue: '{name}'"),
            ));
        }

        // Check for valid characters (alphanumeric, hyphens, underscores, dots, slashes for paths)
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
        {
            return Err(IntentError::validation(
                "spec_name",
                format!("Spec name contains invalid characters: '{name}'. Only alphanumeric, hyphens, underscores, dots, and slashes allowed"),
            ));
        }

        Ok(Self(name))
    }

    /// Get the spec name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to `PathBuf`
    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    /// Get the base name without .cue extension
    #[must_use]
    pub fn base_name(&self) -> &str {
        self.0.strip_suffix(".cue").unwrap_or(&self.0)
    }
}

impl fmt::Display for SpecName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SpecName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<Path> for SpecName {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

// =============================================================================
// URL Types
// =============================================================================

/// Validated URL
///
/// Wraps `url::Url` with validation to ensure only valid URLs are constructed.
/// Provides convenient helper methods for common URL operations.
///
/// # Examples
///
/// ```
/// use intent_core::types::Url;
///
/// let url = Url::try_new("https://api.example.com/v1/users").unwrap();
/// assert_eq!(url.scheme(), "https");
/// assert_eq!(url.host(), Some("api.example.com"));
/// assert_eq!(url.path(), "/v1/users");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url(url::Url);

impl Url {
    /// Create a new Url with validation
    ///
    /// # Errors
    ///
    /// Returns `IntentError::Validation` if the URL is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use intent_core::types::Url;
    ///
    /// let url = Url::try_new("https://example.com").unwrap();
    /// assert_eq!(url.as_str(), "https://example.com/");
    /// ```
    pub fn try_new(url: impl AsRef<str>) -> Result<Self, IntentError> {
        let url_str = url.as_ref();
        url::Url::parse(url_str).map(Self).map_err(|e| {
            IntentError::validation("url", format!("Invalid URL '{url_str}': {e}"))
        })
    }

    /// Get the URL as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Get the URL scheme (e.g., "http", "https")
    ///
    /// # Examples
    ///
    /// ```
    /// use intent_core::types::Url;
    ///
    /// let url = Url::try_new("https://example.com").unwrap();
    /// assert_eq!(url.scheme(), "https");
    /// ```
    #[must_use]
    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    /// Get the URL host as a string
    ///
    /// Returns `None` if the URL has no host (e.g., file:// URLs).
    ///
    /// # Examples
    ///
    /// ```
    /// use intent_core::types::Url;
    ///
    /// let url = Url::try_new("https://api.example.com:8080/path").unwrap();
    /// assert_eq!(url.host(), Some("api.example.com"));
    /// ```
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        self.0.host_str()
    }

    /// Get the URL path
    ///
    /// # Examples
    ///
    /// ```
    /// use intent_core::types::Url;
    ///
    /// let url = Url::try_new("https://example.com/v1/users?id=123").unwrap();
    /// assert_eq!(url.path(), "/v1/users");
    /// ```
    #[must_use]
    pub fn path(&self) -> &str {
        self.0.path()
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

// =============================================================================
// HTTP Types
// =============================================================================

/// HTTP request method
///
/// Standard HTTP methods used in API testing.
/// Supports case-insensitive parsing from strings.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
///
/// use intent_core::types::HttpMethod;
///
/// let method = HttpMethod::from_str("GET").unwrap();
/// assert_eq!(method, HttpMethod::Get);
/// assert_eq!(method.to_string(), "GET");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Patch => write!(f, "PATCH"),
            Self::Delete => write!(f, "DELETE"),
            Self::Head => write!(f, "HEAD"),
            Self::Options => write!(f, "OPTIONS"),
        }
    }
}

impl FromStr for HttpMethod {
    type Err = IntentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "PATCH" => Ok(Self::Patch),
            "DELETE" => Ok(Self::Delete),
            "HEAD" => Ok(Self::Head),
            "OPTIONS" => Ok(Self::Options),
            _ => Err(IntentError::validation(
                "http_method",
                format!("Invalid HTTP method: '{s}'. Must be GET, POST, PUT, PATCH, DELETE, HEAD, or OPTIONS"),
            )),
        }
    }
}

/// HTTP header name
///
/// Validated header name following RFC 7230.
/// Stores names in lowercase for case-insensitive comparison.
/// Only allows alphanumeric characters and hyphens.
///
/// # Examples
///
/// ```
/// use intent_core::types::HeaderName;
///
/// let name = HeaderName::try_new("Content-Type").unwrap();
/// assert_eq!(name.as_str(), "content-type");
///
/// // Case-insensitive equality
/// let name1 = HeaderName::try_new("Content-Type").unwrap();
/// let name2 = HeaderName::try_new("content-type").unwrap();
/// assert_eq!(name1, name2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(String);

impl HeaderName {
    /// Create a new `HeaderName` with validation
    ///
    /// The name is stored in lowercase for case-insensitive comparison.
    /// Only alphanumeric characters and hyphens are allowed.
    ///
    /// # Errors
    ///
    /// Returns `IntentError::Validation` if:
    /// - Name is empty
    /// - Name contains invalid characters (only alphanumeric and hyphen allowed)
    ///
    /// # Examples
    ///
    /// ```
    /// use intent_core::types::HeaderName;
    ///
    /// let name = HeaderName::try_new("Content-Type").unwrap();
    /// assert_eq!(name.as_str(), "content-type");
    ///
    /// let invalid = HeaderName::try_new("Invalid@Header");
    /// assert!(invalid.is_err());
    /// ```
    pub fn try_new(name: impl Into<String>) -> Result<Self, IntentError> {
        let name = name.into();

        if name.is_empty() {
            return Err(IntentError::validation(
                "header_name",
                "Header name cannot be empty",
            ));
        }

        // Validate characters: alphanumeric and hyphens only
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(IntentError::validation(
                "header_name",
                format!("Header name contains invalid characters: '{name}'. Only ASCII alphanumeric and hyphens allowed"),
            ));
        }

        // Store lowercase for case-insensitive comparison
        Ok(Self(name.to_ascii_lowercase()))
    }

    /// Get the header name as a string slice
    ///
    /// Returns the name in lowercase form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for HeaderName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// HTTP header value
///
/// Validated header value that supports any ASCII string.
/// Allows printable ASCII characters and whitespace.
///
/// # Examples
///
/// ```
/// use intent_core::types::HeaderValue;
///
/// let value = HeaderValue::try_new("application/json").unwrap();
/// assert_eq!(value.as_str(), "application/json");
///
/// let with_spaces = HeaderValue::try_new("Bearer token123").unwrap();
/// assert_eq!(with_spaces.as_str(), "Bearer token123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderValue(String);

impl HeaderValue {
    /// Create a new `HeaderValue` with validation
    ///
    /// Supports any valid ASCII string (printable characters + whitespace).
    ///
    /// # Errors
    ///
    /// Returns `IntentError::Validation` if:
    /// - Value is empty
    /// - Value contains non-ASCII characters
    /// - Value contains control characters (except tab, space, CR, LF)
    ///
    /// # Examples
    ///
    /// ```
    /// use intent_core::types::HeaderValue;
    ///
    /// let value = HeaderValue::try_new("application/json").unwrap();
    /// assert_eq!(value.as_str(), "application/json");
    ///
    /// let invalid = HeaderValue::try_new("invalid\x00value");
    /// assert!(invalid.is_err());
    /// ```
    pub fn try_new(value: impl Into<String>) -> Result<Self, IntentError> {
        let value = value.into();

        if value.is_empty() {
            return Err(IntentError::validation(
                "header_value",
                "Header value cannot be empty",
            ));
        }

        // Validate ASCII: printable characters + common whitespace (space, tab, CR, LF)
        if !value.chars().all(|c| {
            c.is_ascii()
                && (c.is_ascii_graphic() || c == ' ' || c == '\t' || c == '\r' || c == '\n')
        }) {
            return Err(IntentError::validation(
                "header_value",
                format!("Header value contains invalid characters: '{value}'. Only ASCII printable characters and whitespace allowed"),
            ));
        }

        Ok(Self(value))
    }

    /// Get the header value as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for HeaderValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// HTTP status code
///
/// Validated HTTP status code (100-599).
/// Common status codes can be constructed with helper methods.
///
/// # Examples
///
/// ```
/// use intent_core::types::StatusCode;
///
/// let status = StatusCode::try_new(200).unwrap();
/// assert_eq!(status.as_u16(), 200);
/// assert!(status.is_success());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatusCode(u16);

impl StatusCode {
    /// Create a new `StatusCode` with validation
    ///
    /// # Errors
    ///
    /// Returns `IntentError::Validation` if the status code is not in the range 100-599
    pub fn try_new(code: u16) -> Result<Self, IntentError> {
        if !(100..=599).contains(&code) {
            return Err(IntentError::validation(
                "status_code",
                format!("Invalid HTTP status code: {code}. Must be in range 100-599"),
            ));
        }
        Ok(Self(code))
    }

    /// Get the status code as a u16
    #[must_use]
    pub const fn as_u16(&self) -> u16 {
        self.0
    }

    /// Check if this is a success status (2xx)
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self.0, 200..=299)
    }

    /// Check if this is a redirect status (3xx)
    #[must_use]
    pub const fn is_redirect(&self) -> bool {
        matches!(self.0, 300..=399)
    }

    /// Check if this is a client error status (4xx)
    #[must_use]
    pub const fn is_client_error(&self) -> bool {
        matches!(self.0, 400..=499)
    }

    /// Check if this is a server error status (5xx)
    #[must_use]
    pub const fn is_server_error(&self) -> bool {
        matches!(self.0, 500..=599)
    }

    /// Check if this is an informational status (1xx)
    #[must_use]
    pub const fn is_informational(&self) -> bool {
        matches!(self.0, 100..=199)
    }

    // Common status codes
    #[must_use]
    pub const fn ok() -> Self {
        Self(200)
    }

    #[must_use]
    pub const fn created() -> Self {
        Self(201)
    }

    #[must_use]
    pub const fn no_content() -> Self {
        Self(204)
    }

    #[must_use]
    pub const fn bad_request() -> Self {
        Self(400)
    }

    #[must_use]
    pub const fn unauthorized() -> Self {
        Self(401)
    }

    #[must_use]
    pub const fn forbidden() -> Self {
        Self(403)
    }

    #[must_use]
    pub const fn not_found() -> Self {
        Self(404)
    }

    #[must_use]
    pub const fn internal_server_error() -> Self {
        Self(500)
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// Time Types
// =============================================================================

/// Duration wrapper with millisecond precision
///
/// Wraps `std::time::Duration` for timeout and timing operations.
/// Provides convenient constructors for common durations.
///
/// # Examples
///
/// ```
/// use intent_core::types::IntentDuration;
///
/// let timeout = IntentDuration::from_secs(30);
/// assert_eq!(timeout.as_millis(), 30_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IntentDuration(std::time::Duration);

impl IntentDuration {
    /// Create a duration from seconds
    #[must_use]
    pub const fn from_secs(secs: u64) -> Self {
        Self(std::time::Duration::from_secs(secs))
    }

    /// Create a duration from milliseconds
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(std::time::Duration::from_millis(millis))
    }

    /// Create a duration from microseconds
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self(std::time::Duration::from_micros(micros))
    }

    /// Get duration in seconds
    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }

    /// Get duration in milliseconds
    #[must_use]
    pub const fn as_millis(&self) -> u128 {
        self.0.as_millis()
    }

    /// Get duration in microseconds
    #[must_use]
    pub const fn as_micros(&self) -> u128 {
        self.0.as_micros()
    }

    /// Get the inner `std::time::Duration`
    #[must_use]
    pub const fn inner(&self) -> std::time::Duration {
        self.0
    }

    /// Check if duration is zero
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0.as_nanos() == 0
    }
}

impl fmt::Display for IntentDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let millis = self.as_millis();
        if millis < 1000 {
            write!(f, "{}ms", millis)
        } else {
            write!(f, "{}s", self.as_secs())
        }
    }
}

impl From<std::time::Duration> for IntentDuration {
    fn from(duration: std::time::Duration) -> Self {
        Self(duration)
    }
}

impl From<IntentDuration> for std::time::Duration {
    fn from(duration: IntentDuration) -> Self {
        duration.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_module_compiles() {
        // Smoke test - module exists and compiles
    }

    // =========================================================================
    // HttpMethod Tests
    // =========================================================================

    #[test]
    fn http_method_get_display() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
    }

    #[test]
    fn http_method_post_display() {
        assert_eq!(HttpMethod::Post.to_string(), "POST");
    }

    #[test]
    fn http_method_all_variants_display() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
        assert_eq!(HttpMethod::Put.to_string(), "PUT");
        assert_eq!(HttpMethod::Patch.to_string(), "PATCH");
        assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
        assert_eq!(HttpMethod::Head.to_string(), "HEAD");
        assert_eq!(HttpMethod::Options.to_string(), "OPTIONS");
    }

    #[test]
    fn http_method_from_str_valid() {
        assert_eq!("GET".parse::<HttpMethod>().ok(), Some(HttpMethod::Get));
        assert_eq!("POST".parse::<HttpMethod>().ok(), Some(HttpMethod::Post));
        assert_eq!("PUT".parse::<HttpMethod>().ok(), Some(HttpMethod::Put));
        assert_eq!("PATCH".parse::<HttpMethod>().ok(), Some(HttpMethod::Patch));
        assert_eq!(
            "DELETE".parse::<HttpMethod>().ok(),
            Some(HttpMethod::Delete)
        );
        assert_eq!("HEAD".parse::<HttpMethod>().ok(), Some(HttpMethod::Head));
        assert_eq!(
            "OPTIONS".parse::<HttpMethod>().ok(),
            Some(HttpMethod::Options)
        );
    }

    #[test]
    fn http_method_from_str_case_insensitive() {
        assert_eq!("get".parse::<HttpMethod>().ok(), Some(HttpMethod::Get));
        assert_eq!("Post".parse::<HttpMethod>().ok(), Some(HttpMethod::Post));
        assert_eq!("pUt".parse::<HttpMethod>().ok(), Some(HttpMethod::Put));
    }

    #[test]
    fn http_method_from_str_invalid() {
        let result = "INVALID".parse::<HttpMethod>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid HTTP method"));
        }
    }

    #[test]
    fn http_method_equality() {
        assert_eq!(HttpMethod::Get, HttpMethod::Get);
        assert_ne!(HttpMethod::Get, HttpMethod::Post);
    }

    // =========================================================================
    // SpecName Tests
    // =========================================================================

    #[test]
    fn spec_name_valid() {
        let result = SpecName::try_new("test.cue");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "test.cue");
        }
    }

    #[test]
    fn spec_name_with_hyphens_underscores() {
        let result = SpecName::try_new("user-api_v2.cue");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "user-api_v2.cue");
        }
    }

    #[test]
    fn spec_name_with_path() {
        let result = SpecName::try_new("specs/user-api.cue");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "specs/user-api.cue");
        }
    }

    #[test]
    fn spec_name_empty_fails() {
        let result = SpecName::try_new("");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn spec_name_missing_extension_fails() {
        let result = SpecName::try_new("test");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("must end with .cue"));
        }
    }

    #[test]
    fn spec_name_invalid_chars_fails() {
        let result = SpecName::try_new("test@spec.cue");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn spec_name_base_name() {
        let result = SpecName::try_new("user-api.cue");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.base_name(), "user-api");
        }
    }

    #[test]
    fn spec_name_display() {
        let result = SpecName::try_new("test.cue");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.to_string(), "test.cue");
        }
    }

    #[test]
    fn spec_name_as_path() {
        let result = SpecName::try_new("specs/test.cue");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_path(), Path::new("specs/test.cue"));
        }
    }

    // =========================================================================
    // Url Tests
    // =========================================================================

    #[test]
    fn url_valid_https() {
        let result = Url::try_new("https://example.com");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.scheme(), "https");
            assert_eq!(url.host(), Some("example.com"));
            assert_eq!(url.path(), "/");
        }
    }

    #[test]
    fn url_valid_http() {
        let result = Url::try_new("http://example.com");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.scheme(), "http");
            assert_eq!(url.host(), Some("example.com"));
        }
    }

    #[test]
    fn url_with_path() {
        let result = Url::try_new("https://api.example.com/v1/users");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.scheme(), "https");
            assert_eq!(url.host(), Some("api.example.com"));
            assert_eq!(url.path(), "/v1/users");
        }
    }

    #[test]
    fn url_with_port() {
        let result = Url::try_new("https://example.com:8080/path");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.host(), Some("example.com"));
            assert_eq!(url.path(), "/path");
        }
    }

    #[test]
    fn url_with_query() {
        let result = Url::try_new("https://example.com/search?q=rust&limit=10");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.path(), "/search");
            assert!(url.as_str().contains("q=rust"));
        }
    }

    #[test]
    fn url_with_fragment() {
        let result = Url::try_new("https://example.com/docs#section");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.path(), "/docs");
            assert!(url.as_str().contains("#section"));
        }
    }

    #[test]
    fn url_localhost() {
        let result = Url::try_new("http://localhost:3000/api");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.scheme(), "http");
            assert_eq!(url.host(), Some("localhost"));
            assert_eq!(url.path(), "/api");
        }
    }

    #[test]
    fn url_ip_address() {
        let result = Url::try_new("http://127.0.0.1:8080/health");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.host(), Some("127.0.0.1"));
            assert_eq!(url.path(), "/health");
        }
    }

    #[test]
    fn url_invalid_empty() {
        let result = Url::try_new("");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid URL"));
        }
    }

    #[test]
    fn url_invalid_no_scheme() {
        let result = Url::try_new("example.com");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid URL"));
        }
    }

    #[test]
    fn url_invalid_malformed() {
        let result = Url::try_new("not a url at all");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid URL"));
        }
    }

    #[test]
    fn url_invalid_scheme() {
        let result = Url::try_new("ht!tp://example.com");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid URL"));
        }
    }

    #[test]
    fn url_display() {
        let result = Url::try_new("https://example.com/path");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.to_string(), "https://example.com/path");
        }
    }

    #[test]
    fn url_as_str() {
        let result = Url::try_new("https://example.com/api");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.as_str(), "https://example.com/api");
        }
    }

    #[test]
    fn url_equality() {
        let result1 = Url::try_new("https://example.com/path");
        let result2 = Url::try_new("https://example.com/path");
        let result3 = Url::try_new("https://different.com/path");
        assert!(result1.is_ok() && result2.is_ok() && result3.is_ok());
        if let (Ok(url1), Ok(url2), Ok(url3)) = (result1, result2, result3) {
            assert_eq!(url1, url2);
            assert_ne!(url1, url3);
        }
    }

    #[test]
    fn url_clone() {
        let result = Url::try_new("https://example.com");
        assert!(result.is_ok());
        if let Ok(url1) = result {
            let url2 = url1.clone();
            assert_eq!(url1, url2);
        }
    }

    #[test]
    fn url_scheme_various() {
        let secure_http = Url::try_new("https://example.com");
        let plain_http = Url::try_new("http://example.com");
        let websocket = Url::try_new("ws://example.com");
        let secure_websocket = Url::try_new("wss://example.com");

        assert!(secure_http.is_ok());
        assert!(plain_http.is_ok());
        assert!(websocket.is_ok());
        assert!(secure_websocket.is_ok());

        if let (Ok(https), Ok(http), Ok(ws), Ok(wss)) =
            (secure_http, plain_http, websocket, secure_websocket)
        {
            assert_eq!(https.scheme(), "https");
            assert_eq!(http.scheme(), "http");
            assert_eq!(ws.scheme(), "ws");
            assert_eq!(wss.scheme(), "wss");
        }
    }

    #[test]
    fn url_host_none_for_file() {
        // file:// URLs may not have a host
        let result = Url::try_new("file:///path/to/file");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.scheme(), "file");
            // host_str() returns None for file URLs
            assert_eq!(url.host(), None);
        }
    }

    #[test]
    fn url_path_root() {
        let result = Url::try_new("https://example.com");
        assert!(result.is_ok());
        if let Ok(url) = result {
            // url crate normalizes to include trailing slash for root
            assert_eq!(url.path(), "/");
        }
    }

    #[test]
    fn url_path_nested() {
        let result = Url::try_new("https://example.com/api/v1/users/123");
        assert!(result.is_ok());
        if let Ok(url) = result {
            assert_eq!(url.path(), "/api/v1/users/123");
        }
    }

    // =========================================================================
    // HeaderName Tests
    // =========================================================================

    #[test]
    fn header_name_valid_simple() {
        let result = HeaderName::try_new("Content-Type");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "content-type");
        }
    }

    #[test]
    fn header_name_valid_lowercase() {
        let result = HeaderName::try_new("content-type");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "content-type");
        }
    }

    #[test]
    fn header_name_valid_uppercase() {
        let result = HeaderName::try_new("CONTENT-TYPE");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "content-type");
        }
    }

    #[test]
    fn header_name_valid_with_hyphens() {
        let result = HeaderName::try_new("X-Custom-Header");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "x-custom-header");
        }
    }

    #[test]
    fn header_name_valid_alphanumeric() {
        let result = HeaderName::try_new("X-API-Key123");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.as_str(), "x-api-key123");
        }
    }

    #[test]
    fn header_name_case_insensitive_equality() {
        let result1 = HeaderName::try_new("Content-Type");
        let result2 = HeaderName::try_new("content-type");
        let result3 = HeaderName::try_new("CONTENT-TYPE");
        assert!(result1.is_ok() && result2.is_ok() && result3.is_ok());
        if let (Ok(name1), Ok(name2), Ok(name3)) = (result1, result2, result3) {
            assert_eq!(name1, name2);
            assert_eq!(name2, name3);
            assert_eq!(name1, name3);
        }
    }

    #[test]
    fn header_name_empty_fails() {
        let result = HeaderName::try_new("");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn header_name_with_spaces_fails() {
        let result = HeaderName::try_new("Content Type");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_name_with_underscore_fails() {
        let result = HeaderName::try_new("Content_Type");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_name_with_special_chars_fails() {
        let result = HeaderName::try_new("Content@Type");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_name_with_colon_fails() {
        let result = HeaderName::try_new("Content:Type");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_name_with_slash_fails() {
        let result = HeaderName::try_new("Content/Type");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_name_display() {
        let result = HeaderName::try_new("Content-Type");
        assert!(result.is_ok());
        if let Ok(name) = result {
            assert_eq!(name.to_string(), "content-type");
        }
    }

    #[test]
    fn header_name_as_ref() {
        let result = HeaderName::try_new("Accept");
        assert!(result.is_ok());
        if let Ok(name) = result {
            let s: &str = name.as_ref();
            assert_eq!(s, "accept");
        }
    }

    #[test]
    fn header_name_clone() {
        let result = HeaderName::try_new("Authorization");
        assert!(result.is_ok());
        if let Ok(name1) = result {
            let name2 = name1.clone();
            assert_eq!(name1, name2);
        }
    }

    #[test]
    fn header_name_common_headers() {
        let headers = vec![
            "Content-Type",
            "Authorization",
            "Accept",
            "User-Agent",
            "Accept-Encoding",
            "Cache-Control",
            "X-Forwarded-For",
            "X-Request-ID",
        ];

        for header in headers {
            let result = HeaderName::try_new(header);
            assert!(result.is_ok(), "Failed to create header: {header}");
        }
    }

    // =========================================================================
    // HeaderValue Tests
    // =========================================================================

    #[test]
    fn header_value_valid_simple() {
        let result = HeaderValue::try_new("application/json");
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.as_str(), "application/json");
        }
    }

    #[test]
    fn header_value_valid_with_spaces() {
        let result = HeaderValue::try_new("Bearer token123");
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.as_str(), "Bearer token123");
        }
    }

    #[test]
    fn header_value_valid_with_numbers() {
        let result = HeaderValue::try_new("12345");
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.as_str(), "12345");
        }
    }

    #[test]
    fn header_value_valid_with_special_chars() {
        let result = HeaderValue::try_new("text/html; charset=utf-8");
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.as_str(), "text/html; charset=utf-8");
        }
    }

    #[test]
    fn header_value_valid_with_tabs() {
        let result = HeaderValue::try_new("value\twith\ttabs");
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.as_str(), "value\twith\ttabs");
        }
    }

    #[test]
    fn header_value_valid_with_crlf() {
        let result = HeaderValue::try_new("line1\r\nline2");
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.as_str(), "line1\r\nline2");
        }
    }

    #[test]
    fn header_value_empty_fails() {
        let result = HeaderValue::try_new("");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn header_value_with_null_fails() {
        let result = HeaderValue::try_new("value\x00with\x00null");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_value_with_control_chars_fails() {
        let result = HeaderValue::try_new("value\x01control");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_value_with_non_ascii_fails() {
        let result = HeaderValue::try_new("value with emoji 🚀");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("invalid characters"));
        }
    }

    #[test]
    fn header_value_display() {
        let result = HeaderValue::try_new("application/json");
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.to_string(), "application/json");
        }
    }

    #[test]
    fn header_value_as_ref() {
        let result = HeaderValue::try_new("text/plain");
        assert!(result.is_ok());
        if let Ok(value) = result {
            let s: &str = value.as_ref();
            assert_eq!(s, "text/plain");
        }
    }

    #[test]
    fn header_value_clone() {
        let result = HeaderValue::try_new("application/xml");
        assert!(result.is_ok());
        if let Ok(value1) = result {
            let value2 = value1.clone();
            assert_eq!(value1, value2);
        }
    }

    #[test]
    fn header_value_equality() {
        let result1 = HeaderValue::try_new("application/json");
        let result2 = HeaderValue::try_new("application/json");
        let result3 = HeaderValue::try_new("text/plain");
        assert!(result1.is_ok() && result2.is_ok() && result3.is_ok());
        if let (Ok(value1), Ok(value2), Ok(value3)) = (result1, result2, result3) {
            assert_eq!(value1, value2);
            assert_ne!(value1, value3);
        }
    }

    #[test]
    fn header_value_case_sensitive() {
        let result1 = HeaderValue::try_new("Application/JSON");
        let result2 = HeaderValue::try_new("application/json");
        assert!(result1.is_ok() && result2.is_ok());
        if let (Ok(value1), Ok(value2)) = (result1, result2) {
            // HeaderValue is case-sensitive (unlike HeaderName)
            assert_ne!(value1, value2);
        }
    }

    #[test]
    fn header_value_common_values() {
        let values = vec![
            "application/json",
            "text/html",
            "text/plain",
            "application/xml",
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            "gzip, deflate, br",
            "max-age=3600",
            "*/*",
            "Mozilla/5.0 (compatible; MSIE 10.0)",
        ];

        for value in values {
            let result = HeaderValue::try_new(value);
            assert!(result.is_ok(), "Failed to create value: {value}");
        }
    }

    // =========================================================================
    // StatusCode Tests
    // =========================================================================

    #[test]
    fn status_code_valid() {
        let result = StatusCode::try_new(200);
        assert!(result.is_ok());
        if let Ok(status) = result {
            assert_eq!(status.as_u16(), 200);
        }
    }

    #[test]
    fn status_code_range_valid() {
        for code in [100, 200, 300, 400, 500, 599] {
            let result = StatusCode::try_new(code);
            assert!(result.is_ok(), "Failed for code: {code}");
        }
    }

    #[test]
    fn status_code_below_100_fails() {
        let result = StatusCode::try_new(99);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid HTTP status code"));
        }
    }

    #[test]
    fn status_code_above_599_fails() {
        let result = StatusCode::try_new(600);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid HTTP status code"));
        }
    }

    #[test]
    fn status_code_is_success() {
        assert!(StatusCode::try_new(200).unwrap().is_success());
        assert!(StatusCode::try_new(299).unwrap().is_success());
        assert!(!StatusCode::try_new(199).unwrap().is_success());
        assert!(!StatusCode::try_new(300).unwrap().is_success());
    }

    #[test]
    fn status_code_is_redirect() {
        assert!(StatusCode::try_new(300).unwrap().is_redirect());
        assert!(StatusCode::try_new(399).unwrap().is_redirect());
        assert!(!StatusCode::try_new(299).unwrap().is_redirect());
        assert!(!StatusCode::try_new(400).unwrap().is_redirect());
    }

    #[test]
    fn status_code_is_client_error() {
        assert!(StatusCode::try_new(400).unwrap().is_client_error());
        assert!(StatusCode::try_new(499).unwrap().is_client_error());
        assert!(!StatusCode::try_new(399).unwrap().is_client_error());
        assert!(!StatusCode::try_new(500).unwrap().is_client_error());
    }

    #[test]
    fn status_code_is_server_error() {
        assert!(StatusCode::try_new(500).unwrap().is_server_error());
        assert!(StatusCode::try_new(599).unwrap().is_server_error());
        assert!(!StatusCode::try_new(499).unwrap().is_server_error());
        assert!(!StatusCode::try_new(600).unwrap_or(StatusCode::ok()).is_server_error());
    }

    #[test]
    fn status_code_is_informational() {
        assert!(StatusCode::try_new(100).unwrap().is_informational());
        assert!(StatusCode::try_new(199).unwrap().is_informational());
        assert!(!StatusCode::try_new(200).unwrap().is_informational());
    }

    #[test]
    fn status_code_common_constructors() {
        assert_eq!(StatusCode::ok().as_u16(), 200);
        assert_eq!(StatusCode::created().as_u16(), 201);
        assert_eq!(StatusCode::no_content().as_u16(), 204);
        assert_eq!(StatusCode::bad_request().as_u16(), 400);
        assert_eq!(StatusCode::unauthorized().as_u16(), 401);
        assert_eq!(StatusCode::forbidden().as_u16(), 403);
        assert_eq!(StatusCode::not_found().as_u16(), 404);
        assert_eq!(StatusCode::internal_server_error().as_u16(), 500);
    }

    #[test]
    fn status_code_display() {
        let status = StatusCode::try_new(404).unwrap();
        assert_eq!(status.to_string(), "404");
    }

    #[test]
    fn status_code_equality() {
        let status1 = StatusCode::try_new(200).unwrap();
        let status2 = StatusCode::try_new(200).unwrap();
        let status3 = StatusCode::try_new(404).unwrap();
        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }

    #[test]
    fn status_code_ordering() {
        let status1 = StatusCode::try_new(200).unwrap();
        let status2 = StatusCode::try_new(404).unwrap();
        assert!(status1 < status2);
        assert!(status2 > status1);
    }

    // =========================================================================
    // IntentDuration Tests
    // =========================================================================

    #[test]
    fn duration_from_secs() {
        let duration = IntentDuration::from_secs(5);
        assert_eq!(duration.as_secs(), 5);
        assert_eq!(duration.as_millis(), 5_000);
    }

    #[test]
    fn duration_from_millis() {
        let duration = IntentDuration::from_millis(1_500);
        assert_eq!(duration.as_millis(), 1_500);
        assert_eq!(duration.as_secs(), 1);
    }

    #[test]
    fn duration_from_micros() {
        let duration = IntentDuration::from_micros(1_000_000);
        assert_eq!(duration.as_micros(), 1_000_000);
        assert_eq!(duration.as_secs(), 1);
    }

    #[test]
    fn duration_as_conversions() {
        let duration = IntentDuration::from_secs(3);
        assert_eq!(duration.as_secs(), 3);
        assert_eq!(duration.as_millis(), 3_000);
        assert_eq!(duration.as_micros(), 3_000_000);
    }

    #[test]
    fn duration_is_zero() {
        let zero = IntentDuration::from_secs(0);
        let non_zero = IntentDuration::from_millis(1);
        assert!(zero.is_zero());
        assert!(!non_zero.is_zero());
    }

    #[test]
    fn duration_display_millis() {
        let duration = IntentDuration::from_millis(500);
        assert_eq!(duration.to_string(), "500ms");
    }

    #[test]
    fn duration_display_secs() {
        let duration = IntentDuration::from_secs(5);
        assert_eq!(duration.to_string(), "5s");
    }

    #[test]
    fn duration_from_std_duration() {
        let std_dur = std::time::Duration::from_secs(10);
        let intent_dur = IntentDuration::from(std_dur);
        assert_eq!(intent_dur.as_secs(), 10);
    }

    #[test]
    fn duration_to_std_duration() {
        let intent_dur = IntentDuration::from_secs(10);
        let std_dur: std::time::Duration = intent_dur.into();
        assert_eq!(std_dur.as_secs(), 10);
    }

    #[test]
    fn duration_equality() {
        let dur1 = IntentDuration::from_millis(1_000);
        let dur2 = IntentDuration::from_secs(1);
        let dur3 = IntentDuration::from_secs(2);
        assert_eq!(dur1, dur2);
        assert_ne!(dur1, dur3);
    }

    #[test]
    fn duration_ordering() {
        let dur1 = IntentDuration::from_secs(1);
        let dur2 = IntentDuration::from_secs(2);
        assert!(dur1 < dur2);
        assert!(dur2 > dur1);
    }

    #[test]
    fn duration_clone() {
        let dur1 = IntentDuration::from_secs(5);
        let dur2 = dur1;
        assert_eq!(dur1, dur2);
    }

    #[test]
    fn duration_inner() {
        let duration = IntentDuration::from_secs(3);
        let inner = duration.inner();
        assert_eq!(inner.as_secs(), 3);
    }
}
