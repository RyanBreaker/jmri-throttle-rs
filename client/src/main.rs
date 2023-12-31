#[cfg(not(target_arch = "wasm32"))]
fn main() {}

mod app;

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "canvas", // hardcode it
                web_options,
                Box::new(|cc| Box::new(app::App::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
