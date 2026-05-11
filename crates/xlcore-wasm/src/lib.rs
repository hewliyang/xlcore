use wasm_bindgen::prelude::*;

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WasmExtractOptions {
    sheet_index: Option<usize>,
    sheet_name: Option<String>,
}

#[wasm_bindgen]
pub fn extract_xlsx(bytes: Vec<u8>, options: JsValue) -> Result<JsValue, JsValue> {
    let options: WasmExtractOptions = if options.is_null() || options.is_undefined() {
        WasmExtractOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(to_js_error)?
    };
    let mut doc = xlcore_io::open_bytes(bytes).map_err(to_js_error)?;
    let layout = xlcore_export::extract_doc_with_options(
        &mut doc,
        &xlcore_export::ExtractOptions {
            sheet_index: options.sheet_index,
            sheet_name: options.sheet_name,
        },
    )
    .map_err(to_js_error)?;
    serde_wasm_bindgen::to_value(&layout).map_err(to_js_error)
}

#[wasm_bindgen(js_name = extractXlsxJson)]
pub fn extract_xlsx_json(bytes: Vec<u8>, options: JsValue) -> Result<String, JsValue> {
    let options: WasmExtractOptions = if options.is_null() || options.is_undefined() {
        WasmExtractOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(to_js_error)?
    };
    let mut doc = xlcore_io::open_bytes(bytes).map_err(to_js_error)?;
    let layout = xlcore_export::extract_doc_with_options(
        &mut doc,
        &xlcore_export::ExtractOptions {
            sheet_index: options.sheet_index,
            sheet_name: options.sheet_name,
        },
    )
    .map_err(to_js_error)?;
    serde_json::to_string(&layout).map_err(to_js_error)
}

fn to_js_error(err: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&err.to_string()).into()
}
