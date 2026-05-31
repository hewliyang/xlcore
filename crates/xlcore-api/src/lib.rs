use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::Path;

use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::sdk::SdkPart;
use serde::{Deserialize, Serialize};
use xlcore_io::spreadsheetml as x;

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InvalidRef,
    MissingSheet,
    DuplicateSheet,
    InvalidSheetName,
    CannotDeleteLastSheet,
    OoxmlWriteError,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
}

impl ApiError {
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            sheet: None,
            reference: None,
            part: None,
        }
    }

    fn with_sheet(mut self, sheet: impl Into<String>) -> Self {
        self.sheet = Some(sheet.into());
        self
    }

    fn with_ref(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }
}

impl From<xlcore_io::XlsxLoadError> for ApiError {
    fn from(value: xlcore_io::XlsxLoadError) -> Self {
        let mut err = Self::new(ApiErrorCode::Other, value.to_string());
        if let xlcore_io::XlsxLoadError::Schema { part, .. } = value {
            err.part = Some(part);
        }
        err
    }
}

impl From<ooxmlsdk::common::SdkError> for ApiError {
    fn from(value: ooxmlsdk::common::SdkError) -> Self {
        Self::new(ApiErrorCode::Other, value.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self::new(ApiErrorCode::Other, value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetInfo {
    pub index: usize,
    pub id: u32,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub row_count: u32,
    pub column_count: u32,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum CellValue {
    Blank,
    String(String),
    Number(f64),
    Boolean(bool),
    Error(String),
}

impl From<&str> for CellValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for CellValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<f64> for CellValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<i32> for CellValue {
    fn from(value: i32) -> Self {
        Self::Number(value as f64)
    }
}

impl From<bool> for CellValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellInfo {
    pub sheet: String,
    pub reference: String,
    pub row: u32,
    pub column: u32,
    pub value: CellValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_index: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutOptions {
    pub sheet_index: Option<usize>,
    pub sheet_name: Option<String>,
}

impl From<LayoutOptions> for xlcore_export::ExtractOptions {
    fn from(value: LayoutOptions) -> Self {
        Self {
            sheet_index: value.sheet_index,
            sheet_name: value.sheet_name,
        }
    }
}

pub struct Workbook {
    doc: xlcore_io::SpreadsheetDocument,
    report: xlcore_io::LoadReport,
}

impl Workbook {
    pub fn new() -> Result<Self> {
        let (doc, report) = xlcore_io::open_bytes_with_report(blank_workbook_bytes()?)?;
        Ok(Self { doc, report })
    }

    pub fn open_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let (doc, report) = xlcore_io::open_bytes_with_report(bytes.into())?;
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

        let wb_part = self.doc.workbook_part()?.clone();
        let ws_part: WorksheetPart = wb_part.add_new_part_auto_id(&mut self.doc)?;
        ws_part.set_root_element(&mut self.doc, empty_worksheet())?;

        let relationship_id = ws_part
            .relationship_id()
            .ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::Other,
                    "new worksheet is missing relationship id",
                )
            })?
            .to_string();

        let workbook = wb_part.root_element_mut(&mut self.doc)?;
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

        let wb_part = self.doc.workbook_part()?.clone();
        let workbook = wb_part.root_element_mut(&mut self.doc)?;
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
        let wb_part = self.doc.workbook_part()?.clone();
        let workbook = wb_part.root_element_mut(&mut self.doc)?;
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
        let _ = wb_part.delete_part_by_id(&mut self.doc, relationship_id.as_str())?;
        self.normalize_active_sheet_after_delete(index as u32)?;
        Ok(())
    }

    pub fn get_cell(&mut self, reference: impl AsRef<str>) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let shared_strings = load_shared_strings(&mut self.doc);
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part.root_element(&mut self.doc)?;
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
        let ws = ws_part.root_element_mut(&mut self.doc)?;
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
        let ws = ws_part.root_element_mut(&mut self.doc)?;
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

    pub fn clear(&mut self, reference: impl AsRef<str>) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part.root_element_mut(&mut self.doc)?;
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
        xlcore_bridge::recalculate_doc_with_writeback(&mut self.doc).map_err(ApiError::from)
    }

    pub fn layout(&mut self, options: LayoutOptions) -> Result<xlcore_export::WorkbookLayout> {
        let options: xlcore_export::ExtractOptions = options.into();
        xlcore_export::extract_doc_with_options(&mut self.doc, &options).map_err(ApiError::from)
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
        let wb_part = self.doc.workbook_part()?;
        Ok(wb_part.root_element(&mut self.doc)?.sheets.x_sheet.clone())
    }

    fn active_sheet_index(&mut self) -> Result<Option<u32>> {
        let wb_part = self.doc.workbook_part()?;
        Ok(wb_part
            .root_element(&mut self.doc)?
            .book_views
            .as_ref()
            .and_then(|views| views.x_workbook_view.first())
            .and_then(|view| view.active_tab))
    }

    fn normalize_active_sheet_after_delete(&mut self, deleted_index: u32) -> Result<()> {
        let wb_part = self.doc.workbook_part()?.clone();
        let workbook = wb_part.root_element_mut(&mut self.doc)?;
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
        let wb_part = self.doc.workbook_part()?;
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
    let ws = part.root_element(doc)?;
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
    let wb_part = doc.workbook_part()?.clone();
    let workbook = wb_part.root_element_mut(doc)?;
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
