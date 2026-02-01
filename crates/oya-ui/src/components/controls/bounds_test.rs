//! Tests for zoom level bounds validation

use super::bounds::ZoomLevel;

#[cfg(test)]
mod zoom_level_tests {
    use super::*;

    #[test]
    fn test_valid_zoom_values() {
        // Test minimum bound
        let zoom = ZoomLevel::new(0.1).expect("0.1 should be valid");
        assert_eq!(zoom.value(), 0.1);

        // Test maximum bound
        let zoom = ZoomLevel::new(5.0).expect("5.0 should be valid");
        assert_eq!(zoom.value(), 5.0);

        // Test middle value
        let zoom = ZoomLevel::new(1.0).expect("1.0 should be valid");
        assert_eq!(zoom.value(), 1.0);

        // Test arbitrary valid value
        let zoom = ZoomLevel::new(2.5).expect("2.5 should be valid");
        assert_eq!(zoom.value(), 2.5);
    }

    #[test]
    fn test_values_below_min_get_clamped() {
        let zoom = ZoomLevel::new(0.05).expect("Should clamp to min");
        assert_eq!(zoom.value(), 0.1);

        let zoom = ZoomLevel::new(-1.0).expect("Negative should clamp to min");
        assert_eq!(zoom.value(), 0.1);

        let zoom = ZoomLevel::new(0.0).expect("Zero should clamp to min");
        assert_eq!(zoom.value(), 0.1);
    }

    #[test]
    fn test_values_above_max_get_clamped() {
        let zoom = ZoomLevel::new(6.0).expect("Should clamp to max");
        assert_eq!(zoom.value(), 5.0);

        let zoom = ZoomLevel::new(100.0).expect("Large value should clamp to max");
        assert_eq!(zoom.value(), 5.0);

        let zoom = ZoomLevel::new(5.1).expect("Slightly over max should clamp");
        assert_eq!(zoom.value(), 5.0);
    }

    #[test]
    fn test_nan_handling() {
        let result = ZoomLevel::new(f32::NAN);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("finite"));
    }

    #[test]
    fn test_infinity_handling() {
        let result = ZoomLevel::new(f32::INFINITY);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("finite"));

        let result = ZoomLevel::new(f32::NEG_INFINITY);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("finite"));
    }

    #[test]
    fn test_zoom_increment() {
        let zoom = ZoomLevel::new(1.0).expect("Valid initial zoom");
        let zoomed = zoom.increment(0.5).expect("Valid increment");
        assert_eq!(zoomed.value(), 1.5);

        // Test increment beyond max gets clamped
        let zoom = ZoomLevel::new(4.8).expect("Valid initial zoom");
        let zoomed = zoom.increment(0.5).expect("Increment clamped to max");
        assert_eq!(zoomed.value(), 5.0);
    }

    #[test]
    fn test_zoom_decrement() {
        let zoom = ZoomLevel::new(2.0).expect("Valid initial zoom");
        let zoomed = zoom.decrement(0.5).expect("Valid decrement");
        assert_eq!(zoomed.value(), 1.5);

        // Test decrement below min gets clamped
        let zoom = ZoomLevel::new(0.3).expect("Valid initial zoom");
        let zoomed = zoom.decrement(0.5).expect("Decrement clamped to min");
        assert_eq!(zoomed.value(), 0.1);
    }

    #[test]
    fn test_default_zoom() {
        let zoom = ZoomLevel::default();
        assert_eq!(zoom.value(), 1.0);
    }

    #[test]
    fn test_min_max_constructors() {
        let min_zoom = ZoomLevel::min();
        assert_eq!(min_zoom.value(), 0.1);

        let max_zoom = ZoomLevel::max();
        assert_eq!(max_zoom.value(), 5.0);
    }

    #[test]
    fn test_extreme_values() {
        // Test f32::MAX gets clamped to max
        let zoom = ZoomLevel::new(f32::MAX).expect("Should clamp to max");
        assert_eq!(zoom.value(), 5.0);

        // Test f32::MIN gets clamped to min (f32::MIN is negative)
        let zoom = ZoomLevel::new(f32::MIN).expect("Should clamp to min");
        assert_eq!(zoom.value(), 0.1);
    }

    #[test]
    fn test_increment_with_nan_delta() {
        let zoom = ZoomLevel::new(1.0).expect("Valid initial zoom");
        let result = zoom.increment(f32::NAN);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrement_with_nan_delta() {
        let zoom = ZoomLevel::new(1.0).expect("Valid initial zoom");
        let result = zoom.decrement(f32::NAN);
        assert!(result.is_err());
    }

    #[test]
    fn test_zoom_level_copy_semantics() {
        let zoom1 = ZoomLevel::new(2.0).expect("Valid zoom");
        let zoom2 = zoom1; // Should copy, not move
        assert_eq!(zoom1.value(), 2.0);
        assert_eq!(zoom2.value(), 2.0);
    }

    #[test]
    fn test_zoom_level_equality() {
        let zoom1 = ZoomLevel::new(2.0).expect("Valid zoom");
        let zoom2 = ZoomLevel::new(2.0).expect("Valid zoom");
        assert_eq!(zoom1, zoom2);

        let zoom3 = ZoomLevel::new(2.1).expect("Valid zoom");
        assert_ne!(zoom1, zoom3);
    }
}
