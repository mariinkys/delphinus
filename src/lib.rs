#![recursion_limit = "256"]
pub mod api;
pub mod app;
pub mod client;
pub mod components;
pub mod model;
pub mod pages;

#[cfg(feature = "ssr")]
pub mod dict;
#[cfg(feature = "ssr")]
pub mod ocr;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
