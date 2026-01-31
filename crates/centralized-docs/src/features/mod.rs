//! Spider-rs feature extensions with type-safe configuration.
//!
//! All feature configuration uses newtype patterns for compile-time safety
//! and zero-cost abstraction when features are disabled.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
// Public APIs that are not yet used in the codebase - allow dead code
#![allow(dead_code)]
// Some functions return Result for API consistency even though they can't fail
#![allow(clippy::unnecessary_wraps)]
// Copy types passed by ref for API consistency
#![allow(clippy::trivially_copy_pass_by_ref)]

use std::time::Duration;
use thiserror::Error;

// ===========================================================================
// ERROR TYPES
// ===========================================================================

/// All feature-related errors
#[derive(Debug, Error, Clone, PartialEq)]
pub enum FeatureError {
    #[allow(dead_code)]
    #[error("invalid cache TTL: must be positive, got {0}s")]
    InvalidCacheTtl(u64),

    #[allow(dead_code)]
    #[error("invalid regex pattern: {pattern}")]
    InvalidRegex { pattern: String },

    #[allow(dead_code)]
    #[error("invalid glob pattern: {pattern}")]
    InvalidGlob { pattern: String },

    #[allow(dead_code)]
    #[error("JavaScript timeout must be at least 1ms, got {0}ms")]
    InvalidJsTimeout(u64),

    #[cfg(feature = "javascript")]
    #[allow(dead_code)]
    #[error("Chrome initialization failed: {0}")]
    ChromeInit(String),

    #[cfg(feature = "anti-detection")]
    #[allow(dead_code)]
    #[error("user agent generation failed")]
    UserAgentGeneration,
}

// ===========================================================================
// CACHE CONFIGURATION
// ===========================================================================

/// Positive duration in seconds (validated at construction)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheTtl(u64);

impl CacheTtl {
    /// Creates a new cache TTL, ensuring it's positive
    ///
    /// # Errors
    ///
    /// Returns `FeatureError::InvalidCacheTtl` if `seconds` is zero.
    #[allow(dead_code)]
    pub fn new(seconds: u64) -> Result<Self, FeatureError> {
        if seconds > 0 {
            Ok(Self(seconds))
        } else {
            Err(FeatureError::InvalidCacheTtl(seconds))
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn as_duration(&self) -> Duration {
        Duration::from_secs(self.0)
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn seconds(&self) -> u64 {
        self.0
    }
}

impl Default for CacheTtl {
    fn default() -> Self {
        // Default 5 minutes
        Self(300)
    }
}

/// Cache configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl: CacheTtl,
}

impl CacheConfig {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ttl: CacheTtl::default(),
        }
    }

    #[must_use]
    pub fn enabled_with_ttl(ttl: CacheTtl) -> Self {
        Self { enabled: true, ttl }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

// ===========================================================================
// FILTERING CONFIGURATION
// ===========================================================================

/// Validated regex pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexPattern(String);

impl RegexPattern {
    /// Creates a new validated regex pattern
    ///
    /// # Errors
    ///
    /// Returns `FeatureError::InvalidRegex` if the pattern is not a valid regex.
    #[allow(dead_code)]
    pub fn new(pattern: String) -> Result<Self, FeatureError> {
        regex::Regex::new(&pattern)
            .map(|_| Self(pattern.clone()))
            .map_err(|_| FeatureError::InvalidRegex { pattern })
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated glob pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobPattern(String);

impl GlobPattern {
    /// Creates a new validated glob pattern
    ///
    /// # Errors
    ///
    /// Returns `FeatureError::InvalidGlob` if the pattern is empty.
    #[allow(dead_code)]
    pub fn new(pattern: String) -> Result<Self, FeatureError> {
        // Basic validation - non-empty
        if pattern.is_empty() {
            return Err(FeatureError::InvalidGlob { pattern });
        }
        Ok(Self(pattern))
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// URL filtering configuration
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilteringConfig {
    pub allow: Vec<GlobPattern>,
    pub deny: Vec<RegexPattern>,
}

impl FilteringConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_allow(mut self, patterns: Vec<GlobPattern>) -> Self {
        self.allow = patterns;
        self
    }

    #[must_use]
    pub fn with_deny(mut self, patterns: Vec<RegexPattern>) -> Self {
        self.deny = patterns;
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.allow.is_empty() && self.deny.is_empty()
    }
}

// ===========================================================================
// JAVASCRIPT RENDERING CONFIGURATION
// ===========================================================================

#[cfg(feature = "javascript")]
pub mod javascript {
    use super::{Duration, FeatureError};

    /// Positive millisecond duration
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Milliseconds(u64);

    impl Milliseconds {
        /// Creates a new positive millisecond duration
        ///
        /// # Errors
        ///
        /// Returns `FeatureError::InvalidJsTimeout` if `ms` is zero.
        #[allow(dead_code)]
        pub fn new(ms: u64) -> Result<Self, FeatureError> {
            if ms > 0 {
                Ok(Self(ms))
            } else {
                Err(FeatureError::InvalidJsTimeout(ms))
            }
        }

        #[must_use]
        #[allow(dead_code)]
        pub fn as_duration(&self) -> Duration {
            Duration::from_millis(self.0)
        }

        #[must_use]
        #[allow(dead_code)]
        pub fn millis(&self) -> u64 {
            self.0
        }
    }

    /// `JavaScript` rendering mode
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum RenderMode {
        /// Auto-detect if `JS` rendering is needed
        #[default]
        Smart,
        /// Always use `Chrome` rendering
        Always,
        /// Never use `Chrome` (`HTTP` only)
        Never,
    }

    /// `JavaScript` rendering configuration
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct JavascriptConfig {
        pub mode: RenderMode,
        pub timeout: Milliseconds,
    }

    impl JavascriptConfig {
        /// Creates a new `JavaScript` rendering configuration
        ///
        /// # Errors
        ///
        /// Never fails - returns Ok for all inputs.
        #[allow(dead_code)]
        pub fn new(mode: RenderMode, timeout: Milliseconds) -> Result<Self, FeatureError> {
            Ok(Self { mode, timeout })
        }

        /// Creates a configuration with smart rendering mode
        ///
        /// # Errors
        ///
        /// Returns `FeatureError::InvalidJsTimeout` if the default timeout validation fails.
        #[allow(dead_code)]
        pub fn smart() -> Result<Self, FeatureError> {
            Ok(Self {
                mode: RenderMode::Smart,
                timeout: Milliseconds::new(30000)?, // 30s default
            })
        }

        /// Creates a configuration with never rendering mode
        ///
        /// # Errors
        ///
        /// Returns `FeatureError::InvalidJsTimeout` if the default timeout validation fails.
        #[allow(dead_code)]
        pub fn never() -> Result<Self, FeatureError> {
            Ok(Self {
                mode: RenderMode::Never,
                timeout: Milliseconds::new(1000)?, // Minimal timeout
            })
        }

        #[must_use]
        #[allow(dead_code)]
        pub fn with_timeout(mut self, timeout: Milliseconds) -> Self {
            self.timeout = timeout;
            self
        }
    }

    impl Default for JavascriptConfig {
        fn default() -> Self {
            Self {
                mode: RenderMode::Smart,
                timeout: Milliseconds(30000),
            }
        }
    }
}

#[cfg(feature = "javascript")]
pub use javascript::{JavascriptConfig, Milliseconds};

#[cfg(feature = "javascript")]
#[allow(unused_imports)]
pub use javascript::RenderMode;

// ===========================================================================
// ANTI-DETECTION CONFIGURATION
// ===========================================================================

#[cfg(feature = "anti-detection")]
pub mod anti_detection {

    /// Anti-detection strategy
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum Strategy {
        /// No anti-detection
        #[default]
        None,
        /// Rotate User-Agent header
        RotatingUserAgent,
        /// Full stealth mode (spoof headers, random `UA`, etc.)
        FullStealth,
    }

    /// Anti-detection configuration
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AntiDetectionConfig {
        pub strategy: Strategy,
    }

    impl AntiDetectionConfig {
        #[must_use]
        #[allow(dead_code)]
        pub fn new(strategy: Strategy) -> Self {
            Self { strategy }
        }

        #[must_use]
        #[allow(dead_code)]
        pub fn none() -> Self {
            Self {
                strategy: Strategy::None,
            }
        }

        #[must_use]
        #[allow(dead_code)]
        pub fn rotating_ua() -> Self {
            Self {
                strategy: Strategy::RotatingUserAgent,
            }
        }

        #[must_use]
        #[allow(dead_code)]
        pub fn full_stealth() -> Self {
            Self {
                strategy: Strategy::FullStealth,
            }
        }
    }

    impl Default for AntiDetectionConfig {
        fn default() -> Self {
            Self::none()
        }
    }
}

#[cfg(feature = "anti-detection")]
pub use anti_detection::AntiDetectionConfig;

#[cfg(feature = "anti-detection")]
#[allow(unused_imports)]
pub use anti_detection::Strategy;

// ===========================================================================
// COMPOSITE FEATURE CONFIG
// ===========================================================================

/// Master feature configuration
///
/// All fields are optional to enable zero-cost when features disabled.
/// Use the builder pattern for construction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FeatureConfig {
    #[cfg(feature = "enhanced")]
    pub cache: Option<CacheConfig>,

    #[cfg(feature = "enhanced")]
    pub filtering: Option<FilteringConfig>,

    #[cfg(feature = "javascript")]
    pub javascript: Option<JavascriptConfig>,

    #[cfg(feature = "anti-detection")]
    pub anti_detection: Option<AntiDetectionConfig>,
}

impl FeatureConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "enhanced")]
    #[must_use]
    pub fn with_cache(mut self, config: CacheConfig) -> Self {
        self.cache = Some(config);
        self
    }

    #[cfg(feature = "enhanced")]
    #[must_use]
    pub fn with_filtering(mut self, config: FilteringConfig) -> Self {
        self.filtering = Some(config);
        self
    }

    #[cfg(feature = "javascript")]
    #[must_use]
    pub fn with_javascript(mut self, config: JavascriptConfig) -> Self {
        self.javascript = Some(config);
        self
    }

    #[cfg(feature = "anti-detection")]
    #[must_use]
    pub fn with_anti_detection(mut self, config: AntiDetectionConfig) -> Self {
        self.anti_detection = Some(config);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        #[allow(unused_mut)]
        let mut empty = true;

        #[cfg(feature = "enhanced")]
        {
            empty = empty && self.cache.is_none() && self.filtering.is_none();
        }

        #[cfg(feature = "javascript")]
        {
            empty = empty && self.javascript.is_none();
        }

        #[cfg(feature = "anti-detection")]
        {
            empty = empty && self.anti_detection.is_none();
        }

        empty
    }
}

// ===========================================================================
// BUILDER FOR CONVENIENCE
// ===========================================================================

/// Builder for `FeatureConfig`
#[derive(Debug, Clone, Default)]
pub struct FeatureConfigBuilder {
    #[cfg(feature = "enhanced")]
    cache: Option<CacheConfig>,
    #[cfg(feature = "enhanced")]
    filtering: Option<FilteringConfig>,
    #[cfg(feature = "javascript")]
    javascript: Option<JavascriptConfig>,
    #[cfg(feature = "anti-detection")]
    anti_detection: Option<AntiDetectionConfig>,
}

impl FeatureConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "enhanced")]
    /// Enables cache with specified TTL
    ///
    /// # Errors
    ///
    /// Returns `FeatureError::InvalidCacheTtl` if `ttl_seconds` is zero.
    #[allow(dead_code)]
    pub fn enable_cache(mut self, ttl_seconds: u64) -> Result<Self, FeatureError> {
        self.cache = Some(CacheConfig::enabled_with_ttl(CacheTtl::new(ttl_seconds)?));
        Ok(self)
    }

    #[cfg(feature = "enhanced")]
    #[must_use]
    #[allow(dead_code)]
    pub fn disable_cache(mut self) -> Self {
        self.cache = Some(CacheConfig::disabled());
        self
    }

    #[cfg(feature = "enhanced")]
    /// Sets allowed URL patterns
    ///
    /// # Errors
    ///
    /// Returns `FeatureError::InvalidGlob` if any pattern is invalid.
    #[allow(dead_code)]
    pub fn allow_patterns(mut self, patterns: Vec<String>) -> Result<Self, FeatureError> {
        let validated = patterns
            .into_iter()
            .map(GlobPattern::new)
            .collect::<Result<Vec<_>, _>>()?;
        self.filtering = Some(FilteringConfig::new().with_allow(validated));
        Ok(self)
    }

    #[cfg(feature = "enhanced")]
    /// Sets denied URL patterns
    ///
    /// # Errors
    ///
    /// Returns `FeatureError::InvalidRegex` if any pattern is invalid.
    #[allow(dead_code)]
    pub fn deny_patterns(mut self, patterns: Vec<String>) -> Result<Self, FeatureError> {
        let validated = patterns
            .into_iter()
            .map(RegexPattern::new)
            .collect::<Result<Vec<_>, _>>()?;
        self.filtering = Some(FilteringConfig::new().with_deny(validated));
        Ok(self)
    }

    #[cfg(feature = "javascript")]
    /// Enables smart `JavaScript` rendering with timeout
    ///
    /// # Errors
    ///
    /// Returns `FeatureError::InvalidJsTimeout` if `timeout_ms` is zero.
    #[allow(dead_code)]
    pub fn smart_js(mut self, timeout_ms: u64) -> Result<Self, FeatureError> {
        self.javascript =
            Some(JavascriptConfig::smart()?.with_timeout(Milliseconds::new(timeout_ms)?));
        Ok(self)
    }

    #[cfg(feature = "anti-detection")]
    #[must_use]
    #[allow(dead_code)]
    pub fn stealth(mut self) -> Self {
        self.anti_detection = Some(AntiDetectionConfig::full_stealth());
        self
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn build(self) -> FeatureConfig {
        FeatureConfig {
            #[cfg(feature = "enhanced")]
            cache: self.cache,
            #[cfg(feature = "enhanced")]
            filtering: self.filtering,
            #[cfg(feature = "javascript")]
            javascript: self.javascript,
            #[cfg(feature = "anti-detection")]
            anti_detection: self.anti_detection,
        }
    }
}

// ===========================================================================
// TESTS
// ===========================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![expect(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_cache_ttl_rejects_zero() {
        assert!(matches!(
            CacheTtl::new(0),
            Err(FeatureError::InvalidCacheTtl(0))
        ));
    }

    #[test]
    fn test_cache_ttl_accepts_positive() {
        assert!(CacheTtl::new(60).is_ok());
    }

    #[test]
    fn test_regex_pattern_validation() {
        assert!(RegexPattern::new(r"\d+".to_string()).is_ok());
        assert!(RegexPattern::new(r"(".to_string()).is_err());
    }

    #[test]
    fn test_glob_pattern_validation() {
        assert!(GlobPattern::new("/docs/*".to_string()).is_ok());
        assert!(GlobPattern::new(String::new()).is_err());
    }

    #[cfg(feature = "javascript")]
    #[test]
    fn test_milliseconds_rejects_zero() {
        assert!(matches!(
            Milliseconds::new(0),
            Err(FeatureError::InvalidJsTimeout(0))
        ));
    }

    #[test]
    fn test_feature_config_is_empty() {
        let config = FeatureConfig::new();
        assert!(config.is_empty());
    }

    #[cfg(feature = "enhanced")]
    #[test]
    fn test_builder_cache() {
        let builder = match FeatureConfigBuilder::new().enable_cache(300) {
            Ok(b) => b,
            Err(e) => panic!("enable_cache failed: {e}"),
        };
        let config = builder.build();
        assert!(config.cache.is_some());
        assert!(config.cache.as_ref().is_some_and(|c| c.enabled));
    }

    #[cfg(feature = "enhanced")]
    #[test]
    fn test_builder_filtering() {
        let builder = match FeatureConfigBuilder::new().allow_patterns(vec!["/docs/*".to_string()])
        {
            Ok(b) => b,
            Err(e) => panic!("allow_patterns failed: {e}"),
        };
        let config = builder.build();
        assert!(config.filtering.is_some());
        assert!(!config.filtering.as_ref().is_none_or(|f| f.allow.is_empty()));
    }

    #[test]
    fn test_cache_config_default_is_disabled() {
        let config = CacheConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_cache_config_enabled_with_ttl() {
        let ttl = match CacheTtl::new(600) {
            Ok(t) => t,
            Err(e) => panic!("CacheTtl::new failed: {e}"),
        };
        let config = CacheConfig::enabled_with_ttl(ttl);
        assert!(config.enabled);
        assert_eq!(config.ttl.seconds(), 600);
    }

    #[test]
    fn test_filtering_config_default_is_empty() {
        let config = FilteringConfig::default();
        assert!(config.is_empty());
    }

    #[test]
    fn test_filtering_config_with_allow() {
        let pattern = GlobPattern::new("/docs/*".to_string());
        assert!(pattern.is_ok(), "GlobPattern::new should succeed");
        let patterns = vec![pattern.unwrap()];
        let config = FilteringConfig::new().with_allow(patterns.clone());
        assert!(!config.allow.is_empty());
        assert_eq!(config.allow.len(), 1);
    }

    #[test]
    fn test_filtering_config_with_deny() {
        let pattern = RegexPattern::new(r"\.pdf$".to_string());
        assert!(pattern.is_ok(), "RegexPattern::new should succeed");
        let patterns = vec![pattern.unwrap()];
        let config = FilteringConfig::new().with_deny(patterns.clone());
        assert!(!config.deny.is_empty());
        assert_eq!(config.deny.len(), 1);
    }

    #[cfg(feature = "javascript")]
    #[test]
    fn test_javascript_config_smart() {
        let config = match JavascriptConfig::smart() {
            Ok(c) => c,
            Err(e) => panic!("JavascriptConfig::smart failed: {e}"),
        };
        assert_eq!(config.mode, RenderMode::Smart);
        assert_eq!(config.timeout.millis(), 30000);
    }

    #[cfg(feature = "javascript")]
    #[test]
    fn test_javascript_config_never() {
        let config = match JavascriptConfig::never() {
            Ok(c) => c,
            Err(e) => panic!("JavascriptConfig::never failed: {e}"),
        };
        assert_eq!(config.mode, RenderMode::Never);
        assert_eq!(config.timeout.millis(), 1000);
    }

    #[cfg(feature = "javascript")]
    #[test]
    fn test_javascript_config_with_timeout() {
        let timeout = match Milliseconds::new(5000) {
            Ok(t) => t,
            Err(e) => panic!("Milliseconds::new failed: {e}"),
        };
        let config = JavascriptConfig {
            mode: RenderMode::Always,
            timeout,
        };
        assert_eq!(config.timeout.millis(), 5000);
    }

    #[cfg(feature = "anti-detection")]
    #[test]
    fn test_anti_detection_config_none() {
        let config = AntiDetectionConfig::none();
        assert_eq!(config.strategy, Strategy::None);
    }

    #[cfg(feature = "anti-detection")]
    #[test]
    fn test_anti_detection_config_rotating_ua() {
        let config = AntiDetectionConfig::rotating_ua();
        assert_eq!(config.strategy, Strategy::RotatingUserAgent);
    }

    #[cfg(feature = "anti-detection")]
    #[test]
    fn test_anti_detection_config_full_stealth() {
        let config = AntiDetectionConfig::full_stealth();
        assert_eq!(config.strategy, Strategy::FullStealth);
    }

    #[test]
    fn test_feature_config_new_is_empty() {
        let config = FeatureConfig::new();
        assert!(config.is_empty());
    }

    #[cfg(feature = "enhanced")]
    #[test]
    fn test_feature_config_with_cache() {
        let ttl = match CacheTtl::new(300) {
            Ok(t) => t,
            Err(e) => panic!("CacheTtl::new failed: {e}"),
        };
        let cache_config = CacheConfig::enabled_with_ttl(ttl);
        let config = FeatureConfig::new().with_cache(cache_config);
        assert!(!config.is_empty());
        assert!(config.cache.is_some());
    }

    #[cfg(feature = "enhanced")]
    #[test]
    fn test_feature_config_with_filtering() {
        let filtering_config = FilteringConfig::new();
        let config = FeatureConfig::new().with_filtering(filtering_config);
        assert!(!config.is_empty());
        assert!(config.filtering.is_some());
    }

    #[cfg(feature = "javascript")]
    #[test]
    fn test_feature_config_with_javascript() {
        let js_config = match JavascriptConfig::smart() {
            Ok(c) => c,
            Err(e) => panic!("JavascriptConfig::smart failed: {e}"),
        };
        let config = FeatureConfig::new().with_javascript(js_config);
        assert!(!config.is_empty());
        assert!(config.javascript.is_some());
    }

    #[cfg(feature = "anti-detection")]
    #[test]
    fn test_feature_config_with_anti_detection() {
        let ad_config = AntiDetectionConfig::full_stealth();
        let config = FeatureConfig::new().with_anti_detection(ad_config);
        assert!(!config.is_empty());
        assert!(config.anti_detection.is_some());
    }

    #[cfg(feature = "enhanced")]
    #[test]
    fn test_feature_config_builder_chain() {
        let builder = match FeatureConfigBuilder::new().enable_cache(600) {
            Ok(b) => b,
            Err(e) => panic!("enable_cache failed: {e}"),
        };
        let builder = match builder.allow_patterns(vec!["/api/*".to_string()]) {
            Ok(b) => b,
            Err(e) => panic!("allow_patterns failed: {e}"),
        };
        let config = builder.build();

        assert!(config.cache.is_some());
        assert!(config.cache.is_some_and(|c| c.enabled));
        assert!(config.filtering.is_some());
        assert!(!config.filtering.is_none_or(|f| f.allow.is_empty()));
    }

    #[test]
    fn test_cache_ttl_as_duration() {
        let ttl = match CacheTtl::new(120) {
            Ok(t) => t,
            Err(e) => panic!("CacheTtl::new failed: {e}"),
        };
        let duration = ttl.as_duration();
        assert_eq!(duration.as_secs(), 120);
    }

    #[test]
    fn test_cache_ttl_default() {
        let ttl = CacheTtl::default();
        assert_eq!(ttl.seconds(), 300); // 5 minutes
    }

    #[test]
    fn test_regex_pattern_as_str() {
        let result = RegexPattern::new(r"\d+".to_string());
        assert!(result.is_ok(), "valid pattern should succeed");
        if let Ok(pattern) = result {
            assert_eq!(pattern.as_str(), r"\d+");
        }
    }

    #[test]
    fn test_glob_pattern_as_str() {
        let result = GlobPattern::new("/docs/*".to_string());
        assert!(result.is_ok(), "valid pattern should succeed");
        if let Ok(pattern) = result {
            assert_eq!(pattern.as_str(), "/docs/*");
        }
    }
}
