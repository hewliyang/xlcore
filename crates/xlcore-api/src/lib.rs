mod styles;

use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::Path;

use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::sdk::SdkPart;
use xlcore_io::spreadsheetml as x;
pub use xlcore_types::{
    AlignmentPatch, ApiCellValue, ApiCellValue as CellValue, ApiError, ApiErrorCode,
    BorderLinePatch, BorderLineStyle, BorderPatch, CellInfo, FillPatch, FontPatch,
    HorizontalAlign, LayoutOptions, RangeInfo, SheetInfo, StylePatch, UnderlinePatch,
    VerticalAlign,
};

pub type Result<T> = std::result::Result<T, ApiError>;

fn load_err_to_api(value: xlcore_io::XlsxLoadError) -> ApiError {
    let mut err = ApiError::new(ApiErrorCode::Other, value.to_string());
    if let xlcore_io::XlsxLoadError::Schema { part, .. } = value {
        err.part = Some(part);
    }
    err
}

pub(crate) fn sdk_err_to_api(value: ooxmlsdk::common::SdkError) -> ApiError {
    ApiError::new(ApiErrorCode::Other, value.to_string())
}

fn anyhow_err_to_api(value: anyhow::Error) -> ApiError {
    ApiError::new(ApiErrorCode::Other, value.to_string())
}

fn extract_options_from_layout(value: LayoutOptions) -> xlcore_export::ExtractOptions {
    xlcore_export::ExtractOptions {
        sheet_index: value.sheet_index,
        sheet_name: value.sheet_name,
    }
}

pub struct Workbook {
    doc: xlcore_io::SpreadsheetDocument,
    report: xlcore_io::LoadReport,
}

impl Workbook {
    pub fn new() -> Result<Self> {
        let (doc, report) =
            xlcore_io::open_bytes_with_report(blank_workbook_bytes()?).map_err(load_err_to_api)?;
        Ok(Self { doc, report })
    }

    pub fn open_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let (doc, report) =
            xlcore_io::open_bytes_with_report(bytes.into()).map_err(load_err_to_api)?;
        Ok(Self { doc, report })
    }

    pub fn open_path(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)
            .map_err(|err| ApiError::new(ApiErrorCode::Other, err.to_string()))?;
        Self::open_bytes(bytes)
    }

    pub fn load_report(&self) -> &xlcore_io::LoadReport {
        &self.report
    }

    pub fn save_bytes(&self) -> Result<Vec<u8>> {
        self.doc
            .to_package_bytes()
            .map_err(|err| ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string()))
    }

    pub fn save_path(&self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path, self.save_bytes()?)
            .map_err(|err| ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string()))
    }

    pub fn sheets(&mut self) -> Result<Vec<SheetInfo>> {
        let active_tab = self.active_sheet_index()?;
        let sheet_entries = self.workbook_sheets()?;
        let ws_parts = self.worksheet_parts_by_relationship_id()?;
        let mut out = Vec::with_capacity(sheet_entries.len());

        for (index, sheet) in sheet_entries.iter().enumerate() {
            let (row_count, column_count) = ws_parts
                .get(sheet.id.as_str())
                .and_then(|part| sheet_dimensions(&mut self.doc, part).ok())
                .unwrap_or((0, 0));
            out.push(SheetInfo {
                index,
                id: sheet.sheet_id,
                name: sheet.name.as_str().to_string(),
                state: sheet.state.as_ref().and_then(sheet_state_name),
                row_count,
                column_count,
                active: active_tab == Some(index as u32),
            });
        }
        Ok(out)
    }

    pub fn create_sheet(&mut self, name: impl AsRef<str>) -> Result<SheetInfo> {
        let name = validate_sheet_name(name.as_ref())?;
        if self.sheet_exists(name)? {
            return Err(ApiError::new(
                ApiErrorCode::DuplicateSheet,
                format!("sheet already exists: {name}"),
            )
            .with_sheet(name));
        }

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let ws_part: WorksheetPart = wb_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        ws_part
            .set_root_element(&mut self.doc, empty_worksheet())
            .map_err(sdk_err_to_api)?;

        let relationship_id = ws_part
            .relationship_id()
            .ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::Other,
                    "new worksheet is missing relationship id",
                )
            })?
            .to_string();

        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let next_sheet_id = workbook
            .sheets
            .x_sheet
            .iter()
            .map(|sheet| sheet.sheet_id)
            .max()
            .unwrap_or(0)
            + 1;
        workbook.sheets.x_sheet.push(x::Sheet {
            name: name.to_string(),
            sheet_id: next_sheet_id,
            state: None,
            id: relationship_id,
            ..Default::default()
        });

        let index = workbook.sheets.x_sheet.len() - 1;
        Ok(SheetInfo {
            index,
            id: next_sheet_id,
            name: name.to_string(),
            state: None,
            row_count: 0,
            column_count: 0,
            active: self.active_sheet_index()? == Some(index as u32),
        })
    }

    pub fn rename_sheet(
        &mut self,
        old_name: impl AsRef<str>,
        new_name: impl AsRef<str>,
    ) -> Result<()> {
        let old_name = old_name.as_ref();
        let new_name = validate_sheet_name(new_name.as_ref())?;
        if old_name != new_name && self.sheet_exists(new_name)? {
            return Err(ApiError::new(
                ApiErrorCode::DuplicateSheet,
                format!("sheet already exists: {new_name}"),
            )
            .with_sheet(new_name));
        }

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(sheet) = workbook
            .sheets
            .x_sheet
            .iter_mut()
            .find(|sheet| sheet.name.as_str() == old_name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {old_name}"),
            )
            .with_sheet(old_name));
        };
        sheet.name = new_name.to_string();
        Ok(())
    }

    pub fn delete_sheet(&mut self, name: impl AsRef<str>) -> Result<()> {
        let name = name.as_ref();
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        if workbook.sheets.x_sheet.len() <= 1 {
            return Err(ApiError::new(
                ApiErrorCode::CannotDeleteLastSheet,
                "cannot delete the last worksheet",
            ));
        }
        let Some(index) = workbook
            .sheets
            .x_sheet
            .iter()
            .position(|sheet| sheet.name.as_str() == name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {name}"),
            )
            .with_sheet(name));
        };
        let relationship_id = workbook.sheets.x_sheet.remove(index).id;
        let _ = wb_part
            .delete_part_by_id(&mut self.doc, relationship_id.as_str())
            .map_err(sdk_err_to_api)?;
        self.normalize_active_sheet_after_delete(index as u32)?;
        Ok(())
    }

    pub fn get_cell(&mut self, reference: impl AsRef<str>) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let shared_strings = load_shared_strings(&mut self.doc);
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ws
            .x_sheet_data
            .x_row
            .iter()
            .find(|row| row.row_index == Some(cell_ref.row))
            .and_then(|row| {
                row.x_c.iter().find(|cell| {
                    cell.cell_reference
                        .as_ref()
                        .and_then(|r| xlcore_io::parse_a1(r.as_str()))
                        == Some((cell_ref.row, cell_ref.column))
                })
            });
        Ok(cell_info_from_cell(
            &cell_ref.sheet,
            cell_ref.row,
            cell_ref.column,
            cell,
            &shared_strings,
        ))
    }

    pub fn set_value(
        &mut self,
        reference: impl AsRef<str>,
        value: impl Into<CellValue>,
    ) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let value = value.into();
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ensure_cell(ws, cell_ref.row, cell_ref.column);
        set_cell_value(cell, &value);
        mark_formulas_stale(&mut self.doc)?;
        self.get_cell(cell_ref.full_reference())
    }

    pub fn set_formula(
        &mut self,
        reference: impl AsRef<str>,
        formula: impl AsRef<str>,
    ) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let formula = normalize_formula(formula.as_ref());
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ensure_cell(ws, cell_ref.row, cell_ref.column);
        cell.data_type = None;
        cell.inline_string = None;
        cell.cell_value = None;
        cell.cell_formula = Some(x::CellFormula {
            xml_content: Some(formula),
            ..Default::default()
        });
        mark_formulas_stale(&mut self.doc)?;
        self.get_cell(cell_ref.full_reference())
    }

    pub fn get_range(&mut self, reference: impl AsRef<str>) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        self.read_range(&range_ref)
    }

    pub fn set_range_values(
        &mut self,
        reference: impl AsRef<str>,
        values: Vec<Vec<CellValue>>,
    ) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        validate_matrix_shape(&values, &range_ref, "values")?;

        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for (r_off, row) in values.iter().enumerate() {
            let row_idx = range_ref.start_row + r_off as u32;
            for (c_off, value) in row.iter().enumerate() {
                let col_idx = range_ref.start_column + c_off as u32;
                let cell = ensure_cell(ws, row_idx, col_idx);
                set_cell_value(cell, value);
            }
        }
        mark_formulas_stale(&mut self.doc)?;
        self.read_range(&range_ref)
    }

    pub fn set_range_formulas(
        &mut self,
        reference: impl AsRef<str>,
        formulas: Vec<Vec<Option<String>>>,
    ) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        validate_matrix_shape(&formulas, &range_ref, "formulas")?;

        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for (r_off, row) in formulas.iter().enumerate() {
            let row_idx = range_ref.start_row + r_off as u32;
            for (c_off, formula) in row.iter().enumerate() {
                let col_idx = range_ref.start_column + c_off as u32;
                let cell = ensure_cell(ws, row_idx, col_idx);
                match formula {
                    Some(text) => {
                        let normalized = normalize_formula(text.as_str());
                        cell.data_type = None;
                        cell.inline_string = None;
                        cell.cell_value = None;
                        cell.cell_formula = Some(x::CellFormula {
                            xml_content: Some(normalized),
                            ..Default::default()
                        });
                    }
                    None => {
                        cell.data_type = None;
                        cell.inline_string = None;
                        cell.cell_value = None;
                        cell.cell_formula = None;
                    }
                }
            }
        }
        mark_formulas_stale(&mut self.doc)?;
        self.read_range(&range_ref)
    }

    pub fn set_style(
        &mut self,
        reference: impl AsRef<str>,
        patch: StylePatch,
    ) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        {
            let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
            let mut existing: HashMap<(u32, u32), Option<u32>> = HashMap::new();
            {
                let ws = ws_part
                    .root_element(&mut self.doc)
                    .map_err(sdk_err_to_api)?;
                for row in &ws.x_sheet_data.x_row {
                    let Some(row_idx) = row.row_index else { continue };
                    if row_idx < range_ref.start_row || row_idx > range_ref.end_row {
                        continue;
                    }
                    for cell in &row.x_c {
                        if let Some((r, c)) = cell
                            .cell_reference
                            .as_ref()
                            .and_then(|r| xlcore_io::parse_a1(r.as_str()))
                        {
                            if c >= range_ref.start_column && c <= range_ref.end_column {
                                existing.insert((r, c), cell.style_index);
                            }
                        }
                    }
                }
            }

            let mut resolved_indexes: HashMap<(u32, u32), u32> = HashMap::new();
            for row_idx in range_ref.start_row..=range_ref.end_row {
                for col_idx in range_ref.start_column..=range_ref.end_column {
                    let current = existing.get(&(row_idx, col_idx)).copied().flatten();
                    let new_idx = styles::resolve_style_index(&mut self.doc, current, &patch)?;
                    resolved_indexes.insert((row_idx, col_idx), new_idx);
                }
            }

            let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
            let ws = ws_part
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            for row_idx in range_ref.start_row..=range_ref.end_row {
                for col_idx in range_ref.start_column..=range_ref.end_column {
                    let cell = ensure_cell(ws, row_idx, col_idx);
                    if let Some(idx) = resolved_indexes.get(&(row_idx, col_idx)) {
                        cell.style_index = Some(*idx);
                    }
                }
            }
        }
        self.read_range(&range_ref)
    }

    pub fn clear_range(&mut self, reference: impl AsRef<str>) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for row_idx in range_ref.start_row..=range_ref.end_row {
            for col_idx in range_ref.start_column..=range_ref.end_column {
                let cell = ensure_cell(ws, row_idx, col_idx);
                cell.data_type = None;
                cell.inline_string = None;
                cell.cell_value = None;
                cell.cell_formula = None;
            }
        }
        mark_formulas_stale(&mut self.doc)?;
        self.read_range(&range_ref)
    }

    fn read_range(&mut self, range_ref: &ResolvedRangeRef) -> Result<RangeInfo> {
        let shared_strings = load_shared_strings(&mut self.doc);
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let rows = (range_ref.end_row - range_ref.start_row + 1) as usize;
        let cols = (range_ref.end_column - range_ref.start_column + 1) as usize;
        let mut values = vec![vec![CellValue::Blank; cols]; rows];
        let mut formulas: Vec<Vec<Option<String>>> = vec![vec![None; cols]; rows];
        for row in &ws.x_sheet_data.x_row {
            let Some(row_idx) = row.row_index else { continue };
            if row_idx < range_ref.start_row || row_idx > range_ref.end_row {
                continue;
            }
            let r_off = (row_idx - range_ref.start_row) as usize;
            for cell in &row.x_c {
                let Some((cr, cc)) = cell
                    .cell_reference
                    .as_ref()
                    .and_then(|r| xlcore_io::parse_a1(r.as_str()))
                else {
                    continue;
                };
                if cr != row_idx
                    || cc < range_ref.start_column
                    || cc > range_ref.end_column
                {
                    continue;
                }
                let c_off = (cc - range_ref.start_column) as usize;
                let raw_v = cell
                    .cell_value
                    .as_ref()
                    .and_then(|value| value.xml_content.as_deref());
                values[r_off][c_off] = read_cell_value(cell, raw_v, &shared_strings);
                formulas[r_off][c_off] = cell
                    .cell_formula
                    .as_ref()
                    .and_then(|formula| formula.xml_content.as_deref().map(str::to_string));
            }
        }
        Ok(RangeInfo {
            sheet: range_ref.sheet.clone(),
            reference: range_ref.range_reference(),
            start_row: range_ref.start_row,
            start_column: range_ref.start_column,
            end_row: range_ref.end_row,
            end_column: range_ref.end_column,
            rows: rows as u32,
            columns: cols as u32,
            values,
            formulas,
        })
    }

    pub fn clear(&mut self, reference: impl AsRef<str>) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ensure_cell(ws, cell_ref.row, cell_ref.column);
        cell.data_type = None;
        cell.inline_string = None;
        cell.cell_value = None;
        cell.cell_formula = None;
        mark_formulas_stale(&mut self.doc)?;
        self.get_cell(cell_ref.full_reference())
    }

    pub fn batch<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        f(self)
    }

    pub fn recalculate(&mut self) -> Result<xlcore_bridge::RecalcWorkbook> {
        xlcore_bridge::recalculate_doc_with_writeback(&mut self.doc).map_err(anyhow_err_to_api)
    }

    pub fn layout(&mut self, options: LayoutOptions) -> Result<xlcore_export::WorkbookLayout> {
        let options = extract_options_from_layout(options);
        xlcore_export::extract_doc_with_options(&mut self.doc, &options).map_err(anyhow_err_to_api)
    }

    pub fn recalculate_layout(
        &mut self,
        options: LayoutOptions,
    ) -> Result<(xlcore_bridge::RecalcWorkbook, xlcore_export::WorkbookLayout)> {
        let recalculated = self.recalculate()?;
        let layout = self.layout(options)?;
        Ok((recalculated, layout))
    }

    fn workbook_sheets(&mut self) -> Result<Vec<x::Sheet>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?;
        Ok(wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?
            .sheets
            .x_sheet
            .clone())
    }

    fn active_sheet_index(&mut self) -> Result<Option<u32>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?;
        Ok(wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?
            .book_views
            .as_ref()
            .and_then(|views| views.x_workbook_view.first())
            .and_then(|view| view.active_tab))
    }

    fn normalize_active_sheet_after_delete(&mut self, deleted_index: u32) -> Result<()> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(book_views) = workbook.book_views.as_mut() else {
            return Ok(());
        };
        let Some(view) = book_views.x_workbook_view.first_mut() else {
            return Ok(());
        };
        if let Some(active) = view.active_tab {
            view.active_tab = if active == deleted_index {
                Some(0)
            } else if active > deleted_index {
                Some(active - 1)
            } else {
                Some(active)
            };
        }
        Ok(())
    }

    fn worksheet_parts_by_relationship_id(&self) -> Result<HashMap<String, WorksheetPart>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?;
        Ok(wb_part
            .worksheet_parts(&self.doc)
            .filter_map(|part| {
                part.relationship_id()
                    .map(|id| (id.to_string(), part.clone()))
            })
            .collect())
    }

    fn worksheet_part_for_sheet(&mut self, sheet_name: &str) -> Result<WorksheetPart> {
        let workbook_sheets = self.workbook_sheets()?;
        let Some(sheet) = workbook_sheets
            .iter()
            .find(|sheet| sheet.name.as_str() == sheet_name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet_name}"),
            )
            .with_sheet(sheet_name));
        };
        let relationship_id = sheet.id.as_str().to_string();
        let ws_parts = self.worksheet_parts_by_relationship_id()?;
        ws_parts.get(&relationship_id).cloned().ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("worksheet part not found for sheet: {sheet_name}"),
            )
            .with_sheet(sheet_name)
        })
    }

    fn sheet_exists(&mut self, name: &str) -> Result<bool> {
        Ok(self
            .workbook_sheets()?
            .iter()
            .any(|sheet| sheet.name.as_str() == name))
    }

    fn default_sheet_name(&mut self) -> Result<String> {
        self.workbook_sheets()?
            .first()
            .map(|sheet| sheet.name.as_str().to_string())
            .ok_or_else(|| ApiError::new(ApiErrorCode::MissingSheet, "workbook has no worksheets"))
    }

    fn resolve_cell_ref(&mut self, reference: &str) -> Result<ResolvedCellRef> {
        let parsed = parse_cell_reference(reference)?;
        let sheet = match parsed.sheet {
            Some(sheet) => sheet,
            None => self.default_sheet_name()?,
        };
        Ok(ResolvedCellRef {
            sheet,
            row: parsed.row,
            column: parsed.column,
        })
    }

    fn resolve_range_ref(&mut self, reference: &str) -> Result<ResolvedRangeRef> {
        let parsed = parse_range_reference(reference)?;
        let sheet = match parsed.sheet {
            Some(sheet) => sheet,
            None => self.default_sheet_name()?,
        };
        Ok(ResolvedRangeRef {
            sheet,
            start_row: parsed.start_row,
            start_column: parsed.start_column,
            end_row: parsed.end_row,
            end_column: parsed.end_column,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedCellRef {
    sheet: Option<String>,
    row: u32,
    column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedCellRef {
    sheet: String,
    row: u32,
    column: u32,
}

impl ResolvedCellRef {
    fn cell_reference(&self) -> String {
        format!("{}{}", xlcore_io::col_label(self.column), self.row)
    }

    fn full_reference(&self) -> String {
        format!(
            "{}!{}",
            quote_sheet_name(&self.sheet),
            self.cell_reference()
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedRangeRef {
    sheet: Option<String>,
    start_row: u32,
    start_column: u32,
    end_row: u32,
    end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedRangeRef {
    sheet: String,
    start_row: u32,
    start_column: u32,
    end_row: u32,
    end_column: u32,
}

impl ResolvedRangeRef {
    fn range_reference(&self) -> String {
        format!(
            "{}{}:{}{}",
            xlcore_io::col_label(self.start_column),
            self.start_row,
            xlcore_io::col_label(self.end_column),
            self.end_row,
        )
    }
}

fn parse_range_reference(reference: &str) -> Result<ParsedRangeRef> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            "range reference is empty",
        ));
    }
    let (sheet, cells) = split_sheet_reference(reference)?;
    let (start_cell, end_cell) = match cells.split_once(':') {
        Some((a, b)) => (a, b),
        None => (cells, cells),
    };
    if start_cell.is_empty() || end_cell.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid range reference: {reference}"),
        )
        .with_ref(reference));
    }
    let (mut r1, mut c1) = parse_cell_address(start_cell).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid range reference: {reference}"),
        )
        .with_ref(reference)
    })?;
    let (mut r2, mut c2) = parse_cell_address(end_cell).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid range reference: {reference}"),
        )
        .with_ref(reference)
    })?;
    if r1 > r2 {
        std::mem::swap(&mut r1, &mut r2);
    }
    if c1 > c2 {
        std::mem::swap(&mut c1, &mut c2);
    }
    Ok(ParsedRangeRef {
        sheet,
        start_row: r1,
        start_column: c1,
        end_row: r2,
        end_column: c2,
    })
}

fn validate_matrix_shape<T>(
    matrix: &[Vec<T>],
    range_ref: &ResolvedRangeRef,
    kind: &str,
) -> Result<()> {
    let expected_rows = (range_ref.end_row - range_ref.start_row + 1) as usize;
    let expected_cols = (range_ref.end_column - range_ref.start_column + 1) as usize;
    if matrix.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::ShapeMismatch,
            format!("{kind} matrix is empty"),
        )
        .with_ref(range_ref.range_reference())
        .with_sheet(&range_ref.sheet));
    }
    if matrix.len() != expected_rows {
        return Err(ApiError::new(
            ApiErrorCode::ShapeMismatch,
            format!(
                "{kind} matrix has {} rows but range expects {}",
                matrix.len(),
                expected_rows
            ),
        )
        .with_ref(range_ref.range_reference())
        .with_sheet(&range_ref.sheet));
    }
    for (idx, row) in matrix.iter().enumerate() {
        if row.len() != expected_cols {
            return Err(ApiError::new(
                ApiErrorCode::ShapeMismatch,
                format!(
                    "{kind} matrix row {} has {} cells but range expects {}",
                    idx,
                    row.len(),
                    expected_cols
                ),
            )
            .with_ref(range_ref.range_reference())
            .with_sheet(&range_ref.sheet));
        }
    }
    Ok(())
}

fn parse_cell_reference(reference: &str) -> Result<ParsedCellRef> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            "cell reference is empty",
        ));
    }
    let (sheet, cell) = split_sheet_reference(reference)?;
    let (row, column) = parse_cell_address(cell).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid cell reference: {reference}"),
        )
        .with_ref(reference)
    })?;
    Ok(ParsedCellRef { sheet, row, column })
}

fn split_sheet_reference(reference: &str) -> Result<(Option<String>, &str)> {
    if let Some(rest) = reference.strip_prefix('\'') {
        let mut sheet = String::new();
        let mut chars = rest.char_indices().peekable();
        while let Some((idx, ch)) = chars.next() {
            if ch == '\'' {
                if matches!(chars.peek(), Some((_, '\''))) {
                    sheet.push('\'');
                    let _ = chars.next();
                    continue;
                }
                let after_quote = &rest[idx + ch.len_utf8()..];
                let Some(cell) = after_quote.strip_prefix('!') else {
                    return Err(ApiError::new(
                        ApiErrorCode::InvalidRef,
                        format!("invalid sheet reference: {reference}"),
                    )
                    .with_ref(reference));
                };
                return Ok((Some(sheet), cell));
            }
            sheet.push(ch);
        }
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("unterminated sheet name in reference: {reference}"),
        )
        .with_ref(reference));
    }

    if let Some((sheet, cell)) = reference.rsplit_once('!') {
        if sheet.is_empty() || cell.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidRef,
                format!("invalid sheet reference: {reference}"),
            )
            .with_ref(reference));
        }
        return Ok((Some(sheet.to_string()), cell));
    }

    Ok((None, reference))
}

fn parse_cell_address(cell: &str) -> Option<(u32, u32)> {
    let mut chars = cell.chars().peekable();
    if matches!(chars.peek(), Some('$')) {
        let _ = chars.next();
    }

    let mut col = 0u32;
    let mut saw_col = false;
    while let Some(ch) = chars.peek().copied() {
        if !ch.is_ascii_alphabetic() {
            break;
        }
        saw_col = true;
        col = col
            .checked_mul(26)?
            .checked_add(ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1)?;
        let _ = chars.next();
    }

    if matches!(chars.peek(), Some('$')) {
        let _ = chars.next();
    }

    let mut row = 0u32;
    let mut saw_row = false;
    while let Some(ch) = chars.peek().copied() {
        if !ch.is_ascii_digit() {
            return None;
        }
        saw_row = true;
        row = row.checked_mul(10)?.checked_add(ch as u32 - b'0' as u32)?;
        let _ = chars.next();
    }

    if saw_col && saw_row && row > 0 && col > 0 {
        Some((row, col))
    } else {
        None
    }
}

fn quote_sheet_name(sheet: &str) -> String {
    if sheet
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return sheet.to_string();
    }
    format!("'{}'", sheet.replace('\'', "''"))
}

fn validate_sheet_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.len() > 31 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidSheetName,
            "sheet names must be 1 to 31 characters",
        )
        .with_sheet(name));
    }
    if name
        .chars()
        .any(|ch| matches!(ch, ':' | '\\' | '/' | '?' | '*' | '[' | ']'))
    {
        return Err(ApiError::new(
            ApiErrorCode::InvalidSheetName,
            format!("invalid sheet name: {name}"),
        )
        .with_sheet(name));
    }
    Ok(name)
}

fn empty_worksheet() -> x::Worksheet {
    x::Worksheet {
        x_sheet_data: Box::new(x::SheetData::default()),
        ..Default::default()
    }
}

fn ensure_cell(ws: &mut x::Worksheet, row: u32, column: u32) -> &mut x::Cell {
    let row_pos = match ws
        .x_sheet_data
        .x_row
        .binary_search_by_key(&row, |existing| existing.row_index.unwrap_or(u32::MAX))
    {
        Ok(pos) => pos,
        Err(pos) => {
            ws.x_sheet_data.x_row.insert(
                pos,
                x::Row {
                    row_index: Some(row),
                    ..Default::default()
                },
            );
            pos
        }
    };

    let row_ref = &mut ws.x_sheet_data.x_row[row_pos];
    let cell_ref = format!("{}{}", xlcore_io::col_label(column), row);
    let cell_pos = match row_ref.x_c.binary_search_by_key(&column, |existing| {
        existing
            .cell_reference
            .as_ref()
            .and_then(|r| xlcore_io::parse_a1(r.as_str()))
            .map(|(_, c)| c)
            .unwrap_or(u32::MAX)
    }) {
        Ok(pos) => pos,
        Err(pos) => {
            row_ref.x_c.insert(
                pos,
                x::Cell {
                    cell_reference: Some(cell_ref),
                    ..Default::default()
                },
            );
            pos
        }
    };
    &mut row_ref.x_c[cell_pos]
}

fn set_cell_value(cell: &mut x::Cell, value: &CellValue) {
    cell.cell_formula = None;
    cell.inline_string = None;
    match value {
        CellValue::Blank => {
            cell.data_type = None;
            cell.cell_value = None;
        }
        CellValue::String(value) => {
            cell.data_type = Some(x::CellValues::InlineString);
            cell.cell_value = None;
            cell.inline_string = Some(Box::new(x::InlineString {
                text: Some(x::Text {
                    xml_content: Some(value.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }));
        }
        CellValue::Number(value) => {
            cell.data_type = None;
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(format_number(*value)),
                ..Default::default()
            });
        }
        CellValue::Boolean(value) => {
            cell.data_type = Some(x::CellValues::Boolean);
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(if *value { "1" } else { "0" }.to_string()),
                ..Default::default()
            });
        }
        CellValue::Error(value) => {
            cell.data_type = Some(x::CellValues::Error);
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(value.clone()),
                ..Default::default()
            });
        }
    }
}

fn cell_info_from_cell(
    sheet: &str,
    row: u32,
    column: u32,
    cell: Option<&x::Cell>,
    shared_strings: &[String],
) -> CellInfo {
    let reference = format!("{}{}", xlcore_io::col_label(column), row);
    let Some(cell) = cell else {
        return CellInfo {
            sheet: sheet.to_string(),
            reference,
            row,
            column,
            value: CellValue::Blank,
            formula: None,
            style_index: None,
        };
    };
    let raw_v = cell
        .cell_value
        .as_ref()
        .and_then(|value| value.xml_content.as_deref());
    CellInfo {
        sheet: sheet.to_string(),
        reference,
        row,
        column,
        value: read_cell_value(cell, raw_v, shared_strings),
        formula: cell
            .cell_formula
            .as_ref()
            .and_then(|formula| formula.xml_content.as_deref().map(str::to_string)),
        style_index: cell.style_index,
    }
}

fn read_cell_value(cell: &x::Cell, raw_v: Option<&str>, shared_strings: &[String]) -> CellValue {
    match cell.data_type {
        Some(x::CellValues::SharedString) => raw_v
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|idx| shared_strings.get(idx).cloned())
            .map(CellValue::String)
            .unwrap_or_else(|| CellValue::String(String::new())),
        Some(x::CellValues::InlineString) => CellValue::String(inline_string_text(cell)),
        Some(x::CellValues::Boolean) => {
            CellValue::Boolean(matches!(raw_v, Some("1" | "true" | "TRUE")))
        }
        Some(x::CellValues::Error) => CellValue::Error(raw_v.unwrap_or("#ERROR!").to_string()),
        Some(x::CellValues::String) => CellValue::String(raw_v.unwrap_or("").to_string()),
        _ => raw_v
            .and_then(|value| value.parse::<f64>().ok())
            .map(CellValue::Number)
            .unwrap_or(CellValue::Blank),
    }
}

fn inline_string_text(cell: &x::Cell) -> String {
    let Some(inline) = cell.inline_string.as_ref() else {
        return String::new();
    };
    if !inline.x_r.is_empty() {
        let mut out = String::new();
        for run in &inline.x_r {
            out.push_str(run.text.xml_content.as_deref().unwrap_or(""));
        }
        return out;
    }
    inline
        .text
        .as_ref()
        .and_then(|text| text.xml_content.as_deref())
        .unwrap_or("")
        .to_string()
}

fn load_shared_strings(doc: &mut xlcore_io::SpreadsheetDocument) -> Vec<String> {
    let Ok(wb_part) = doc.workbook_part() else {
        return Vec::new();
    };
    let Some(sst_part) = wb_part.shared_string_table_part(doc) else {
        return Vec::new();
    };
    let Ok(sst) = sst_part.root_element(doc) else {
        return Vec::new();
    };
    let mut strings = Vec::with_capacity(sst.x_si.len());
    for item in &sst.x_si {
        if let Some(text) = &item.text {
            strings.push(text.xml_content.as_deref().unwrap_or("").to_string());
            continue;
        }
        let mut out = String::new();
        for run in &item.x_r {
            out.push_str(run.text.xml_content.as_deref().unwrap_or(""));
        }
        strings.push(out);
    }
    strings
}

fn sheet_dimensions(
    doc: &mut xlcore_io::SpreadsheetDocument,
    part: &WorksheetPart,
) -> Result<(u32, u32)> {
    let ws = part.root_element(doc).map_err(sdk_err_to_api)?;
    let mut rows = 0;
    let mut cols = 0;
    for row in &ws.x_sheet_data.x_row {
        let row_idx = row.row_index.unwrap_or(0);
        rows = rows.max(row_idx);
        for cell in &row.x_c {
            if let Some((_, c)) = cell
                .cell_reference
                .as_ref()
                .and_then(|reference| xlcore_io::parse_a1(reference.as_str()))
            {
                cols = cols.max(c);
            }
        }
    }
    Ok((rows, cols))
}

fn sheet_state_name(state: &x::SheetStateValues) -> Option<String> {
    match state {
        x::SheetStateValues::Visible => None,
        x::SheetStateValues::Hidden => Some("hidden".to_string()),
        x::SheetStateValues::VeryHidden => Some("veryHidden".to_string()),
    }
}

fn normalize_formula(formula: &str) -> String {
    formula
        .trim()
        .strip_prefix('=')
        .unwrap_or(formula.trim())
        .to_string()
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

fn mark_formulas_stale(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<()> {
    let wb_part = doc.workbook_part().map_err(sdk_err_to_api)?.clone();
    let workbook = wb_part.root_element_mut(doc).map_err(sdk_err_to_api)?;
    let calc = workbook
        .calculation_properties
        .get_or_insert_with(Default::default);
    calc.full_calculation_on_load = Some(true);
    calc.force_full_calculation = Some(true);
    calc.calculation_completed = Some(false);
    Ok(())
}

fn blank_workbook_bytes() -> Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        zip.start_file("[Content_Types].xml", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("_rels", options).map_err(zip_err)?;
        zip.start_file("_rels/.rels", options).map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("xl", options).map_err(zip_err)?;
        zip.start_file("xl/workbook.xml", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <bookViews><workbookView activeTab="0"/></bookViews>
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
  <calcPr calcMode="auto" fullCalcOnLoad="1" forceFullCalc="1"/>
</workbook>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("xl/_rels", options).map_err(zip_err)?;
        zip.start_file("xl/_rels/workbook.xml.rels", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("xl/worksheets", options)
            .map_err(zip_err)?;
        zip.start_file("xl/worksheets/sheet1.xml", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
</worksheet>"#,
        )
        .map_err(zip_err)?;

        zip.finish().map_err(zip_err)?;
    }
    Ok(buffer.into_inner())
}

fn zip_err(err: impl std::fmt::Display) -> ApiError {
    ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cell_references() {
        assert_eq!(
            parse_cell_reference("'Q1 Inputs'!$B$12").unwrap(),
            ParsedCellRef {
                sheet: Some("Q1 Inputs".to_string()),
                row: 12,
                column: 2,
            }
        );
        assert_eq!(
            parse_cell_reference("AA10").unwrap(),
            ParsedCellRef {
                sheet: None,
                row: 10,
                column: 27,
            }
        );
    }

    #[test]
    fn creates_sets_recalculates_saves_and_reopens() {
        let mut workbook = Workbook::new().unwrap();
        assert_eq!(workbook.sheets().unwrap()[0].name, "Sheet1");

        workbook.set_value("Sheet1!A1", "Units").unwrap();
        workbook.set_value("Sheet1!B1", 10.0).unwrap();
        workbook.set_formula("Sheet1!C1", "=B1*2").unwrap();

        let recalc = workbook.recalculate().unwrap();
        assert_eq!(
            recalc.cell("Sheet1", "C1").unwrap().value,
            xlcore_engine::CellValue::Number(20.0)
        );

        let bytes = workbook.save_bytes().unwrap();
        let mut reopened = Workbook::open_bytes(bytes).unwrap();
        assert_eq!(
            reopened.get_cell("Sheet1!A1").unwrap().value,
            CellValue::String("Units".to_string())
        );
        assert_eq!(
            reopened.get_cell("Sheet1!C1").unwrap().value,
            CellValue::Number(20.0)
        );
        assert_eq!(
            reopened.get_cell("Sheet1!C1").unwrap().formula.as_deref(),
            Some("B1*2")
        );
    }

    #[test]
    fn creates_and_renames_sheets() {
        let mut workbook = Workbook::new().unwrap();
        workbook.create_sheet("Scenario").unwrap();
        workbook.rename_sheet("Scenario", "Inputs").unwrap();
        workbook.set_value("Inputs!A1", "ok").unwrap();

        let sheets = workbook.sheets().unwrap();
        assert_eq!(
            sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["Sheet1", "Inputs"]
        );

        workbook.delete_sheet("Sheet1").unwrap();
        assert_eq!(workbook.sheets().unwrap()[0].name, "Inputs");
        assert_eq!(
            workbook.get_cell("Inputs!A1").unwrap().value,
            CellValue::String("ok".to_string())
        );
    }

    #[test]
    fn parses_range_references() {
        let plain = parse_range_reference("A1:B3").unwrap();
        assert_eq!(
            plain,
            ParsedRangeRef {
                sheet: None,
                start_row: 1,
                start_column: 1,
                end_row: 3,
                end_column: 2,
            }
        );
        let qualified = parse_range_reference("'Q1 Inputs'!$B$2:$C$4").unwrap();
        assert_eq!(
            qualified,
            ParsedRangeRef {
                sheet: Some("Q1 Inputs".to_string()),
                start_row: 2,
                start_column: 2,
                end_row: 4,
                end_column: 3,
            }
        );
        let single = parse_range_reference("Sheet1!C5").unwrap();
        assert_eq!(single.start_row, 5);
        assert_eq!(single.end_row, 5);
        assert_eq!(single.start_column, 3);
        assert_eq!(single.end_column, 3);
        let reversed = parse_range_reference("B3:A1").unwrap();
        assert_eq!(reversed.start_row, 1);
        assert_eq!(reversed.end_row, 3);
        assert_eq!(reversed.start_column, 1);
        assert_eq!(reversed.end_column, 2);

        assert!(parse_range_reference("").is_err());
        assert!(parse_range_reference("A1:").is_err());
        assert!(parse_range_reference(":B2").is_err());
        assert!(parse_range_reference("NOT_A_REF").is_err());
    }

    #[test]
    fn range_round_trip_values_formulas_and_clear() {
        let mut workbook = Workbook::new().unwrap();
        workbook
            .set_range_values(
                "Sheet1!A1:B2",
                vec![
                    vec![CellValue::String("Region".into()), CellValue::String("Units".into())],
                    vec![CellValue::String("North".into()), CellValue::Number(10.0)],
                ],
            )
            .unwrap();
        workbook
            .set_range_formulas(
                "Sheet1!C1:C2",
                vec![vec![None], vec![Some("=B2*2".to_string())]],
            )
            .unwrap();

        let range = workbook.get_range("Sheet1!A1:C2").unwrap();
        assert_eq!(range.rows, 2);
        assert_eq!(range.columns, 3);
        assert_eq!(range.reference, "A1:C2");
        assert_eq!(
            range.values[0][0],
            CellValue::String("Region".to_string())
        );
        assert_eq!(range.values[1][1], CellValue::Number(10.0));
        assert_eq!(range.formulas[0][2], None);
        assert_eq!(range.formulas[1][2].as_deref(), Some("B2*2"));

        let recalc = workbook.recalculate().unwrap();
        assert_eq!(
            recalc.cell("Sheet1", "C2").unwrap().value,
            xlcore_engine::CellValue::Number(20.0)
        );

        let bytes = workbook.save_bytes().unwrap();
        let mut reopened = Workbook::open_bytes(bytes).unwrap();
        let reread = reopened.get_range("Sheet1!A1:C2").unwrap();
        assert_eq!(reread.values[1][1], CellValue::Number(10.0));
        assert_eq!(reread.formulas[1][2].as_deref(), Some("B2*2"));
        assert_eq!(reread.values[1][2], CellValue::Number(20.0));

        let cleared = reopened.clear_range("Sheet1!A1:C2").unwrap();
        assert!(cleared.values.iter().flatten().all(|v| matches!(v, CellValue::Blank)));
        assert!(cleared.formulas.iter().flatten().all(|f| f.is_none()));
    }

    #[test]
    fn range_shape_mismatch_is_diagnosed() {
        let mut workbook = Workbook::new().unwrap();
        let err = workbook
            .set_range_values(
                "Sheet1!A1:B2",
                vec![vec![CellValue::Number(1.0), CellValue::Number(2.0)]],
            )
            .unwrap_err();
        assert_eq!(err.code, ApiErrorCode::ShapeMismatch);
        assert_eq!(err.reference.as_deref(), Some("A1:B2"));
        assert_eq!(err.sheet.as_deref(), Some("Sheet1"));
    }

    #[test]
    fn set_style_applies_font_fill_border_align_and_numfmt() {
        let mut workbook = Workbook::new().unwrap();
        workbook.set_value("Sheet1!A1", "Hello").unwrap();
        workbook.set_value("Sheet1!B1", 1234.5).unwrap();

        let patch = StylePatch {
            font: Some(FontPatch {
                bold: Some(true),
                color: Some("#FF0000".to_string()),
                size: Some(14.0),
                ..Default::default()
            }),
            fill: Some(FillPatch {
                color: Some("E2F0D9".to_string()),
            }),
            border: Some(BorderPatch {
                all: Some(BorderLinePatch {
                    style: BorderLineStyle::Thin,
                    color: Some("000000".to_string()),
                }),
                ..Default::default()
            }),
            alignment: Some(AlignmentPatch {
                horizontal: Some(HorizontalAlign::Center),
                wrap: Some(true),
                ..Default::default()
            }),
            number_format: Some("#,##0.00".to_string()),
        };
        workbook.set_style("Sheet1!A1:B1", patch).unwrap();
        let a1 = workbook.get_cell("Sheet1!A1").unwrap();
        let b1 = workbook.get_cell("Sheet1!B1").unwrap();
        let idx_a = a1.style_index.unwrap();
        let idx_b = b1.style_index.unwrap();
        assert!(idx_a > 0);
        assert_eq!(idx_a, idx_b);

        let bytes = workbook.save_bytes().unwrap();
        let mut reopened = Workbook::open_bytes(bytes).unwrap();
        assert_eq!(
            reopened.get_cell("Sheet1!A1").unwrap().style_index,
            Some(idx_a)
        );

        let layout = reopened.layout(LayoutOptions::default()).unwrap();
        let xf = &layout.styles.cell_xfs[idx_a as usize];
        let font = &layout.styles.fonts[xf.font_id.unwrap() as usize];
        assert!(font.bold);
        assert_eq!(font.size, Some(14.0));
        assert_eq!(
            font.color.as_ref().and_then(|c| c.rgb.as_deref()),
            Some("FFFF0000")
        );
        let fill = &layout.styles.fills[xf.fill_id.unwrap() as usize];
        assert_eq!(fill.pattern_type.as_deref(), Some("solid"));
        assert_eq!(
            fill.fg_color.as_ref().and_then(|c| c.rgb.as_deref()),
            Some("FFE2F0D9")
        );
        assert!(xf.wrap_text);
        assert_eq!(xf.horizontal_alignment.as_deref(), Some("center"));
        let num_fmt_id = xf.num_fmt_id.unwrap();
        assert_eq!(num_fmt_id, 4);
    }

    #[test]
    fn set_style_dedupes_across_cells_and_invalid_color_errors() {
        let mut workbook = Workbook::new().unwrap();
        let bold = StylePatch {
            font: Some(FontPatch { bold: Some(true), ..Default::default() }),
            ..Default::default()
        };
        workbook.set_style("Sheet1!A1", bold.clone()).unwrap();
        workbook.set_style("Sheet1!B1", bold).unwrap();
        assert_eq!(
            workbook.get_cell("Sheet1!A1").unwrap().style_index,
            workbook.get_cell("Sheet1!B1").unwrap().style_index
        );

        let err = workbook
            .set_style(
                "Sheet1!A1",
                StylePatch {
                    fill: Some(FillPatch { color: Some("notacolor".into()) }),
                    ..Default::default()
                },
            )
            .unwrap_err();
        assert_eq!(err.code, ApiErrorCode::UnsupportedStyle);
    }

    #[test]
    fn layout_reflects_mutated_cells() {
        let mut workbook = Workbook::new().unwrap();
        workbook
            .batch(|tx| {
                tx.set_value("Sheet1!A1", "Label")?;
                tx.set_value("Sheet1!B1", 42.0)?;
                Ok(())
            })
            .unwrap();

        let layout = workbook.layout(LayoutOptions::default()).unwrap();
        let sheet = &layout.sheets[0];
        assert_eq!(sheet.max_row, 1);
        assert_eq!(sheet.max_col, 2);
        assert_eq!(sheet.cells.count, 2);
        assert!(sheet.value_pool.iter().any(|value| value == "Label"));
        assert!(sheet.value_pool.iter().any(|value| value == "42"));
    }
}
