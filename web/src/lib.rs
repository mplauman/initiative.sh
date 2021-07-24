mod utils;

use initiative_core as core;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn command(input: &str) -> String {
    app().command(input)
}

#[wasm_bindgen]
pub fn autocomplete(input: &str) -> JsValue {
    JsValue::from_serde(&app().autocomplete(input)).unwrap()
}

static mut APP: Option<core::app::App> = None;

#[no_mangle]
pub extern "C" fn app() -> &'static mut core::app::App {
    utils::set_panic_hook();

    unsafe {
        if APP.is_none() {
            APP = Some(core::app());
        }

        APP.as_mut().unwrap()
    }
}
