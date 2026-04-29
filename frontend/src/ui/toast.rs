//! Toast/Notification System
//!
//! Pure state layer + Dioxus UI components for transient notifications.
//! Supports four severity levels, auto-dismiss via gloo_timers, and
//! bounded capacity (max 20 toasts, default 5 visible).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

// ── Type Definitions ─────────────────────────────────────────────────────────

pub type ToastId = Uuid;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToastSeverity {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ToastError {
    #[error("toast capacity must be between 1 and 20, got {0}")]
    InvalidCapacity(usize),

    #[error("toast message must not be empty")]
    EmptyMessage,

    #[error("toast duration must be between 1ms and 30000ms, got {0}ms")]
    InvalidDuration(u64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Toast {
    pub id: ToastId,
    pub message: String,
    pub severity: ToastSeverity,
    pub created_at: DateTime<Utc>,
    pub auto_dismiss_at: Option<DateTime<Utc>>,
}

// ── ToastDuration ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastDuration(pub Duration);

impl ToastDuration {
    /// Validates and constructs a `ToastDuration`.
    ///
    /// Returns `Err(ToastError::InvalidDuration)` when `dur` is zero or exceeds 30 000 ms.
    pub fn new(dur: Duration) -> Result<Self, ToastError> {
        let ms = dur.as_millis() as u64;
        if ms == 0 || ms > 30_000 {
            return Err(ToastError::InvalidDuration(ms));
        }
        Ok(Self(dur))
    }

    /// Returns the wrapped `std::time::Duration`.
    #[must_use]
    pub fn inner(&self) -> Duration {
        self.0
    }
}

impl Default for ToastDuration {
    /// Default duration is 3 000 ms.
    fn default() -> Self {
        Self(Duration::from_millis(3_000))
    }
}

// ── ToastStoreState ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ToastStoreState {
    pub toasts: Vec<Toast>,
    pub capacity: usize,
}

impl ToastStoreState {
    /// Creates an empty store with the given capacity (1..=20).
    pub fn new(capacity: usize) -> Result<Self, ToastError> {
        if capacity == 0 || capacity > 20 {
            return Err(ToastError::InvalidCapacity(capacity));
        }
        Ok(Self { toasts: Vec::new(), capacity })
    }

    /// Pushes a new toast. Prepends at index 0. Evicts oldest when at capacity.
    pub fn push(
        self,
        message: String,
        severity: ToastSeverity,
        duration: ToastDuration,
    ) -> Result<Self, ToastError> {
        if message.trim().is_empty() {
            return Err(ToastError::EmptyMessage);
        }
        let ms = duration.inner().as_millis() as u64;
        if ms == 0 || ms > 30_000 {
            return Err(ToastError::InvalidDuration(ms));
        }
        let created_at = Utc::now();
        let chrono_dur = chrono::Duration::milliseconds(i64::try_from(ms).unwrap_or(i64::MAX));
        let auto_dismiss_at = Some(created_at + chrono_dur);
        let toast = Toast { id: Uuid::new_v4(), message, severity, created_at, auto_dismiss_at };
        let toasts = std::iter::once(toast).chain(self.toasts).take(self.capacity).collect();
        Ok(Self { toasts, capacity: self.capacity })
    }

    /// Removes the toast with the given ID. No-op when ID not found.
    #[must_use]
    pub fn dismiss(self, id: ToastId) -> Self {
        Self {
            toasts: self.toasts.into_iter().filter(|t| t.id != id).collect(),
            capacity: self.capacity,
        }
    }

    /// Removes all toasts. Capacity is preserved.
    #[must_use]
    pub fn clear_all(self) -> Self {
        Self { toasts: Vec::new(), capacity: self.capacity }
    }

    /// Removes all toasts whose `auto_dismiss_at` is in the past relative to `now`.
    #[must_use]
    pub fn evict_expired(self, now: DateTime<Utc>) -> Self {
        Self {
            toasts: self.toasts.into_iter().filter(|t| !is_expired(t, now)).collect(),
            capacity: self.capacity,
        }
    }
}

// ── Free Functions ───────────────────────────────────────────────────────────

/// Returns `true` when `toast.auto_dismiss_at` is `Some(time)` and `now >= time`.
pub fn is_expired(toast: &Toast, now: DateTime<Utc>) -> bool {
    toast.auto_dismiss_at.is_some_and(|t| now >= t)
}

// ── Dioxus UI Components (WASM only) ─────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod view {
    use super::{Toast, ToastSeverity};
    use crate::hooks::use_toast::ToastStore;
    use dioxus::prelude::*;

    /// Severity-specific Tailwind classes for toast styling.
    fn severity_classes(severity: ToastSeverity) -> (&'static str, &'static str, &'static str) {
        match severity {
            ToastSeverity::Success => {
                ("border-emerald-300 bg-emerald-50", "text-emerald-600", "text-emerald-800")
            }
            ToastSeverity::Error => ("border-red-300 bg-red-50", "text-red-600", "text-red-800"),
            ToastSeverity::Info => ("border-blue-300 bg-blue-50", "text-blue-600", "text-blue-800"),
            ToastSeverity::Warning => {
                ("border-amber-300 bg-amber-50", "text-amber-600", "text-amber-800")
            }
        }
    }

    /// Severity-specific icon SVG paths.
    fn severity_icon(severity: ToastSeverity) -> &'static str {
        match severity {
            ToastSeverity::Success => {
                "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
            }
            ToastSeverity::Error => {
                "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
            }
            ToastSeverity::Info => {
                "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            }
            ToastSeverity::Warning => {
                "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.5c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z"
            }
        }
    }

    /// A single toast notification card.
    #[component]
    pub fn ToastItem(toast: Toast, store: ToastStore) -> Element {
        let (border_bg, icon_color, text_color) = severity_classes(toast.severity);
        let icon_path = severity_icon(toast.severity);
        let toast_id = toast.id;

        rsx! {
            div {
                class: "animate-slide-in-right pointer-events-auto flex items-start gap-2.5 rounded-lg border {border_bg} px-3 py-2.5 shadow-lg backdrop-blur-sm",
                role: "alert",
                svg {
                    class: "h-4 w-4 shrink-0 {icon_color}",
                    fill: "none",
                    view_box: "0 0 24 24",
                    stroke: "currentColor",
                    stroke_width: "2",
                    path { d: "{icon_path}" }
                }
                p { class: "flex-1 text-[12px] leading-snug {text_color}", "{toast.message}" }
                button {
                    class: "flex h-4 w-4 shrink-0 items-center justify-center rounded {text_color} opacity-60 transition-opacity hover:opacity-100",
                    r#type: "button",
                    aria_label: "Dismiss notification",
                    onclick: move |_| {
                        store.dismiss(toast_id);
                    },
                    svg {
                        class: "h-3 w-3",
                        fill: "none",
                        view_box: "0 0 24 24",
                        stroke: "currentColor",
                        stroke_width: "2",
                        path { d: "M6 18L18 6M6 6l12 12" }
                    }
                }
            }
        }
    }

    /// Container that renders all active toasts in a fixed position (top-right).
    #[component]
    pub fn ToastContainer(store: ToastStore) -> Element {
        let toasts_signal = store.toasts();
        let toasts = toasts_signal.read();

        if toasts.toasts.is_empty() {
            return rsx! {};
        }

        rsx! {
            div {
                class: "pointer-events-none fixed right-4 top-16 z-50 flex flex-col gap-2",
                aria_live: "polite",
                aria_label: "Notifications",
                for toast in toasts.toasts.iter() {
                    ToastItem {
                        key: "{toast.id}",
                        toast: toast.clone(),
                        store: store,
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use view::{ToastContainer, ToastItem};
