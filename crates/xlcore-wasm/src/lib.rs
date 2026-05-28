use wasm_bindgen::prelude::*;
use xlcore_io::{LoadReport, XlsxLoadError};

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WasmExtractOptions {
    sheet_index: Option<usize>,
    sheet_name: Option<String>,
}

#[derive(serde::Serialize)]
struct WasmSuccess<'a, T> {
    layout: &'a T,
    report: &'a LoadReport,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmErrorPayload {
    code: &'static str,
    message: String,
    part: Option<String>,
    schema_kind: Option<String>,
    ty: Option<String>,
    field: Option<String>,
    value: Option<String>,
}

impl WasmErrorPayload {
    fn from_load_error(err: &XlsxLoadError) -> Self {
        let mut p = Self {
            code: err.code(),
            message: err.to_string(),
            part: None,
            schema_kind: None,
            ty: None,
            field: None,
            value: None,
        };
        match err {
            XlsxLoadError::Schema {
                part,
                kind,
                ty,
                field,
                value,
                ..
            } => {
                p.part = Some(part.clone());
                p.schema_kind = Some(kind.as_str().to_string());
                p.ty = ty.clone();
                p.field = field.clone();
                p.value = value.clone();
            }
            XlsxLoadError::MissingPart { part } => {
                p.part = Some((*part).to_string());
            }
            _ => {}
        }
        p
    }
}

#[wasm_bindgen]
pub fn extract_xlsx(bytes: Vec<u8>, options: JsValue) -> Result<JsValue, JsValue> {
    let (layout, report) = run_extract(bytes, options)?;
    let envelope = WasmSuccess {
        layout: &layout,
        report: &report,
    };
    serde_wasm_bindgen::to_value(&envelope).map_err(other_err_to_js)
}

#[wasm_bindgen(js_name = extractXlsxJson)]
pub fn extract_xlsx_json(bytes: Vec<u8>, options: JsValue) -> Result<String, JsValue> {
    let (layout, report) = run_extract(bytes, options)?;
    let envelope = WasmSuccess {
        layout: &layout,
        report: &report,
    };
    serde_json::to_string(&envelope).map_err(other_err_to_js)
}

fn run_extract(
    bytes: Vec<u8>,
    options: JsValue,
) -> Result<(xlcore_export::WorkbookLayout, xlcore_io::LoadReport), JsValue> {
    let options = parse_options(options)?;
    let (mut doc, report) = xlcore_io::open_bytes_with_report(bytes).map_err(load_err_to_js)?;
    let layout = xlcore_export::extract_doc_with_options(
        &mut doc,
        &xlcore_export::ExtractOptions {
            sheet_index: options.sheet_index,
            sheet_name: options.sheet_name,
        },
    )
    .map_err(other_err_to_js)?;
    Ok((layout, report))
}

fn parse_options(options: JsValue) -> Result<WasmExtractOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        Ok(WasmExtractOptions::default())
    } else {
        serde_wasm_bindgen::from_value(options).map_err(other_err_to_js)
    }
}

fn load_err_to_js(err: XlsxLoadError) -> JsValue {
    let payload = WasmErrorPayload::from_load_error(&err);
    let js_err = js_sys::Error::new(&payload.message);

    if let Ok(payload_val) = serde_wasm_bindgen::to_value(&payload) {
        if let Ok(entries) =
            js_sys::Object::entries(&payload_val.into()).dyn_into::<js_sys::Array>()
        {
            for entry in entries.iter() {
                let Ok(pair) = entry.dyn_into::<js_sys::Array>() else {
                    continue;
                };
                let key = pair.get(0);
                let value = pair.get(1);
                let _ = js_sys::Reflect::set(&js_err, &key, &value);
            }
        }
    }
    js_err.into()
}

fn other_err_to_js(err: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&err.to_string()).into()
}
