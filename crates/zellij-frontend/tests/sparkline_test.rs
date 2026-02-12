//! Tests for sparkline rendering functionality

use zellij_frontend::sparkline::{
    SparklineBuilder, SparklineConfig, SparklineError, render_sparkline,
};

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
    assert_eq!(output.chars().count(), 4);
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
    let result: Result<String, SparklineError> = render_sparkline(&[], 5);
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
fn test_render_sparkline_pads_or_truncates() {
    let result = render_sparkline(&[50], 10);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.chars().count(), 10);
}

#[test]
fn test_render_sparkline_single_value() {
    let result = render_sparkline(&[50], 1);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.chars().count(), 1);
}

#[test]
fn test_render_sparkline_with_config() {
    let config = SparklineConfig {
        width: 5,
        min_char: '░',
        max_char: '█',
    };
    let result = SparklineBuilder::new(&[0, 25, 50, 75, 100])
        .with_config(&config)
        .build();
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.chars().count(), 5);
}

#[test]
fn test_render_sparkline_large_array() {
    let data: Vec<u8> = (0..100).collect();
    let result = render_sparkline(&data, 20);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.chars().count(), 20);
}

#[test]
fn test_render_sparkline_all_same_middle() {
    let data = [50_u8; 8];
    let result = render_sparkline(&data, 8);
    assert!(result.is_ok());
    let output = result.unwrap();
    let chars: Vec<char> = output.chars().collect();
    assert!(chars.iter().all(|&c| c == chars[0]));
}

#[test]
fn test_render_sparkline_produces_valid_utf8() {
    let result = render_sparkline(&[0, 25, 50, 75, 100], 5);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.is_char_boundary(0));
    assert!(output.is_char_boundary(output.len()));
}

#[test]
fn test_render_sparkline_all_characters_valid() {
    let result = render_sparkline(&[0, 14, 28, 42, 57, 71, 85, 100], 8);
    assert!(result.is_ok());
    let output = result.unwrap();
    let valid_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    for c in output.chars() {
        assert!(valid_chars.contains(&c), "Invalid sparkline char: {c}");
    }
}
