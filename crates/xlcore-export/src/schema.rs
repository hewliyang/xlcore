use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool {
    !*b
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookLayout {
    pub sheets: Vec<Sheet>,
    pub styles: Styles,
    pub shared_strings: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_string_runs: Vec<Vec<TextRun>>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dxfs: Vec<Dxf>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_styles: Vec<CustomTableStyle>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defined_names: Vec<DefinedName>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_sheet_index: Option<u32>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct DefinedName {
    pub name: String,

    pub formula: String,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_sheet_id: Option<u32>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub colors: Vec<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major_font: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minor_font: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct TextRun {
    pub text: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_style: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vert_align: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<u8>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub index: u32,
    pub name: String,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_color: Option<Color>,

    pub max_row: u32,
    pub max_col: u32,

    pub default_col_width_px: f32,

    pub default_row_height_px: f32,

    pub cols: Vec<Col>,

    #[cfg_attr(feature = "typescript", ts(skip))]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Row>,
    pub merges: Vec<Merge>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_filter_range: Option<Merge>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub freeze: Option<Freeze>,
    pub show_grid_lines: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditional_formats: Vec<ConditionalFormat>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drawings: Vec<Drawing>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<Table>,

    #[serde(default)]
    pub cells: ColumnarCells,

    #[serde(default)]
    pub row_meta: RowMetaBlob,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_pool: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formula_pool: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inline_runs: Vec<Vec<TextRun>>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hyperlinks: Vec<Hyperlink>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pivots: Vec<Pivot>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_pr: Option<OutlinePr>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sparkline_groups: Vec<SparklineGroup>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct SparklineGroup {
    pub spark_type: String,

    pub line_weight: f32,
    pub markers: bool,
    pub high: bool,
    pub low: bool,
    pub first: bool,
    pub last: bool,
    pub negative: bool,

    pub display_x_axis: bool,
    pub right_to_left: bool,

    pub display_empty_cells_as: String,

    pub min_axis_type: String,
    pub max_axis_type: String,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_min: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_max: Option<f64>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_min: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_max: Option<f64>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_series: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_negative: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_axis: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_markers: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_first: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_last: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_high: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_low: Option<String>,

    pub sparklines: Vec<Sparkline>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Sparkline {
    pub r: u32,

    pub c: u32,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,

    pub values: Vec<Option<f64>>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct OutlinePr {
    pub summary_below: bool,
    pub summary_right: bool,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Pivot {
    pub name: String,

    pub range: Merge,

    pub filter_arrow_cells: Vec<CellRef>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CellRef {
    pub r: u32,
    pub c: u32,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Hyperlink {
    pub range: Merge,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub r: u32,

    pub c: u32,

    pub author: String,

    pub text: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<TextRun>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub name: String,

    pub display_name: String,

    pub range: Merge,

    pub header_row_count: u32,

    pub totals_row_count: u32,

    pub columns: Vec<TableColumn>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<TableStyle>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub has_auto_filter: bool,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    pub name: String,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totals_row_function: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totals_row_label: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct TableStyle {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_first_column: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_last_column: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_row_stripes: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_column_stripes: bool,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CustomTableStyle {
    pub name: String,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whole_table: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_row: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_row: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_row_stripe: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second_row_stripe: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_column: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_column: Option<u32>,
}

mod charts;
pub use charts::*;

mod tail;
pub use tail::*;
