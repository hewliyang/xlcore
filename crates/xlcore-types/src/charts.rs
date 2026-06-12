use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Column,
    Bar,
    Line,
    Pie,
    Area,
    Scatter,
    Bubble,
    Doughnut,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "snake_case")]
pub enum ChartStacking {
    Clustered,
    Stacked,
    PercentStacked,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Axis tick-mark style (`c:majorTickMark`/`c:minorTickMark`, OOXML `ST_TickMark`).
pub enum TickMark {
    Cross,
    Inside,
    Outside,
    None,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Tick-label placement (`c:tickLblPos`, OOXML `ST_TickLblPos`).
pub enum TickLabelPosition {
    High,
    Low,
    NextTo,
    None,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Whether the value axis crosses between categories or at their midpoints
/// (`c:crossBetween`, OOXML `ST_CrossBetween`).
pub enum CrossBetween {
    Between,
    MidpointCategory,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// A chart category or value axis (`c:catAx`/`c:valAx`), distilled from
/// ooxmlsdk `CategoryAxis`/`ValueAxis` per `scripts/schema_diff.py`.
///
/// Intentionally not modeled here (preserved on update, author via raw XML):
/// `spPr`/`txPr` styling, `label_rotation` (txPr bodyPr rot), `display_units`,
/// `pictureOptions`, `extLst`, multi-level category labels, and date-axis fields.
/// `min`/`max`/`major_unit`/`major_gridlines`/`number_format` are also surfaced
/// in the xlsx-preview renderer; the remainder round-trips for Excel.
pub struct ChartAxisPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Hide the axis entirely (`c:delete = 1`).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// `c:scaling/c:min`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// `c:scaling/c:max`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// `c:scaling/c:logBase` (2..=1000); value axis only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_base: Option<f64>,
    /// Reverse axis direction (`c:scaling/c:orientation = maxMin`).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversed: Option<bool>,
    /// `c:majorUnit`; value axis only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major_unit: Option<f64>,
    /// `c:minorUnit`; value axis only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minor_unit: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major_gridlines: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minor_gridlines: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major_tick_mark: Option<TickMark>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minor_tick_mark: Option<TickMark>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_label_position: Option<TickLabelPosition>,
    /// `c:numFmt`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_format: Option<String>,
    /// `c:crossBetween`; value axis only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_between: Option<CrossBetween>,
    /// `c:crossesAt`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crosses_at: Option<f64>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "snake_case")]
/// Where the chart's legend sits relative to the plot area.
///
/// Use `None` to suppress the legend entirely (Excel UI: "No Legend"). `TopRight`
/// corresponds to Excel's "Overlay Legend at Right" position.
pub enum ChartLegendPosition {
    Right,
    Left,
    Top,
    Bottom,
    TopRight,
    /// Hide the legend. Equivalent to Excel's "No Legend" / unchecking Chart Elements → Legend.
    None,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "snake_case")]
pub enum ChartDataLabelPosition {
    Center,
    InsideEnd,
    InsideBase,
    OutsideEnd,
    Top,
    Bottom,
    Left,
    Right,
    BestFit,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ChartDataLabels {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_value: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_category_name: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_series_name: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_percent: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_legend_key: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<ChartDataLabelPosition>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Two-cell anchor for a chart or image, in OOXML `xdr:twoCellAnchor` coordinates.
///
/// All row/column indices are **0-based** (column A = 0, row 1 = 0), matching the
/// underlying `<xdr:col>` / `<xdr:row>` elements. The anchor's bottom-right corner
/// is the top-left of the cell at `(to_column, to_row)` plus any EMU offsets, so
/// to span columns A..=E inclusive use `from_column: 0, to_column: 5`.
pub struct ChartAnchor {
    /// 0-based column index of the top-left anchor cell (A = 0).
    pub from_column: u32,
    /// 0-based row index of the top-left anchor cell (row 1 = 0).
    pub from_row: u32,
    /// 0-based column index of the bottom-right anchor cell (exclusive of any offset).
    pub to_column: u32,
    /// 0-based row index of the bottom-right anchor cell (exclusive of any offset).
    pub to_row: u32,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_column_offset_emu: Option<i64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_row_offset_emu: Option<i64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_column_offset_emu: Option<i64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_row_offset_emu: Option<i64>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(untagged)]
/// Anchor input: either a two-cell A1 range string (`"A1:E15"`, optionally
/// sheet-qualified) or an explicit {@link ChartAnchor}. The range form is
/// resolved to a `ChartAnchor` in the Rust facade.
pub enum AnchorSpec {
    A1(String),
    Cells(ChartAnchor),
}

impl Default for AnchorSpec {
    fn default() -> Self {
        AnchorSpec::Cells(ChartAnchor::default())
    }
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeriesPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_ref: Option<String>,
    pub values_ref: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_values_ref: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bubble_sizes_ref: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeriesInfo {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_ref: Option<String>,
    pub values_ref: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_values_ref: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bubble_sizes_ref: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct ChartPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub kind: ChartKind,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_position: Option<ChartLegendPosition>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_ref: Option<String>,
    pub series: Vec<ChartSeriesPatch>,
    pub anchor: AnchorSpec,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_axis: Option<ChartAxisPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_axis: Option<ChartAxisPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stacking: Option<ChartStacking>,
    /// `c:gapWidth` (0..=500); bar/column charts only. Space between bar clusters
    /// as a percentage of bar width.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_width: Option<u16>,
    /// `c:overlap` (-100..=100); bar/column charts only. Stacked charts force 100.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap: Option<i8>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ChartInfo {
    pub sheet: String,
    pub id: String,
    pub name: String,
    pub kind: ChartKind,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_position: Option<ChartLegendPosition>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_ref: Option<String>,
    pub series: Vec<ChartSeriesInfo>,
    pub anchor: ChartAnchor,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_axis: Option<ChartAxisPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_axis: Option<ChartAxisPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stacking: Option<ChartStacking>,
    /// `c:gapWidth` (0..=500); bar/column charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_width: Option<u16>,
    /// `c:overlap` (-100..=100); bar/column charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap: Option<i8>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct ChartUpdate {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_position: Option<ChartLegendPosition>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_ref: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<Vec<ChartSeriesPatch>>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<AnchorSpec>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_axis_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_axis: Option<ChartAxisPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_axis: Option<ChartAxisPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stacking: Option<ChartStacking>,
    /// `c:gapWidth` (0..=500); bar/column charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_width: Option<u16>,
    /// `c:overlap` (-100..=100); bar/column charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap: Option<i8>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
}
