use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool {
    !*b
}

/// One drawing object placed on the sheet, with its xlsx cell-anchor.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Drawing {
    /// `chart`, `image`, `shape` (only `chart` and `image` are rendered).
    pub kind: String,
    pub anchor: DrawingAnchor,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart: Option<Chart>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
}
/// Inline-encoded raster image extracted from `xl/media/*`.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    /// `data:image/png;base64,...` style URI, ready to feed to <img>.
    pub data_uri: String,
}
/// twoCellAnchor: from/to cell indices (0-based) + EMU offsets within the cell.
/// 1 EMU = 1/9525 px at 96 DPI.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct DrawingAnchor {
    pub from_col: u32,
    #[cfg_attr(feature = "typescript", ts(type = "number"))]
    pub from_col_off_emu: i64,
    pub from_row: u32,
    #[cfg_attr(feature = "typescript", ts(type = "number"))]
    pub from_row_off_emu: i64,
    pub to_col: u32,
    #[cfg_attr(feature = "typescript", ts(type = "number"))]
    pub to_col_off_emu: i64,
    pub to_row: u32,
    #[cfg_attr(feature = "typescript", ts(type = "number"))]
    pub to_row_off_emu: i64,
    /// `oneCellAnchor` width in EMU. When set, the renderer uses this value
    /// instead of the approximated `to` cell.
    #[cfg_attr(feature = "typescript", ts(type = "number | null", optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext_emu_cx: Option<i64>,
    /// `oneCellAnchor` height in EMU. See `ext_emu_cx`.
    #[cfg_attr(feature = "typescript", ts(type = "number | null", optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext_emu_cy: Option<i64>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Chart {
    /// `column`, `bar`, `line`, `pie`, `area`, `scatter`, `unknown`.
    /// `column` and `bar` collapse into one `BarChart` schema entry; the
    /// barDir attribute distinguishes them.
    #[serde(rename = "type")]
    pub chart_type: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub series: Vec<ChartSeries>,
    /// X-axis labels (categories). Often pulled from the cat strRef cache;
    /// if absent the renderer falls back to series-relative indices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    /// Formula reference (e.g. `Sheet1!$B$1:$E$1`) used to populate
    /// `categories` from live workbook data when the chart's strCache is
    /// empty. Resolution happens after sheets are extracted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_ref: Option<String>,
    /// `t`, `b`, `l`, `r`, `tr` (ECMA-376 legend positions).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_pos: Option<String>,
    /// Number-format for the value axis (e.g. "$#,##0").
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_format: Option<String>,
    /// Number-format string for category-axis labels. When omitted, labels
    /// are rendered as plain text.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_format: Option<String>,
    /// `clustered`, `stacked`, `percentStacked`, `standard` (line/area).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouping: Option<String>,
    /// `col` or `bar` (only meaningful for chart_type == bar).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_dir: Option<String>,
    /// Scatter style: `line`, `lineMarker`, `marker`, `smooth`,
    /// `smoothMarker`. Only meaningful for chart_type == scatter.
    /// When `None`, the renderer treats the chart as marker-only
    /// (matches Excel's UI default even though the OOXML enum
    /// default is `line`).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scatter_style: Option<String>,
    /// Chart-level `<c:dLbls>` — the per-chart-group default. Series-
    /// level `dataLabels` overrides on a per-series basis. None ⇒ no
    /// labels (Excel's default for every chart type).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<DataLabels>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secondary_axis: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_format_secondary: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_min: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_max: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_min_secondary: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_max_secondary: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_gap_width: Option<u16>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_overlap: Option<i8>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_axis_title_secondary: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_major_gridlines: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_major_gridlines_secondary: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disp_units: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disp_units_label: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disp_units_secondary: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disp_units_label_secondary: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bubble_scale: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_represents: Option<String>,
}

/// `<c:dLbls>` — what to print next to each data point. Mirrors the
/// OOXML `CT_DLbls` block. Rendered text per point is built by joining
/// the enabled fields with `separator` (default `", "`):
///
///   `[seriesName][sep][category][sep][value | percent]`
///
/// Empty when extracted from `<c:delete val="1"/>` (suppression marker).
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct DataLabels {
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_value: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_category: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_series_name: bool,
    /// Show value as % of category total. Pie/doughnut natively in
    /// Excel; we honor it on any chart type that has a category total.
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_percent: bool,
    /// `ctr`, `inEnd`, `inBase`, `outEnd`, `t`, `b`, `l`, `r`, `bestFit`.
    /// None ⇒ chart-type default (`outEnd` for column, `r` for bar,
    /// `ctr` for line/scatter, `outEnd`/`bestFit` for pie).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    /// String inserted between fields when more than one show* is on.
    /// Default `", "`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    /// Number-format code for the value field, e.g. `"0.0%"`. None
    /// falls back to the chart's `valueFormat`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_fmt: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub point_overrides: Vec<PointDataLabel>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct PointDataLabel {
    pub idx: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub delete: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_fmt: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_value: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_category: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_series_name: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_percent: Option<bool>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeries {
    pub name: String,
    /// Formula for the series name (e.g. `Sheet1!$A$2`). Resolved after
    /// sheet extraction if `name` is empty.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_ref: Option<String>,
    /// CSS color string. May come from explicit spPr.solidFill or, more
    /// commonly, an Office theme accent (`accent1..accent6`).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub values: Vec<f64>,
    /// Formula for the values range (e.g. `Sheet1!$B$2:$E$2`). Resolved
    /// after sheet extraction if `values` is empty.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values_ref: Option<String>,
    /// Numeric x-values for scatter / bubble series. Empty for chart
    /// types that use the chart-level `categories` array instead.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub x_values: Vec<f64>,
    /// Formula for the x-values range (scatter only). Resolved after
    /// sheet extraction if `x_values` is empty.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_values_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bubble_sizes: Vec<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bubble_sizes_ref: Option<String>,
    /// Per-data-point CSS color overrides, parallel to `values` (one
    /// entry per category). Empty string at index `i` means "use the
    /// series-level `color` (or the renderer's per-slice palette for
    /// pie/doughnut)". Sourced from `<c:dPt>` children with explicit
    /// `spPr` fills. Empty Vec when no `<c:dPt>` overrides exist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub point_colors: Vec<String>,
    /// Per-series `<c:dLbls>`. Overrides chart-level `data_labels`
    /// when present.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<DataLabels>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_group: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart_type: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_symbol: Option<String>,
}
