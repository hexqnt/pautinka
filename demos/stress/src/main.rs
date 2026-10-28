#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    match native::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("Browser window is required")
            .document()
            .expect("Browser document is required");
        let canvas = document
            .get_element_by_id("pautinka")
            .expect("Canvas element is required")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Element must be a canvas");
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_| Ok(Box::new(pautinka_stress::Demo::default()))),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            if result.is_ok() {
                loading.remove();
            } else {
                loading.set_text_content(Some(
                    "Unable to start the demo. Enable WebGL2 and reload the page.",
                ));
            }
        }
    });
}
