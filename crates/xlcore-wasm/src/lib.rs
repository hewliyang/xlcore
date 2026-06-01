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

    #[wasm_bindgen(js_name = warnings)]
    pub fn warnings(&mut self) -> Result<JsValue, JsValue> {
        let warnings = self.workbook_mut()?.warnings().to_vec();
        serde_wasm_bindgen::to_value(&warnings).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = takeWarnings)]
    pub fn take_warnings(&mut self) -> Result<JsValue, JsValue> {
        let warnings = self.workbook_mut()?.take_warnings();
        serde_wasm_bindgen::to_value(&warnings).map_err(other_err_to_js)
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

    #[wasm_bindgen(js_name = clearWith)]
    pub fn clear_with(&mut self, reference: &str, mode: JsValue) -> Result<JsValue, JsValue> {
        let mode: xlcore_api::ClearMode =
            serde_wasm_bindgen::from_value(mode).map_err(other_err_to_js)?;
        let cell = self
            .workbook_mut()?
            .clear_with(reference, mode)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&cell).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = getRange)]
    pub fn get_range(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let range = self
            .workbook_mut()?
            .get_range(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    pub fn dependencies(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .dependencies(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    pub fn precedents(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .precedents(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    pub fn dependents(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .dependents(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setRangeValues)]
    pub fn set_range_values(
        &mut self,
        reference: &str,
        values: JsValue,
    ) -> Result<JsValue, JsValue> {
        let values = range_values_from_js(values)?;
        let range = self
            .workbook_mut()?
            .set_range_values(reference, values)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setRangeFormulas)]
    pub fn set_range_formulas(
        &mut self,
        reference: &str,
        formulas: JsValue,
    ) -> Result<JsValue, JsValue> {
        let formulas = range_formulas_from_js(formulas)?;
        let range = self
            .workbook_mut()?
            .set_range_formulas(reference, formulas)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setStyle)]
    pub fn set_style(&mut self, reference: &str, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::StylePatch = if patch.is_null() || patch.is_undefined() {
            xlcore_api::StylePatch::default()
        } else {
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?
        };
        let range = self
            .workbook_mut()?
            .set_style(reference, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    pub fn merges(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let merges = self
            .workbook_mut()?
            .merges(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&merges).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = addMerge)]
    pub fn add_merge(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .add_merge(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeMerge)]
    pub fn remove_merge(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .remove_merge(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    pub fn hyperlinks(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .hyperlinks(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setHyperlink)]
    pub fn set_hyperlink(&mut self, reference: &str, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::HyperlinkPatch = if patch.is_null() || patch.is_undefined() {
            xlcore_api::HyperlinkPatch::default()
        } else {
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?
        };
        let info = self
            .workbook_mut()?
            .set_hyperlink(reference, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeHyperlink)]
    pub fn remove_hyperlink(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_hyperlink(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = autoFilter)]
    pub fn auto_filter(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .auto_filter(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setAutoFilter)]
    pub fn set_auto_filter(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .set_auto_filter(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeAutoFilter)]
    pub fn remove_auto_filter(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .remove_auto_filter(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setAutoFilterColumn)]
    pub fn set_auto_filter_column(
        &mut self,
        sheet: &str,
        patch: JsValue,
    ) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::AutoFilterColumnPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_auto_filter_column(sheet, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeAutoFilterColumn)]
    pub fn remove_auto_filter_column(
        &mut self,
        sheet: &str,
        column_offset: u32,
    ) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_auto_filter_column(sheet, column_offset)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    pub fn comments(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .comments(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setComment)]
    pub fn set_comment(&mut self, reference: &str, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::CommentPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_comment(reference, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeComment)]
    pub fn remove_comment(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_comment(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = threadedNotes)]
    pub fn threaded_notes(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .threaded_notes(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = addThreadedNote)]
    pub fn add_threaded_note(
        &mut self,
        reference: &str,
        patch: JsValue,
    ) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::ThreadedNotePatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .add_threaded_note(reference, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = replyThreadedNote)]
    pub fn reply_threaded_note(
        &mut self,
        parent_id: &str,
        patch: JsValue,
    ) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::ThreadedNotePatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .reply_threaded_note(parent_id, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeThreadedThread)]
    pub fn remove_threaded_thread(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_threaded_thread(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = dataValidations)]
    pub fn data_validations(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .data_validations(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setDataValidation)]
    pub fn set_data_validation(
        &mut self,
        reference: &str,
        patch: JsValue,
    ) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::DataValidationPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_data_validation(reference, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeDataValidation)]
    pub fn remove_data_validation(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_data_validation(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = conditionalFormats)]
    pub fn conditional_formats(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .conditional_formats(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setConditionalFormat)]
    pub fn set_conditional_format(
        &mut self,
        reference: &str,
        patch: JsValue,
    ) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::ConditionalFormatRulePatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_conditional_format(reference, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = clearConditionalFormats)]
    pub fn clear_conditional_formats(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .clear_conditional_formats(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = definedNames)]
    pub fn defined_names(&mut self) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .defined_names()
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setDefinedName)]
    pub fn set_defined_name(&mut self, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::DefinedNamePatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_defined_name(patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeDefinedName)]
    pub fn remove_defined_name(
        &mut self,
        name: &str,
        scope: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_defined_name(name, scope.as_deref())
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    pub fn tables(&mut self, sheet: Option<String>) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .tables(sheet.as_deref())
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setTable)]
    pub fn set_table(&mut self, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::TablePatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_table(patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeTable)]
    pub fn remove_table(&mut self, name: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_table(name)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    pub fn charts(&mut self, sheet: Option<String>) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .charts(sheet.as_deref())
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setChart)]
    pub fn set_chart(&mut self, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::ChartPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_chart(patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeChart)]
    pub fn remove_chart(&mut self, sheet: &str, id: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_chart(sheet, id)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    pub fn images(&mut self, sheet: Option<String>) -> Result<JsValue, JsValue> {
        let list = self
            .workbook_mut()?
            .images(sheet.as_deref())
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&list).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setImage)]
    pub fn set_image(&mut self, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::ImagePatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_image(patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeImage)]
    pub fn remove_image(&mut self, sheet: &str, id: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_image(sheet, id)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = properties)]
    pub fn properties(&mut self) -> Result<JsValue, JsValue> {
        let props = self.workbook_mut()?.properties().map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&props).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setProperties)]
    pub fn set_properties(&mut self, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::WorkbookPropertiesPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let props = self
            .workbook_mut()?
            .set_properties(patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&props).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = calcProperties)]
    pub fn calc_properties(&mut self) -> Result<JsValue, JsValue> {
        let calc = self
            .workbook_mut()?
            .calc_properties()
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&calc).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setCalcProperties)]
    pub fn set_calc_properties(&mut self, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::CalcPropertiesPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let calc = self
            .workbook_mut()?
            .set_calc_properties(patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&calc).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = sheetProtection)]
    pub fn sheet_protection(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .sheet_protection(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setSheetProtection)]
    pub fn set_sheet_protection(
        &mut self,
        sheet: &str,
        patch: JsValue,
    ) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::SheetProtectionPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_sheet_protection(sheet, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeSheetProtection)]
    pub fn remove_sheet_protection(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_sheet_protection(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = workbookProtection)]
    pub fn workbook_protection(&mut self) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .workbook_protection()
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setWorkbookProtection)]
    pub fn set_workbook_protection(&mut self, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::WorkbookProtectionPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_workbook_protection(patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removeWorkbookProtection)]
    pub fn remove_workbook_protection(&mut self) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_workbook_protection()
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = pageSetup)]
    pub fn page_setup(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .page_setup(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setPageSetup)]
    pub fn set_page_setup(&mut self, sheet: &str, patch: JsValue) -> Result<JsValue, JsValue> {
        let patch: xlcore_api::SheetPageSetupPatch =
            serde_wasm_bindgen::from_value(patch).map_err(other_err_to_js)?;
        let info = self
            .workbook_mut()?
            .set_page_setup(sheet, patch)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = removePageSetup)]
    pub fn remove_page_setup(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let removed = self
            .workbook_mut()?
            .remove_page_setup(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&removed).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = clearRange)]
    pub fn clear_range(&mut self, reference: &str) -> Result<JsValue, JsValue> {
        let range = self
            .workbook_mut()?
            .clear_range(reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = clearRangeWith)]
    pub fn clear_range_with(
        &mut self,
        reference: &str,
        mode: JsValue,
    ) -> Result<JsValue, JsValue> {
        let mode: xlcore_api::ClearMode =
            serde_wasm_bindgen::from_value(mode).map_err(other_err_to_js)?;
        let range = self
            .workbook_mut()?
            .clear_range_with(reference, mode)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = copyRange)]
    pub fn copy_range(
        &mut self,
        src_reference: &str,
        dst_reference: &str,
    ) -> Result<JsValue, JsValue> {
        let range = self
            .workbook_mut()?
            .copy_range(src_reference, dst_reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = fillRange)]
    pub fn fill_range(
        &mut self,
        src_reference: &str,
        dst_reference: &str,
    ) -> Result<JsValue, JsValue> {
        let range = self
            .workbook_mut()?
            .fill_range(src_reference, dst_reference)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
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

    #[wasm_bindgen(js_name = moveSheet)]
    pub fn move_sheet(&mut self, name: &str, to_index: usize) -> Result<JsValue, JsValue> {
        let sheet = self
            .workbook_mut()?
            .move_sheet(name, to_index)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&sheet).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setSheetVisibility)]
    pub fn set_sheet_visibility(
        &mut self,
        name: &str,
        visibility: &str,
    ) -> Result<JsValue, JsValue> {
        let visibility = match visibility {
            "visible" => xlcore_api::SheetVisibility::Visible,
            "hidden" => xlcore_api::SheetVisibility::Hidden,
            "veryHidden" => xlcore_api::SheetVisibility::VeryHidden,
            other => {
                return Err(other_err_to_js(format!(
                    "unknown sheet visibility: {other}"
                )))
            }
        };
        let sheet = self
            .workbook_mut()?
            .set_sheet_visibility(name, visibility)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&sheet).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setRowHeight)]
    pub fn set_row_height(
        &mut self,
        sheet: &str,
        row: u32,
        height: f64,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .set_row_height(sheet, row, height)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = setRowVisible)]
    pub fn set_row_visible(
        &mut self,
        sheet: &str,
        row: u32,
        visible: bool,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .set_row_visible(sheet, row, visible)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = setColumnWidth)]
    pub fn set_column_width(
        &mut self,
        sheet: &str,
        column: u32,
        width: f64,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .set_column_width(sheet, column, width)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = setColumnVisible)]
    pub fn set_column_visible(
        &mut self,
        sheet: &str,
        column: u32,
        visible: bool,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .set_column_visible(sheet, column, visible)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = insertRows)]
    pub fn insert_rows(
        &mut self,
        sheet: &str,
        before: u32,
        count: u32,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .insert_rows(sheet, before, count)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = deleteRows)]
    pub fn delete_rows(
        &mut self,
        sheet: &str,
        start: u32,
        count: u32,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .delete_rows(sheet, start, count)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = insertColumns)]
    pub fn insert_columns(
        &mut self,
        sheet: &str,
        before: u32,
        count: u32,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .insert_columns(sheet, before, count)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = deleteColumns)]
    pub fn delete_columns(
        &mut self,
        sheet: &str,
        start: u32,
        count: u32,
    ) -> Result<(), JsValue> {
        self.workbook_mut()?
            .delete_columns(sheet, start, count)
            .map_err(api_err_to_js)
    }

    #[wasm_bindgen(js_name = setFreeze)]
    pub fn set_freeze(
        &mut self,
        sheet: &str,
        frozen_rows: u32,
        frozen_columns: u32,
    ) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .set_freeze(sheet, frozen_rows, frozen_columns)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = getFreeze)]
    pub fn get_freeze(&mut self, sheet: &str) -> Result<JsValue, JsValue> {
        let info = self
            .workbook_mut()?
            .get_freeze(sheet)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&info).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setActiveSheet)]
    pub fn set_active_sheet(&mut self, name: &str) -> Result<JsValue, JsValue> {
        let sheet = self
            .workbook_mut()?
            .set_active_sheet(name)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&sheet).map_err(other_err_to_js)
    }

    pub fn search(&mut self, query: &str, options: JsValue) -> Result<JsValue, JsValue> {
        let options: xlcore_api::SearchOptions = if options.is_null() || options.is_undefined() {
            xlcore_api::SearchOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(other_err_to_js)?
        };
        let hits = self
            .workbook_mut()?
            .search(query, options)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&hits).map_err(other_err_to_js)
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

fn range_values_from_js(value: JsValue) -> Result<Vec<Vec<xlcore_api::CellValue>>, JsValue> {
    let rows: js_sys::Array = value
        .dyn_into()
        .map_err(|_| other_err_to_js("range values must be a 2D array of cells"))?;
    let mut out: Vec<Vec<xlcore_api::CellValue>> = Vec::with_capacity(rows.length() as usize);
    for row in rows.iter() {
        let row_arr: js_sys::Array = row
            .dyn_into()
            .map_err(|_| other_err_to_js("range values must be a 2D array of cells"))?;
        let mut row_out = Vec::with_capacity(row_arr.length() as usize);
        for cell in row_arr.iter() {
            row_out.push(cell_value_from_js(cell)?);
        }
        out.push(row_out);
    }
    Ok(out)
}

fn range_formulas_from_js(value: JsValue) -> Result<Vec<Vec<Option<String>>>, JsValue> {
    let rows: js_sys::Array = value
        .dyn_into()
        .map_err(|_| other_err_to_js("range formulas must be a 2D array of strings or null"))?;
    let mut out: Vec<Vec<Option<String>>> = Vec::with_capacity(rows.length() as usize);
    for row in rows.iter() {
        let row_arr: js_sys::Array = row.dyn_into().map_err(|_| {
            other_err_to_js("range formulas must be a 2D array of strings or null")
        })?;
        let mut row_out = Vec::with_capacity(row_arr.length() as usize);
        for cell in row_arr.iter() {
            if cell.is_null() || cell.is_undefined() {
                row_out.push(None);
            } else if let Some(text) = cell.as_string() {
                row_out.push(Some(text));
            } else {
                return Err(other_err_to_js(
                    "range formula entries must be strings or null",
                ));
            }
        }
        out.push(row_out);
    }
    Ok(out)
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
