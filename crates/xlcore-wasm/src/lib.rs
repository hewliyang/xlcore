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

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WasmCsvOptions {
    delimiter: Option<String>,
    max_rows: Option<usize>,
    sheet_name: Option<String>,
}

fn parse_csv_options(options: JsValue) -> Result<xlcore_tabular::CsvOptions, JsValue> {
    let raw: WasmCsvOptions = if options.is_null() || options.is_undefined() {
        WasmCsvOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(other_err_to_js)?
    };
    let mut opts = xlcore_tabular::CsvOptions::default();
    if let Some(d) = raw.delimiter {
        let bytes = d.as_bytes();
        let byte = if d.eq_ignore_ascii_case("tab") {
            b'\t'
        } else if bytes.len() == 1 {
            bytes[0]
        } else {
            return Err(other_err_to_js(format!(
                "csv delimiter must be a single byte (got {d:?})"
            )));
        };
        opts.delimiter = Some(byte);
    }
    if let Some(m) = raw.max_rows {
        opts.max_rows = m;
    }
    if let Some(n) = raw.sheet_name {
        opts.sheet_name = n;
    }
    Ok(opts)
}

#[wasm_bindgen]
pub fn extract_csv(bytes: Vec<u8>, options: JsValue) -> Result<JsValue, JsValue> {
    let opts = parse_csv_options(options)?;
    let (layout, report) = xlcore_tabular::extract_csv(&bytes, &opts).map_err(other_err_to_js)?;
    let envelope = WasmSuccess {
        layout: &layout,
        report: &report,
    };
    serde_wasm_bindgen::to_value(&envelope).map_err(other_err_to_js)
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WasmParquetOptions {
    max_rows: Option<usize>,
    sheet_name: Option<String>,
}

fn parse_parquet_options(options: JsValue) -> Result<xlcore_tabular::ParquetOptions, JsValue> {
    let raw: WasmParquetOptions = if options.is_null() || options.is_undefined() {
        WasmParquetOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(other_err_to_js)?
    };
    let mut opts = xlcore_tabular::ParquetOptions::default();
    if let Some(m) = raw.max_rows {
        opts.max_rows = m;
    }
    if let Some(n) = raw.sheet_name {
        opts.sheet_name = n;
    }
    Ok(opts)
}

#[wasm_bindgen]
pub fn extract_parquet(bytes: Vec<u8>, options: JsValue) -> Result<JsValue, JsValue> {
    let opts = parse_parquet_options(options)?;
    let (layout, report) =
        xlcore_tabular::extract_parquet(&bytes, &opts).map_err(other_err_to_js)?;
    let envelope = WasmSuccess {
        layout: &layout,
        report: &report,
    };
    serde_wasm_bindgen::to_value(&envelope).map_err(other_err_to_js)
}

#[wasm_bindgen]
pub struct WorkbookHandle {
    workbook: Option<xlcore_api::Workbook>,
}

#[wasm_bindgen]
impl WorkbookHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WorkbookHandle, JsValue> {
        Ok(Self {
            workbook: Some(xlcore_api::Workbook::new().map_err(api_err_to_js)?),
        })
    }

    pub fn open(bytes: Vec<u8>) -> Result<WorkbookHandle, JsValue> {
        Ok(Self {
            workbook: Some(xlcore_api::Workbook::open_bytes(bytes).map_err(api_err_to_js)?),
        })
    }

    pub fn sheets(&mut self) -> Result<JsValue, JsValue> {
        let sheets = self.workbook_mut()?.sheets().map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&sheets).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = getCell)]
    pub fn get_cell(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let cell = self
            .workbook_mut()?
            .get_cell(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&cell).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setValue)]
    pub fn set_value(&mut self, reference: &str, value: JsValue) -> Result<JsValue, JsValue> {
        let value = cell_value_from_js(value)?;
        let cell = self
            .workbook_mut()?
            .set_value(reference, value)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&cell).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setFormula)]
    pub fn set_formula(&mut self, reference: &str, formula: &str) -> Result<JsValue, JsValue> {
        let cell = self
            .workbook_mut()?
            .set_formula(reference, formula)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&cell).map_err(other_err_to_js)
    }

    pub fn clear(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let cell = self
            .workbook_mut()?
            .clear(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&cell).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = createSheet)]
    pub fn create_sheet(&mut self, name: &str) -> Result<JsValue, JsValue> {
        let sheet = self
            .workbook_mut()?
            .create_sheet(name)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&sheet).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = renameSheet)]
    pub fn rename_sheet(&mut self, old_name: &str, new_name: &str) -> Result<(), JsValue> {
        self.workbook_mut()?
            .rename_sheet(old_name, new_name)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = deleteSheet)]
    pub fn delete_sheet(&mut self, name: &str) -> Result<(), JsValue> {
        self.workbook_mut()?
            .delete_sheet(name)
            .map_err(api_err_to_js)
    }

    pub fn recalculate(&mut self) -> Result<JsValue, JsValue> {
        let recalculated = self.workbook_mut()?.recalculate().map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&recalculated).map_err(other_err_to_js)
    }

    pub fn layout(&mut self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = parse_layout_options(options)?;
        let layout = self
            .workbook_mut()?
            .layout(options)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&layout).map_err(other_err_to_js)
    }

    pub fn save(&mut self) -> Result<Vec<u8>, JsValue> {
        self.workbook_mut()?.save_bytes().map_err(api_err_to_js)
    }

    pub fn dispose(&mut self) {
        self.workbook = None;
    }
}

impl WorkbookHandle {
    fn workbook_mut(&mut self) -> Result<&mut xlcore_api::Workbook, JsValue> {
        self.workbook
            .as_mut()
            .ok_or_else(|| other_err_to_js("workbook handle has been disposed"))
    }
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

fn parse_layout_options(options: JsValue) -> Result<xlcore_api::LayoutOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        Ok(xlcore_api::LayoutOptions::default())
    } else {
        serde_wasm_bindgen::from_value(options).map_err(other_err_to_js)
    }
}

fn cell_value_from_js(value: JsValue) -> Result<xlcore_api::CellValue, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(xlcore_api::CellValue::Blank);
    }
    if let Some(value) = value.as_bool() {
        return Ok(xlcore_api::CellValue::Boolean(value));
    }
    if let Some(value) = value.as_f64() {
        return Ok(xlcore_api::CellValue::Number(value));
    }
    if let Some(value) = value.as_string() {
        if value.starts_with('#') {
            return Ok(xlcore_api::CellValue::Error(value));
        }
        return Ok(xlcore_api::CellValue::String(value));
    }
    serde_wasm_bindgen::from_value(value).map_err(other_err_to_js)
}

fn api_err_to_js(err: xlcore_api::ApiError) -> JsValue {
    let js_err = js_sys::Error::new(&err.message);
    if let Ok(payload_val) = serde_wasm_bindgen::to_value(&err) {
        copy_object_entries_to_error(&js_err, payload_val);
    }
    js_err.into()
}

fn load_err_to_js(err: XlsxLoadError) -> JsValue {
    let payload = WasmErrorPayload::from_load_error(&err);
    let js_err = js_sys::Error::new(&payload.message);

    if let Ok(payload_val) = serde_wasm_bindgen::to_value(&payload) {
        copy_object_entries_to_error(&js_err, payload_val);
    }
    js_err.into()
}

fn other_err_to_js(err: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&err.to_string()).into()
}

fn copy_object_entries_to_error(js_err: &js_sys::Error, payload_val: JsValue) {
    if let Ok(entries) = js_sys::Object::entries(&payload_val.into()).dyn_into::<js_sys::Array>() {
        for entry in entries.iter() {
            let Ok(pair) = entry.dyn_into::<js_sys::Array>() else {
                continue;
            };
            let key = pair.get(0);
            let value = pair.get(1);
            let _ = js_sys::Reflect::set(js_err, &key, &value);
        }
    }
}
