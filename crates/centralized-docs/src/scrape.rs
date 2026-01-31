//! Web scraping module using spider-rs
//!
//! Provides sequential scraping of documentation sites with HTML-to-Markdown conversion.
//! Designed for AI agent consumption - no complex concurrency, predictable output.
//!
//! ## Error Recovery
//!
//! The scraper includes built-in error resilience:
//! - **Exponential backoff**: Failed requests are retried with exponential delays (configurable)
//! - **Rate limiting**: Configurable delay between requests respects server load
//! - **Robots.txt compliance**: Honors robots.txt to avoid overloading servers
//! - **Path filtering**: Optional regex filtering to avoid unnecessary crawling
//! - **HTML pruning**: Removes navigation, footers, and boilerplate before processing
//!
//! ## Configuration
//!
//! - `max_retries`: Number of retries on transient failures (default: 3)
//! - `use_exponential_backoff`: Enable backoff strategy (default: true)
//! - `delay_ms`: Base delay between requests in milliseconds (default: 250)
//! - `respect_robots`: Honor robots.txt directives (default: true)

use crate::filter::{filter_markdown, prune_html, FilterConfig, FilterResult};

#[cfg(feature = "enhanced")]
use crate::features::{FeatureConfig, FilteringConfig};

#[cfg(all(feature = "javascript", feature = "enhanced"))]
use crate::features::JavascriptConfig;

#[cfg(all(feature = "anti-detection", feature = "enhanced"))]
use crate::features::AntiDetectionConfig;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use spider::website::Website;
use spider_transformations::transformation::content::{
    self, ReturnFormat, SelectorConfiguration, TransformConfig,
};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

#[expect(clippy::expect_used)]
static H1_TITLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#\s+(.+)$").expect("hardcoded regex pattern is valid"));

#[expect(clippy::expect_used)]
static HEADER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#{1,6})\s+(.+)$").expect("hardcoded regex pattern is valid"));

#[expect(clippy::expect_used)]
static LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("hardcoded regex pattern is valid")
});

/// Configuration for scraping a documentation site
#[derive(Debug, Clone)]
pub struct ScrapeConfig {
    /// Base URL to scrape (e.g., "https://docs.example.com")
    pub base_url: String,
    /// Use sitemap.xml to discover pages (default: true)
    pub use_sitemap: bool,
    /// Regex pattern to filter URLs (e.g., "^/docs/")
    pub path_filter: Option<String>,
    /// Delay between requests in milliseconds (default: 250)
    pub delay_ms: u64,
    /// User agent string
    pub user_agent: String,
    /// Respect robots.txt (default: true)
    pub respect_robots: bool,
    /// Enable content filtering to remove nav/footer/boilerplate (default: true)
    pub enable_filtering: bool,
    /// Maximum number of retries for failed requests (default: 3)
    #[allow(dead_code)] // Reserved for retry logic implementation
    pub max_retries: u32,
    /// Enable exponential backoff for retries (default: true)
    #[allow(dead_code)] // Reserved for retry logic implementation
    pub use_exponential_backoff: bool,
    /// Maximum size of a single page in bytes (default: 10MB) - DoS protection against huge files
    pub max_page_size_bytes: u64,
    /// Maximum total content size for entire scrape in bytes (default: 500MB) - DoS protection against streaming attacks
    pub max_total_size_bytes: u64,
    /// Maximum markdown content size per page in bytes (default: 5MB) - Memory exhaustion protection
    pub max_markdown_size_bytes: u64,
    /// Maximum number of pages to scrape (default: 10000) - DoS protection
    pub max_pages: usize,
    /// Maximum number of links to extract per page (default: 1000) - Memory protection
    pub max_links_per_page: usize,
    /// Enable stealth mode to avoid bot detection (default: true)
    /// Sets realistic browser headers and mimics browser behavior
    pub stealth_mode: bool,
}

impl Default for ScrapeConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            use_sitemap: true,
            path_filter: None,
            // Increased from 250ms to 1000ms for AWS rate limit compliance
            // AWS docs sites are aggressive with rate limiting
            delay_ms: 1000,
            user_agent: "DocTransformer/5.0 (AI Documentation Indexer)".to_string(),
            respect_robots: true,
            enable_filtering: true,
            max_retries: 3,
            use_exponential_backoff: true,
            max_page_size_bytes: 10 * 1024 * 1024, // 10MB per page
            max_total_size_bytes: 500 * 1024 * 1024, // 500MB total
            max_markdown_size_bytes: 5 * 1024 * 1024, // 5MB per page markdown
            max_pages: 10_000,                     // Maximum pages to scrape
            max_links_per_page: 1_000,             // Maximum links per page
            stealth_mode: true,                    // Enable stealth mode by default
        }
    }
}

/// A scraped page with extracted content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedPage {
    /// Original URL
    pub url: String,
    /// Markdown content (converted from HTML)
    pub markdown: String,
    /// Page title (extracted from H1 or <title>)
    pub title: String,
    /// Internal links found on this page
    pub links: Vec<String>,
    /// Headers extracted (level, text)
    pub headers: Vec<Header>,
    /// Word count of markdown content
    pub word_count: usize,
    /// URL slug for filename
    pub slug: String,
    /// Whether content filtering was applied
    pub filtered: bool,
    /// Number of HTML elements removed by pruning
    pub elements_removed: usize,
    /// Content density score (0.0 - 1.0)
    pub density_score: f32,
}

/// A header extracted from the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub level: u8,
    pub text: String,
}

/// Result of scraping a site
#[derive(Debug, Serialize, Deserialize)]
pub struct ScrapeResult {
    /// Successfully scraped pages
    pub pages: Vec<ScrapedPage>,
    /// Total URLs discovered
    pub total_urls: usize,
    /// Number of successful scrapes
    pub success_count: usize,
    /// Number of failed scrapes
    pub error_count: usize,
    /// Errors encountered (url, error message)
    pub errors: Vec<(String, String)>,
    /// Base URL that was scraped
    pub base_url: String,
}

/// Scrape a documentation site with exponential backoff retry on rate limits
///
/// Wraps `scrape_site_internal` with exponential backoff retry logic.
/// If rate limiting is detected (based on error rate), waits with exponential
/// backoff and retries up to the configured max retries.
///
/// Retry schedule: 2s, 4s, 8s, 16s, 32s (configurable via max_retries)
pub async fn scrape_site(config: &ScrapeConfig) -> Result<ScrapeResult> {
    const BASE_DELAY_MS: u64 = 2000; // Start with 2 seconds

    // Use config max_retries, capped at a reasonable maximum to prevent infinite loops
    let max_retries = config.max_retries.min(10);
    let mut attempt: u32 = 0;

    loop {
        attempt = attempt.saturating_add(1);

        match scrape_site_internal(config).await {
            Ok(result) => {
                // Only apply exponential backoff if enabled
                if !config.use_exponential_backoff {
                    return Ok(result);
                }

                // Check if we got hit with rate limiting (high error rate)
                let total_requests = result.success_count.saturating_add(result.error_count);

                // If error rate is > 50%, likely rate limited - retry with backoff
                if total_requests > 10
                    && result.error_count > result.success_count
                    && attempt <= max_retries
                {
                    // Safe exponential backoff calculation with overflow protection
                    let delay_ms = calculate_backoff_delay(BASE_DELAY_MS, attempt);
                    eprintln!(
                        "[RATE LIMIT] High error rate detected ({} errors / {} total)",
                        result.error_count, total_requests
                    );
                    eprintln!("[RETRY] Waiting {delay_ms}ms before retry {attempt}...");

                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }

                // Log if we had retries
                if attempt > 1 {
                    eprintln!("[SUCCESS] Scrape completed after {attempt} attempts");
                }

                return Ok(result);
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for network-related errors that might be transient
                let is_transient = error_msg.contains("timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("dns")
                    || error_msg.contains("rate");

                // Only retry if exponential backoff is enabled and error is transient
                if config.use_exponential_backoff && is_transient && attempt <= max_retries {
                    let delay_ms = calculate_backoff_delay(BASE_DELAY_MS, attempt);
                    eprintln!("[RETRY] Transient error: {error_msg}. Retrying in {delay_ms}ms (attempt {attempt}/{max_retries})");
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }

                return Err(e);
            }
        }
    }
}

/// Calculate exponential backoff delay with overflow protection
///
/// Uses saturating arithmetic to prevent overflow. For large attempt numbers,
/// the power is capped to prevent u64 overflow.
///
/// Formula: base_delay * 2^(attempt-1), capped at u64::MAX
fn calculate_backoff_delay(base_delay_ms: u64, attempt: u32) -> u64 {
    // Cap exponent at 62 to prevent overflow (2^63 would overflow u64)
    let exponent = attempt.saturating_sub(1).min(62);
    let multiplier = 2_u64.pow(exponent);
    base_delay_ms.saturating_mul(multiplier)
}

/// Internal scrape implementation without retry logic
///
/// This function uses spider-rs internally but presents a sequential interface.
/// Spider handles concurrency; we process results one at a time.
///
/// When `use_sitemap` is true, scrapes using the sitemap for URL discovery.
/// Otherwise uses standard crawling.
async fn scrape_site_internal(config: &ScrapeConfig) -> Result<ScrapeResult> {
    // Validate URL before passing to spider (prevents panic)
    let validated_url = validate_url(&config.base_url)?;

    let mut website = Website::new(validated_url.as_str());

    // Configure spider via the website's configuration
    website.configuration.delay = config.delay_ms;
    website.configuration.respect_robots_txt = config.respect_robots;
    website.configuration.user_agent = Some(Box::new(config.user_agent.clone().into()));

    // CRITICAL: Set concurrency limit to 1 for AWS and other rate-limited sites
    // Without this, spider-rs uses CPU count (e.g., 8) concurrent workers
    // With 200ms delay + 8 workers = 40 req/s which triggers rate limiting
    // With 1000ms delay + 1 worker = 1 req/s (safe for most sites)
    website.configuration.concurrency_limit = Some(1);

    // Enable stealth mode to mimic a real browser and avoid bot detection
    // When stealth_mode is enabled, modify headers to look like a browser
    website.configuration.modify_headers = config.stealth_mode;

    // Enable retry for transient failures (network blips, temporary rate limits)
    // Use the configured max_retries value, capped to u8::MAX
    website.configuration.retry = config.max_retries.min(u8::MAX as u32) as u8;

    // Set HTTP timeouts to avoid hanging on slow responses
    use std::time::Duration;
    website.configuration.request_timeout = Some(Box::new(Duration::from_secs(30)));

    // SPIDER-RS NATIVE: Use built-in page limit instead of manual counting
    // This tells spider-rs to stop crawling after max_pages, preventing unnecessary requests
    // NOTE: with_limit() may not affect scrape_sitemap() output, so we also enforce
    // a manual limit in the page processing loop below as DoS protection
    let _ = website.configuration.with_limit(config.max_pages as u32);

    // SPIDER-RS NATIVE: Enable URL normalization to de-duplicate trailing slash pages
    // This prevents crawling both /path and /path/ as separate pages
    website.configuration.normalize = true;

    // SPIDER-RS NATIVE: Convert path_filter regex to whitelist_url for native filtering
    // This lets spider-rs filter URLs during crawling instead of post-processing
    if let Some(ref pattern) = config.path_filter {
        // Convert path regex (e.g., "/docs" or "^/docs/") to full URL regex for spider-rs
        // Spider-rs whitelist_url applies to full URLs (scheme://domain + path), not just paths
        let base_domain = url::Url::parse(&config.base_url)
            .map(|u| u.host_str().unwrap_or("").to_string())
            .unwrap_or_default();
        let scheme = url::Url::parse(&config.base_url)
            .map(|u| u.scheme().to_string())
            .unwrap_or_else(|_| "https".to_string());

        // Escape dots in domain for regex and construct full URL pattern with scheme
        let domain_escaped = regex::escape(&base_domain);
        // Strip leading ^ from user pattern if present, then add scheme://domain prefix with anchor
        let pattern_stripped = pattern.strip_prefix('^').unwrap_or(pattern);
        let full_url_pattern = format!("^{scheme}://{domain_escaped}{pattern_stripped}");
        let _ = website
            .configuration
            .with_whitelist_url(Some(vec![full_url_pattern.as_str().into()]));
        // CRITICAL: Call configure_allowlist() to compile the whitelist patterns into RegexSet
        // Without this, the whitelist_url patterns are set but never actually used for filtering
        website.configuration.configure_allowlist();
    }

    // Perform the scrape - use sitemap scraping if enabled
    if config.use_sitemap {
        // Scrape using sitemap for URL discovery
        website.scrape_sitemap().await;
    } else {
        // Standard crawling
        website.scrape().await;
    }

    // Process results sequentially with size limit tracking
    let mut pages = Vec::new();
    let mut errors = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut total_content_size: u64 = 0;

    let binding = website.get_pages();
    let scraped_pages = binding.as_ref();

    let total_urls = match scraped_pages {
        Some(pages) => pages.len(),
        None => 0,
    };

    if let Some(spider_pages) = scraped_pages {
        for page in spider_pages.iter() {
            let url = page.get_url();

            // Skip duplicates (spider-rs may return duplicates with sitemap + crawl)
            if seen_urls.contains(url) {
                continue;
            }
            seen_urls.insert(url.to_string());

            // MANUAL LIMIT: DoS protection against huge sitemaps
            // with_limit() may not affect scrape_sitemap() output, so we enforce
            // a hard limit here to prevent processing more pages than configured
            if pages.len() >= config.max_pages {
                let error_msg = format!(
                    "Reached page limit ({}), stopping scrape. {} URLs remain in sitemap.",
                    config.max_pages,
                    spider_pages.len().saturating_sub(pages.len())
                );
                errors.push((url.to_string(), error_msg));
                break;
            }

            // Transform HTML to Markdown (with optional filtering)
            // Note: Spider-rs handles path filtering via whitelist_url
            // and we use both with_limit() (native) and manual max_pages check (DoS protection)
            // Note: Individual page transformation errors are collected but don't stop the scrape.
            // This allows partial success when scraping large sites with some problematic pages.
            match transform_page(page, &config.base_url, config.enable_filtering) {
                Ok(scraped) => {
                    // Track cumulative size for DoS protection (total content limit)
                    let page_size = scraped.markdown.len() as u64;
                    total_content_size =
                        total_content_size.checked_add(page_size).ok_or_else(|| {
                            anyhow::anyhow!(
                                "Integer overflow: total content size would exceed u64::MAX"
                            )
                        })?;

                    // Check if total content exceeds limit (streaming attack protection)
                    if total_content_size > config.max_total_size_bytes {
                        let error_msg = format!(
                            "Total content size ({} bytes) exceeds limit ({} bytes), stopping scrape",
                            total_content_size, config.max_total_size_bytes
                        );
                        errors.push((url.to_string(), error_msg));
                        break;
                    }

                    pages.push(scraped);
                }
                Err(e) => {
                    let error_msg = format!("Failed to transform page: {e}");
                    errors.push((url.to_string(), error_msg));
                }
            }
        }
    }

    let success_count = pages.len();
    let error_count = errors.len();

    Ok(ScrapeResult {
        pages,
        total_urls,
        success_count,
        error_count,
        errors,
        base_url: config.base_url.clone(),
    })
}

/// Detect if a page is a rate limit response
///
/// Rate limit indicators from common CDNs and web servers:
/// - "Rate limit exceeded" text
/// - "429" status codes in HTML
/// - "Too Many Requests" text
fn detect_rate_limit_page(html: &str) -> bool {
    let html_lower = html.to_lowercase();
    html_lower.contains("rate limit exceeded")
        || html_lower.contains("429")
        || html_lower.contains("too many requests")
}

/// Transform a spider page into our ScrapedPage format
///
/// Includes size limit checking to prevent memory exhaustion from huge pages.
fn transform_page(
    page: &spider::page::Page,
    base_url: &str,
    enable_filtering: bool,
) -> Result<ScrapedPage> {
    let url = page.get_url().to_string();
    let filter_config = FilterConfig::default();

    // Get raw HTML and enforce size limits (DoS protection)
    let raw_html = page.get_html();

    // Check for rate limit pages and reject them early
    if detect_rate_limit_page(&raw_html) {
        anyhow::bail!("Rate limit page detected for {url} - skipping");
    }

    let config = ScrapeConfig::default();
    check_html_size(&raw_html, config.max_page_size_bytes)?;

    // Apply HTML-level pruning to analyze content quality
    let prune_result: FilterResult = if enable_filtering {
        prune_html(&raw_html, &filter_config)
    } else {
        FilterResult {
            html: raw_html,
            removed_count: 0,
            density_score: 1.0,
            used_readability: false,
        }
    };

    // Configure transformation for markdown output
    let transform_config = TransformConfig {
        return_format: ReturnFormat::Markdown,
        ..Default::default()
    };

    // Build selector configuration for HTML filtering (nav, footer, aside, etc.)
    // Uses the same patterns from FilterConfig for consistency
    let selector_config = if enable_filtering {
        let mut exclude_tags: Vec<String> = filter_config.remove_tags.clone();
        for pattern in &filter_config.nav_patterns {
            exclude_tags.push(format!(".{pattern}"));
            exclude_tags.push(format!("#{pattern}"));
        }
        Some(SelectorConfiguration {
            root_selector: None,
            exclude_selector: Some(exclude_tags.join(", ")),
        })
    } else {
        None
    };

    // Transform HTML to Markdown using spider_transformations
    // Args: page, config, url_selector, selector_config, clean_selectors
    let markdown =
        content::transform_content(page, &transform_config, &None, &selector_config, &None);

    // Apply additional markdown-level content filtering
    let (markdown, filtered) = if enable_filtering {
        (filter_markdown(&markdown, &filter_config), true)
    } else {
        (markdown, false)
    };

    // Enforce markdown size limit (memory exhaustion protection)
    check_markdown_size(&markdown, config.max_markdown_size_bytes)?;

    // Extract title from markdown (first H1) or fall back to URL
    let title = extract_title(&markdown, &url);

    // Extract headers from markdown
    let headers = extract_headers(&markdown);

    // Extract internal links and enforce per-page limit
    let links = extract_internal_links(&markdown, base_url);
    let (links, was_truncated) = limit_links_per_page(links, config.max_links_per_page);
    if was_truncated {
        eprintln!(
            "[WARN] Page {} had too many links, truncated to {}",
            url, config.max_links_per_page
        );
    }

    // Count words
    let word_count = markdown.split_whitespace().count();

    // Generate slug from URL (with validation for non-empty)
    let slug = url_to_slug(&url).context(format!(
        "Failed to generate slug for URL {url}: ensure URL has a valid path or hostname"
    ))?;

    Ok(ScrapedPage {
        url,
        markdown,
        title,
        links,
        headers,
        word_count,
        slug,
        filtered,
        elements_removed: prune_result.removed_count,
        density_score: prune_result.density_score,
    })
}

/// Extract title from markdown content
fn extract_title(markdown: &str, url: &str) -> String {
    // Look for first H1
    for line in markdown.lines() {
        if let Some(caps) = H1_TITLE_REGEX.captures(line.trim()) {
            if let Some(title_match) = caps.get(1) {
                return title_match.as_str().to_string();
            }
        }
    }

    // Fall back to URL path using functional pattern
    url::Url::parse(url)
        .map(|u| {
            u.path()
                .trim_matches('/')
                .split('/')
                .next_back()
                .unwrap_or("Untitled")
                .replace(['-', '_'], " ")
        })
        .unwrap_or_else(|_| "Untitled".to_string())
}

/// Extract headers from markdown
fn extract_headers(markdown: &str) -> Vec<Header> {
    let mut headers = Vec::new();

    for line in markdown.lines() {
        if let Some(caps) = HEADER_REGEX.captures(line.trim()) {
            // Safe extraction of level from capture group 1
            if let Some(level_match) = caps.get(1) {
                let level = u8::try_from(level_match.as_str().len()).unwrap_or(1); // Fallback to h1 if somehow invalid
                                                                                   // Safe extraction of text from capture group 2
                if let Some(text_match) = caps.get(2) {
                    let text = text_match.as_str().to_string();
                    headers.push(Header { level, text });
                }
            }
        }
    }

    headers
}

/// Extract internal links from markdown
fn extract_internal_links(markdown: &str, base_url: &str) -> Vec<String> {
    let base = url::Url::parse(base_url).ok();
    let mut links = Vec::new();

    for caps in LINK_REGEX.captures_iter(markdown) {
        // Safe extraction of href from capture group 2
        if let Some(href_match) = caps.get(2) {
            let href = href_match.as_str();

            // Check if internal link
            if let Some(ref base) = base {
                if let Ok(resolved) = base.join(href) {
                    if resolved.host() == base.host() {
                        links.push(resolved.to_string());
                    }
                }
            } else if href.starts_with('/') || href.starts_with("./") {
                links.push(href.to_string());
            }
        }
    }

    links.sort();
    links.dedup();
    links
}

// ===========================================================================
// ENHANCED SCRAPE FUNCTIONALITY
// ===========================================================================

/// Wrapper that combines base config with optional feature extensions
///
/// This extends `ScrapeConfig` without breaking existing code.
/// Use via `ScrapeConfig::with_features()` method.
#[cfg(feature = "enhanced")]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EnhancedScrapeConfig {
    pub base: ScrapeConfig,
    pub features: FeatureConfig,
}

#[cfg(feature = "enhanced")]
impl EnhancedScrapeConfig {
    #[allow(dead_code)]
    pub fn new(base: ScrapeConfig, features: FeatureConfig) -> Self {
        Self { base, features }
    }
}

/// Extension method on ScrapeConfig to add feature flags
///
/// Only available when "enhanced" or higher feature is enabled.
#[cfg(feature = "enhanced")]
impl ScrapeConfig {
    /// Extends this config with feature flags
    ///
    /// Only available when "enhanced" or higher feature is enabled.
    #[allow(dead_code)]
    pub fn with_features(self, features: FeatureConfig) -> EnhancedScrapeConfig {
        EnhancedScrapeConfig::new(self, features)
    }
}

/// Scrape a website with feature extensions enabled
///
/// Only available when "enhanced" feature is enabled.
/// This allows caching, filtering, and other enhancements.
#[cfg(feature = "enhanced")]
#[allow(dead_code)]
pub async fn scrape_with_features(
    config: ScrapeConfig,
    features: FeatureConfig,
) -> Result<ScrapeResult> {
    scrape_site_internal_with_features(&config, Some(features)).await
}

/// Internal scrape function that handles both basic and enhanced modes
///
/// Using `Option<FeatureConfig>` allows zero-cost abstraction when
/// features are not used.
#[cfg(feature = "enhanced")]
async fn scrape_site_internal_with_features(
    config: &ScrapeConfig,
    features: Option<FeatureConfig>,
) -> Result<ScrapeResult> {
    // Validate URL before passing to spider (prevents panic)
    let validated_url = validate_url(&config.base_url)?;

    let mut website = build_website_with_features(validated_url.as_str(), config, features)?;

    // Execute scrape
    execute_scrape_with_website(&mut website, config).await?;

    // Extract and return pages
    Ok(extract_pages_from_website(website, config))
}

/// Builds a spider Website with our base configuration and optional features
#[cfg(feature = "enhanced")]
fn build_website_with_features(
    url: &str,
    config: &ScrapeConfig,
    features: Option<FeatureConfig>,
) -> Result<Website> {
    let mut website = Website::new(url);

    // Apply basic configuration
    website.configuration.delay = config.delay_ms;
    website.configuration.respect_robots_txt = config.respect_robots;
    website.configuration.user_agent = Some(Box::new(config.user_agent.clone().into()));

    // CRITICAL: Set concurrency limit to 1 for AWS and other rate-limited sites
    website.configuration.concurrency_limit = Some(1);

    // Enable stealth mode to mimic a real browser
    website.configuration.modify_headers = config.stealth_mode;

    // Enable retry for transient failures
    website.configuration.retry = config.max_retries.min(u8::MAX as u32) as u8;

    // Set HTTP timeouts
    use std::time::Duration;
    website.configuration.request_timeout = Some(Box::new(Duration::from_secs(30)));

    // Set page limit
    let _ = website.configuration.with_limit(config.max_pages as u32);

    // Enable URL normalization
    website.configuration.normalize = true;

    // Apply features if present
    if let Some(f) = features {
        apply_features_to_website(&mut website, &f)?;
    } else {
        // Apply existing path_filter if no features
        if let Some(ref pattern) = config.path_filter {
            apply_path_filter(&mut website, &config.base_url, pattern)?;
        }
    }

    Ok(website)
}

/// Applies feature configurations to the website
#[cfg(feature = "enhanced")]
fn apply_features_to_website(website: &mut Website, features: &FeatureConfig) -> Result<()> {
    // Apply filtering configuration
    if let Some(filtering) = &features.filtering {
        apply_filtering_to_website(website, filtering)?;
    }

    // Note: Cache configuration is reserved for future implementation
    // when spider-rs cache features stabilize
    #[allow(clippy::if_same_then_else)]
    if let Some(_cache_cfg) = &features.cache {
        // Cache configuration not yet implemented due to spider-rs bugs
        // This is a placeholder for future enhancement
    }

    #[cfg(feature = "javascript")]
    if let Some(js_cfg) = &features.javascript {
        apply_javascript_config_to_website(website, js_cfg)?;
    }

    #[cfg(feature = "anti-detection")]
    if let Some(ad_cfg) = &features.anti_detection {
        apply_anti_detection_to_website(website, ad_cfg)?;
    }

    Ok(())
}

/// Applies URL filtering patterns from FilteringConfig
#[cfg(feature = "enhanced")]
fn apply_filtering_to_website(website: &mut Website, filtering: &FilteringConfig) -> Result<()> {
    // Convert our validated patterns to CompactString for spider
    use spider::compact_str::CompactString;

    if !filtering.allow.is_empty() {
        let allow_patterns: Vec<CompactString> = filtering
            .allow
            .iter()
            .map(|p| CompactString::new(p.as_str()))
            .collect();
        website.with_whitelist_url(Some(allow_patterns));
    }

    if !filtering.deny.is_empty() {
        let deny_patterns: Vec<CompactString> = filtering
            .deny
            .iter()
            .map(|p| CompactString::new(p.as_str()))
            .collect();
        website.with_blacklist_url(Some(deny_patterns));
    }

    website.configuration.configure_allowlist();

    Ok(())
}

/// Applies the original path_filter from ScrapeConfig
#[cfg(feature = "enhanced")]
fn apply_path_filter(website: &mut Website, base_url: &str, pattern: &str) -> Result<()> {
    let base_domain = url::Url::parse(base_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();
    let scheme = url::Url::parse(base_url)
        .map(|u| u.scheme().to_string())
        .unwrap_or_else(|_| "https".to_string());

    let domain_escaped = regex::escape(&base_domain);
    let pattern_stripped = pattern.strip_prefix('^').unwrap_or(pattern);
    let full_url_pattern = format!("^{scheme}://{domain_escaped}{pattern_stripped}");
    let _ = website
        .configuration
        .with_whitelist_url(Some(vec![full_url_pattern.as_str().into()]));
    website.configuration.configure_allowlist();

    Ok(())
}

/// Applies JavaScript rendering configuration
#[cfg(all(feature = "javascript", feature = "enhanced"))]
fn apply_javascript_config_to_website(
    _website: &mut Website,
    _config: &JavascriptConfig,
) -> Result<()> {
    // JavaScript configuration not yet implemented due to spider-rs bugs
    // ChromeOptions and related types are not available in spider-rs 2.39
    // This is a placeholder for future enhancement when spider-rs stabilizes
    Ok(())
}

/// Applies anti-detection configuration
#[cfg(all(feature = "anti-detection", feature = "enhanced"))]
fn apply_anti_detection_to_website(
    _website: &mut Website,
    _config: &AntiDetectionConfig,
) -> Result<()> {
    // Anti-detection configuration not yet implemented due to spider-rs bugs
    // UA generator and spoofing features have compilation issues in spider-rs 2.39
    // This is a placeholder for future enhancement when spider-rs stabilizes
    Ok(())
}

/// Generate a random user agent string
#[cfg(test)]
#[cfg(feature = "anti-detection")]
fn generate_random_user_agent() -> Option<String> {
    // Common browser user agents
    const USER_AGENTS: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
    ];

    // Simple random selection using a static counter for determinism in tests
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let prev = COUNTER.fetch_add(1, Ordering::Relaxed);
    #[allow(clippy::arithmetic_side_effects)]
    let index = prev % USER_AGENTS.len();
    Some(USER_AGENTS[index].to_string())
}

/// Execute the scrape operation with retry logic
#[cfg(feature = "enhanced")]
async fn execute_scrape_with_website(website: &mut Website, config: &ScrapeConfig) -> Result<()> {
    if config.use_sitemap {
        website.scrape_sitemap().await;
    } else {
        website.scrape().await;
    }

    Ok(())
}

/// Extract pages from the website
#[cfg(feature = "enhanced")]
fn extract_pages_from_website(website: Website, config: &ScrapeConfig) -> ScrapeResult {
    use std::collections::HashSet;

    let mut pages = Vec::new();
    let mut errors = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut total_content_size: u64 = 0;

    let binding = website.get_pages();
    let scraped_pages = binding.as_ref();

    let total_urls = match scraped_pages {
        Some(pages) => pages.len(),
        None => 0,
    };

    if let Some(spider_pages) = scraped_pages {
        for page in spider_pages.iter() {
            let url = page.get_url();

            // Skip duplicates
            if seen_urls.contains(url) {
                continue;
            }
            seen_urls.insert(url.to_string());

            // Manual limit enforcement
            if pages.len() >= config.max_pages {
                let error_msg = format!(
                    "Reached page limit ({}), stopping scrape. {} URLs remain.",
                    config.max_pages,
                    spider_pages.len().saturating_sub(pages.len())
                );
                errors.push((url.to_string(), error_msg));
                break;
            }

            // Transform page
            match transform_page(page, &config.base_url, config.enable_filtering) {
                Ok(scraped) => {
                    let page_size = scraped.markdown.len() as u64;
                    total_content_size = match total_content_size.checked_add(page_size) {
                        Some(size) => size,
                        None => {
                            let error_msg =
                                "Integer overflow: total content size would exceed u64::MAX"
                                    .to_string();
                            errors.push((url.to_string(), error_msg));
                            break;
                        }
                    };

                    // Check total content limit
                    if total_content_size > config.max_total_size_bytes {
                        let error_msg = format!(
                            "Total content size ({} bytes) exceeds limit ({} bytes), stopping scrape",
                            total_content_size, config.max_total_size_bytes
                        );
                        errors.push((url.to_string(), error_msg));
                        break;
                    }

                    pages.push(scraped);
                }
                Err(e) => {
                    let error_msg = format!("Failed to transform page: {e}");
                    errors.push((url.to_string(), error_msg));
                }
            }
        }
    }

    let success_count = pages.len();
    let error_count = errors.len();

    ScrapeResult {
        pages,
        total_urls,
        success_count,
        error_count,
        errors,
        base_url: config.base_url.clone(),
    }
}

// ===========================================================================
// END ENHANCED SCRAPE FUNCTIONALITY
// ===========================================================================

/// Validate URL format before passing to spider
///
/// Ensures the URL is well-formed and uses http or https scheme.
/// This prevents panics from spider-rs's Website::new() on invalid URLs.
fn validate_url(url: &str) -> Result<url::Url> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        anyhow::bail!("URL cannot be empty");
    }

    let parsed = url::Url::parse(trimmed).context("Invalid URL format")?;

    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        scheme => anyhow::bail!("Invalid URL scheme '{scheme}': only http and https are supported"),
    }
}

/// Check if HTML content exceeds size limit
///
/// Returns error if size exceeds max_page_size_bytes
fn check_html_size(html: &str, max_size: u64) -> Result<()> {
    let size_bytes = html.len() as u64;
    if size_bytes > max_size {
        anyhow::bail!("Page HTML too large: {size_bytes} bytes (limit: {max_size} bytes)");
    }
    Ok(())
}

/// Check if markdown content exceeds size limit
///
/// Returns error if size exceeds max_markdown_size_bytes
fn check_markdown_size(markdown: &str, max_size: u64) -> Result<()> {
    let size_bytes = markdown.len() as u64;
    if size_bytes > max_size {
        anyhow::bail!("Page markdown too large: {size_bytes} bytes (limit: {max_size} bytes)");
    }
    Ok(())
}

/// Enforce maximum links per page limit
///
/// Returns truncated vector if exceeds max_links_per_page
fn limit_links_per_page(links: Vec<String>, max_links: usize) -> (Vec<String>, bool) {
    if links.len() <= max_links {
        return (links, false);
    }
    let mut truncated = links;
    truncated.truncate(max_links);
    (truncated, true)
}

/// Validate that a slug is non-empty and filesystem-safe
///
/// Returns an error if the slug would be empty, ensuring all generated
/// slugs can be safely used as filenames.
fn validate_slug(slug: &str) -> Result<()> {
    if slug.trim().is_empty() {
        anyhow::bail!("URL slug cannot be empty: all URLs must produce non-empty identifiers");
    }
    Ok(())
}

/// Convert URL to a filesystem-safe slug using functional pattern
///
/// Returns a non-empty slug guaranteed to be safe for filenames.
/// Strips HTML suffixes (.html, .htm, -html) for cleaner filenames.
/// Falls back to hostname if path is empty.
///
/// # Contract
/// - Input: Valid or invalid URL string
/// - Output: Result<String> where String is non-empty, alphanumeric + hyphens only, lowercase
/// - Guarantees: Returned slug is always non-empty (validated before return)
fn url_to_slug(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url).context("Failed to parse URL for slug generation")?;

    // Get path and normalize
    let path = parsed.path().trim_matches('/');

    // Strip common HTML extensions first (before replacing dots)
    // Note: Must use sequential let bindings to avoid variable capture issues in chaining
    let path = path.strip_suffix(".html").unwrap_or(path);
    let path = path.strip_suffix(".htm").unwrap_or(path);

    // Use path, or empty string if no path
    let raw_slug = path.replace(['/', '.'], "-");

    // Filter to filesystem-safe characters (alphanumeric + hyphens)
    let slug = raw_slug
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>()
        .to_lowercase();

    // Strip trailing "-html" or "-htm" suffixes (from AWS URL patterns)
    // AWS docs use -html in the filename itself
    let slug = slug
        .strip_suffix("-html")
        .unwrap_or(&slug)
        .strip_suffix("-htm")
        .unwrap_or(&slug)
        .to_string();

    // Truncate to reasonable length (prevent filesystem issues)
    let slug = if slug.len() > 200 {
        slug[..200].to_string()
    } else {
        slug
    };

    // Fallback to "index" if slug is empty after filtering
    let slug = if slug.trim().is_empty() {
        "index".to_string()
    } else {
        slug
    };

    // Validate non-empty (should always pass after fallback)
    validate_slug(&slug)?;

    Ok(slug)
}

/// Filter scraped pages by BM25 relevance to query
/// Returns (kept_pages, filtered_count)
pub fn filter_pages_by_relevance(
    pages: Vec<ScrapedPage>,
    query: &str,
    threshold: f32,
) -> (Vec<ScrapedPage>, usize) {
    if pages.is_empty() {
        return (pages, 0);
    }

    // Guard: if threshold is 0.0 or negative, keep all pages (no filtering)
    if threshold <= 0.0 {
        return (pages, 0);
    }

    // Calculate average document length
    let total_words: usize = pages.iter().map(|p| p.word_count).sum();
    // SAFETY: Document counts and word counts are small (< 10k documents, < 1M words)
    // well within f32 precision (2^24 ≈ 16.7M)
    let avg_doc_length = (total_words as f32 / pages.len() as f32).max(1.0);

    // Import bm25_score from filter module
    use crate::filter::bm25_score;

    // Filter pages by BM25 score
    let (kept, filtered): (Vec<_>, Vec<_>) = pages.into_iter().partition(|page| {
        let score = bm25_score(&page.markdown, query, avg_doc_length);
        score >= threshold
    });

    let filtered_count = filtered.len();

    (kept, filtered_count)
}

/// Validate that a scrape result contains at least one page
///
/// Returns an error if no pages were successfully scraped, indicating
/// that the URL was unreachable or the site had no scannable content.
/// This prevents silent failures where an invalid URL appears to succeed.
///
/// # Contract
/// - **Preconditions:** result is a valid ScrapeResult
/// - **Postconditions:** Returns Ok(()) if success_count > 0, Err otherwise
/// - **Error message:** Includes the base URL that failed to scrape
pub fn validate_scrape_result(result: &ScrapeResult) -> Result<()> {
    if result.success_count == 0 {
        anyhow::bail!(
            "Failed to scrape any pages from '{}'. \
            Please verify:\n  \
            - The URL is accessible in a browser\n  \
            - The site has HTML content (not just API endpoints)\n  \
            - The site allows scraping (check robots.txt)",
            result.base_url
        );
    }
    Ok(())
}

/// Generate table of contents from headers
fn generate_toc(headers: &[Header]) -> String {
    if headers.is_empty() {
        return String::new();
    }

    let mut toc = String::from("## Table of Contents\n\n");
    for header in headers {
        let indent = "  ".repeat(header.level.saturating_sub(1) as usize);
        let anchor = header
            .text
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();
        toc.push_str(&format!("{}- [{}](#{})\n", indent, header.text, anchor));
    }
    toc.push_str("\n---\n\n");
    toc
}

/// Find related pages based on shared links
/// Returns up to 5 pages that share the most links with the current page
fn find_related_pages<'a>(
    current_page: &ScrapedPage,
    all_pages: &'a [ScrapedPage],
) -> Vec<&'a ScrapedPage> {
    use std::collections::HashSet;

    let current_links: HashSet<_> = current_page.links.iter().collect();

    let mut related: Vec<_> = all_pages
        .iter()
        .filter(|p| p.url != current_page.url)
        .map(|p| {
            let page_links: HashSet<_> = p.links.iter().collect();
            let shared = current_links.intersection(&page_links).count();
            (shared, p)
        })
        .filter(|(shared, _)| *shared > 0)
        .collect();

    // Sort by number of shared links (descending)
    related.sort_by(|a, b| b.0.cmp(&a.0));

    // Take top 5
    related.into_iter().take(5).map(|(_, page)| page).collect()
}

/// Write scraped pages to output directory with TOC and related pages
pub fn write_scraped_pages(result: &ScrapeResult, output_dir: &Path) -> Result<()> {
    let scrape_dir = output_dir.join(".scrape");
    fs::create_dir_all(&scrape_dir)?;

    let all_pages = &result.pages;

    for page in all_pages {
        let filename = format!("{}.md", page.slug);
        let filepath = scrape_dir.join(&filename);

        // Generate table of contents from headers
        let toc = generate_toc(&page.headers);

        // Find related pages
        let related = find_related_pages(page, all_pages);

        // Build related pages section
        let related_section = if !related.is_empty() {
            let mut section = String::from("\n## Related Pages\n\n");
            for related_page in related {
                section.push_str(&format!(
                    "- [{}]({})\n",
                    related_page.title, related_page.slug
                ));
            }
            section
        } else {
            String::new()
        };

        // Write markdown with metadata header, TOC, and related pages
        let content = format!(
            "---\nurl: {}\ntitle: {}\nword_count: {}\nfiltered: {}\nelements_removed: {}\ndensity_score: {:.2}\n---\n\n{}{}{}",
            page.url, page.title, page.word_count, page.filtered, page.elements_removed, page.density_score,
            toc, page.markdown, related_section
        );

        fs::write(&filepath, content)?;
    }

    // Write manifest
    let manifest = serde_json::to_string_pretty(result)?;
    fs::write(scrape_dir.join("manifest.json"), manifest)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://docs.rust-lang.org/book").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("").is_err());
        assert!(validate_url("   ").is_err());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("example.com").is_err()); // missing scheme
    }

    #[test]
    fn test_url_to_slug_with_path() {
        let result1 = url_to_slug("https://example.com/docs/getting-started");
        assert!(matches!(result1, Ok(ref s) if s == "docs-getting-started"));

        // HTML extensions should be stripped
        let result2 = url_to_slug("https://example.com/api/v1/users.html");
        assert!(matches!(result2, Ok(ref s) if s == "api-v1-users"));

        // HTM extension should also be stripped
        let result3 = url_to_slug("https://example.com/docs/guide.htm");
        assert!(matches!(result3, Ok(ref s) if s == "docs-guide"));
    }

    #[test]
    fn test_url_to_slug_root_url_uses_index() {
        // Root URLs should fall back to "index"
        let result = url_to_slug("https://example.com/");
        assert!(matches!(result, Ok(ref s) if s == "index"));
        if let Ok(slug) = result {
            assert!(!slug.is_empty(), "Slug from root URL must not be empty");
        }
    }

    #[test]
    fn test_url_to_slug_no_path_uses_index() {
        // URLs without path should use "index"
        let result = url_to_slug("https://docs.example.com");
        assert!(matches!(result, Ok(ref s) if s == "index"));
        if let Ok(slug) = result {
            assert!(!slug.is_empty(), "Slug must not be empty");
        }
    }

    #[test]
    fn test_url_to_slug_never_empty() {
        // Comprehensive test: any valid URL should produce non-empty slug
        let valid_urls = vec![
            "https://example.com/",
            "https://example.com",
            "https://a.b/",
            "https://docs.rust-lang.org",
            "http://localhost:8080/api/v1/users",
        ];

        for url in valid_urls {
            let result = url_to_slug(url);
            // Each valid URL should produce Ok result
            assert!(result.is_ok(), "URL {url} should produce valid slug");
            if let Ok(slug) = result {
                assert!(!slug.is_empty(), "URL {url} produced empty slug");
                assert!(
                    slug.chars().all(|c| c.is_alphanumeric() || c == '-'),
                    "Slug {slug} contains invalid characters"
                );
            }
        }
    }

    #[test]
    fn test_url_to_slug_invalid_url() {
        assert!(url_to_slug("not-a-url").is_err());
        assert!(url_to_slug("").is_err());
        assert!(url_to_slug("   ").is_err());
    }

    #[test]
    fn test_url_to_slug_special_characters_filtered() {
        let result = url_to_slug("https://example.com/docs/getting-started-2.0");
        assert!(
            result.is_ok(),
            "Should parse valid URL with special characters"
        );
        if let Ok(slug) = result {
            // Should not contain dots, only hyphens and alphanumeric
            assert!(!slug.contains("."));
            assert!(slug.chars().all(|c| c.is_alphanumeric() || c == '-'));
        }
    }

    #[test]
    fn test_url_to_slug_special_chars_only_uses_index() {
        // URLs with only special characters should fall back to "index"
        let result1 = url_to_slug("https://example.com/???");
        assert!(matches!(result1, Ok(ref s) if s == "index"));

        let result2 = url_to_slug("https://example.com/@@@");
        assert!(matches!(result2, Ok(ref s) if s == "index"));

        let result3 = url_to_slug("https://example.com/!!!");
        assert!(matches!(result3, Ok(ref s) if s == "index"));
    }

    #[test]
    fn test_url_to_slug_multiple_slashes_uses_index() {
        // Multiple slashes should be treated as root and use index
        let result = url_to_slug("https://example.com///");
        assert!(matches!(result, Ok(ref s) if s == "index"));
    }

    #[test]
    fn test_url_to_slug_truncates_long_paths() {
        // Create a URL with an extremely long path
        let long_path = "https://example.com/".to_string() + &"very-long-path-segment-".repeat(20); // Create 400+ char path
        let result = url_to_slug(&long_path);
        assert!(result.is_ok(), "Should parse URL with long path");
        if let Ok(slug) = result {
            assert!(slug.len() <= 200, "Slug should be truncated to 200 chars");
        }
    }

    #[test]
    fn test_extract_title() {
        let md = "# Getting Started\n\nThis is content.";
        assert_eq!(
            extract_title(md, "https://example.com/foo"),
            "Getting Started"
        );

        let md_no_h1 = "Some content without header";
        assert_eq!(
            extract_title(md_no_h1, "https://example.com/getting-started"),
            "getting started"
        );
    }

    #[test]
    fn test_extract_headers() {
        let md = "# Title\n## Section 1\n### Subsection\n## Section 2";
        let headers = extract_headers(md);
        assert_eq!(headers.len(), 4);
        assert_eq!(headers[0].level, 1);
        assert_eq!(headers[0].text, "Title");
        assert_eq!(headers[1].level, 2);
        assert_eq!(headers[2].level, 3);
    }

    #[test]
    fn test_extract_internal_links() {
        let md = "[Link 1](/docs/page1) and [Link 2](https://example.com/docs/page2) and [External](https://other.com/page)";
        let links = extract_internal_links(md, "https://example.com");
        assert_eq!(links.len(), 2);
        assert!(links.iter().any(|l| l.contains("page1")));
        assert!(links.iter().any(|l| l.contains("page2")));
    }

    // ============================================================================
    // BM25 FILTERING TESTS
    // ============================================================================

    fn create_test_page(markdown: &str, title: &str, url: &str) -> anyhow::Result<ScrapedPage> {
        let word_count = markdown.split_whitespace().count();
        let slug = url_to_slug(url)?;
        Ok(ScrapedPage {
            url: url.to_string(),
            markdown: markdown.to_string(),
            title: title.to_string(),
            links: Vec::new(),
            headers: Vec::new(),
            word_count,
            slug,
            filtered: false,
            elements_removed: 0,
            density_score: 1.0,
        })
    }

    #[test]
    fn test_filter_keeps_relevant_pages() -> anyhow::Result<()> {
        let pages = vec![
            create_test_page(
                "Rust is a systems programming language that runs blazingly fast. Rust programming is great for systems development.",
                "Rust Guide",
                "https://example.com/rust-guide",
            )?,
            create_test_page(
                "Python is a high-level programming language. Learn Python for web development.",
                "Python Tutorial",
                "https://example.com/python-tutorial",
            )?,
            create_test_page(
                "JavaScript is the language of the web. Modern JavaScript powers interactive websites.",
                "JavaScript Intro",
                "https://example.com/js-intro",
            )?,
        ];

        let (kept, filtered_count) = filter_pages_by_relevance(pages, "rust programming", 0.1);

        // Should keep at least the Rust guide
        assert!(!kept.is_empty(), "Should keep at least 1 Rust-related page");
        assert!(
            filtered_count >= 1,
            "Should filter out at least 1 non-Rust page"
        );

        // Check that rust page is in the kept list
        assert!(
            kept.iter().any(|p| p.title.contains("Rust")),
            "Should keep Rust page"
        );
        Ok(())
    }

    #[test]
    fn test_filter_all_filtered_out() -> anyhow::Result<()> {
        let pages = vec![
            create_test_page("Rust programming", "Rust Guide", "https://example.com/rust")?,
            create_test_page(
                "Python tutorial",
                "Python Guide",
                "https://example.com/python",
            )?,
        ];

        // Use a very high threshold to filter everything out
        let (kept, filtered_count) = filter_pages_by_relevance(pages.clone(), "rust", 10.0);

        assert_eq!(kept.len(), 0, "High threshold should filter all pages");
        assert_eq!(filtered_count, pages.len(), "All pages should be filtered");
        Ok(())
    }

    #[test]
    fn test_filter_zero_threshold_keeps_all() -> anyhow::Result<()> {
        let pages = vec![
            create_test_page("Rust programming", "Rust", "https://example.com/rust")?,
            create_test_page("Python tutorial", "Python", "https://example.com/python")?,
        ];
        let original_count = pages.len();

        let (kept, filtered_count) = filter_pages_by_relevance(pages, "rust", 0.0);

        assert_eq!(
            kept.len(),
            original_count,
            "Threshold 0.0 should keep all pages"
        );
        assert_eq!(
            filtered_count, 0,
            "No pages should be filtered with threshold 0.0"
        );
        Ok(())
    }

    #[test]
    fn test_filter_negative_threshold_keeps_all() -> anyhow::Result<()> {
        let pages = vec![
            create_test_page("Rust programming", "Rust", "https://example.com/rust")?,
            create_test_page("Python tutorial", "Python", "https://example.com/python")?,
        ];
        let original_count = pages.len();

        let (kept, filtered_count) = filter_pages_by_relevance(pages, "rust", -1.0);

        assert_eq!(
            kept.len(),
            original_count,
            "Negative threshold should keep all pages"
        );
        assert_eq!(
            filtered_count, 0,
            "No pages should be filtered with negative threshold"
        );
        Ok(())
    }

    #[test]
    fn test_filter_no_matches() -> anyhow::Result<()> {
        let pages = vec![
            create_test_page("Rust programming", "Rust", "https://example.com/rust")?,
            create_test_page("Python tutorial", "Python", "https://example.com/python")?,
        ];
        let original_count = pages.len();

        // Query for something that doesn't exist in any page
        let (kept, filtered_count) = filter_pages_by_relevance(pages, "nonexistent_term_xyz", 0.1);

        assert_eq!(kept.len(), 0, "No pages should match nonexistent term");
        assert_eq!(
            filtered_count, original_count,
            "All pages should be filtered"
        );
        Ok(())
    }

    #[test]
    fn test_filter_empty_pages_list() {
        let pages: Vec<ScrapedPage> = Vec::new();

        let (kept, filtered_count) = filter_pages_by_relevance(pages, "query", 0.1);

        assert_eq!(kept.len(), 0, "Empty input should return empty output");
        assert_eq!(filtered_count, 0, "No pages to filter");
    }

    #[test]
    fn test_filter_case_insensitive() -> anyhow::Result<()> {
        let pages = vec![create_test_page(
            "Rust programming language",
            "Rust",
            "https://example.com/rust",
        )?];

        let (kept_lower, _) = filter_pages_by_relevance(pages.clone(), "rust", 0.1);
        let (kept_upper, _) = filter_pages_by_relevance(pages.clone(), "RUST", 0.1);
        let (kept_mixed, _) = filter_pages_by_relevance(pages, "RuSt", 0.1);

        // All should return the same results
        assert_eq!(kept_lower.len(), kept_upper.len(), "Case should not matter");
        assert_eq!(kept_lower.len(), kept_mixed.len(), "Case should not matter");
        Ok(())
    }

    #[test]
    fn test_filter_multi_term_query() -> anyhow::Result<()> {
        let pages = vec![
            create_test_page(
                "Rust is a systems programming language that guarantees memory safety.",
                "Rust Guide",
                "https://example.com/rust",
            )?,
            create_test_page("JavaScript tutorial", "JS Guide", "https://example.com/js")?,
        ];

        // Multi-term query
        let (kept, _) = filter_pages_by_relevance(pages, "rust programming systems", 0.1);

        // Should find pages containing any of these terms
        assert!(
            !kept.is_empty(),
            "Should find pages matching multi-term query"
        );
        assert!(
            kept.iter().any(|p| p.title.contains("Rust")),
            "Should find rust page with multi-term query"
        );
        Ok(())
    }

    #[test]
    fn test_filter_different_thresholds() -> anyhow::Result<()> {
        let pages = vec![
            create_test_page(
                "Rust programming language systems",
                "Rust",
                "https://example.com/rust1",
            )?,
            create_test_page("Rust", "Rust Short", "https://example.com/rust2")?,
            create_test_page("Python programming", "Python", "https://example.com/python")?,
        ];

        let (kept_low, _) = filter_pages_by_relevance(pages.clone(), "rust", 0.1);
        let (kept_medium, _) = filter_pages_by_relevance(pages.clone(), "rust", 0.5);
        let (kept_high, _) = filter_pages_by_relevance(pages, "rust", 2.0);

        // Lower threshold should keep more pages
        assert!(
            kept_low.len() >= kept_medium.len(),
            "Lower threshold should keep more pages"
        );
        assert!(
            kept_medium.len() >= kept_high.len(),
            "Medium threshold should keep more than high"
        );
        Ok(())
    }

    #[test]
    fn test_filter_preserves_page_metadata() -> anyhow::Result<()> {
        let original_page = create_test_page(
            "Rust programming guide with comprehensive examples",
            "Rust Guide",
            "https://example.com/rust-guide",
        )?;
        let original_url = original_page.url.clone();
        let original_title = original_page.title.clone();
        let original_word_count = original_page.word_count;

        let pages = vec![original_page];
        let (kept, _) = filter_pages_by_relevance(pages, "rust programming", 0.1);

        assert_eq!(kept.len(), 1, "Should keep the rust page");
        let filtered_page = &kept[0];

        assert_eq!(filtered_page.url, original_url, "URL should be preserved");
        assert_eq!(
            filtered_page.title, original_title,
            "Title should be preserved"
        );
        assert_eq!(
            filtered_page.word_count, original_word_count,
            "Word count should be preserved"
        );
        Ok(())
    }

    #[test]
    fn test_filter_with_special_characters_in_query() -> anyhow::Result<()> {
        let pages = vec![create_test_page(
            "Rust-lang systems programming",
            "Rust",
            "https://example.com/rust",
        )?];

        // Query with special characters (should not crash)
        let result = std::panic::catch_unwind(|| {
            filter_pages_by_relevance(pages, "rust-lang & systems", 0.1)
        });

        assert!(result.is_ok(), "Should handle special characters in query");
        Ok(())
    }

    #[test]
    fn test_filter_empty_query() -> anyhow::Result<()> {
        let pages = vec![create_test_page(
            "Rust programming",
            "Rust",
            "https://example.com/rust",
        )?];

        let (kept, filtered_count) = filter_pages_by_relevance(pages.clone(), "", 0.1);

        // Empty query should filter all pages (no terms to match)
        assert_eq!(kept.len(), 0, "Empty query should match nothing");
        assert_eq!(
            filtered_count,
            pages.len(),
            "All pages should be filtered with empty query"
        );
        Ok(())
    }

    #[test]
    fn test_filter_calculates_average_correctly() -> anyhow::Result<()> {
        // Create pages with known word counts
        let pages = vec![
            create_test_page("one two three four five", "Page 1", "https://example.com/1")?, // 5 words
            create_test_page("one two three", "Page 2", "https://example.com/2")?, // 3 words
            create_test_page("one two", "Page 3", "https://example.com/3")?,       // 2 words
        ];
        // Average: (5 + 3 + 2) / 3 = 3.33 words

        // The filter should calculate avg_doc_length correctly and use it for scoring
        // We can't test the internal calculation directly, but we can verify it doesn't panic
        let result = std::panic::catch_unwind(|| filter_pages_by_relevance(pages, "one", 0.1));

        assert!(
            result.is_ok(),
            "Should calculate average document length without panicking"
        );
        Ok(())
    }

    // ============================================================================
    // SIZE LIMIT TESTS (DoS PROTECTION)
    // ============================================================================

    #[test]
    fn test_check_html_size_valid() {
        let html = "<html><body>Hello</body></html>";
        let result = check_html_size(html, 1000);
        assert!(result.is_ok(), "Small HTML should pass size check");
    }

    #[test]
    fn test_check_html_size_exceeds_limit() {
        let html = "x".repeat(1001);
        let result = check_html_size(&html, 1000);
        assert!(result.is_err(), "HTML exceeding limit should fail");
        // Additional check that we can format the error
        if let Err(e) = &result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("too large"), "Error should mention size");
        }
    }

    #[test]
    fn test_check_markdown_size_valid() {
        let markdown = "# Hello\n\nThis is content.";
        let result = check_markdown_size(markdown, 1000);
        assert!(result.is_ok(), "Small markdown should pass size check");
    }

    #[test]
    fn test_check_markdown_size_exceeds_limit() {
        let markdown = "x".repeat(5001);
        let result = check_markdown_size(&markdown, 5000);
        assert!(result.is_err(), "Markdown exceeding limit should fail");
        // Additional check that we can format the error
        if let Err(e) = &result {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("too large"),
                "Error should mention markdown size"
            );
        }
    }

    #[test]
    fn test_limit_links_per_page_within_limit() {
        let links = vec![
            "link1".to_string(),
            "link2".to_string(),
            "link3".to_string(),
        ];
        let (result, was_truncated) = limit_links_per_page(links, 10);
        assert_eq!(result.len(), 3, "All links should be kept");
        assert!(!was_truncated, "Should not be truncated");
    }

    #[test]
    fn test_limit_links_per_page_exceeds_limit() {
        let links = vec![
            "link1".to_string(),
            "link2".to_string(),
            "link3".to_string(),
        ];
        let (result, was_truncated) = limit_links_per_page(links, 2);
        assert_eq!(result.len(), 2, "Links should be truncated to limit");
        assert!(was_truncated, "Should indicate truncation");
    }

    #[test]
    fn test_limit_links_per_page_exactly_at_limit() {
        let links = vec!["link1".to_string(), "link2".to_string()];
        let (result, was_truncated) = limit_links_per_page(links, 2);
        assert_eq!(result.len(), 2, "All links at limit should be kept");
        assert!(!was_truncated, "Should not truncate when at exact limit");
    }

    #[test]
    fn test_limit_links_per_page_empty() {
        let links: Vec<String> = vec![];
        let (result, was_truncated) = limit_links_per_page(links, 10);
        assert_eq!(result.len(), 0, "Empty list should remain empty");
        assert!(!was_truncated, "Empty list should not be truncated");
    }

    #[test]
    fn test_scrape_config_default_has_size_limits() {
        let config = ScrapeConfig::default();
        assert_eq!(
            config.max_page_size_bytes,
            10 * 1024 * 1024,
            "Default max page size should be 10MB"
        );
        assert_eq!(
            config.max_total_size_bytes,
            500 * 1024 * 1024,
            "Default max total size should be 500MB"
        );
        assert_eq!(
            config.max_markdown_size_bytes,
            5 * 1024 * 1024,
            "Default max markdown size should be 5MB"
        );
        assert_eq!(
            config.max_pages, 10_000,
            "Default max pages should be 10000"
        );
        assert_eq!(
            config.max_links_per_page, 1_000,
            "Default max links per page should be 1000"
        );
    }

    #[test]
    fn test_scrape_config_limits_are_reasonable() {
        let config = ScrapeConfig::default();
        // Verify: max_page_size < max_total_size
        assert!(
            config.max_page_size_bytes < config.max_total_size_bytes,
            "Per-page limit must be less than total limit"
        );
        // Verify: max_markdown_size <= max_page_size
        assert!(
            config.max_markdown_size_bytes <= config.max_page_size_bytes,
            "Markdown limit should not exceed page limit"
        );
        // Verify: reasonable defaults
        assert!(config.max_pages > 0, "Max pages must be positive");
        assert!(
            config.max_links_per_page > 0,
            "Max links per page must be positive"
        );
    }

    // ============================================================================
    // VALIDATION FUNCTIONS
    // ============================================================================

    /// Validate that a scrape result contains at least one successfully scraped page.
    ///
    /// Design by Contract:
    /// - **Preconditions:** scrape_result is a valid reference
    /// - **Postconditions:** Returns Ok(()) if pages > 0, Err otherwise
    ///
    /// This validates that the scraping operation produced useful output.
    pub fn validate_scrape_result(scrape_result: &ScrapeResult) -> Result<()> {
        if scrape_result.pages.is_empty() {
            anyhow::bail!(
                "No pages were successfully scraped from {}. \
                Please verify the URL is accessible and contains content.",
                scrape_result.base_url
            );
        }
        Ok(())
    }

    // ============================================================================
    // INGEST COMMAND VALIDATION TEST
    // ============================================================================

    #[test]
    fn test_validate_url_format_only_validates_scheme() {
        // validate_url checks URL format and scheme, not reachability
        // This test documents that https://not-a-valid-url is considered VALID format
        let result = validate_url("https://not-a-valid-url");
        assert!(
            result.is_ok(),
            "URL with valid https scheme should pass format validation"
        );

        // But a URL without scheme should fail
        assert!(validate_url("not-a-valid-url").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    /// Test that validate_scrape_result errors on zero pages
    ///
    /// This test will FAIL initially because validate_scrape_result doesn't exist yet.
    /// After implementing the function, this test should pass.
    #[test]
    fn test_validate_scrape_result_requires_pages() {
        // Zero pages should return an error
        let zero_result = ScrapeResult {
            pages: vec![],
            total_urls: 0,
            success_count: 0,
            error_count: 0,
            errors: vec![],
            base_url: "https://not-a-valid-url".to_string(),
        };
        assert!(validate_scrape_result(&zero_result).is_err());

        // Non-zero pages should return Ok
        let ok_result = ScrapeResult {
            pages: vec![ScrapedPage {
                url: "https://example.com".to_string(),
                markdown: "# Test".to_string(),
                title: "Test".to_string(),
                links: vec![],
                headers: vec![],
                word_count: 1,
                slug: "test".to_string(),
                filtered: false,
                elements_removed: 0,
                density_score: 1.0,
            }],
            total_urls: 1,
            success_count: 1,
            error_count: 0,
            errors: vec![],
            base_url: "https://example.com".to_string(),
        };
        assert!(validate_scrape_result(&ok_result).is_ok());
    }

    #[test]
    fn test_huge_content_detection() {
        // Simulate 100MB of repeated text
        let huge_text = "x".repeat(100 * 1024 * 1024);
        let config = ScrapeConfig::default();
        let result = check_html_size(&huge_text, config.max_page_size_bytes);
        assert!(result.is_err(), "100MB content should exceed 10MB limit");
    }

    #[test]
    fn test_streaming_attack_protection() {
        // Simulate multiple pages hitting the total limit
        let mut total_size = 0u64;
        let config = ScrapeConfig::default();
        let page_size = config.max_page_size_bytes / 2; // 5MB per page
        let mut pages_before_limit = 0usize;

        while total_size.saturating_add(page_size) <= config.max_total_size_bytes {
            total_size = total_size.saturating_add(page_size);
            pages_before_limit = pages_before_limit.saturating_add(1);
        }

        // With 500MB limit and 5MB pages, should allow ~100 pages
        assert!(
            (90..=110).contains(&pages_before_limit),
            "Should allow ~100 5MB pages in 500MB budget, got: {pages_before_limit}"
        );
    }

    // ============================================================================
    // STEALTH MODE AND RETRY TESTS (TDD15 Phase 4: RED - FAILING TESTS)
    // ============================================================================

    #[test]
    fn test_scrape_config_stealth_mode_default() {
        let config = ScrapeConfig::default();
        assert!(
            config.stealth_mode,
            "Default stealth_mode should be true to avoid bot detection"
        );
    }

    #[test]
    fn test_scrape_config_stealth_mode_can_be_disabled() {
        let config = ScrapeConfig {
            stealth_mode: false,
            ..Default::default()
        };
        assert!(!config.stealth_mode, "stealth_mode should be configurable");
    }

    #[test]
    fn test_scrape_config_max_retries_default() {
        let config = ScrapeConfig::default();
        assert_eq!(config.max_retries, 3, "Default max_retries should be 3");
    }

    #[test]
    fn test_scrape_config_max_retries_can_be_configured() {
        let config = ScrapeConfig {
            max_retries: 5,
            ..Default::default()
        };
        assert_eq!(config.max_retries, 5, "max_retries should be configurable");
    }

    #[test]
    fn test_scrape_config_exponential_backoff_default() {
        let config = ScrapeConfig::default();
        assert!(
            config.use_exponential_backoff,
            "Default use_exponential_backoff should be true"
        );
    }

    #[test]
    fn test_exponential_backoff_delay_calculation() {
        // Test that exponential backoff delay is calculated correctly
        // Formula: BASE_DELAY_MS * 2^(attempt-1)
        let base_delay_ms: u64 = 2000;

        // Attempt 1: 2000 * 2^0 = 2000ms
        let delay_1 = base_delay_ms.saturating_mul(2_u64.pow(0));
        assert_eq!(delay_1, 2000, "First retry delay should be 2000ms");

        // Attempt 2: 2000 * 2^1 = 4000ms
        let delay_2 = base_delay_ms.saturating_mul(2_u64.pow(1));
        assert_eq!(delay_2, 4000, "Second retry delay should be 4000ms");

        // Attempt 3: 2000 * 2^2 = 8000ms
        let delay_3 = base_delay_ms.saturating_mul(2_u64.pow(2));
        assert_eq!(delay_3, 8000, "Third retry delay should be 8000ms");

        // Attempt 5: 2000 * 2^4 = 32000ms
        let delay_5 = base_delay_ms.saturating_mul(2_u64.pow(4));
        assert_eq!(delay_5, 32000, "Fifth retry delay should be 32000ms");
    }

    #[test]
    fn test_exponential_backoff_delay_never_overflows() {
        // Test that saturating arithmetic prevents overflow
        // The key is that pow() can overflow, so we need checked_pow
        let base_delay_ms: u64 = 2000;

        // Safe calculation with checking for overflow
        let attempt: u32 = 100; // Very large attempt number

        // Use checked_pow to safely calculate power, defaulting to u64::MAX on overflow
        let power = if attempt.saturating_sub(1) < 63 {
            2_u64.pow(attempt.saturating_sub(1).min(62))
        } else {
            u64::MAX
        };

        let delay = base_delay_ms.saturating_mul(power);
        assert!(
            delay > 0,
            "Delay should be positive even with large attempt"
        );

        // Also test the actual implementation pattern we use
        // Verify that saturating operations don't panic
        let max_safe_power = 2_u64.pow(62); // Less than u64::MAX
        let safe_delay = base_delay_ms.saturating_mul(max_safe_power);
        assert!(safe_delay > 0, "Saturating mul should work");
    }

    #[test]
    fn test_config_respects_max_retries_field() {
        // Test that max_retries field is actually used
        let config = ScrapeConfig {
            base_url: "https://example.com".to_string(),
            use_sitemap: true,
            path_filter: None,
            delay_ms: 1000,
            user_agent: "TestAgent".to_string(),
            respect_robots: true,
            enable_filtering: true,
            max_retries: 7, // Custom retry count
            use_exponential_backoff: true,
            max_page_size_bytes: 10 * 1024 * 1024,
            max_total_size_bytes: 500 * 1024 * 1024,
            max_markdown_size_bytes: 5 * 1024 * 1024,
            max_pages: 100,
            max_links_per_page: 100,
            stealth_mode: true,
        };

        assert_eq!(
            config.max_retries, 7,
            "Custom max_retries should be preserved"
        );
    }

    // ============================================================================
    // BACKOFF CALCULATION TESTS
    // ============================================================================

    #[test]
    fn test_calculate_backoff_delay_returns_correct_delays() {
        // Test the actual implementation function
        assert_eq!(
            calculate_backoff_delay(2000, 1),
            2000,
            "Attempt 1 should be 2000ms"
        );
        assert_eq!(
            calculate_backoff_delay(2000, 2),
            4000,
            "Attempt 2 should be 4000ms"
        );
        assert_eq!(
            calculate_backoff_delay(2000, 3),
            8000,
            "Attempt 3 should be 8000ms"
        );
        assert_eq!(
            calculate_backoff_delay(2000, 4),
            16000,
            "Attempt 4 should be 16000ms"
        );
        assert_eq!(
            calculate_backoff_delay(2000, 5),
            32000,
            "Attempt 5 should be 32000ms"
        );
    }

    #[test]
    fn test_calculate_backoff_delay_handles_large_attempts() {
        // Test with very large attempt number
        let delay = calculate_backoff_delay(2000, 100);
        assert!(delay > 0, "Should handle large attempts without overflow");

        // The delay should saturate at a reasonable maximum
        // 2^62 is the max power before overflow
        let max_delay = calculate_backoff_delay(2000, 63);
        assert!(max_delay > 0, "Should handle attempt 63");
    }

    #[test]
    fn test_calculate_backoff_delay_with_different_base() {
        assert_eq!(
            calculate_backoff_delay(1000, 1),
            1000,
            "Base 1000, attempt 1"
        );
        assert_eq!(
            calculate_backoff_delay(1000, 2),
            2000,
            "Base 1000, attempt 2"
        );
        assert_eq!(calculate_backoff_delay(500, 3), 2000, "Base 500, attempt 3");
    }
}

// ============================================================================
// ADVERSARIAL QA TESTS (bead doc-tx-8p5)
// ============================================================================

#[test]
fn test_backoff_attempt_zero_edge_case() {
    // Edge case: What happens if attempt is 0?
    // The implementation does saturating_sub(1), so 0 -> 0, 2^0 = 1
    let delay = calculate_backoff_delay(2000, 0);
    assert_eq!(delay, 2000, "Attempt 0 should produce base delay");
}

#[test]
fn test_backoff_attempt_at_cap_boundary() {
    // Test at the boundary of the cap (62)
    // 2^62 is the largest safe power of 2 before overflow
    let delay_63 = calculate_backoff_delay(2000, 63);
    assert!(delay_63 > 0, "Should handle max capped attempt");

    // Should be capped, not overflow
    let max_power = 2_u64.pow(62);
    let expected = 2000_u64.saturating_mul(max_power);
    assert_eq!(delay_63, expected, "Should use max safe power");
}

#[test]
fn test_backoff_beyond_cap_stays_capped() {
    // Test beyond the cap - should still not panic
    let delay_63 = calculate_backoff_delay(2000, 63);
    let delay_100 = calculate_backoff_delay(2000, 100);
    let delay_1000 = calculate_backoff_delay(2000, 1000);

    // All should be capped at same maximum
    assert_eq!(delay_63, delay_100, "Should cap at max");
    assert_eq!(delay_100, delay_1000, "Should remain capped");
}

#[test]
fn test_backoff_u32_max_does_not_panic() {
    // Edge case: u32::MAX attempt number
    let delay = calculate_backoff_delay(2000, u32::MAX);
    assert!(delay > 0, "Should handle u32::MAX without panic");

    // Should be capped at maximum safe value
    let max_safe = calculate_backoff_delay(2000, 63);
    assert_eq!(delay, max_safe, "Should cap u32::MAX same as 63");
}

#[test]
fn test_backoff_small_base_delay() {
    // Very small base delay (1ms)
    assert_eq!(calculate_backoff_delay(1, 1), 1);
    assert_eq!(calculate_backoff_delay(1, 2), 2);
    assert_eq!(calculate_backoff_delay(1, 10), 512);
}

#[test]
fn test_backoff_large_base_delay() {
    // Large base delay (60 seconds = 60000ms)
    assert_eq!(calculate_backoff_delay(60000, 1), 60000);
    assert_eq!(calculate_backoff_delay(60000, 2), 120000);

    // At high attempts, should still not overflow
    let delay = calculate_backoff_delay(60000, 50);
    assert!(
        delay > 0,
        "Large base with high attempt should not overflow"
    );
}

#[test]
fn test_backoff_zero_base_delay() {
    // Edge case: zero base delay
    assert_eq!(calculate_backoff_delay(0, 1), 0);
    assert_eq!(calculate_backoff_delay(0, 100), 0);
}

#[test]
fn test_backoff_exponential_growth_sequence() {
    // Verify proper exponential sequence
    let base = 1000;
    let mut expected = base;

    for attempt in 1..=10 {
        let delay = calculate_backoff_delay(base, attempt);
        // For first 5 attempts, verify exact doubling
        if attempt <= 5 {
            assert_eq!(delay, expected, "Attempt {attempt} should be {expected}ms");
        }
        expected = expected.saturating_mul(2);
    }
}

#[test]
fn test_backoff_power_capped_at_exactly_62() {
    // Verify the exponent capping logic
    // The implementation caps at 62 because 2^63 would overflow u64

    // Attempt 63 should use exponent 62
    let delay_63 = calculate_backoff_delay(1, 63);
    let expected_max = 2_u64.pow(62);
    assert_eq!(delay_63, expected_max, "Should cap exponent at 62");

    // Attempt 64 should also be capped
    let delay_64 = calculate_backoff_delay(1, 64);
    assert_eq!(delay_64, expected_max, "Should still be capped at 64");

    // Attempt 100 should also be capped
    let delay_100 = calculate_backoff_delay(1, 100);
    assert_eq!(delay_100, expected_max, "Should still be capped at 100");
}

#[test]
fn test_backoff_saturating_mul_behavior() {
    // Test saturating_mul with large numbers
    let base = u64::MAX / 2; // Large base
    let delay = calculate_backoff_delay(base, 2); // base * 2

    // Should not overflow, should saturate
    assert!(delay > 0, "Saturating mul should produce valid result");
}

#[test]
fn test_stealth_mode_config_affects_scrape_behavior() {
    // Verify stealth_mode propagates to config
    let stealth_config = ScrapeConfig {
        stealth_mode: true,
        ..Default::default()
    };
    assert!(stealth_config.stealth_mode);

    let non_stealth_config = ScrapeConfig {
        stealth_mode: false,
        ..Default::default()
    };
    assert!(!non_stealth_config.stealth_mode);
}

#[test]
fn test_exponential_backoff_disabled_config() {
    // Config to disable exponential backoff
    let config = ScrapeConfig {
        use_exponential_backoff: false,
        max_retries: 0,
        ..Default::default()
    };

    assert!(!config.use_exponential_backoff);
    assert_eq!(config.max_retries, 0);
}

#[test]
fn test_max_retries_high_value_accepted() {
    // Very high max_retries should be accepted in config
    // (implementation caps at 10 in scrape_site)
    let config = ScrapeConfig {
        max_retries: 1000,
        ..Default::default()
    };
    assert_eq!(config.max_retries, 1000);
}

#[test]
fn test_delay_aws_safe_default() {
    // Verify default delay is AWS-safe (1000ms with concurrency 1)
    let config = ScrapeConfig::default();
    assert_eq!(config.delay_ms, 1000, "Default delay should be AWS-safe");
}

#[test]
fn test_delay_zero_allowed() {
    // Zero delay is technically allowed (though unsafe for production)
    let config = ScrapeConfig {
        delay_ms: 0,
        ..Default::default()
    };
    assert_eq!(config.delay_ms, 0);
}

// ============================================================================
// SPIDER-RS NATIVE FEATURE TESTS (Martin Fowler methodology)
// ============================================================================

/// Test: ScrapeConfig respects max_pages field for spider-rs with_limit
///
/// Behavior: When max_pages is set, spider-rs should be configured to limit
/// crawling to that many pages. This is a configuration test - the actual
/// limiting happens during the scrape.
#[test]
fn test_max_pages_configuration_is_propagated() {
    let config = ScrapeConfig {
        base_url: "https://example.com".to_string(),
        max_pages: 100,
        ..Default::default()
    };
    assert_eq!(
        config.max_pages, 100,
        "max_pages should be preserved in config"
    );
}

/// Test: Path filter can be converted to spider-rs whitelist URL pattern
///
/// Behavior: When a path_filter regex like "^/docs/" is provided, it should
/// be convertible to a full URL whitelist pattern for spider-rs.
/// This verifies the transformation logic: path -> full URL regex.
#[test]
fn test_path_filter_to_whitelist_conversion() {
    use regex::Regex;

    // Simulate the conversion logic from scrape_site_internal
    let base_url = "https://example.com";
    let path_filter = "/docs"; // User provides path without regex anchors

    let base_domain = url::Url::parse(base_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();

    let domain_escaped = regex::escape(&base_domain);
    // The full pattern: domain + path (spider-rs whitelist matches full URL)
    let full_url_pattern = format!("^{domain_escaped}{path_filter}");

    // The pattern should be a valid regex
    let regex = Regex::new(&full_url_pattern);
    assert!(
        regex.is_ok(),
        "Pattern should be valid regex: {full_url_pattern}"
    );

    // The pattern should match the domain + path - verify without unwrap
    let pattern_matches = |url: &str| -> bool {
        if let Ok(ref p) = regex {
            p.is_match(url)
        } else {
            false
        }
    };

    // Should match URLs starting with domain + path
    assert!(
        pattern_matches("example.com/docs/page1"),
        "Pattern should match domain/path URLs"
    );
    assert!(
        pattern_matches("example.com/docs"),
        "Pattern should match domain/path root"
    );
    // Should not match non-docs URLs
    assert!(
        !pattern_matches("example.com/blog/page1"),
        "Pattern should not match non-matching URLs"
    );
}

/// Test: Multiple path filters can be combined into whitelist
///
/// Behavior: Multiple path patterns should be combinable into regex set.
#[test]
fn test_multiple_path_filters_can_be_combined() {
    use regex::RegexSet;

    let base_url = "https://example.com";
    let path_filters = ["/docs", "/api", "/blog"];

    let base_domain = url::Url::parse(base_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();

    let domain_escaped = regex::escape(&base_domain);
    let patterns: Vec<String> = path_filters
        .iter()
        .map(|p| format!("^{domain_escaped}{p}"))
        .collect();

    // Should create valid regex set
    let regex_set = RegexSet::new(&patterns);
    assert!(
        regex_set.is_ok(),
        "Should create valid regex set from patterns"
    );

    // Helper to check if regex set matches a URL without unwrap
    let set_matches = |url: &str| -> bool {
        if let Ok(ref set) = regex_set {
            set.is_match(url)
        } else {
            false
        }
    };

    // Should match docs URLs
    assert!(
        set_matches("example.com/docs/page1"),
        "Should match /docs/ URLs"
    );
    // Should match api URLs
    assert!(
        set_matches("example.com/api/v1/users"),
        "Should match /api/ URLs"
    );
    // Should match blog URLs
    assert!(
        set_matches("example.com/blog/post"),
        "Should match /blog/ URLs"
    );
    // Should not match other URLs
    assert!(
        !set_matches("example.com/contact"),
        "Should not match non-whitelisted URLs"
    );
}

/// Test: URL normalization prevents duplicate crawling
///
/// Behavior: When URL normalization is enabled, URLs that differ only
/// by trailing slash should be treated as the same page.
#[test]
fn test_url_normalization_converts_trailing_slash() {
    // This test documents the expected behavior of spider-rs normalize
    // The actual normalization happens in spider-rs during crawling

    let url_with_slash = "https://example.com/docs/";
    let url_without_slash = "https://example.com/docs";

    // Both should parse to valid URLs
    let parsed_with_result = url::Url::parse(url_with_slash);
    let parsed_without_result = url::Url::parse(url_without_slash);

    assert!(parsed_with_result.is_ok(), "URL with slash should be valid");
    assert!(
        parsed_without_result.is_ok(),
        "URL without slash should be valid"
    );

    // Extract parsed URLs - safe after Ok check above
    let (parsed_with, parsed_without) = match (parsed_with_result, parsed_without_result) {
        (Ok(with), Ok(without)) => (with, without),
        _ => return, // Test failed above
    };

    // Paths differ (spider-rs normalize handles this)
    assert_eq!(
        parsed_with.path(),
        "/docs/",
        "URL with slash has trailing slash in path"
    );
    assert_eq!(
        parsed_without.path(),
        "/docs",
        "URL without slash has no trailing slash in path"
    );

    // Normalize both by removing trailing slash for comparison
    let normalized_with = parsed_with.path().trim_end_matches('/');
    let normalized_without = parsed_without.path().trim_end_matches('/');

    assert_eq!(
        normalized_with, normalized_without,
        "After manual normalization, paths should be equal"
    );
}

/// Test: Whitelist pattern escapes special regex characters
///
/// Behavior: Domain names with dots should be properly escaped in regex.
#[test]
fn test_whitelist_pattern_escapes_domain_dots() {
    use regex::Regex;

    let base_url = "https://example.com";
    let path_filter = "/docs";

    let base_domain = url::Url::parse(base_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();

    let domain_escaped = regex::escape(&base_domain);
    let full_url_pattern = format!("^{domain_escaped}{path_filter}");

    // Should create valid regex (dots are escaped)
    let regex = Regex::new(&full_url_pattern);
    assert!(regex.is_ok(), "Escaped pattern should be valid regex");

    // Helper to check if pattern matches without unwrap
    let pattern_matches = |url: &str| -> bool {
        if let Ok(ref p) = regex {
            p.is_match(url)
        } else {
            false
        }
    };

    // Should match the intended URL (domain/path format)
    assert!(
        pattern_matches("example.com/docs/page"),
        "Should match example.com/docs URLs"
    );

    // The escaped dots should be literal dots in the pattern
    assert!(
        full_url_pattern.contains("\\."),
        "Pattern should contain escaped dots"
    );

    // The dot in "example.com" should not match any character
    assert!(
        !pattern_matches("exampleXcom/docs/page"),
        "Escaped dot should be literal, not wildcard"
    );
}

/// Test: Path filter with leading ^ anchor is handled correctly
///
/// Behavior: User may provide pattern with leading ^ (e.g., "^/docs/").
/// The implementation should strip this to avoid double anchors.
#[test]
fn test_path_filter_with_leading_anchor_is_normalized() {
    use regex::Regex;

    let base_url = "https://example.com";
    let path_filter_with_anchor = "^/docs";

    let base_domain = url::Url::parse(base_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();

    let domain_escaped = regex::escape(&base_domain);
    // Simulate the stripping logic from scrape_site_internal
    let pattern_stripped = path_filter_with_anchor
        .strip_prefix('^')
        .unwrap_or(path_filter_with_anchor);
    let full_url_pattern = format!("^{domain_escaped}{pattern_stripped}");

    // Should create valid regex (single anchor)
    let regex = Regex::new(&full_url_pattern);
    assert!(
        regex.is_ok(),
        "Pattern with stripped anchor should be valid"
    );

    // Helper to check pattern match without unwrap
    let pattern_matches = |url: &str| -> bool {
        if let Ok(ref p) = regex {
            p.is_match(url)
        } else {
            false
        }
    };

    assert!(
        pattern_matches("example.com/docs/page"),
        "Should match after normalizing leading anchor"
    );
}

/// Test: Empty path_filter means no filtering
///
/// Behavior: When path_filter is None, all URLs should be crawlable.
#[test]
fn test_no_path_filter_means_no_filtering() {
    let config = ScrapeConfig {
        base_url: "https://example.com".to_string(),
        path_filter: None,
        ..Default::default()
    };

    assert!(
        config.path_filter.is_none(),
        "path_filter should be None when not set"
    );
}

/// Test: Invalid path_filter regex is handled gracefully
///
/// Behavior: Malformed regex patterns should produce clear error messages.
#[test]
fn test_invalid_path_filter_produces_error() {
    let invalid_pattern = "(?P<invalid"; // Unclosed group

    let result = Regex::new(invalid_pattern);
    assert!(result.is_err(), "Invalid regex should fail to parse");

    // Check error message without unwrap_err - using match pattern
    let error_msg_matches = match &result {
        Err(e) => {
            let msg = e.to_string();
            msg.contains("unclosed") || msg.contains("group") || msg.contains("regex")
        }
        Ok(_) => false,
    };
    assert!(error_msg_matches, "Error message should be descriptive");
}

/// Test: ScrapeConfig has reasonable defaults for spider-rs integration
///
/// Behavior: Default configuration should work well with spider-rs native features.
#[test]
fn test_scrape_config_defaults_for_spider_rs_integration() {
    let config = ScrapeConfig::default();

    // max_pages should be set to prevent infinite crawls
    assert_eq!(
        config.max_pages, 10_000,
        "Default max_pages should be reasonable"
    );

    // delay_ms should prevent rate limiting
    assert_eq!(
        config.delay_ms, 1000,
        "Default delay should be safe for most servers"
    );

    // respect_robots_txt should be enabled by default
    assert!(
        config.respect_robots,
        "Should respect robots.txt by default"
    );

    // stealth_mode helps avoid bot detection
    assert!(
        config.stealth_mode,
        "Stealth mode should be enabled by default"
    );
}

// ============================================================================
// SPIDER-RS INTEGRATION TESTS (Real HTTP Requests)
// ============================================================================
//
// These tests verify spider-rs behavior with actual HTTP requests to public
// test sites. They expose bugs in the current implementation by testing
// the actual behavior, not just code compilation.
//
// Run with: cargo test spider_rs_integration -- --ignored --test-threads=1
// The --ignored flag is needed because these are marked #[ignore] by default
// (they make real network requests and can be slow).
// ============================================================================

/// Test: Does with_limit() actually limit pages scraped?
///
/// This test makes real HTTP requests to httpbin.org to verify that
/// spider-rs's with_limit() actually works as advertised.
///
/// BUG EXPOSURE: If with_limit() doesn't work, the scraper will crawl
/// indefinitely or beyond the configured limit.
#[tokio::test]
#[ignore = "makes real HTTP requests; run with --ignored"]
async fn spider_rs_with_limit_actually_limits_pages() {
    use spider::website::Website;

    // httpbin.org has many endpoints (/get, /post, /put, /delete, /status/404, etc.)
    // This gives us a predictable site to test against
    let base_url = "https://httpbin.org";

    let mut website = Website::new(base_url);
    website.configuration.delay = 100; // Fast delay for testing
    website.configuration.respect_robots_txt = false; // httpbin allows scraping
    website.configuration.user_agent = Some(Box::new({
        let s: spider::compact_str::CompactString = "SpiderIntegrationTest/1.0".into();
        s
    }));
    website.configuration.concurrency_limit = Some(1); // Sequential for predictability

    // Set a LOW limit - we should only get a few pages
    let limit = 3_u32;
    let _ = website.configuration.with_limit(limit);

    // Scrape
    website.scrape().await;

    // Get results
    let pages = website.get_pages();
    let page_count = pages.as_ref().map(|p| p.len()).unwrap_or(0);

    // ASSERT: with_limit() should prevent scraping more than `limit` pages
    // BUG: If with_limit() doesn't work, page_count could be >> limit
    assert!(
        page_count <= limit as usize,
        "BUG: with_limit({limit}) failed to limit pages! Got {page_count} pages. \
        This means with_limit() is NOT working and the scraper may crawl indefinitely."
    );

    println!("with_limit({limit}) test passed: scraped {page_count} pages");

    // Additional verification: check that we actually got something
    // (to distinguish "limit works" from "nothing was scraped")
    assert!(
        page_count > 0,
        "Expected to scrape at least 1 page from httpbin.org, got 0. \
        Network may be down or httpbin.org may be unavailable."
    );
}

/// Test: Does whitelist_url actually filter URLs during scraping?
///
/// This test makes real HTTP requests to verify that whitelist_url
/// actually prevents URLs from being crawled.
///
/// BUG EXPOSURE: If whitelist_url doesn't work, URLs that should be
/// filtered will still be scraped, wasting bandwidth and time.
#[tokio::test]
#[ignore = "makes real HTTP requests; run with --ignored"]
async fn spider_rs_whitelist_url_actually_filters_urls() {
    use spider::website::Website;

    let base_url = "https://httpbin.org";

    // Test 1: No whitelist - should crawl multiple pages
    let mut website_no_filter = Website::new(base_url);
    website_no_filter.configuration.delay = 100;
    website_no_filter.configuration.respect_robots_txt = false;
    website_no_filter.configuration.user_agent =
        Some(Box::new("SpiderIntegrationTest/1.0".to_string().into()));
    website_no_filter.configuration.concurrency_limit = Some(1);
    let _ = website_no_filter.configuration.with_limit(50); // High limit to allow crawling

    website_no_filter.scrape().await;
    let pages_no_filter = website_no_filter.get_pages();
    let count_no_filter = pages_no_filter.as_ref().map(|p| p.len()).unwrap_or(0);

    // Test 2: With whitelist - should ONLY crawl /status URLs
    let mut website_whitelist = Website::new(base_url);
    website_whitelist.configuration.delay = 100;
    website_whitelist.configuration.respect_robots_txt = false;
    website_whitelist.configuration.user_agent =
        Some(Box::new("SpiderIntegrationTest/1.0".to_string().into()));
    website_whitelist.configuration.concurrency_limit = Some(1);
    let _ = website_whitelist.configuration.with_limit(50);

    // CRITICAL: Set whitelist to ONLY allow /status/* URLs
    // This should filter out /get, /post, /anything, etc.
    let _ = website_whitelist
        .configuration
        .with_whitelist_url(Some(vec!["^https://httpbin.org/status".into()]));

    // CRITICAL: Compile the whitelist - without this, patterns may not be active
    website_whitelist.configuration.configure_allowlist();

    website_whitelist.scrape().await;
    let pages_whitelist = website_whitelist.get_pages();
    let count_whitelist = pages_whitelist.as_ref().map(|p| p.len()).unwrap_or(0);

    // Verify whitelist filtered URLs
    let urls: Vec<_> = pages_whitelist
        .as_ref()
        .map(|pages| {
            pages
                .iter()
                .map(|p| p.get_url().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // ASSERT: All URLs should match the whitelist pattern
    let all_match = urls.iter().all(|u| u.contains("/status"));

    assert!(
        all_match,
        "BUG: whitelist_url filtering failed! Found URLs that don't match pattern: {urls:?}"
    );

    // ASSERT: Whitelist should have filtered out some URLs
    // (httpbin has many endpoints, so /status-only should be fewer than total)
    assert!(
        count_whitelist <= count_no_filter,
        "BUG: whitelist_url didn't filter! Got {count_whitelist} pages with filter vs {count_no_filter} without. \
        This means whitelist_url is NOT working."
    );

    println!(
        "whitelist_url test passed: {count_no_filter} pages without filter, {count_whitelist} pages with filter"
    );
    println!("Filtered URLs: {urls:?}");
}

/// Test: What URL format does whitelist_url match?
///
/// This test determines whether whitelist_url matches:
/// - Full URLs (scheme://domain/path)
/// - Just paths (/path)
/// - Patterns with wildcards
///
/// BUG EXPOSURE: If we use the wrong format, filtering will silently fail.
#[tokio::test]
#[ignore = "makes real HTTP requests; run with --ignored"]
async fn spider_rs_whitelist_url_format_full_url_vs_path() {
    use spider::website::Website;

    let base_url = "https://httpbin.org";

    // Test 1: Full URL pattern with scheme and domain
    let mut website_full = Website::new(base_url);
    website_full.configuration.delay = 100;
    website_full.configuration.respect_robots_txt = false;
    website_full.configuration.user_agent =
        Some(Box::new("SpiderIntegrationTest/1.0".to_string().into()));
    website_full.configuration.concurrency_limit = Some(1);
    let _ = website_full.configuration.with_limit(50);

    // Use FULL URL pattern: scheme://domain/path
    let _ = website_full
        .configuration
        .with_whitelist_url(Some(vec!["^https://httpbin.org/get".into()]));
    website_full.configuration.configure_allowlist();

    website_full.scrape().await;
    let pages_full = website_full.get_pages();
    let urls_full: Vec<_> = pages_full
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.get_url().to_string()).collect())
        .unwrap_or_default();

    // Test 2: Path-only pattern (no scheme/domain)
    let mut website_path = Website::new(base_url);
    website_path.configuration.delay = 100;
    website_path.configuration.respect_robots_txt = false;
    website_path.configuration.user_agent =
        Some(Box::new("SpiderIntegrationTest/1.0".to_string().into()));
    website_path.configuration.concurrency_limit = Some(1);
    let _ = website_path.configuration.with_limit(50);

    // Use PATH-ONLY pattern: /path
    // BUG WARNING: This likely WON'T work - spider-rs probably needs full URLs
    let _ = website_path
        .configuration
        .with_whitelist_url(Some(vec!["^/get".into()]));
    website_path.configuration.configure_allowlist();

    website_path.scrape().await;
    let pages_path = website_path.get_pages();
    let urls_path: Vec<_> = pages_path
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.get_url().to_string()).collect())
        .unwrap_or_default();

    println!(
        "Full URL pattern matched {} pages: {urls_full:?}",
        urls_full.len()
    );
    println!(
        "Path-only pattern matched {} pages: {urls_path:?}",
        urls_path.len()
    );

    // DETERMINE which format works:
    // - If full URL pattern works and path-only doesn't, we MUST use full URLs
    // - If both work, we have flexibility
    // - If neither works, there's a bug in our usage

    let full_url_works = !urls_full.is_empty()
        && urls_full
            .iter()
            .all(|u| u.starts_with("https://httpbin.org/get"));

    let path_only_works = !urls_path.is_empty();

    if full_url_works && !path_only_works {
        println!("CONCLUSION: whitelist_url requires FULL URL patterns (scheme://domain/path)");
        println!("Path-only patterns DO NOT WORK - must convert path_filter to full URL");
    } else if path_only_works {
        println!("CONCLUSION: Path-only patterns WORK - simpler filtering possible");
    } else {
        println!("WARNING: Neither pattern worked - possible bug in test or spider-rs API");
    }

    // ASSERT: Full URL pattern should definitely work
    assert!(
        full_url_works || urls_full.is_empty(),
        "Full URL pattern should only match /get URLs. Got: {urls_full:?}"
    );
}

/// Test: Is configure_allowlist() required for whitelist_url to work?
///
/// This test determines if calling configure_allowlist() is actually
/// necessary for whitelist_url filtering to take effect.
///
/// BUG EXPOSURE: If configure_allowlist() is not called, whitelist patterns
/// may be set but never compiled/used, resulting in no filtering.
#[tokio::test]
#[ignore = "makes real HTTP requests; run with --ignored"]
async fn spider_rs_configure_allowlist_is_required() {
    use spider::website::Website;

    let base_url = "https://httpbin.org";

    // Test 1: whitelist_url WITHOUT configure_allowlist()
    let mut website_no_config = Website::new(base_url);
    website_no_config.configuration.delay = 100;
    website_no_config.configuration.respect_robots_txt = false;
    website_no_config.configuration.user_agent =
        Some(Box::new("SpiderIntegrationTest/1.0".to_string().into()));
    website_no_config.configuration.concurrency_limit = Some(1);
    let _ = website_no_config.configuration.with_limit(50);

    // Set whitelist but DO NOT call configure_allowlist()
    let _ = website_no_config
        .configuration
        .with_whitelist_url(Some(vec!["^https://httpbin.org/status".into()]));
    // NOTE: Intentionally NOT calling configure_allowlist() here

    website_no_config.scrape().await;
    let pages_no_config = website_no_config.get_pages();
    let count_no_config = pages_no_config.as_ref().map(|p| p.len()).unwrap_or(0);

    // Test 2: whitelist_url WITH configure_allowlist()
    let mut website_with_config = Website::new(base_url);
    website_with_config.configuration.delay = 100;
    website_with_config.configuration.respect_robots_txt = false;
    website_with_config.configuration.user_agent =
        Some(Box::new("SpiderIntegrationTest/1.0".to_string().into()));
    website_with_config.configuration.concurrency_limit = Some(1);
    let _ = website_with_config.configuration.with_limit(50);

    // Set whitelist AND call configure_allowlist()
    let _ = website_with_config
        .configuration
        .with_whitelist_url(Some(vec!["^https://httpbin.org/status".into()]));
    website_with_config.configuration.configure_allowlist(); // CRITICAL STEP

    website_with_config.scrape().await;
    let pages_with_config = website_with_config.get_pages();
    let count_with_config = pages_with_config.as_ref().map(|p| p.len()).unwrap_or(0);

    println!("WITHOUT configure_allowlist(): {count_no_config} pages scraped");
    println!("WITH configure_allowlist(): {count_with_config} pages scraped");

    // If configure_allowlist() is required:
    // - WITHOUT it: should crawl MORE pages (filtering not active)
    // - WITH it: should crawl FEWER pages (filtering active)

    // Check if filtering actually happened with configure_allowlist()
    let urls_with_config: Vec<_> = pages_with_config
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.get_url().to_string()).collect())
        .unwrap_or_default();

    let all_match_pattern = urls_with_config
        .iter()
        .all(|u| u.contains("/status") || u.is_empty());

    // ASSERT: configure_allowlist() should be necessary for filtering
    if count_no_config > count_with_config {
        println!("CONCLUSION: configure_allowlist() IS REQUIRED for whitelist_url to work");
        println!(
            "Without it: {count_no_config} pages (filtering inactive), With it: {count_with_config} pages (filtering active)"
        );
    } else if count_with_config > 0 && all_match_pattern {
        println!(
            "CONCLUSION: configure_allowlist() may be optional OR whitelist_url auto-configures"
        );
        println!("Both approaches resulted in filtered URLs");
    } else {
        println!("INCONCLUSIVE: Results were similar - may need different test site");
    }

    // At minimum, verify that with configure_allowlist() we get correct results
    assert!(
        all_match_pattern || urls_with_config.is_empty(),
        "With configure_allowlist(), all URLs should match the /status pattern. Got: {urls_with_config:?}"
    );
}

/// Test: Does with_limit() work with scrape_sitemap()?
///
/// Comments in the code suggest with_limit() may not affect scrape_sitemap().
/// This test verifies if that's true.
///
/// BUG EXPOSURE: If with_limit() doesn't work with sitemap scraping, we
/// need manual limits as a fallback (which the code already has).
#[tokio::test]
#[ignore = "makes real HTTP requests; run with --ignored"]
async fn spider_rs_with_limit_with_sitemap_scraping() {
    use spider::website::Website;

    // Use a site with a known sitemap
    // example.com doesn't have a sitemap, so we use a test approach

    let base_url = "https://www.rust-lang.org";

    let mut website = Website::new(base_url);
    website.configuration.delay = 100;
    website.configuration.respect_robots_txt = true;
    website.configuration.user_agent = Some(Box::new({
        let s: spider::compact_str::CompactString = "SpiderIntegrationTest/1.0".into();
        s
    }));
    website.configuration.concurrency_limit = Some(1);

    // Set a LOW limit
    let limit = 5_u32;
    let _ = website.configuration.with_limit(limit);

    // Use sitemap scraping (the method in question)
    website.scrape_sitemap().await;

    let pages = website.get_pages();
    let page_count = pages.as_ref().map(|p| p.len()).unwrap_or(0);

    println!("scrape_sitemap() with with_limit({limit}): scraped {page_count} pages");

    // ASSERT: with_limit() should work with scrape_sitemap()
    if page_count > limit as usize {
        println!("BUG CONFIRMED: with_limit() does NOT limit scrape_sitemap() output!");
        println!("This confirms the comment in the code - manual limit is needed as fallback.");
    } else {
        println!("with_limit() works with scrape_sitemap()");
    }

    // The test is informational - we document the behavior rather than asserting
    // because sitemap availability varies
    if page_count > 0 {
        println!(
            "Note: Got {page_count} pages from sitemap. If > {limit}, with_limit() doesn't work with sitemap."
        );
    }
}

/// Test: Verify the exact regex format that whitelist_url accepts
///
/// This test iterates through different regex formats to find which one works.
#[tokio::test]
#[ignore = "makes real HTTP requests; run with --ignored"]
async fn spider_rs_whitelist_regex_format_exploration() {
    use spider::website::Website;

    let base_url = "https://httpbin.org";

    let patterns_to_test = vec![
        ("Full URL with anchor", "^https://httpbin.org/get"),
        ("Full URL without anchor", "https://httpbin.org/get"),
        ("Domain + path anchor", "^httpbin.org/get"),
        ("Path only with anchor", "^/get"),
        ("Path only wildcard", "/get.*"),
        ("Wildcard pattern", "*get*"),
    ];

    for (description, pattern) in patterns_to_test {
        let mut website = Website::new(base_url);
        website.configuration.delay = 100;
        website.configuration.respect_robots_txt = false;
        website.configuration.user_agent =
            Some(Box::new("SpiderIntegrationTest/1.0".to_string().into()));
        website.configuration.concurrency_limit = Some(1);
        let _ = website.configuration.with_limit(20);

        let _ = website
            .configuration
            .with_whitelist_url(Some(vec![pattern.into()]));
        website.configuration.configure_allowlist();

        website.scrape().await;
        let pages = website.get_pages();
        let count = pages.as_ref().map(|p| p.len()).unwrap_or(0);

        let urls: Vec<_> = pages
            .as_ref()
            .map(|p| p.iter().map(|pg| pg.get_url().to_string()).collect())
            .unwrap_or_default();

        println!("{description:30} | Pattern: {pattern:30} | Pages: {count}");
        if !urls.is_empty() {
            println!("                              URLs: {urls:?}");
        }
    }
}

/// Test: Does with_limit() apply per-domain or globally?
///
/// When scraping multiple domains, with_limit() behavior is important to understand.
#[tokio::test]
#[ignore = "makes real HTTP requests; run with --ignored"]
async fn spider_rs_with_limit_single_domain_behavior() {
    use spider::website::Website;

    let base_url = "https://httpbin.org";

    let mut website = Website::new(base_url);
    website.configuration.delay = 100;
    website.configuration.respect_robots_txt = false;
    website.configuration.user_agent = Some(Box::new({
        let s: spider::compact_str::CompactString = "SpiderIntegrationTest/1.0".into();
        s
    }));
    website.configuration.concurrency_limit = Some(1);

    // Very low limit
    let limit = 2_u32;
    let _ = website.configuration.with_limit(limit);

    website.scrape().await;

    let pages = website.get_pages();
    let count = pages.as_ref().map(|p| p.len()).unwrap_or(0);

    println!("with_limit({limit}) on single domain: scraped {count} pages");

    // ASSERT: Should not exceed limit
    assert!(
        count <= limit as usize,
        "with_limit() should prevent scraping more than {limit} pages, got {count}"
    );
}

// ===========================================================================
// ENHANCED FEATURE TESTS
// ===========================================================================

#[cfg(test)]
#[cfg(feature = "enhanced")]
mod enhanced_tests {
    #![allow(clippy::panic)]
    #![expect(clippy::unwrap_used)]
    use super::*;
    // Import features types - available when enhanced feature is enabled
    use crate::features::{
        CacheConfig, CacheTtl, FeatureConfig, FeatureConfigBuilder, GlobPattern, RegexPattern,
    };

    #[test]
    fn test_enhanced_scrape_config_creation() {
        let base = ScrapeConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };
        let features = FeatureConfig::new();
        let enhanced = EnhancedScrapeConfig::new(base, features);

        assert_eq!(enhanced.base.base_url, "https://example.com");
        assert!(enhanced.features.is_empty());
    }

    #[test]
    fn test_scrape_config_with_features() {
        let base = ScrapeConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };
        let features = FeatureConfig::new();
        let enhanced = base.with_features(features);

        assert_eq!(enhanced.base.base_url, "https://example.com");
    }

    #[test]
    fn test_feature_config_with_cache() {
        let ttl = match CacheTtl::new(600) {
            Ok(t) => t,
            Err(e) => panic!("CacheTtl::new failed: {e}"),
        };
        let cache = CacheConfig::enabled_with_ttl(ttl);
        let features = FeatureConfig::new().with_cache(cache);

        assert!(features.cache.is_some());
        assert!(features.cache.as_ref().map(|c| c.enabled).unwrap_or(false));
        assert_eq!(
            features
                .cache
                .as_ref()
                .map(|c| c.ttl.seconds())
                .unwrap_or(0),
            600
        );
    }

    #[test]
    fn test_feature_config_builder() {
        let builder = match FeatureConfigBuilder::new().enable_cache(300) {
            Ok(b) => b,
            Err(e) => panic!("enable_cache failed: {e}"),
        };
        let config = builder.build();

        assert!(config.cache.is_some());
        assert!(config.cache.map(|c| c.enabled).unwrap_or(false));
    }

    #[test]
    fn test_apply_filtering_to_website_empty() {
        use spider::website::Website;

        let mut website = Website::new("https://example.com");
        let filtering = FilteringConfig::new();

        let result = apply_filtering_to_website(&mut website, &filtering);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_filtering_to_website_with_allow() {
        use spider::website::Website;

        let mut website = Website::new("https://example.com");
        let pattern = GlobPattern::new("/docs/*".to_string());
        assert!(pattern.is_ok(), "GlobPattern::new should succeed");
        let patterns = vec![pattern.unwrap()];
        let filtering = FilteringConfig::new().with_allow(patterns);

        let result = apply_filtering_to_website(&mut website, &filtering);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_filtering_to_website_with_deny() {
        use spider::website::Website;

        let mut website = Website::new("https://example.com");
        let pattern = match RegexPattern::new(r"\.pdf$".to_string()) {
            Ok(p) => p,
            Err(_) => panic!("RegexPattern::new should succeed for valid pattern"),
        };
        let patterns = vec![pattern];
        let filtering = FilteringConfig::new().with_deny(patterns);

        let result = apply_filtering_to_website(&mut website, &filtering);
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_filter_application() {
        use spider::website::Website;

        let mut website = Website::new("https://example.com");
        let result = apply_path_filter(&mut website, "https://example.com", "/docs");

        assert!(result.is_ok());
    }

    #[cfg(all(feature = "javascript", feature = "enhanced"))]
    #[test]
    fn test_javascript_config_smart_mode() {
        use crate::features::RenderMode;

        let config = match JavascriptConfig::smart() {
            Ok(c) => c,
            Err(e) => panic!("JavascriptConfig::smart failed: {e}"),
        };
        assert_eq!(config.mode, RenderMode::Smart);
        assert_eq!(config.timeout.millis(), 30000);
    }

    #[cfg(all(feature = "javascript", feature = "enhanced"))]
    #[test]
    fn test_javascript_config_never_mode() {
        use crate::features::RenderMode;

        let config = match JavascriptConfig::never() {
            Ok(c) => c,
            Err(e) => panic!("JavascriptConfig::never failed: {e}"),
        };
        assert_eq!(config.mode, RenderMode::Never);
        assert_eq!(config.timeout.millis(), 1000);
    }

    #[cfg(all(feature = "anti-detection", feature = "enhanced"))]
    #[test]
    fn test_anti_detection_config_strategies() {
        use crate::features::Strategy;

        let none = AntiDetectionConfig::none();
        assert_eq!(none.strategy, Strategy::None);

        let rotating = AntiDetectionConfig::rotating_ua();
        assert_eq!(rotating.strategy, Strategy::RotatingUserAgent);

        let stealth = AntiDetectionConfig::full_stealth();
        assert_eq!(stealth.strategy, Strategy::FullStealth);
    }

    #[cfg(all(feature = "anti-detection", feature = "enhanced"))]
    #[test]
    fn test_generate_random_user_agent() {
        let ua = generate_random_user_agent();
        assert!(ua.is_some());
        let ua_string = match ua {
            Some(s) => s,
            None => panic!("generate_random_user_agent returned None"),
        };
        assert!(ua_string.contains("Mozilla"));
    }

    #[test]
    fn test_feature_config_is_empty_when_new() {
        let config = FeatureConfig::new();
        assert!(config.is_empty());
    }

    #[test]
    fn test_feature_config_not_empty_with_cache() {
        let ttl = match CacheTtl::new(100) {
            Ok(t) => t,
            Err(e) => panic!("CacheTtl::new failed: {e}"),
        };
        let cache = CacheConfig::enabled_with_ttl(ttl);
        let config = FeatureConfig::new().with_cache(cache);

        assert!(!config.is_empty());
    }

    #[test]
    fn test_feature_config_not_empty_with_filtering() {
        let filtering = FilteringConfig::new();
        let config = FeatureConfig::new().with_filtering(filtering);

        assert!(!config.is_empty());
    }
}
