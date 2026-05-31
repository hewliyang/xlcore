use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InvalidRef,
    MissingSheet,
    DuplicateSheet,
    InvalidSheetName,
    CannotDeleteLastSheet,
    ShapeMismatch,
    UnsupportedStyle,
    MergeOverlap,
    OoxmlWriteError,
    Other,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct MergeInfo {
    pub sheet: String,
    pub reference: String,
    pub start_row: u32,
    pub start_column: u32,
    pub end_row: u32,
    pub end_column: u32,
    pub rows: u32,
    pub columns: u32,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct StylePatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<FontPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<FillPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<BorderPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<AlignmentPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_format: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct FontPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<UnderlinePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum UnderlinePatch {
    None,
    Single,
    Double,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct FillPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct BorderPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all: Option<BorderLinePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<BorderLinePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<BorderLinePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<BorderLinePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<BorderLinePatch>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct BorderLinePatch {
    pub style: BorderLineStyle,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum BorderLineStyle {
    #[default]
    None,
    Thin,
    Medium,
    Thick,
    Dashed,
    Dotted,
    Double,
    Hair,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal: Option<HorizontalAlign>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical: Option<VerticalAlign>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_rotation: Option<i32>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum HorizontalAlign {
    General,
    Left,
    Center,
    Right,
    Fill,
    Justify,
    CenterContinuous,
    Distributed,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
    Justify,
    Distributed,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

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

    pub fn with_sheet(mut self, sheet: impl Into<String>) -> Self {
        self.sheet = Some(sheet.into());
        self
    }

    pub fn with_ref(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct SheetInfo {
    pub index: usize,
    pub id: u32,
    pub name: String,
    #[cfg_attr(
        feature = "typescript",
        ts(type = "\"hidden\" | \"veryHidden\"", optional)
    )]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub row_count: u32,
    pub column_count: u32,
    pub active: bool,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum ApiCellValue {
    Blank,
    String(String),
    Number(f64),
    Boolean(bool),
    Error(String),
}

impl From<&str> for ApiCellValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for ApiCellValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<f64> for ApiCellValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<i32> for ApiCellValue {
    fn from(value: i32) -> Self {
        Self::Number(value as f64)
    }
}

impl From<bool> for ApiCellValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CellInfo {
    pub sheet: String,
    pub reference: String,
    pub row: u32,
    pub column: u32,
    pub value: ApiCellValue,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_index: Option<u32>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct RangeInfo {
    pub sheet: String,
    pub reference: String,
    pub start_row: u32,
    pub start_column: u32,
    pub end_row: u32,
    pub end_column: u32,
    pub rows: u32,
    pub columns: u32,
    pub values: Vec<Vec<ApiCellValue>>,
    pub formulas: Vec<Vec<Option<String>>>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct LayoutOptions {
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub sheet_index: Option<usize>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub sheet_name: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(
        export,
        export_to = "../../../packages/xlsx-preview/src/api-schema/",
        rename = "EngineCellValue"
    )
)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum EngineCellValue {
    Blank,
    String(String),
    Number(f64),
    Boolean(bool),
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct RecalcWorkbook {
    pub sheets: Vec<RecalcSheet>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct RecalcSheet {
    pub index: u32,
    pub name: String,
    pub cells: Vec<RecalcCell>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct RecalcCell {
    pub r: u32,
    pub c: u32,
    pub formula: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_value: Option<EngineCellValue>,
    pub value: EngineCellValue,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<FormulaFallback>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct FormulaFallback {
    pub kind: String,
    pub message: String,
}

impl RecalcWorkbook {
    pub fn sheet(&self, name: &str) -> Option<&RecalcSheet> {
        self.sheets.iter().find(|sheet| sheet.name == name)
    }

    pub fn cell(&self, sheet_name: &str, cell_ref: &str) -> Option<&RecalcCell> {
        let (r, c) = parse_a1(cell_ref)?;
        self.sheet(sheet_name)?.cell(r, c)
    }
}

impl RecalcSheet {
    pub fn cell(&self, r: u32, c: u32) -> Option<&RecalcCell> {
        self.cells.iter().find(|cell| cell.r == r && cell.c == c)
    }
}

fn parse_a1(reference: &str) -> Option<(u32, u32)> {
    let mut col = 0u32;
    let mut row = 0u32;
    let mut in_col = true;
    for ch in reference.chars() {
        if in_col && ch.is_ascii_alphabetic() {
            col = col * 26 + (ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1);
        } else if ch.is_ascii_digit() {
            in_col = false;
            row = row * 10 + (ch as u32 - b'0' as u32);
        } else {
            return None;
        }
    }
    (row > 0 && col > 0).then_some((row, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recalc_workbook_finds_cells_by_a1_reference() {
        let workbook = RecalcWorkbook {
            sheets: vec![RecalcSheet {
                index: 0,
                name: "Sheet1".to_string(),
                cells: vec![RecalcCell {
                    r: 2,
                    c: 3,
                    formula: "A1+B1".to_string(),
                    cached_value: None,
                    value: EngineCellValue::Number(7.0),
                    fallback: None,
                }],
            }],
        };

        assert_eq!(
            workbook.cell("Sheet1", "C2").map(|cell| &cell.value),
            Some(&EngineCellValue::Number(7.0))
        );
        assert!(workbook.cell("Sheet1", "2C").is_none());
    }
}
