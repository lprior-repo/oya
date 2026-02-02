//! RequestAnimationFrame-based animation loop
//!
//! Provides a functional, panic-free RAF loop for smooth 60fps canvas rendering.
//! Integrates with Leptos reactive state and Page Visibility API for automatic pause/resume.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, Document, Window};

/// Type alias for RAF closure to reduce complexity
type RafClosure = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

/// RAF animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    /// Animation is running
    Running,
    /// Animation is paused (e.g., tab hidden)
    Paused,
    /// Animation has been stopped/cleaned up
    Stopped,
}

/// Frame timing information
#[derive(Debug, Clone, Copy)]
pub struct FrameTiming {
    /// Current timestamp from RAF (milliseconds)
    pub timestamp: f64,
    /// Delta time since last frame (milliseconds)
    pub delta: f64,
    /// Frames per second (calculated)
    pub fps: f64,
}

impl FrameTiming {
    /// Create initial frame timing
    const fn initial(timestamp: f64) -> Self {
        Self {
            timestamp,
            delta: 0.0,
            fps: 60.0,
        }
    }

    /// Calculate next frame timing
    fn next(self, timestamp: f64) -> Self {
        let delta = timestamp - self.timestamp;
        let fps = if delta > 0.0 { 1000.0 / delta } else { 60.0 };

        Self {
            timestamp,
            delta,
            fps,
        }
    }
}

/// RAF animation loop errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum RafError {
    /// Failed to get window object
    #[error("failed to get window: window is not available")]
    WindowNotAvailable,

    /// Failed to get document object
    #[error("failed to get document: document is not available")]
    DocumentNotAvailable,

    /// Failed to request animation frame
    #[error("failed to request animation frame: {0}")]
    RequestFailed(String),

    /// Failed to cancel animation frame
    #[error("failed to cancel animation frame: {0}")]
    CancelFailed(String),

    /// Failed to add visibility change listener
    #[error("failed to add visibility listener: {0}")]
    VisibilityListenerFailed(String),

    /// Failed to cast closure
    #[error("failed to cast closure: type mismatch")]
    ClosureCastFailed,
}

/// RAF animation handle for cleanup
#[derive(Clone)]
pub struct AnimationHandle {
    window: Window,
    request_id: Rc<RefCell<Option<i32>>>,
    state_signal: RwSignal<AnimationState>,
}

impl AnimationHandle {
    /// Stop the animation loop
    ///
    /// # Errors
    ///
    /// Returns error if cancellation fails
    pub fn stop(&self) -> Result<(), RafError> {
        // Set state to stopped
        self.state_signal.set(AnimationState::Stopped);

        // Cancel RAF if one is scheduled
        if let Some(id) = *self.request_id.borrow() {
            self.window
                .cancel_animation_frame(id)
                .map_err(|e| RafError::CancelFailed(format!("{e:?}")))?;
            *self.request_id.borrow_mut() = None;
        }

        Ok(())
    }

    /// Pause the animation loop
    pub fn pause(&self) {
        self.state_signal.set(AnimationState::Paused);
    }

    /// Resume the animation loop
    pub fn resume(&self) {
        self.state_signal.set(AnimationState::Running);
    }

    /// Get current animation state
    #[must_use]
    pub fn state(&self) -> AnimationState {
        self.state_signal.get()
    }
}

/// Start `RequestAnimationFrame` loop
///
/// Creates a RAF loop that calls the render function every frame with timing information.
/// Automatically pauses when tab is hidden and resumes when visible.
///
/// # Errors
///
/// Returns error if:
/// - Window or document are not available
/// - RAF scheduling fails
/// - Visibility API setup fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::raf::{start_animation_loop, FrameTiming};
/// use leptos::*;
///
/// let handle = start_animation_loop(move |timing| {
///     // Render frame with delta time
///     log!("FPS: {}, Delta: {}ms", timing.fps, timing.delta);
/// })?;
///
/// // Later: cleanup
/// handle.stop()?;
/// # Ok::<(), oya_ui::components::canvas::raf::RafError>(())
/// ```
pub fn start_animation_loop<F>(render_fn: F) -> Result<AnimationHandle, RafError>
where
    F: Fn(FrameTiming) + 'static,
{
    // Get window and document
    let window = get_window()?;
    let document = get_document(&window)?;

    // Create state signal
    let state_signal = RwSignal::new(AnimationState::Running);

    // Shared state for RAF callback
    let request_id = Rc::new(RefCell::new(None::<i32>));
    let request_id_clone = request_id.clone();

    // Frame timing state
    let timing = Rc::new(RefCell::new(None::<FrameTiming>));

    // Create RAF closure
    let closure: RafClosure = Rc::new(RefCell::new(None));
    let closure_clone = closure.clone();

    let window_clone = window.clone();
    let render_fn = Rc::new(render_fn);

    *closure.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp: f64| {
        // Check if animation should run
        if state_signal.get() != AnimationState::Running {
            return;
        }

        // Calculate frame timing
        let current_timing = timing
            .borrow()
            .map_or_else(|| FrameTiming::initial(timestamp), |t| t.next(timestamp));

        *timing.borrow_mut() = Some(current_timing);

        // Call render function
        render_fn(current_timing);

        // Schedule next frame if still running
        if state_signal.get() == AnimationState::Running {
            let next_id = schedule_next_frame(&window_clone, &closure_clone);
            if let Ok(id) = next_id {
                *request_id_clone.borrow_mut() = Some(id);
            }
        }
    }) as Box<dyn FnMut(f64)>));

    // Schedule first frame
    let first_id = schedule_next_frame(&window, &closure)?;
    *request_id.borrow_mut() = Some(first_id);

    // Setup Page Visibility API listener
    setup_visibility_listener(&document, state_signal)?;

    Ok(AnimationHandle {
        window,
        request_id,
        state_signal,
    })
}

/// Get window object
fn get_window() -> Result<Window, RafError> {
    web_sys::window().ok_or(RafError::WindowNotAvailable)
}

/// Get document from window
fn get_document(window: &Window) -> Result<Document, RafError> {
    window.document().ok_or(RafError::DocumentNotAvailable)
}

/// Schedule next RAF frame
fn schedule_next_frame(window: &Window, closure: &RafClosure) -> Result<i32, RafError> {
    closure
        .borrow()
        .as_ref()
        .ok_or(RafError::ClosureCastFailed)
        .and_then(|cb| {
            window
                .request_animation_frame(cb.as_ref().unchecked_ref())
                .map_err(|e| RafError::RequestFailed(format!("{e:?}")))
        })
}

/// Setup Page Visibility API listener
///
/// Automatically pauses animation when tab is hidden, resumes when visible
fn setup_visibility_listener(
    document: &Document,
    state_signal: RwSignal<AnimationState>,
) -> Result<(), RafError> {
    let closure = Closure::wrap(Box::new(move || {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };

        let is_hidden = document.hidden();

        // Update state based on visibility
        let current_state = state_signal.get();
        if is_hidden && current_state == AnimationState::Running {
            state_signal.set(AnimationState::Paused);
        } else if !is_hidden && current_state == AnimationState::Paused {
            state_signal.set(AnimationState::Running);
        }
    }) as Box<dyn FnMut()>);

    document
        .add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref())
        .map_err(|e| RafError::VisibilityListenerFailed(format!("{e:?}")))?;

    // Keep closure alive
    closure.forget();

    Ok(())
}

/// Start RAF loop with canvas context
///
/// Convenience function that provides both canvas context and timing to render function.
///
/// # Errors
///
/// Returns error if RAF loop setup fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::raf::start_canvas_animation_loop;
/// use web_sys::CanvasRenderingContext2d;
///
/// let context = /* ... */;
/// let handle = start_canvas_animation_loop(context, move |ctx, timing| {
///     // Clear and render
///     ctx.clear_rect(0.0, 0.0, 800.0, 600.0);
///     // ... render logic ...
/// })?;
/// # Ok::<(), oya_ui::components::canvas::raf::RafError>(())
/// ```
pub fn start_canvas_animation_loop<F>(
    ctx: CanvasRenderingContext2d,
    render_fn: F,
) -> Result<AnimationHandle, RafError>
where
    F: Fn(&CanvasRenderingContext2d, FrameTiming) + 'static,
{
    let ctx = Rc::new(ctx);
    let render_fn = Rc::new(render_fn);

    start_animation_loop(move |timing| {
        render_fn(&ctx, timing);
    })
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_frame_timing_initial() {
        let timing = FrameTiming::initial(1000.0);
        assert!((timing.timestamp - 1000.0).abs() < f64::EPSILON);
        assert!((timing.delta - 0.0).abs() < f64::EPSILON);
        assert!((timing.fps - 60.0).abs() < f64::EPSILON);
    }

    #[wasm_bindgen_test]
    fn test_frame_timing_next() {
        let timing = FrameTiming::initial(1000.0);
        let next = timing.next(1016.67);

        assert!((next.timestamp - 1016.67).abs() < f64::EPSILON);
        assert!((next.delta - 16.67).abs() < 0.01);
        assert!((next.fps - 60.0).abs() < 1.0);
    }

    #[wasm_bindgen_test]
    fn test_animation_state_transitions() {
        let state = AnimationState::Running;
        assert_eq!(state, AnimationState::Running);

        let state = AnimationState::Paused;
        assert_eq!(state, AnimationState::Paused);

        let state = AnimationState::Stopped;
        assert_eq!(state, AnimationState::Stopped);
    }

    #[wasm_bindgen_test]
    fn test_get_window_succeeds() -> Result<(), RafError> {
        let _window = get_window()?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_document_succeeds() -> Result<(), RafError> {
        let window = get_window()?;
        let _document = get_document(&window)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_animation_loop_starts() -> Result<(), RafError> {
        let called = Rc::new(RefCell::new(false));

        let _handle = start_animation_loop(move |_timing| {
            *called.borrow_mut() = true;
        })?;

        // Note: Can't test actual rendering in sync test
        // Would need async test with delay
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_animation_handle_stop() -> Result<(), RafError> {
        let handle = start_animation_loop(|_timing| {})?;

        handle.stop()?;
        assert_eq!(handle.state(), AnimationState::Stopped);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_animation_handle_pause_resume() -> Result<(), RafError> {
        let handle = start_animation_loop(|_timing| {})?;

        assert_eq!(handle.state(), AnimationState::Running);

        handle.pause();
        assert_eq!(handle.state(), AnimationState::Paused);

        handle.resume();
        assert_eq!(handle.state(), AnimationState::Running);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_no_panics_on_multiple_stops() -> Result<(), RafError> {
        let handle = start_animation_loop(|_timing| {})?;

        handle.stop()?;
        handle.stop()?; // Should not panic
        handle.stop()?; // Should not panic

        Ok(())
    }
}
