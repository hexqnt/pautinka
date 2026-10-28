#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let demo = pautinka_demo::Demo::default();
    eframe::run_native(
        demo.language().app_title(),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1440.0, 960.0])
                .with_min_inner_size([360.0, 480.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(demo))),
    )
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
        let demo = pautinka_demo::Demo::default();
        let language = demo.language();
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |_| Ok(Box::new(demo))),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            if result.is_ok() {
                loading.remove();
            } else {
                loading.set_text_content(Some(match language {
                    pautinka_demo::Language::English => {
                        "Unable to start the demo. Enable WebGL2 and reload the page."
                    }
                    pautinka_demo::Language::Russian => {
                        "Не удалось запустить демо. Включите WebGL2 и перезагрузите страницу."
                    }
                }));
            }
        }
    });
}
