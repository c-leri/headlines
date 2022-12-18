mod headlines;

pub use headlines::Headlines;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
#[cfg(target_arch = "wasm32")]
use eframe::WebOptions;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn main_web(canvas_id: &str)
{
    let headlines = Headlines::new();
    tracing_wasm::set_as_global_default();
    if let Err(_) = eframe::start_web(canvas_id, WebOptions::default(), Box::new(|cc| Box::new(headlines.init(cc)))).await
    {
        tracing::error!("Error starting the web app");
    }
}