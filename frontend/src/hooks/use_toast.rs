#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

#[cfg(target_arch = "wasm32")]
use crate::ui::toast::{ToastDuration, ToastSeverity, ToastStoreState};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
pub struct ToastStore {
    state: Signal<ToastStoreState>,
}

#[cfg(target_arch = "wasm32")]
impl ToastStore {
    /// Pushes a new toast with the given message and severity.
    /// Uses default duration (3s). Errors are silently ignored.
    pub fn push(&mut self, message: String, severity: ToastSeverity) {
        let current = self.state.read().clone();
        let next = current.push(message, severity, ToastDuration::default());
        if let Ok(new_state) = next {
            self.state.set(new_state);
        }
    }

    /// Pushes a new toast with custom duration. Errors are silently ignored.
    pub fn push_with_duration(
        &mut self,
        message: String,
        severity: ToastSeverity,
        duration: ToastDuration,
    ) {
        let current = self.state.read().clone();
        let next = current.push(message, severity, duration);
        if let Ok(new_state) = next {
            self.state.set(new_state);
        }
    }

    /// Dismisses a toast by ID.
    pub fn dismiss(&mut self, id: crate::ui::toast::ToastId) {
        let current = self.state.read().clone();
        self.state.set(current.dismiss(id));
    }

    /// Clears all toasts.
    pub fn clear_all(&mut self) {
        let current = self.state.read().clone();
        self.state.set(current.clear_all());
    }

    /// Returns a read signal to the current toasts.
    #[must_use]
    pub fn toasts(&self) -> ReadSignal<ToastStoreState> {
        ReadSignal::from(self.state)
    }
}

#[cfg(target_arch = "wasm32")]
pub fn provide_toast_context() -> ToastStore {
    let state = use_signal(|| {
        ToastStoreState::new(5)
            .unwrap_or_else(|_| ToastStoreState { toasts: Vec::new(), capacity: 5 })
    });
    let store = ToastStore { state };
    provide_context(store);

    // Spawn auto-dismiss eviction loop
    let store_clone = store;
    use_future(move || {
        let mut s = store_clone;
        async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(500).await;
                let current = s.state.read().clone();
                let now = chrono::Utc::now();
                let evicted = current.clone().evict_expired(now);
                if evicted != current {
                    s.state.set(evicted);
                }
            }
        }
    });

    store
}

#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn use_toast() -> ToastStore {
    use_context::<ToastStore>()
}
