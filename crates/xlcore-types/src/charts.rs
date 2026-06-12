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
    Stock,
    /// 3D clustered column (`c:bar3DChart` with `barDir=col`).
    #[serde(rename = "column3d")]
    Column3D,
    /// 3D bar (`c:bar3DChart` with `barDir=bar`).
    #[serde(rename = "bar3d")]
    Bar3D,
    /// 3D line (`c:line3DChart`).
    #[serde(rename = "line3d")]
    Line3D,
    /// 3D pie (`c:pie3DChart`); no axes.
    #[serde(rename = "pie3d")]
    Pie3D,
    /// 3D area (`c:area3DChart`).
    #[serde(rename = "area3d")]
    Area3D,
    /// 3D surface (`c:surface3DChart`); needs the `c:serAx` third axis.
    #[serde(rename = "surface3d")]
    Surface3D,
    /// 2D surface contour (`c:surfaceChart`); top-down view, needs `c:serAx`.
    Surface,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
/// 3D bar/column shape (`c:shape/@val`, OOXML `ST_Shape`), transliterated from
/// ooxmlsdk `ShapeValues`. Bar3D/Column3D charts only.
pub enum Bar3DShape {
    #[serde(rename = "cone")]
    Cone,
    #[serde(rename = "coneToMax")]
    ConeToMax,
    #[serde(rename = "box")]
    Box,
    #[serde(rename = "cylinder")]
    Cylinder,
    #[serde(rename = "pyramid")]
    Pyramid,
    #[serde(rename = "pyramidToMax")]
    PyramidToMaximum,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// 3D view settings (`c:view3D`), distilled from ooxmlsdk `View3D`. Applies to
/// any 3D chart kind (bar3D/column3D/line3D/pie3D/area3D). Round-trips for
/// Excel; the xlsx-preview renderer draws 3D charts flat (no rotation/depth).
///
/// schema-excluded: extLst
pub struct ChartView3D {
    /// `c:rotX/@val` (-90..=90); rotation about the x-axis in degrees.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rot_x: Option<i8>,
    /// `c:rotY/@val` (0..=360); rotation about the y-axis in degrees.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rot_y: Option<u16>,
    /// `c:perspective/@val` (0..=240); perspective distance. Ignored when
    /// `right_angle_axes` is `true`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<u8>,
    /// `c:rAngAx/@val`; render axes at right angles (no perspective).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_angle_axes: Option<bool>,
    /// `c:depthPercent/@val` (20..=2000); depth of the plot as a percent of width.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_percent: Option<u16>,
    /// `c:hPercent/@val` (5..=500); height of the plot as a percent of width.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_percent: Option<u16>,
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
/// How blank/empty source cells are plotted (`c:dispBlanksAs`, OOXML
/// `ST_DispBlanksAs`). `Gap` leaves a hole, `Zero` plots zero, `Span` bridges
/// across the gap (line/area charts). Excel defaults new charts to `Gap`.
pub enum DispBlanksAs {
    Span,
    Gap,
    Zero,
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
///
/// schema-excluded: xmlns, spPr
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
/// Preset line dash pattern (`a:prstDash/@val`, OOXML `ST_PresetLineDashVal`),
/// transliterated from ooxmlsdk `PresetLineDashValues`.
pub enum LineDash {
    #[serde(rename = "solid")]
    Solid,
    #[serde(rename = "dot")]
    Dot,
    #[serde(rename = "dash")]
    Dash,
    #[serde(rename = "lgDash")]
    LargeDash,
    #[serde(rename = "dashDot")]
    DashDot,
    #[serde(rename = "lgDashDot")]
    LargeDashDot,
    #[serde(rename = "lgDashDotDot")]
    LargeDashDotDot,
    #[serde(rename = "sysDash")]
    SystemDash,
    #[serde(rename = "sysDot")]
    SystemDot,
    #[serde(rename = "sysDashDot")]
    SystemDashDot,
    #[serde(rename = "sysDashDotDot")]
    SystemDashDotDot,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Series connecting-line / outline styling (`spPr/a:ln`), distilled from
/// ooxmlsdk `Outline`.
///
/// On line/scatter/radar series this is the connecting line; on bar/area/pie
/// series it is the shape outline. `width`/`dash` are renderer-visible. Setting
/// `none: true` emits `a:ln/a:noFill` (hidden line, markers-only) and overrides
/// `width`/`dash`.
///
/// Intentionally not modeled (preserved on update, author via raw XML): `cap`,
/// `cmpd`, `algn`, `custDash`, line joins, head/tail ends, gradient/pattern
/// line fills, `extLst`.
pub struct ChartLine {
    /// `a:ln/@w` in EMU (1 pt = 12700 EMU, 0..=20116800).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_emu: Option<i32>,
    /// `a:ln/a:prstDash/@val`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dash: Option<LineDash>,
    /// Hide the line (`a:ln/a:noFill`); markers-only. Overrides `width`/`dash`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub none: Option<bool>,
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
/// Per-point `fill` and `explosion` are modeled. The xlsx-preview renderer
/// reads `fill` for bar/column/pie/doughnut series (e.g. the waterfall-via-noFill
/// idiom) and `explosion` for pie/doughnut slice offset.
/// Intentionally not modeled (preserved on update, author via raw XML):
/// `invertIfNegative`, per-point `marker`, `bubble3D`,
/// non-solid `spPr` styling, `pictureOptions`, `extLst`.
///
/// schema-excluded: invertIfNegative, marker, bubble3D, pictureOptions
pub struct ChartDataPoint {
    /// `c:idx/@val`; 0-based data-point index within the series.
    pub index: u32,
    /// `c:spPr` solid fill: 6-hex `RRGGBB` / 8-hex `AARRGGBB`, or the literal
    /// `"none"` for an explicit no-fill (`a:noFill`).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// `c:explosion/@val`; pie/doughnut slice offset as a percent of radius
    /// (0..=400). Renderer-visible for pie/doughnut charts.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explosion: Option<u32>,
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
/// `spPr`/`txPr` styling (beyond `label_rotation`), `pictureOptions`, `extLst`,
/// multi-level category labels, and date-axis fields.
/// `min`/`max`/`major_unit`/`major_gridlines`/`number_format` are also surfaced
/// in the xlsx-preview renderer; the remainder round-trips for Excel.
///
/// schema-excluded: spPr, auto, lblAlgn, lblOffset, tickLblSkip, tickMarkSkip, noMultiLvlLbl
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
    /// Tick-label rotation in whole degrees (-90..=90), stored as `c:txPr`'s
    /// `a:bodyPr/@rot` in 60000ths of a degree. Round-trips for Excel; the
    /// xlsx-preview renderer draws axis labels horizontally regardless.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_rotation: Option<i32>,
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
    /// Per-point label overrides (`c:dLbl`), keyed by 0-based data-point `index`.
    /// Each entry either deletes that point's label or overrides individual
    /// show flags / position / number format; unset fields inherit the
    /// series-level settings above.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub per_point: Vec<ChartDataLabel>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// A single per-point data label (`c:dLbl`) inside `c:dLbls`.
///
/// `index` is the 0-based data-point this label applies to. Set `delete: true`
/// to suppress that point's label entirely (all other fields ignored). Otherwise
/// any set field overrides the series-level `ChartDataLabels` for this point only.
///
/// schema-excluded: layout (manual position), tx (rich-text override), spPr
/// (label shape/fill), txPr (label font), showBubbleSize, extLst.
pub struct ChartDataLabel {
    /// 0-based data-point index this override applies to (`c:idx/@val`).
    pub index: u32,
    /// Suppress this point's label (`c:delete`). When `true`, all other fields
    /// are ignored.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub delete: bool,
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
    /// `c:numFmt/@formatCode`; number format applied to this point's value.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
/// Trendline regression type (`c:trendlineType/@val`, OOXML `ST_TrendlineType`),
/// transliterated from ooxmlsdk `TrendlineValues`.
pub enum TrendlineKind {
    #[serde(rename = "exp")]
    Exponential,
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "log")]
    Logarithmic,
    #[serde(rename = "movingAvg")]
    MovingAverage,
    #[serde(rename = "poly")]
    Polynomial,
    #[serde(rename = "power")]
    Power,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
/// A series trendline (`c:trendline`), distilled from ooxmlsdk `Trendline`.
///
/// Supported on bar/column, line, area, scatter and bubble series only (Excel
/// disallows trendlines on pie/doughnut/radar/stock). `polynomial_order` applies
/// to `Polynomial`, `period` to `MovingAverage`; `intercept` to
/// exp/linear/poly/power. Round-trips for Excel; the xlsx-preview renderer does
/// not draw trendlines.
///
/// Intentionally not modeled (preserved on update, author via raw XML): `spPr`
/// line styling, `trendlineLbl` label text/layout, `extLst`.
///
/// schema-excluded: spPr, trendlineLbl
pub struct ChartTrendline {
    /// `c:trendlineType/@val`.
    #[serde(rename = "type")]
    pub kind: TrendlineKind,
    /// `c:name`; custom trendline label name overriding Excel's auto label.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `c:order/@val` (2..=6); polynomial trendlines only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polynomial_order: Option<u8>,
    /// `c:period/@val` (>=2); moving-average trendlines only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<u32>,
    /// `c:forward/@val`; periods to project forward.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward: Option<f64>,
    /// `c:backward/@val`; periods to project backward.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backward: Option<f64>,
    /// `c:intercept/@val`; forced y-intercept.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intercept: Option<f64>,
    /// `c:dispEq/@val`; show the regression equation on the chart.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_equation: Option<bool>,
    /// `c:dispRSqr/@val`; show the R² value on the chart.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_r_squared: Option<bool>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
/// Error-bar axis direction (`c:errDir/@val`, OOXML `ST_ErrDir`), transliterated
/// from ooxmlsdk `ErrorBarDirectionValues`.
pub enum ChartErrorDirection {
    #[default]
    #[serde(rename = "x")]
    X,
    #[serde(rename = "y")]
    Y,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
/// Which sides the error bars extend (`c:errBarType/@val`, OOXML `ST_ErrBarType`),
/// transliterated from ooxmlsdk `ErrorBarValues`.
pub enum ChartErrorBarType {
    #[default]
    #[serde(rename = "both")]
    Both,
    #[serde(rename = "minus")]
    Minus,
    #[serde(rename = "plus")]
    Plus,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
/// How error-bar magnitudes are computed (`c:errValType/@val`, OOXML
/// `ST_ErrValType`), transliterated from ooxmlsdk `ErrorValues`.
pub enum ChartErrorValueType {
    #[default]
    #[serde(rename = "cust")]
    Custom,
    #[serde(rename = "fixedVal")]
    FixedValue,
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "stdDev")]
    StandardDeviation,
    #[serde(rename = "stdErr")]
    StandardError,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
/// Series error bars (`c:errBars`), distilled from ooxmlsdk `ErrorBars`.
///
/// Supported on bar/column, line, area, scatter and bubble series only (Excel
/// disallows error bars on pie/doughnut/radar/stock). `value` carries the
/// magnitude for `FixedValue`/`Percentage` and the multiplier for
/// `StandardDeviation`/`StandardError`; `plusRef`/`minusRef` (range formulas) or
/// `plusValues`/`minusValues` (inline literals) carry per-point magnitudes for
/// `Custom`. Round-trips for Excel; the xlsx-preview renderer does not draw
/// error bars.
///
/// Intentionally not modeled (preserved on update, author via raw XML): `spPr`
/// line styling, numbering caches, `extLst`.
///
/// schema-excluded: spPr
pub struct ChartErrorBars {
    /// `c:errDir/@val`; the axis the bars run along. Omitted (Excel default) for
    /// bar/column/line/area; set `Y` (and a second `X` set) for scatter/bubble.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<ChartErrorDirection>,
    /// `c:errBarType/@val`; which sides the bars extend.
    pub bar_type: ChartErrorBarType,
    /// `c:errValType/@val`; how magnitudes are computed.
    pub value_type: ChartErrorValueType,
    /// `c:val/@val`; magnitude (fixed/percentage) or multiplier (stdDev/stdErr).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// `c:noEndCap/@val`; draw the bars without end caps (T-less).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_end_cap: Option<bool>,
    /// `c:plus` as a range formula (`numRef`); custom positive magnitudes.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plus_ref: Option<String>,
    /// `c:minus` as a range formula (`numRef`); custom negative magnitudes.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minus_ref: Option<String>,
    /// `c:plus` as inline literals (`numLit`); custom positive magnitudes.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plus_values: Option<Vec<f64>>,
    /// `c:minus` as inline literals (`numLit`); custom negative magnitudes.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minus_values: Option<Vec<f64>>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// Chart data table (`c:dTable` in `c:plotArea`), distilled from ooxmlsdk
/// `DataTable`.
///
/// A grid of the source values drawn beneath a cartesian plot. Cartesian charts
/// only (column/bar/line/area); Excel rejects data tables on
/// pie/doughnut/scatter/bubble/radar/stock. Round-trips for Excel; the
/// xlsx-preview renderer does not draw the data table.
///
/// Intentionally not modeled (preserved on update, author via raw XML): `spPr`
/// shape/border styling, `txPr` font, `extLst`.
///
/// schema-excluded: spPr, txPr
pub struct ChartDataTable {
    /// `c:showHorzBorder/@val`; draw horizontal cell borders.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_horizontal_border: Option<bool>,
    /// `c:showVertBorder/@val`; draw vertical cell borders.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_vertical_border: Option<bool>,
    /// `c:showOutline/@val`; draw the table's outline border.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_outline: Option<bool>,
    /// `c:showKeys/@val`; show the series legend keys in the first column.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_keys: Option<bool>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
/// A chart series. `name`/`color`/`marker`/`line`/`smooth` etc. are flat sugar;
/// the series text, refs (cat/val/xVal/yVal), idx and order are derived from the
/// patch fields and the series' position.
///
/// schema-excluded: spPr, pictureOptions, shape, explosion
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
    /// Connecting-line / outline styling (`spPr/a:ln`): width, dash, hidden.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<ChartLine>,
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
    /// Flip the fill color for negative values (`c:invertIfNegative`); bar/column
    /// and bubble series only. Round-trips for Excel.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invert_if_negative: Option<bool>,
    /// Series regression trendline (`c:trendline`). Bar/column, line, area,
    /// scatter and bubble series only. Round-trips for Excel.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trendline: Option<ChartTrendline>,
    /// Series error bars (`c:errBars`). Bar/column, line, area, scatter and
    /// bubble series only. Round-trips for Excel.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_bars: Option<ChartErrorBars>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
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
    /// Connecting-line / outline styling; see {@link ChartSeriesPatch.line}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<ChartLine>,
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
    /// Negative-value fill flip; see {@link ChartSeriesPatch.invertIfNegative}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invert_if_negative: Option<bool>,
    /// Series regression trendline; see {@link ChartSeriesPatch.trendline}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trendline: Option<ChartTrendline>,
    /// Series error bars; see {@link ChartSeriesPatch.errorBars}.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_bars: Option<ChartErrorBars>,
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
    /// `c:hiLowLines`; stock charts only. Vertical line spanning each category's
    /// high/low values. Defaults to `true` when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hi_low_lines: Option<bool>,
    /// `c:upDownBars`; stock charts only. Open/close bars (white up, black down).
    /// Defaults to `true` for open-high-low-close (4+ series).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up_down_bars: Option<bool>,
    /// `c:dropLines`; stock charts only. Vertical line from each point to the
    /// category axis. Defaults to `false` when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_lines: Option<bool>,
    /// `c:dispBlanksAs`; how blank source cells are plotted. Defaults to `Gap`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disp_blanks_as: Option<DispBlanksAs>,
    /// `c:varyColors`; color each data point (or each series) differently. Excel
    /// defaults pie/doughnut/bubble charts to `true` and others to `false`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vary_colors: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
    /// `c:dTable`; data table beneath the plot. Cartesian charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_table: Option<ChartDataTable>,
    /// `c:view3D`; 3D view settings. 3D chart kinds only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_3d: Option<ChartView3D>,
    /// `c:shape`; bar/column 3D shape. Bar3D/Column3D charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_shape: Option<Bar3DShape>,
    /// `c:wireframe`; draw the surface as a wireframe (lines only) instead of
    /// filled bands. Surface/Surface3D charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wireframe: Option<bool>,
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
    /// `c:hiLowLines`; stock charts only. Vertical line spanning each category's
    /// high/low values. Defaults to `true` when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hi_low_lines: Option<bool>,
    /// `c:upDownBars`; stock charts only. Open/close bars (white up, black down).
    /// Defaults to `true` for open-high-low-close (4+ series).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up_down_bars: Option<bool>,
    /// `c:dropLines`; stock charts only. Vertical line from each point to the
    /// category axis. Defaults to `false` when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_lines: Option<bool>,
    /// `c:dispBlanksAs`; how blank source cells are plotted. Defaults to `Gap`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disp_blanks_as: Option<DispBlanksAs>,
    /// `c:varyColors`; color each data point (or each series) differently. Excel
    /// defaults pie/doughnut/bubble charts to `true` and others to `false`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vary_colors: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
    /// `c:dTable`; data table beneath the plot. Cartesian charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_table: Option<ChartDataTable>,
    /// `c:view3D`; 3D view settings. 3D chart kinds only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_3d: Option<ChartView3D>,
    /// `c:shape`; bar/column 3D shape. Bar3D/Column3D charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_shape: Option<Bar3DShape>,
    /// `c:wireframe`; draw the surface as a wireframe (lines only) instead of
    /// filled bands. Surface/Surface3D charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wireframe: Option<bool>,
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
    /// `c:hiLowLines`; stock charts only. Vertical line spanning each category's
    /// high/low values. Defaults to `true` when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hi_low_lines: Option<bool>,
    /// `c:upDownBars`; stock charts only. Open/close bars (white up, black down).
    /// Defaults to `true` for open-high-low-close (4+ series).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up_down_bars: Option<bool>,
    /// `c:dropLines`; stock charts only. Vertical line from each point to the
    /// category axis. Defaults to `false` when omitted.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_lines: Option<bool>,
    /// `c:dispBlanksAs`; how blank source cells are plotted. Defaults to `Gap`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disp_blanks_as: Option<DispBlanksAs>,
    /// `c:varyColors`; color each data point (or each series) differently. Excel
    /// defaults pie/doughnut/bubble charts to `true` and others to `false`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vary_colors: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_labels: Option<ChartDataLabels>,
    /// `c:dTable`; data table beneath the plot. Cartesian charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_table: Option<ChartDataTable>,
    /// `c:view3D`; 3D view settings. 3D chart kinds only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_3d: Option<ChartView3D>,
    /// `c:shape`; bar/column 3D shape. Bar3D/Column3D charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_shape: Option<Bar3DShape>,
    /// `c:wireframe`; draw the surface as a wireframe (lines only) instead of
    /// filled bands. Surface/Surface3D charts only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wireframe: Option<bool>,
}
