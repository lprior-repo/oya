use crate::graph::Viewport;

#[must_use]
pub fn is_valid_zoom(zoom: f32) -> bool {
    zoom.is_finite() && zoom.abs() > f32::EPSILON
}

#[must_use]
pub fn safe_canvas_point(page: (f32, f32), origin: (f32, f32)) -> Option<(f32, f32)> {
    if !page.0.is_finite() || !page.1.is_finite() || !origin.0.is_finite() || !origin.1.is_finite()
    {
        return None;
    }
    Some((page.0 - origin.0, page.1 - origin.1))
}

#[must_use]
pub fn safe_canvas_from_viewport(
    page: (f32, f32),
    origin: (f32, f32),
    viewport: &Viewport,
) -> Option<(f32, f32)> {
    let zoom_val = viewport.zoom;
    if !is_valid_zoom(zoom_val) {
        return None;
    }

    let (mx, my) = safe_canvas_point(page, origin)?;
    Some(((mx - viewport.x) / zoom_val, (my - viewport.y) / zoom_val))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{is_valid_zoom, safe_canvas_from_viewport, safe_canvas_point};
    use crate::graph::Viewport;

    #[test]
    fn given_zero_zoom_when_validating_then_zoom_is_invalid() {
        assert!(!is_valid_zoom(0.0));
    }

    #[test]
    fn given_nan_zoom_when_validating_then_zoom_is_invalid() {
        assert!(!is_valid_zoom(f32::NAN));
    }

    #[test]
    fn given_positive_zoom_when_validating_then_zoom_is_valid() {
        assert!(is_valid_zoom(1.0));
    }

    #[test]
    fn given_non_finite_input_when_mapping_to_canvas_then_it_returns_none() {
        let result = safe_canvas_point((f32::NAN, 20.0), (5.0, 10.0));

        assert!(result.is_none());
    }

    #[test]
    fn given_valid_inputs_when_mapping_to_canvas_then_offset_is_computed() {
        let result = safe_canvas_point((50.0, 70.0), (10.0, 20.0));

        assert_eq!(result, Some((40.0, 50.0)));
    }

    #[test]
    fn given_valid_inputs_when_mapping_to_viewport_then_point_is_transformed() {
        let viewport = Viewport { x: 10.0, y: 20.0, zoom: 2.0_f32.clamp(0.15, 3.0) };

        let result = safe_canvas_from_viewport((50.0, 70.0), (0.0, 0.0), &viewport);

        assert_eq!(result, Some((20.0, 25.0)));
    }

    #[test]
    fn given_invalid_origin_when_mapping_to_canvas_then_none_is_returned() {
        let result = safe_canvas_point((10.0, 10.0), (f32::NAN, 0.0));

        assert!(result.is_none());
    }

    #[test]
    fn given_negative_zoom_when_validating_then_zoom_is_valid() {
        // Negative zoom is valid (just inverts the canvas)
        assert!(is_valid_zoom(-1.0));
    }

    #[test]
    fn given_negative_infinite_zoom_when_validating_then_zoom_is_invalid() {
        assert!(!is_valid_zoom(f32::NEG_INFINITY));
    }

    #[test]
    fn given_clamped_zoom_when_below_min_then_zoom_is_clamped_to_min() {
        // clamp enforces [0.15, 3.0], so negative values clamp to MIN_ZOOM
        assert_eq!((-2.0_f32).clamp(0.15, 3.0), 0.15);
    }
}
