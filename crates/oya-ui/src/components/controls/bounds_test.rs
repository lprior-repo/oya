//! Tests for zoom level bounds validation

use super::bounds::ZoomLevel;

#[cfg(test)]
mod zoom_level_tests {
    use super::*;

    #[test]
    fn test_valid_zoom_values() -> Result<(), String> {
        // Test minimum bound
        let zoom = ZoomLevel::new(0.1)?;
        assert_eq!(zoom.value(), 0.1);

        // Test maximum bound
        let zoom = ZoomLevel::new(5.0)?;
        assert_eq!(zoom.value(), 5.0);

        // Test middle value
        let zoom = ZoomLevel::new(1.0)?;
        assert_eq!(zoom.value(), 1.0);

        // Test arbitrary valid value
        let zoom = ZoomLevel::new(2.5)?;
        assert_eq!(zoom.value(), 2.5);
        Ok(())
    }

    #[test]
    fn test_values_below_min_get_clamped() -> Result<(), String> {
        let zoom = ZoomLevel::new(0.05)?;
        assert_eq!(zoom.value(), 0.1);

        let zoom = ZoomLevel::new(-1.0)?;
        assert_eq!(zoom.value(), 0.1);

        let zoom = ZoomLevel::new(0.0)?;
        assert_eq!(zoom.value(), 0.1);
        Ok(())
    }

    #[test]
    fn test_values_above_max_get_clamped() -> Result<(), String> {
        let zoom = ZoomLevel::new(6.0)?;
        assert_eq!(zoom.value(), 5.0);

        let zoom = ZoomLevel::new(100.0)?;
        assert_eq!(zoom.value(), 5.0);

        let zoom = ZoomLevel::new(5.1)?;
        assert_eq!(zoom.value(), 5.0);
        Ok(())
    }

    #[test]
    fn test_nan_handling() {
        let result = ZoomLevel::new(f32::NAN);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("finite"));
        }
    }

    #[test]
    fn test_infinity_handling() {
        let result = ZoomLevel::new(f32::INFINITY);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("finite"));
        }

        let result = ZoomLevel::new(f32::NEG_INFINITY);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("finite"));
        }
    }

    #[test]
    fn test_zoom_increment() -> Result<(), String> {
        let zoom = ZoomLevel::new(1.0)?;
        let zoomed = zoom.increment(0.5)?;
        assert_eq!(zoomed.value(), 1.5);

        // Test increment beyond max gets clamped
        let zoom = ZoomLevel::new(4.8)?;
        let zoomed = zoom.increment(0.5)?;
        assert_eq!(zoomed.value(), 5.0);
        Ok(())
    }

    #[test]
    fn test_zoom_decrement() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let zoomed = zoom.decrement(0.5)?;
        assert_eq!(zoomed.value(), 1.5);

        // Test decrement below min gets clamped
        let zoom = ZoomLevel::new(0.3)?;
        let zoomed = zoom.decrement(0.5)?;
        assert_eq!(zoomed.value(), 0.1);
        Ok(())
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
    fn test_extreme_values() -> Result<(), String> {
        // Test f32::MAX gets clamped to max
        let zoom = ZoomLevel::new(f32::MAX)?;
        assert_eq!(zoom.value(), 5.0);

        // Test f32::MIN gets clamped to min (f32::MIN is negative)
        let zoom = ZoomLevel::new(f32::MIN)?;
        assert_eq!(zoom.value(), 0.1);
        Ok(())
    }

    #[test]
    fn test_increment_with_nan_delta() -> Result<(), String> {
        let zoom = ZoomLevel::new(1.0)?;
        let result = zoom.increment(f32::NAN);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_decrement_with_nan_delta() -> Result<(), String> {
        let zoom = ZoomLevel::new(1.0)?;
        let result = zoom.decrement(f32::NAN);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_zoom_level_copy_semantics() -> Result<(), String> {
        let zoom1 = ZoomLevel::new(2.0)?;
        let zoom2 = zoom1; // Should copy, not move
        assert_eq!(zoom1.value(), 2.0);
        assert_eq!(zoom2.value(), 2.0);
        Ok(())
    }

    #[test]
    fn test_zoom_level_equality() -> Result<(), String> {
        let zoom1 = ZoomLevel::new(2.0)?;
        let zoom2 = ZoomLevel::new(2.0)?;
        assert_eq!(zoom1, zoom2);

        let zoom3 = ZoomLevel::new(2.1)?;
        assert_ne!(zoom1, zoom3);
        Ok(())
    }
}
