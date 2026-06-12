use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum ClearMode {
    #[default]
    All,
    Values,
    Formulas,
    #[serde(alias = "formats")]
    Styles,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct FreezeInfo {
    pub sheet: String,
    pub frozen_rows: u32,
    pub frozen_columns: u32,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum SheetVisibility {
    Visible,
    Hidden,
    VeryHidden,
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

impl ApiCellValue {
    pub fn from_scalar_string(value: String) -> Self {
        if value.starts_with('#') {
            Self::Error(value)
        } else {
            Self::String(value)
        }
    }
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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum SearchTarget {
    #[default]
    Values,
    Formulas,
    Both,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum SearchMode {
    #[default]
    Substring,
    Exact,
    Wildcard,
    Regex,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum SearchHit {
    Value,
    Formula,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(default)]
    pub target: SearchTarget,
    #[serde(default)]
    pub mode: SearchMode,
    #[serde(default)]
    pub case_sensitive: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_results: Option<usize>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub sheet: String,
    pub reference: String,
    pub row: u32,
    pub column: u32,
    pub hit: SearchHit,
    pub matched: String,
    pub value: ApiCellValue,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_scalar_string_maps_hash_prefix_to_error() {
        assert_eq!(
            ApiCellValue::from_scalar_string("#DIV/0!".to_string()),
            ApiCellValue::Error("#DIV/0!".to_string())
        );
        assert_eq!(
            ApiCellValue::from_scalar_string("#NotAnError".to_string()),
            ApiCellValue::Error("#NotAnError".to_string())
        );
        assert_eq!(
            ApiCellValue::from_scalar_string("hello".to_string()),
            ApiCellValue::String("hello".to_string())
        );
        assert_eq!(
            ApiCellValue::from_scalar_string(String::new()),
            ApiCellValue::String(String::new())
        );
    }
}
