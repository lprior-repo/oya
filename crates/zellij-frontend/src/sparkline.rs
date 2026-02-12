//! Sparkline rendering module for terminal UI
//!
//! Provides ASCII sparkline charts using Unicode block characters.
//! Sparklines are small, inline charts that display data trends.
//!
//! # Example
//!
//! ```
//! use zellij_frontend::sparkline::{render_sparkline, SparklineConfig};
//!
//! let result = render_sparkline(&[0, 25, 50, 75, 100], 5).unwrap();
//! // Produces something like: ▁▂▄▆█
//! ```

use thiserror::Error;

/// Unicode block characters for sparkline rendering (8 levels)
const SPARKLINE_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Errors that can occur during sparkline rendering
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SparklineError {
    /// Width parameter is zero
    #[error("Sparkline width must be greater than 0")]
    ZeroWidth,

    /// Invalid character configuration
    #[error("Invalid sparkline character configuration")]
    InvalidConfig,
}

/// Configuration for sparkline rendering
#[derive(Debug, Clone, PartialEq)]
pub struct SparklineConfig {
    /// Width of the output sparkline (number of characters)
    pub width: usize,
    /// Character to use for minimum value
    pub min_char: char,
    /// Character to use for maximum value
    pub max_char: char,
}

impl Default for SparklineConfig {
    fn default() -> Self {
        Self {
            width: 10,
            min_char: '▁',
            max_char: '█',
        }
    }
}

/// Render a sparkline from an array of u8 values.
///
/// Normalizes values relative to the maximum in the data set.
/// Uses Unicode block characters ▁▂▃▄▅▆▇█ for visualization.
///
/// # Arguments
///
/// * `data` - Array of values (0-255, normalized relative to max)
/// * `width` - Desired output width (number of characters)
///
/// # Returns
///
/// String containing sparkline characters, or error if width is 0.
///
/// # Errors
///
/// Returns `SparklineError::ZeroWidth` if width is 0.
///
/// # Example
///
/// ```
/// use zellij_frontend::sparkline::render_sparkline;
///
/// let result = render_sparkline(&[0, 50, 100], 3).unwrap();
/// assert_eq!(result.chars().count(), 3);
/// ```
pub fn render_sparkline(data: &[u8], width: usize) -> Result<String, SparklineError> {
    if width == 0 {
        return Err(SparklineError::ZeroWidth);
    }

    if data.is_empty() {
        return Ok(String::new());
    }

    let config = SparklineConfig {
        width,
        ..SparklineConfig::default()
    };
    render_sparkline_with_config(data, &config)
}

/// Builder for sparkline rendering with custom configuration.
pub struct SparklineBuilder<'a> {
    data: &'a [u8],
    config: SparklineConfig,
}

impl<'a> SparklineBuilder<'a> {
    /// Create a new builder with data.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            config: SparklineConfig::default(),
        }
    }

    /// Set custom width.
    #[must_use]
    pub fn width(mut self, width: usize) -> Self {
        self.config.width = width;
        self
    }

    /// Set custom configuration.
    #[must_use]
    pub fn with_config(mut self, config: &SparklineConfig) -> Self {
        self.config = config.clone();
        self
    }

    /// Build the sparkline string.
    ///
    /// # Errors
    ///
    /// Returns `SparklineError::ZeroWidth` if width is 0.
    pub fn build(self) -> Result<String, SparklineError> {
        render_sparkline_with_config(self.data, &self.config)
    }
}

/// Render sparkline with custom configuration.
///
/// # Errors
///
/// Returns `SparklineError::ZeroWidth` if width is 0.
fn render_sparkline_with_config(
    data: &[u8],
    config: &SparklineConfig,
) -> Result<String, SparklineError> {
    if config.width == 0 {
        return Err(SparklineError::ZeroWidth);
    }

    if data.is_empty() {
        return Ok(String::new());
    }

    // Find max value for normalization (saturating at 1 to avoid div by 0)
    let max_val = data.iter().copied().max().unwrap_or(1).max(1);

    // Sample or pad data to match width
    let sampled = sample_data(data, config.width);

    // Convert each value to a sparkline character
    let result: String = sampled
        .iter()
        .map(|&val| value_to_sparkline_char(val, max_val))
        .collect();

    Ok(result)
}

/// Sample data to fit target width.
///
/// If data is shorter than width, pads with last value.
/// If data is longer than width, samples at regular intervals.
fn sample_data(data: &[u8], target_width: usize) -> Vec<u8> {
    if data.len() == target_width {
        return data.to_vec();
    }

    if data.len() > target_width {
        // Downsample: pick values at regular intervals
        let step = data.len() as f64 / target_width as f64;
        (0..target_width)
            .map(|i| {
                let idx = ((i as f64) * step) as usize;
                data.get(idx).copied().unwrap_or(0)
            })
            .collect()
    } else {
        // Pad: repeat last value
        let mut result = data.to_vec();
        let last_val = data.last().copied().unwrap_or(0);
        while result.len() < target_width {
            result.push(last_val);
        }
        result
    }
}

/// Convert a value to a sparkline character.
///
/// Normalizes value relative to max and maps to one of 8 block characters.
fn value_to_sparkline_char(value: u8, max_val: u8) -> char {
    if max_val == 0 {
        return SPARKLINE_CHARS[0];
    }

    // Normalize to 0-7 range
    let normalized = (value as u32 * 7) / max_val as u32;
    let index = normalized.min(7) as usize;

    SPARKLINE_CHARS[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_sparkline_basic() {
        let result = render_sparkline(&[0, 50, 100], 3);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.chars().count(), 3);
    }

    #[test]
    fn test_render_sparkline_ascending() {
        let result = render_sparkline(&[10, 20, 30, 40], 4);
        assert!(result.is_ok());
        let output = result.unwrap();
        let chars: Vec<char> = output.chars().collect();
        assert!(chars[0] <= chars[1]);
        assert!(chars[1] <= chars[2]);
        assert!(chars[2] <= chars[3]);
    }

    #[test]
    fn test_render_sparkline_all_max() {
        let data = [100_u8; 10];
        let result = render_sparkline(&data, 10);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.chars().all(|c| c == '█'));
    }

    #[test]
    fn test_render_sparkline_all_min() {
        let data = [0_u8; 5];
        let result = render_sparkline(&data, 5);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.chars().all(|c| c == '▁'));
    }

    #[test]
    fn test_render_sparkline_empty() {
        let result = render_sparkline(&[], 5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_render_sparkline_zero_width() {
        let result = render_sparkline(&[50, 60, 70], 0);
        assert!(matches!(result, Err(SparklineError::ZeroWidth)));
    }

    #[test]
    fn test_render_sparkline_clamps_high_values() {
        let result = render_sparkline(&[255, 255, 255], 3);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.chars().count(), 3);
    }

    #[test]
    fn test_render_sparkline_pads_data() {
        let result = render_sparkline(&[50], 10);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.chars().count(), 10);
    }

    #[test]
    fn test_sample_data_same_length() {
        let data = [1_u8, 2, 3];
        let sampled = sample_data(&data, 3);
        assert_eq!(sampled, vec![1, 2, 3]);
    }

    #[test]
    fn test_sample_data_downsample() {
        let data = [1_u8, 2, 3, 4, 5, 6];
        let sampled = sample_data(&data, 3);
        assert_eq!(sampled.len(), 3);
    }

    #[test]
    fn test_sample_data_pad() {
        let data = [1_u8, 2];
        let sampled = sample_data(&data, 5);
        assert_eq!(sampled, vec![1, 2, 2, 2, 2]);
    }

    #[test]
    fn test_value_to_sparkline_char_zero() {
        assert_eq!(value_to_sparkline_char(0, 100), '▁');
    }

    #[test]
    fn test_value_to_sparkline_char_max() {
        assert_eq!(value_to_sparkline_char(100, 100), '█');
    }

    #[test]
    fn test_value_to_sparkline_char_mid() {
        let c = value_to_sparkline_char(50, 100);
        assert!(c >= '▃' && c <= '▄');
    }

    #[test]
    fn test_value_to_sparkline_char_zero_max() {
        assert_eq!(value_to_sparkline_char(0, 0), '▁');
    }

    #[test]
    fn test_sparkline_builder() {
        let result = SparklineBuilder::new(&[0, 50, 100]).width(3).build();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().chars().count(), 3);
    }

    #[test]
    fn test_sparkline_config_default() {
        let config = SparklineConfig::default();
        assert_eq!(config.width, 10);
        assert_eq!(config.min_char, '▁');
        assert_eq!(config.max_char, '█');
    }
}
