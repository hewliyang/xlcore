use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn extract_xlsx(bytes: Vec<u8>) -> Result<JsValue, JsValue> {
    let mut doc = xlcore_io::open_bytes(bytes).map_err(to_js_error)?;
    let layout = xlcore_export::extract_doc(&mut doc).map_err(to_js_error)?;
    serde_wasm_bindgen::to_value(&layout).map_err(to_js_error)
}

#[wasm_bindgen(js_name = extractXlsxJson)]
pub fn extract_xlsx_json(bytes: Vec<u8>) -> Result<String, JsValue> {
    let mut doc = xlcore_io::open_bytes(bytes).map_err(to_js_error)?;
    let layout = xlcore_export::extract_doc(&mut doc).map_err(to_js_error)?;
    serde_json::to_string(&layout).map_err(to_js_error)
}

fn to_js_error(err: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&err.to_string()).into()
}
