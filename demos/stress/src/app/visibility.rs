use std::{cell::Cell, rc::Rc};

use wasm_bindgen::{JsCast as _, closure::Closure};
use web_sys::Document;

pub(super) struct Visibility {
    resumed: Rc<Cell<bool>>,
    document: Document,
    listener: Closure<dyn FnMut()>,
}

impl Default for Visibility {
    fn default() -> Self {
        let document = web_sys::window()
            .expect("Browser window is required")
            .document()
            .expect("Browser document is required");
        let resumed = Rc::new(Cell::new(false));
        let flag = Rc::clone(&resumed);
        let observed = document.clone();
        let listener = Closure::new(move || {
            if !observed.hidden() {
                flag.set(true);
            }
        });
        document
            .add_event_listener_with_callback("visibilitychange", listener.as_ref().unchecked_ref())
            .expect("Visibility listener must be installed");
        Self {
            resumed,
            document,
            listener,
        }
    }
}

impl Visibility {
    pub fn take_resumed(&self) -> bool {
        self.resumed.replace(false)
    }
}

impl Drop for Visibility {
    fn drop(&mut self) {
        let _ = self.document.remove_event_listener_with_callback(
            "visibilitychange",
            self.listener.as_ref().unchecked_ref(),
        );
    }
}
