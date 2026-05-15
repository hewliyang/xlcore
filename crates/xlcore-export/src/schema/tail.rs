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
    /// One or more rectangular ranges (from the sqref attribute).
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
    /// `colorScale`, `dataBar`, `iconSet`, `cellIs`, `expression`, `top10`,
    /// `aboveAverage`, `containsText`, `notContainsText`, `beginsWith`,
    /// `endsWith`, `duplicateValues`, `uniqueValues`, `timePeriod`.
    /// `expression` still requires a formula engine; everything else
    /// is evaluated by the renderer using cell values + workbook stats.
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
    /// Comparison operator for `cellIs` rules: `equal`, `notEqual`,
    /// `greaterThan`, `greaterThanOrEqual`, `lessThan`,
    /// `lessThanOrEqual`, `between`, `notBetween`. None for non-cellIs.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    /// Operand formulas/literals. `cellIs` has 1 (most operators) or 2
    /// (between/notBetween); `expression` has the rule formula here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operands: Vec<String>,
    /// Index into `WorkbookLayout.dxfs` for the differential format to
    /// apply when this rule matches.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dxf_id: Option<u32>,
    /// If true, do not evaluate lower-priority rules on the same cell
    /// once this rule matches.
    #[serde(default, skip_serializing_if = "is_false")]
    pub stop_if_true: bool,

    // ---------- top10 / aboveAverage / containsText / timePeriod ----------
    /// `top10`: number of items (or percent) to highlight. Default 10.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    /// `top10`: when true, take the bottom-N instead of the top-N.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bottom: bool,
    /// `top10`: when true, `rank` is a percentage (0–100), not a count.
    #[serde(default, skip_serializing_if = "is_false")]
    pub percent: bool,
    /// `aboveAverage`: when false, the rule means *below* average.
    /// Default true. Only meaningful for `kind="aboveAverage"`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above_average: Option<bool>,
    /// `aboveAverage`: include cells equal to the average (default false).
    #[serde(default, skip_serializing_if = "is_false")]
    pub equal_average: bool,
    /// `aboveAverage`: when set, highlight cells whose distance from the
    /// average exceeds N standard deviations (positive N for above,
    /// negative for below).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub std_dev: Option<i32>,
    /// `containsText` / `notContainsText` / `beginsWith` / `endsWith`:
    /// the literal text to match.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// `timePeriod`: one of `yesterday`, `today`, `tomorrow`,
    /// `last7Days`, `lastWeek`, `thisWeek`, `nextWeek`, `lastMonth`,
    /// `thisMonth`, `nextMonth`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_period: Option<String>,
}

/// Differential format — a sparse style overlay applied on top of a
/// cell's base style when a CF rule matches. Mirrors `<x:dxf>` in
/// `xl/styles.xml`. Every field is optional; missing fields mean
/// "inherit from base".
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
    /// OOXML `<u val="..."/>` variant; see `Font.underline_style`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_style: Option<String>,
    /// Fill foreground color (solid pattern). Background is rare in dxfs.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    /// Override number-format code, e.g. `"0.00%"`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_fmt: Option<String>,
    /// `<vertAlign val="..."/>` override from a dxf font block. See
    /// `TextRun.vert_align`.
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

/// `dataBar` conditional-format rule. Fills each cell with a horizontal
/// bar whose length is proportional to `(value - min) / (max - min)`,
/// constrained to `[min_length_pct%, max_length_pct%]` of the cell
/// width. When the data range straddles zero the bar splits at the
/// origin: negatives paint `negative_color` leftward, positives paint
/// `color` rightward. Mirrors `<x:dataBar>` in worksheet XML; defaults
/// match ECMA-376 §18.3.1.28.
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
    /// Fill color for positive (or all) bar segments. Defaults to
    /// Excel's standard `#638EC6` blue when the source XML omits the
    /// `<color>` child (some writers do).
    pub color: Color,
    /// Fill color for negative bar segments. None ⇒ red `#FF0000`
    /// (Excel default), but renderer should only use it when the data
    /// range actually contains negatives.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_color: Option<Color>,
    /// Minimum bar length as percent of cell width (default 10).
    pub min_length_pct: u32,
    /// Maximum bar length as percent of cell width (default 90).
    pub max_length_pct: u32,
    /// When false, the cell value is hidden and only the bar paints.
    pub show_value: bool,
    /// When true (Excel 2010+ default), the bar fill paints as a
    /// linear gradient from `color` at the axis to transparent at
    /// the bar's outer edge. When false, paints as a solid block.
    /// Stored only on the x14 extension (`<x14:dataBar gradient="..."/>`),
    /// which we don't parse yet — defaults to `true` to match what
    /// modern Excel + SpreadJS author and what users see by default.
    pub gradient: bool,
}

/// `iconSet` conditional-format rule. Picks one icon from a named
/// preset (e.g. `3TrafficLights1`, `5Arrows`) per cell based on the
/// value's position in the user-supplied thresholds. Mirrors
/// `<x:iconSet>` in worksheet XML; ECMA-376 §18.3.1.49.
///
/// `cfvos.len()` always equals N (3/4/5) and the first stop is the
/// implicit "low" icon — the matching index for a value v is the
/// largest k such that v meets the threshold at cfvos[k]. Per spec
/// the comparison is `>=` by default and `>` when `gte=false` on the
/// stop, but legacy `<x:iconSet>` doesn't expose `gte` so we treat
/// every threshold as `>=`.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfIconSet {
    /// One of the OOXML preset IDs: `3Arrows`, `3ArrowsGray`,
    /// `3Flags`, `3TrafficLights1`, `3TrafficLights2`, `3Signs`,
    /// `3Symbols`, `3Symbols2`, `4Arrows`, `4ArrowsGray`,
    /// `4RedToBlack`, `4Rating`, `4TrafficLights`, `5Arrows`,
    /// `5ArrowsGray`, `5Rating`, `5Quarters`.
    pub icon_set: String,
    /// N stops (3, 4, or 5). `cfvos[0]` is the low-icon anchor
    /// (typically `percent 0`); subsequent stops define the
    /// thresholds for the higher icons.
    pub cfvos: Vec<CfvoStop>,
    /// When false, the cell value is hidden and only the icon paints.
    pub show_value: bool,
    /// When true, the icon order is reversed (high values get the
    /// first icon in the set).
    pub reverse: bool,
}

/// Conditional-format value object — the `<x:cfvo>` child of
/// `colorScale`/`dataBar`/`iconSet`. Color-scale CFVOs carry their own
/// color and live on `CfColorScaleStop`; data bars share this colorless
/// shape between the min and max stop.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfvoStop {
    /// `min`, `max`, `num`, `percent`, `percentile`, `formula`,
    /// `automin`, `automax`.
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
    /// `min`, `max`, `num`, `percent`, `percentile`, `formula`.
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
    /// OOXML `<col outlineLevel="N">` (0..=7). 0 = no grouping; the
    /// renderer paints a bracket above the column header(s) covering
    /// each contiguous run at level >= 1. Spec caps at 7.
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
    /// OOXML `<row outlineLevel="N">` (0..=7). Wire-only on this
    /// transient struct; gets folded into `RowMetaBlob.outline_level`
    /// during `compactify_sheet`. Always 0 in serialized JSON
    /// (Sheet.rows is `ts(skip)`-hidden).
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
    /// Row, 1-based.
    pub r: u32,
    /// Col, 1-based.
    pub c: u32,
    /// `n` numeric, `s` shared string (value is the index as a string),
    /// `inline` inline string, `b` boolean ("0"/"1"), `e` error, `str` plain
    /// string from a formula, `f` formula (cached value goes in `value`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Source-cached value (raw, unformatted). Numbers are decimal strings.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_index: Option<u32>,
    /// Rich-text runs for `inline` cells that carry `<r>` children. For
    /// shared-string cells, look up runs via `WorkbookLayout.shared_string_runs`
    /// using `value` as the SST index. Empty when the cell is plain text.
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
    /// 1-based: rows above this index are frozen.
    pub top_row: u32,
    /// 1-based: cols left of this index are frozen.
    pub left_col: u32,
}
// ============================================================
// Styles
// ============================================================
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
    /// `<cellXfs>` — direct cell formats. `Cell.styleIndex` indexes into this.
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
    /// OOXML `<u val="..."/>` variant when not the default `single`.
    /// See `TextRun.underline_style` for values + renderer behavior.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_style: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// OOXML `<vertAlign val="..."/>` on the cell font (see
    /// `TextRun.vert_align`). `"superscript"` / `"subscript"`; absent =
    /// `"baseline"` (default).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vert_align: Option<String>,
    /// OOXML `<family val="N"/>` — see `TextRun.family`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<u8>,
    /// OOXML `<scheme val="major|minor"/>` — see `TextRun.scheme`. When
    /// set, the renderer resolves the typeface from the workbook theme
    /// (`WorkbookLayout.theme.major_font` / `minor_font`) instead of the
    /// `<name>` cache stored on this font.
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
    /// "solid", "none", "pattern", "gradient".
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_type: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg_color: Option<Color>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<Color>,
    /// Gradient stops in source order. Each stop carries its OOXML
    /// `position` (0..1 along the gradient axis for `linear`, or 0..1
    /// from the inner convergence rect outward for `path`/radial).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gradient_stops: Vec<GradientStop>,
    /// `"linear"` (default) or `"path"` (radial-ish, with a rectangular
    /// inner-convergence region). Only meaningful when
    /// `pattern_type == "gradient"`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_type: Option<String>,
    /// Linear gradient angle in degrees (OOXML `degree`). 0 = left→right,
    /// 90 = top→bottom, 180 = right→left, 270 = bottom→top. Ignored when
    /// `gradient_type == "path"`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient_degree: Option<f64>,
    /// Path-gradient inner-convergence rectangle, expressed as fractions
    /// of cell width/height inset from each side (0..1). Defaults to 0
    /// when missing (rect collapses to a point at the relevant corner).
    /// Ignored unless `gradient_type == "path"`.
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
    /// Position along the gradient axis. 0..1.
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
    /// `<border diagonalUp="1">`: paint the `/` slash
    /// (bottom-left → top-right).
    #[serde(default, skip_serializing_if = "is_false")]
    pub diagonal_up: bool,
    /// `<border diagonalDown="1">`: paint the `\` slash
    /// (top-left → bottom-right).
    #[serde(default, skip_serializing_if = "is_false")]
    pub diagonal_down: bool,
    /// Style + color for whichever diagonal(s) `diagonalUp`/`diagonalDown`
    /// enabled. Both diagonals share one `<diagonal>` child in OOXML.
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
    /// "thin","medium","thick","double","dotted","dashed","hair", etc.
    pub style: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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
    /// "left","center","right","general","fill","justify" (lower-case).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horizontal_alignment: Option<String>,
    /// "top","center","bottom".
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
/// Color: at least one of `rgb`, `theme`, or `indexed` is set.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Color {
    /// 8-char "AARRGGBB" or 6-char "RRGGBB".
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rgb: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<u32>,
    /// -1.0..1.0 (negative = darker, positive = lighter).
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

// ============================================================
// Columnar storage
// ============================================================
//
// All blobs are base64-encoded little-endian typed-array bytes. The
// browser decodes them once via `atob` + a typed-array view (zero copy
// past the b64 step). Skipping per-cell JSON objects shrinks the wire
// (~2× after gzip on big sheets) AND collapses millions of small JS
// allocations into a handful of typed arrays — the latter is the
// bigger runtime win.
//
// Layout invariants:
//   * `cells.{r,c,kind,valueIdx,formulaIdx,styleIdx,runsIdx}` all have
//     length == `cells.count`.
//   * Records are sorted by (r asc, c asc within r).
//   * `cells.rowPtr` has length == `rowMeta.count + 1`. Cells for
//     `rowMeta.index[i]` live in `[rowPtr[i], rowPtr[i+1])`.
//   * `kind` is the small enum below; ASCII values match for grep'ability
//     but the wire is numeric.
//
// Cell-kind enum (matches `Cell.kind` strings):
//   0 = `n`     numeric
//   1 = `s`     shared string (value = SST index as decimal string)
//   2 = `inline` inline string
//   3 = `b`     boolean ("0"/"1")
//   4 = `e`     error
//   5 = `str`   plain string from a formula
//   6 = `f`     formula (cached value lives in `value`)

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ColumnarCells {
    pub count: u32,
    /// 1-based row indices, u32 LE.
    pub r: String,
    /// 1-based col indices, u32 LE.
    pub c: String,
    /// Kind enum, u8.
    pub kind: String,
    /// Index into `Sheet.value_pool`, i32 LE; -1 = no value.
    pub value_idx: String,
    /// Index into `Sheet.formula_pool`, i32 LE; -1 = no formula.
    pub formula_idx: String,
    /// `Cell.styleIndex`, i32 LE; -1 = no explicit style.
    pub style_idx: String,
    /// Index into `Sheet.inline_runs`, i32 LE; -1 = no inline runs.
    pub runs_idx: String,
    /// Row-pointer array: cells for `row_meta.index[i]` live in
    /// `[row_ptr[i], row_ptr[i+1])`. u32 LE, length == `row_meta.count + 1`.
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
    /// 1-based row indices, u32 LE. Sorted ascending.
    pub index: String,
    /// f32 LE; NaN means "use sheet's default row height".
    pub height_px: String,
    /// i32 LE; -1 = no row-level style override.
    pub style_idx: String,
    /// u8: 0/1.
    pub hidden: String,
    /// u8: OOXML `<row outlineLevel="N">`, 0..=7. 0 = no grouping.
    /// Empty string when every row is at level 0 (the common case);
    /// renderer treats absent blob as all-zeros.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub outline_level: String,
}
