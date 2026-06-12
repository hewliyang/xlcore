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
    Radar,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Radar plot style (`c:radarStyle`, OOXML `ST_RadarStyle`). Radar charts only.
///
/// `Standard` draws connecting lines only, `Marker` adds point markers, `Filled`
/// fills each series' polygon. Defaults to `Standard` when omitted.
pub enum RadarStyle {
    Standard,
    Marker,
    Filled,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Built-in axis display-unit scale (`c:builtInUnit`, OOXML `ST_BuiltInUnit`).
///
/// Each variant divides the value-axis labels by its power of ten (e.g. `Millions`
/// shows `5` for `5_000_000`). The xlsx-preview renderer applies the factor to the
/// tick labels and draws the unit name as a label band.
pub enum BuiltInUnit {
    Hundreds,
    Thousands,
    TenThousands,
    HundredThousands,
    Millions,
    TenMillions,
    HundredMillions,
    Billions,
    Trillions,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(untagged)]
/// Value-axis display units (`c:dispUnits`), distilled from ooxmlsdk `DisplayUnits`.
///
/// Either a {@link BuiltInUnit} name (`"millions"`) or a custom divisor number
/// (`1000000`). Both scale the axis tick labels; built-in units also render their
/// name as a label band. Intentionally not modeled (preserved on update, author
/// via raw XML): a custom `dispUnitsLbl` text/styling and `extLst`.
pub enum DisplayUnits {
    Builtin(BuiltInUnit),
    Custom(f64),
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Which value axis a series is plotted against in a combo chart.
///
/// `Secondary` puts the series on a second value axis (`c:valAx` at the right of
/// the plot area), the standard idiom for charts mixing series at different
/// scales (e.g. revenue bars + margin-% line).
pub enum ChartAxisGroup {
    Primary,
    Secondary,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Data-point marker symbol (`c:marker/c:symbol`, OOXML `ST_MarkerStyle`).
/// Applies to line and scatter series; ignored for other chart kinds.
pub enum MarkerStyle {
    Auto,
    Circle,
    Dash,
    Diamond,
    Dot,
    None,
    Picture,
    Plus,
    Square,
    Star,
    Triangle,
    X,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Series data-point marker (`c:marker`), distilled from ooxmlsdk `Marker`.
///
/// Intentionally not modeled (preserved on update, author via raw XML):
/// `spPr` styling, `pictureOptions`, `extLst`.
pub struct ChartMarker {
    /// `c:symbol/@val`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<MarkerStyle>,
    /// `c:size/@val` (2..=72).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u8>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// A single data point override (`c:dPt`), distilled from ooxmlsdk `DataPoint`.
///
/// Currently only per-point fill is modeled, which the xlsx-preview renderer
/// reads for bar/column/pie/doughnut series (e.g. the waterfall-via-noFill idiom).
/// Intentionally not modeled (preserved on update, author via raw XML):
/// `invertIfNegative`, per-point `marker`, `bubble3D`, `explosion`,
/// non-solid `spPr` styling, `pictureOptions`, `extLst`.
pub struct ChartDataPoint {
    /// `c:idx/@val`; 0-based data-point index within the series.
    pub index: u32,
    /// `c:spPr` solid fill: 6-hex `RRGGBB` / 8-hex `AARRGGBB`, or the literal
    /// `"none"` for an explicit no-fill (`a:noFill`).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
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
/// `spPr`/`txPr` styling, `label_rotation` (txPr bodyPr rot),
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
    /// `c:dispUnits`; value axis only. A built-in unit name or custom divisor that
    /// scales the value-axis labels. Renderer-visible.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_units: Option<DisplayUnits>,
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
    /// `c:numFmt/@formatCode`; number format applied to the displayed value.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_format: Option<String>,
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
    /// `c:marker`; line/scatter series only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<ChartMarker>,
    /// Smooth the connecting line with a spline (`c:smooth`); line/scatter series
    /// only. On a scatter chart any smoothed series sets the chart's
    /// `c:scatterStyle` to `smoothMarker`, which the xlsx-preview renderer draws
    /// as curved.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smooth: Option<bool>,
    /// Per-data-point fill overrides (`c:dPt`). Each entry recolors one point by
    /// index; use `fill: "none"` for the waterfall-via-noFill idiom.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_points: Option<Vec<ChartDataPoint>>,
    /// Per-series chart type, overriding the chart's `kind` to build a combo
    /// chart. Only `Column`/`Bar`/`Line`/`Area` are valid here; mixing those on
    /// one chart emits multiple `c:barChart`/`c:lineChart`/`c:areaChart` groups.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChartKind>,
    /// Plot this series on the primary or secondary value axis. `Secondary`
    /// synthesizes a right-hand `c:valAx`. Combo/cartesian charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<ChartAxisGroup>,
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
    /// `c:marker`; line/scatter series only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<ChartMarker>,
    /// Spline-smoothed line (`c:smooth`); see {@link ChartSeriesPatch.smooth}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smooth: Option<bool>,
    /// Per-data-point fill overrides (`c:dPt`); see {@link ChartSeriesPatch.dataPoints}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_points: Option<Vec<ChartDataPoint>>,
    /// Per-series chart type for combo charts; see {@link ChartSeriesPatch.kind}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChartKind>,
    /// Primary/secondary value-axis group; see {@link ChartSeriesPatch.axis}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<ChartAxisGroup>,
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
    /// `c:radarStyle`; radar charts only. Defaults to `Standard` when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radar_style: Option<RadarStyle>,
    /// `c:holeSize` (10..=90); doughnut charts only. Inner-hole diameter as a
    /// percentage of the chart radius. Defaults to 50 when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_size: Option<u8>,
    /// `c:firstSliceAng` (0..=360); pie/doughnut charts only. Clockwise rotation
    /// of the first slice from the top (12 o'clock), in degrees.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_slice_angle: Option<u16>,
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
    /// `c:radarStyle`; radar charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radar_style: Option<RadarStyle>,
    /// `c:holeSize` (10..=90); doughnut charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_size: Option<u8>,
    /// `c:firstSliceAng` (0..=360); pie/doughnut charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_slice_angle: Option<u16>,
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
    /// `c:radarStyle`; radar charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radar_style: Option<RadarStyle>,
    /// `c:holeSize` (10..=90); doughnut charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_size: Option<u8>,
    /// `c:firstSliceAng` (0..=360); pie/doughnut charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_slice_angle: Option<u16>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
}
