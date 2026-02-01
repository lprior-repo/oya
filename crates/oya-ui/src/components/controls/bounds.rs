//! Zoom level bounds validation with compile-time safety
//!
//! This module provides a validated `ZoomLevel` type that enforces bounds
//! at the type level, ensuring all zoom operations stay within the valid
//! range of 0.1 to 5.0.

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 5.0;

/// A validated zoom level that guarantees values are within [0.1, 5.0] range.
///
/// This type uses the Railway-Oriented Programming pattern to ensure that
/// all zoom values are valid and bounded. Invalid inputs (NaN, infinity) are
/// rejected, and out-of-bounds values are clamped to the valid range.
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::bounds::ZoomLevel;
///
/// // Valid zoom levels
/// let zoom = ZoomLevel::new(1.0)?;
/// assert_eq!(zoom.value(), 1.0);
///
/// // Out-of-bounds values are clamped
/// let zoom = ZoomLevel::new(10.0)?;
/// assert_eq!(zoom.value(), 5.0);
///
/// // Invalid values are rejected
/// assert!(ZoomLevel::new(f32::NAN).is_err());
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomLevel {
    value: f32,
}

impl ZoomLevel {
    /// Creates a new `ZoomLevel`, clamping the value to [MIN_ZOOM, MAX_ZOOM].
    ///
    /// # Errors
    ///
    /// Returns an error if the input is NaN or infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let zoom = ZoomLevel::new(2.5)?;
    /// assert_eq!(zoom.value(), 2.5);
    ///
    /// // Clamping behavior
    /// let zoom = ZoomLevel::new(10.0)?;
    /// assert_eq!(zoom.value(), 5.0);
    ///
    /// let zoom = ZoomLevel::new(0.05)?;
    /// assert_eq!(zoom.value(), 0.1);
    /// # Ok::<(), String>(())
    /// ```
    pub fn new(value: f32) -> Result<Self, String> {
        if value.is_nan() || value.is_infinite() {
            return Err("Zoom level must be a finite number".to_string());
        }

        let clamped = value.max(MIN_ZOOM).min(MAX_ZOOM);
        Ok(Self { value: clamped })
    }

    /// Returns the current zoom level value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let zoom = ZoomLevel::new(2.5)?;
    /// assert_eq!(zoom.value(), 2.5);
    /// # Ok::<(), String>(())
    /// ```
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Increments the zoom level by the given delta, clamping to bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if the delta is NaN or infinite, or if the resulting
    /// value would be NaN or infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let zoom = ZoomLevel::new(1.0)?;
    /// let zoomed = zoom.increment(0.5)?;
    /// assert_eq!(zoomed.value(), 1.5);
    ///
    /// // Clamped to max
    /// let zoom = ZoomLevel::new(4.8)?;
    /// let zoomed = zoom.increment(0.5)?;
    /// assert_eq!(zoomed.value(), 5.0);
    /// # Ok::<(), String>(())
    /// ```
    pub fn increment(&self, delta: f32) -> Result<Self, String> {
        Self::new(self.value + delta)
    }

    /// Decrements the zoom level by the given delta, clamping to bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if the delta is NaN or infinite, or if the resulting
    /// value would be NaN or infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let zoom = ZoomLevel::new(2.0)?;
    /// let zoomed = zoom.decrement(0.5)?;
    /// assert_eq!(zoomed.value(), 1.5);
    ///
    /// // Clamped to min
    /// let zoom = ZoomLevel::new(0.3)?;
    /// let zoomed = zoom.decrement(0.5)?;
    /// assert_eq!(zoomed.value(), 0.1);
    /// # Ok::<(), String>(())
    /// ```
    pub fn decrement(&self, delta: f32) -> Result<Self, String> {
        Self::new(self.value - delta)
    }

    /// Returns the minimum allowed zoom level.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let min = ZoomLevel::min();
    /// assert_eq!(min.value(), 0.1);
    /// ```
    pub fn min() -> Self {
        Self { value: MIN_ZOOM }
    }

    /// Returns the maximum allowed zoom level.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let max = ZoomLevel::max();
    /// assert_eq!(max.value(), 5.0);
    /// ```
    pub fn max() -> Self {
        Self { value: MAX_ZOOM }
    }
}

impl Default for ZoomLevel {
    /// Returns the default zoom level of 1.0 (100%).
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let zoom = ZoomLevel::default();
    /// assert_eq!(zoom.value(), 1.0);
    /// ```
    fn default() -> Self {
        Self { value: 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MIN_ZOOM, 0.1);
        assert_eq!(MAX_ZOOM, 5.0);
    }

    #[test]
    fn test_min_max_constructors_are_valid() {
        let min = ZoomLevel::min();
        assert!(min.value >= MIN_ZOOM);
        assert!(min.value <= MAX_ZOOM);

        let max = ZoomLevel::max();
        assert!(max.value >= MIN_ZOOM);
        assert!(max.value <= MAX_ZOOM);
    }
}
