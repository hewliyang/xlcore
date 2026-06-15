use super::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalFormat {
    pub ranges: Vec<Merge>,
    pub rules: Vec<CfRule>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfRule {
    pub priority: i32,

    pub kind: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scale: Option<CfColorScale>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_bar: Option<CfDataBar>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_set: Option<CfIconSet>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operands: Vec<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dxf_id: Option<u32>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub stop_if_true: bool,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub bottom: bool,

    #[serde(default, skip_serializing_if = "is_false")]
    pub percent: bool,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above_average: Option<bool>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub equal_average: bool,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub std_dev: Option<i32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_period: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Dxf {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_color: Option<Color>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strike: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_style: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_fmt: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vert_align: Option<String>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfColorScale {
    pub stops: Vec<CfColorScaleStop>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfDataBar {
    pub min: CfvoStop,
    pub max: CfvoStop,

    pub color: Color,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_color: Option<Color>,

    pub min_length_pct: u32,

    pub max_length_pct: u32,

    pub show_value: bool,

    pub gradient: bool,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfIconSet {
    pub icon_set: String,

    pub cfvos: Vec<CfvoStop>,

    pub show_value: bool,

    pub reverse: bool,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfvoStop {
    #[serde(rename = "type")]
    pub cfvo_type: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub val: Option<String>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfColorScaleStop {
    #[serde(rename = "type")]
    pub cfvo_type: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub val: Option<String>,
    pub color: Color,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Col {
    pub min: u32,
    pub max: u32,
    pub width_px: f32,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_index: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,

    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub outline_level: u8,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub index: u32,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_px: Option<f32>,
    pub cells: Vec<Cell>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_index: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,

    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub outline_level: u8,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub r: u32,

    pub c: u32,

    #[serde(rename = "type")]
    pub kind: String,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_index: Option<u32>,

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
pub struct Merge {
    pub r1: u32,
    pub c1: u32,
    pub r2: u32,
    pub c2: u32,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Freeze {
    pub top_row: u32,

    pub left_col: u32,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Styles {
    pub fonts: Vec<Font>,
    pub fills: Vec<Fill>,
    pub borders: Vec<Border>,

    pub cell_xfs: Vec<CellFormat>,
    pub num_fmts: Vec<NumberFormat>,
    pub default_font: String,
    pub default_font_size: f32,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Font {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
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
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_type: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg_color: Option<Color>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<Color>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gradient_stops: Vec<GradientStop>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_type: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_degree: Option<f64>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_left: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_right: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_top: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_bottom: Option<f64>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct GradientStop {
    pub position: f64,
    pub color: Color,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Border {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<BorderLine>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<BorderLine>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<BorderLine>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<BorderLine>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub diagonal_up: bool,

    #[serde(default, skip_serializing_if = "is_false")]
    pub diagonal_down: bool,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagonal: Option<BorderLine>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct BorderLine {
    pub style: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CellFormat {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_id: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_id: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_id: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_fmt_id: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horizontal_alignment: Option<String>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_alignment: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub wrap_text: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_rotation: Option<i32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub shrink_to_fit: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub justify_last_line: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_order: Option<u32>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct NumberFormat {
    pub id: u32,
    pub format_code: String,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Color {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rgb: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<u32>,

    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<f64>,
}
fn is_false(b: &bool) -> bool {
    !*b
}
fn is_zero_u8(n: &u8) -> bool {
    *n == 0
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ColumnarCells {
    pub count: u32,

    pub r: String,

    pub c: String,

    pub kind: String,

    pub value_idx: String,

    pub formula_idx: String,

    pub style_idx: String,

    pub runs_idx: String,

    pub row_ptr: String,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct RowMetaBlob {
    pub count: u32,

    pub index: String,

    pub height_px: String,

    pub style_idx: String,

    pub hidden: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub outline_level: String,
}
