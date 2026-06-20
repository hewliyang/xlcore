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

fn parse_csv_options(options: JsValue) -> Result<xlcore_tabular::CsvOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        Ok(Default::default())
    } else {
        serde_wasm_bindgen::from_value(options).map_err(other_err_to_js)
    }
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

fn parse_parquet_options(options: JsValue) -> Result<xlcore_tabular::ParquetOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        Ok(Default::default())
    } else {
        serde_wasm_bindgen::from_value(options).map_err(other_err_to_js)
    }
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

macro_rules! api_methods {
    ( $( { $($method:tt)* } )* ) => {
        $( api_methods!(@m $($method)*); )*
    };

    (@m $rust:ident as $js:literal ( $($args:tt)* ) -> $ret:tt) => {
        api_methods!(@emit $rust [ #[wasm_bindgen(js_name = $js)] ] $ret ( $($args)* ));
    };
    (@m $rust:ident ( $($args:tt)* ) -> $ret:tt) => {
        api_methods!(@emit $rust [] $ret ( $($args)* ));
    };

    (@emit $rust:ident [$($attr:tt)*] $ret:tt ( $($args:tt)* )) => {
        api_methods!(@munch $rust [$($attr)*] $ret {} {} {} ( $($args)* ));
    };

    (@munch $r:ident [$($a:tt)*] $ret:tt { $($sig:tt)* } { $($pre:tt)* } { $($call:tt)* } ()) => {
        api_methods!(@build $r [$($a)*] $ret { $($sig)* } { $($pre)* } { $($call)* });
    };

    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( s $n:ident $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: &str, } { $($pre)* } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( os $n:ident $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: Option<String>, } { $($pre)* let $n = $n.as_deref(); } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( u32 $n:ident $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: u32, } { $($pre)* } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( u8 $n:ident $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: u8, } { $($pre)* } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( usize $n:ident $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: usize, } { $($pre)* } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( f64 $n:ident $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: f64, } { $($pre)* } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( bool $n:ident $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: bool, } { $($pre)* } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( de $n:ident : $t:ty $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: JsValue, } { $($pre)* let $n: $t = serde_wasm_bindgen::from_value($n).map_err(other_err_to_js)?; } { $($call)* $n, } ( $($($rest)*)? ));
    };
    (@munch $r:ident [$($a:tt)*] $ret:tt {$($sig:tt)*} {$($pre:tt)*} {$($call:tt)*} ( deopt $n:ident : $t:ty $(, $($rest:tt)*)? )) => {
        api_methods!(@munch $r [$($a)*] $ret { $($sig)* $n: JsValue, } { $($pre)* let $n: $t = if $n.is_null() || $n.is_undefined() { Default::default() } else { serde_wasm_bindgen::from_value($n).map_err(other_err_to_js)? }; } { $($call)* $n, } ( $($($rest)*)? ));
    };

    (@build $r:ident [$($a:tt)*] json { $($sig:tt)* } { $($pre:tt)* } { $($call:tt)* }) => {
        #[wasm_bindgen]
        impl WorkbookHandle {
            $($a)*
            pub fn $r(&mut self, $($sig)*) -> Result<JsValue, JsValue> {
                $($pre)*
                let __ret = self.workbook_mut()?.$r($($call)*).map_err(api_err_to_js)?;
                serde_wasm_bindgen::to_value(&__ret).map_err(other_err_to_js)
            }
        }
    };
    (@build $r:ident [$($a:tt)*] unit { $($sig:tt)* } { $($pre:tt)* } { $($call:tt)* }) => {
        #[wasm_bindgen]
        impl WorkbookHandle {
            $($a)*
            pub fn $r(&mut self, $($sig)*) -> Result<(), JsValue> {
                $($pre)*
                self.workbook_mut()?.$r($($call)*).map_err(api_err_to_js)
            }
        }
    };
    (@build $r:ident [$($a:tt)*] bool { $($sig:tt)* } { $($pre:tt)* } { $($call:tt)* }) => {
        #[wasm_bindgen]
        impl WorkbookHandle {
            $($a)*
            pub fn $r(&mut self, $($sig)*) -> Result<bool, JsValue> {
                $($pre)*
                self.workbook_mut()?.$r($($call)*).map_err(api_err_to_js)
            }
        }
    };
}

// Hand-written bindings: constructors, custom JS-value marshaling, and the
// name-mismatched/slice-returning methods that the table can't express.
#[wasm_bindgen]
impl WorkbookHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WorkbookHandle, JsValue> {
        console_error_panic_hook::set_once();
        Ok(Self {
            workbook: Some(xlcore_api::Workbook::new().map_err(api_err_to_js)?),
        })
    }

    pub fn open(bytes: Vec<u8>) -> Result<WorkbookHandle, JsValue> {
        Ok(Self {
            workbook: Some(xlcore_api::Workbook::open_bytes(bytes).map_err(api_err_to_js)?),
        })
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

    #[wasm_bindgen(js_name = setValue)]
    pub fn set_value(
        &mut self,
        sheet: &str,
        reference: &str,
        value: JsValue,
    ) -> Result<JsValue, JsValue> {
        let value = cell_value_from_js(value)?;
        let cell = self
            .workbook_mut()?
            .set_value_in(sheet, reference, value)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&cell).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setRangeValues)]
    pub fn set_range_values(
        &mut self,
        sheet: &str,
        reference: &str,
        values: JsValue,
    ) -> Result<JsValue, JsValue> {
        let values = range_values_from_js(values)?;
        let range = self
            .workbook_mut()?
            .set_range_values_in(sheet, reference, values)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = appendRows)]
    pub fn append_rows(&mut self, sheet: &str, rows: JsValue) -> Result<JsValue, JsValue> {
        let rows = range_values_from_js(rows)?;
        let range = self
            .workbook_mut()?
            .append_rows(sheet, rows)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    #[wasm_bindgen(js_name = setRangeFormulas)]
    pub fn set_range_formulas(
        &mut self,
        sheet: &str,
        reference: &str,
        formulas: JsValue,
    ) -> Result<JsValue, JsValue> {
        let formulas = range_formulas_from_js(formulas)?;
        let range = self
            .workbook_mut()?
            .set_range_formulas_in(sheet, reference, formulas)
            .map_err(api_err_to_js)?;
        serde_wasm_bindgen::to_value(&range).map_err(other_err_to_js)
    }

    pub fn save(&mut self) -> Result<Vec<u8>, JsValue> {
        self.workbook_mut()?.save_bytes().map_err(api_err_to_js)
    }

    pub fn dispose(&mut self) {
        self.workbook = None;
    }
}

// Marshaling-only bindings generated from the declarative method table below.
// Each row is `{ rust_name [as "jsName"] ( args ) -> ret }`; arg kinds are
// `s` (&str), `os` (Option<String>), `u32`/`u8`/`usize`/`f64`/`bool`,
// `de name: Ty` (serde from JsValue), `deopt name: Ty` (defaulted on
// null/undefined); ret is `json`, `unit`, or `bool`. The generated code is
// pure marshaling + `api_err_to_js` — no semantics. A future pyo3/napi emitter
// consumes the same table by adding sibling `@build`/`@munch` backends.
api_methods! {
    { sheets () -> json }
    { get_cell_in as "getCell" (s sheet, s reference) -> json }
    { set_formula_in as "setFormula" (s sheet, s reference, s formula) -> json }
    { clear_in as "clear" (s sheet, s reference) -> json }
    { clear_with_in as "clearWith" (s sheet, s reference, de mode: xlcore_api::ClearMode) -> json }
    { get_range_in as "getRange" (s sheet, s reference) -> json }
    { dependencies_in as "dependencies" (s sheet, s reference) -> json }
    { precedents_in as "precedents" (s sheet, s reference) -> json }
    { dependents_in as "dependents" (s sheet, s reference) -> json }
    { parse_formula_references_in as "parseFormulaReferences" (s sheet, s anchor, s formula) -> json }
    { function_names as "functionNames" () -> json }
    { set_style_in as "setStyle" (s sheet, s reference, deopt patch: xlcore_api::StylePatch) -> json }
    { set_rich_text_in as "setRichText" (s sheet, s reference, de runs: Vec<xlcore_api::RichTextRun>) -> json }
    { merges (s sheet) -> json }
    { add_merge as "addMerge" (s sheet, s reference) -> json }
    { remove_merge as "removeMerge" (s sheet, s reference) -> json }
    { hyperlinks (s sheet) -> json }
    { set_hyperlink as "setHyperlink" (s sheet, s reference, deopt patch: xlcore_api::HyperlinkPatch) -> json }
    { remove_hyperlink as "removeHyperlink" (s sheet, s reference) -> json }
    { auto_filter as "autoFilter" (s sheet) -> json }
    { set_auto_filter as "setAutoFilter" (s sheet, s reference) -> json }
    { remove_auto_filter as "removeAutoFilter" (s sheet) -> json }
    { set_auto_filter_column as "setAutoFilterColumn" (s sheet, de patch: xlcore_api::AutoFilterColumnPatch) -> json }
    { remove_auto_filter_column as "removeAutoFilterColumn" (s sheet, u32 column_offset) -> json }
    { set_auto_filter_sort as "setAutoFilterSort" (s sheet, u32 column_offset, bool descending) -> unit }
    { remove_auto_filter_sort as "removeAutoFilterSort" (s sheet) -> unit }
    { comments (s sheet) -> json }
    { set_comment as "setComment" (s sheet, s reference, de patch: xlcore_api::CommentPatch) -> json }
    { remove_comment as "removeComment" (s sheet, s reference) -> json }
    { threaded_notes as "threadedNotes" (s sheet) -> json }
    { add_threaded_note as "addThreadedNote" (s sheet, s reference, de patch: xlcore_api::ThreadedNotePatch) -> json }
    { reply_threaded_note as "replyThreadedNote" (s parent_id, de patch: xlcore_api::ThreadedNotePatch) -> json }
    { remove_threaded_thread as "removeThreadedThread" (s sheet, s reference) -> json }
    { data_validations as "dataValidations" (s sheet) -> json }
    { set_data_validation as "setDataValidation" (s sheet, s reference, de patch: xlcore_api::DataValidationPatch) -> json }
    { remove_data_validation as "removeDataValidation" (s sheet, s reference) -> json }
    { conditional_formats as "conditionalFormats" (s sheet) -> json }
    { set_conditional_format as "setConditionalFormat" (s sheet, s reference, de patch: xlcore_api::ConditionalFormatRulePatch) -> json }
    { clear_conditional_formats as "clearConditionalFormats" (s sheet, s reference) -> json }
    { defined_names as "definedNames" () -> json }
    { set_defined_name as "setDefinedName" (de patch: xlcore_api::DefinedNamePatch) -> json }
    { remove_defined_name as "removeDefinedName" (s name, os scope) -> json }
    { named_styles as "namedStyles" () -> json }
    { set_named_style as "setNamedStyle" (de patch: xlcore_api::NamedStylePatch) -> json }
    { remove_named_style as "removeNamedStyle" (s name) -> json }
    { tables (os sheet) -> json }
    { set_table as "setTable" (s sheet, de patch: xlcore_api::TablePatch) -> json }
    { remove_table as "removeTable" (s name) -> json }
    { charts (os sheet) -> json }
    { set_chart as "setChart" (s sheet, de patch: xlcore_api::ChartPatch) -> json }
    { remove_chart as "removeChart" (s sheet, s id) -> json }
    { update_chart as "updateChart" (s sheet, s id, de update: xlcore_api::ChartUpdate) -> json }
    { chart_exs as "chartExs" (os sheet) -> json }
    { set_chart_ex as "setChartEx" (s sheet, de patch: xlcore_api::ChartExPatch) -> json }
    { update_chart_ex as "updateChartEx" (s sheet, s id, de update: xlcore_api::ChartExUpdate) -> json }
    { remove_chart_ex as "removeChartEx" (s sheet, s id) -> json }
    { pivots (os sheet) -> json }
    { set_pivot as "setPivot" (s sheet, de patch: xlcore_api::PivotPatch) -> json }
    { pivot_preview as "pivotPreview" (s sheet, de patch: xlcore_api::PivotPatch) -> json }
    { update_pivot as "updatePivot" (s sheet, s id, de update: xlcore_api::PivotUpdate) -> json }
    { remove_pivot as "removePivot" (s sheet, s id) -> json }
    { images (os sheet) -> json }
    { set_image as "setImage" (s sheet, de patch: xlcore_api::ImagePatch) -> json }
    { update_image as "updateImage" (s sheet, s id, de update: xlcore_api::ImageUpdate) -> json }
    { remove_image as "removeImage" (s sheet, s id) -> json }
    { shapes (os sheet) -> json }
    { set_shape as "setShape" (s sheet, de patch: xlcore_api::ShapePatch) -> json }
    { remove_shape as "removeShape" (s sheet, s id) -> json }
    { sparkline_groups as "sparklineGroups" (os sheet) -> json }
    { set_sparkline_group as "setSparklineGroup" (s sheet, de patch: xlcore_api::SparklineGroupPatch) -> json }
    { remove_sparkline_group as "removeSparklineGroup" (s sheet, s id) -> json }
    { properties () -> json }
    { set_properties as "setProperties" (de patch: xlcore_api::WorkbookPropertiesPatch) -> json }
    { calc_properties as "calcProperties" () -> json }
    { set_calc_properties as "setCalcProperties" (de patch: xlcore_api::CalcPropertiesPatch) -> json }
    { sheet_protection as "sheetProtection" (s sheet) -> json }
    { set_sheet_protection as "setSheetProtection" (s sheet, de patch: xlcore_api::SheetProtectionPatch) -> json }
    { remove_sheet_protection as "removeSheetProtection" (s sheet) -> json }
    { workbook_protection as "workbookProtection" () -> json }
    { set_workbook_protection as "setWorkbookProtection" (de patch: xlcore_api::WorkbookProtectionPatch) -> json }
    { remove_workbook_protection as "removeWorkbookProtection" () -> json }
    { page_setup as "pageSetup" (s sheet) -> json }
    { set_page_setup as "setPageSetup" (s sheet, de patch: xlcore_api::SheetPageSetupPatch) -> json }
    { remove_page_setup as "removePageSetup" (s sheet) -> json }
    { sheet_properties as "sheetProperties" (s sheet) -> json }
    { set_sheet_properties as "setSheetProperties" (s sheet, de patch: xlcore_api::SheetPropertiesPatch) -> json }
    { clear_range_in as "clearRange" (s sheet, s reference) -> json }
    { clear_range_with_in as "clearRangeWith" (s sheet, s reference, de mode: xlcore_api::ClearMode) -> json }
    { copy_range_in as "copyRange" (s src_sheet, s src_reference, s dst_sheet, s dst_reference) -> json }
    { move_range_in as "moveRange" (s src_sheet, s src_reference, s dst_sheet, s dst_reference) -> json }
    { fill_range_in as "fillRange" (s src_sheet, s src_reference, s dst_sheet, s dst_reference) -> json }
    { create_sheet as "createSheet" (s name) -> json }
    { rename_sheet as "renameSheet" (s old_name, s new_name) -> unit }
    { delete_sheet as "deleteSheet" (s name) -> unit }
    { move_sheet as "moveSheet" (s name, usize to_index) -> json }
    { set_row_height as "setRowHeight" (s sheet, u32 row, f64 height) -> unit }
    { set_row_visible as "setRowVisible" (s sheet, u32 row, bool visible) -> unit }
    { set_column_width as "setColumnWidth" (s sheet, u32 column, f64 width) -> unit }
    { auto_fit_column as "autoFitColumn" (s sheet, u32 column, deopt min_width: Option<f64>, deopt max_width: Option<f64>) -> json }
    { auto_fit_columns as "autoFitColumns" (s sheet, u32 start, u32 end, deopt min_width: Option<f64>, deopt max_width: Option<f64>) -> json }
    { set_column_visible as "setColumnVisible" (s sheet, u32 column, bool visible) -> unit }
    { insert_rows as "insertRows" (s sheet, u32 before, u32 count) -> unit }
    { delete_rows as "deleteRows" (s sheet, u32 start, u32 count) -> unit }
    { insert_columns as "insertColumns" (s sheet, u32 before, u32 count) -> unit }
    { delete_columns as "deleteColumns" (s sheet, u32 start, u32 count) -> unit }
    { group_rows as "groupRows" (s sheet, u32 start, u32 end, u8 level, bool collapsed) -> unit }
    { group_columns as "groupColumns" (s sheet, u32 start, u32 end, u8 level, bool collapsed) -> unit }
    { set_show_grid_lines as "setShowGridLines" (s sheet, bool visible) -> bool }
    { get_show_grid_lines as "getShowGridLines" (s sheet) -> bool }
    { set_freeze as "setFreeze" (s sheet, u32 frozen_rows, u32 frozen_columns) -> json }
    { get_freeze as "getFreeze" (s sheet) -> json }
    { set_active_sheet as "setActiveSheet" (s name) -> json }
    { set_sheet_visibility as "setSheetVisibility" (s name, de visibility: xlcore_api::SheetVisibility) -> json }
    { search (s query, deopt options: xlcore_api::SearchOptions) -> json }
    { recalculate (bool errors_only) -> json }
    { layout (deopt options: xlcore_api::LayoutOptions) -> json }
    { part_names as "partNames" () -> json }
    { get_part_xml as "getPartXml" (s name) -> json }
    { set_part_xml as "setPartXml" (s name, s xml) -> unit }
    { remove_part_xml as "removePartXml" (s name) -> json }
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
        let row_arr: js_sys::Array = row
            .dyn_into()
            .map_err(|_| other_err_to_js("range formulas must be a 2D array of strings or null"))?;
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
        return Ok(xlcore_api::CellValue::from_scalar_string(value));
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
