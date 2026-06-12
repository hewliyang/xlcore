use serde::{Deserialize, Serialize};

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
