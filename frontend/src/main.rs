#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use oya_frontend::hooks;
#[cfg(target_arch = "wasm32")]
use oya_frontend::ui;

#[cfg(target_arch = "wasm32")]
const fn should_end_canvas_interaction(
    is_dragging: bool,
    is_panning: bool,
    is_marquee: bool,
    is_connecting: bool,
) -> bool {
    is_dragging || is_panning || is_marquee || is_connecting
}

#[cfg(target_arch = "wasm32")]
struct GlobalMouseupListenerInner {
    window: web_sys::Window,
    callback: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MouseEvent)>,
}

#[cfg(target_arch = "wasm32")]
impl Drop for GlobalMouseupListenerInner {
    fn drop(&mut self) {
        use wasm_bindgen::JsCast;

        let _ = self
            .window
            .remove_event_listener_with_callback("mouseup", self.callback.as_ref().unchecked_ref());
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct GlobalMouseupListener {
    _inner: std::rc::Rc<GlobalMouseupListenerInner>,
}

#[cfg(target_arch = "wasm32")]
fn register_global_mouseup_listener(
    canvas: hooks::use_canvas_interaction::CanvasInteraction,
    selection: hooks::use_selection::SelectionState,
) -> Option<GlobalMouseupListener> {
    use wasm_bindgen::{closure::Closure, JsCast};
    use web_sys::window;

    let window = window()?;
    let canvas_end = canvas;
    let selection_end = selection;
    let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_evt| {
        if should_end_canvas_interaction(
            canvas_end.is_dragging(),
            canvas_end.is_panning(),
            canvas_end.is_marquee(),
            canvas_end.is_connecting(),
        ) {
            canvas_end.end_interaction();
        }
        selection_end.clear_pending_drag();
    });

    match window.add_event_listener_with_callback("mouseup", callback.as_ref().unchecked_ref()) {
        Ok(()) => {}
        Err(_) => return None,
    }

    Some(GlobalMouseupListener {
        _inner: std::rc::Rc::new(GlobalMouseupListenerInner { window, callback }),
    })
}

#[cfg(target_arch = "wasm32")]
mod wasm_app {
    use super::*;

    #[component]
    pub fn App() -> Element {
        let _workflow = hooks::provide_workflow_state_context();
        let selection = hooks::provide_selection_context();
        let canvas = hooks::provide_canvas_interaction_context();
        let _panels = hooks::provide_ui_panels_context();
        let _sidebar = hooks::provide_sidebar_context();
        let _restate = hooks::provide_restate_sync_context();
        let _toast = hooks::provide_toast_context();

        let _global_mouseup_listener =
            use_hook(move || register_global_mouseup_listener(canvas, selection));

        use_effect(move || {
            use wasm_bindgen::{JsCast, JsValue};
            use web_sys::window;

            let Some(win) = window() else {
                return;
            };

            if let Ok(tailwind) = js_sys::Reflect::get(win.as_ref(), &JsValue::from_str("tailwind"))
            {
                if let Ok(refresh) = js_sys::Reflect::get(&tailwind, &JsValue::from_str("refresh"))
                {
                    if let Some(refresh_fn) = refresh.dyn_ref::<js_sys::Function>() {
                        let _ = refresh_fn.call0(&tailwind);
                    }
                }
            }
        });

        rsx! {
            ui::AppShell {}
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    launch(wasm_app::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("This binary is only available for wasm32 target");
    std::process::exit(1);
}
