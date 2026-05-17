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
    /// Vector shape tree (`<xdr:sp>` / `<xdr:grpSp>`). Only set when
    /// `kind == "shape"`. A drawing anchor wraps a tree of shape nodes
    /// (one per `<xdr:sp>`) whose positions are stored relative to the
    /// anchor bbox (0..1). Nested groups are flattened — the extractor
    /// applies each `<xdr:grpSp>`'s `xfrm/chOff/chExt` mapping during
    /// the walk so the renderer only ever sees leaf `<xdr:sp>` nodes.
    /// See ECMA-376 §19.3 for the shape model and §20.1.7.6 for the
    /// group-transform semantics.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
}

/// `<xdr:sp>` autoshape (rectangles, callouts, banners, sticky notes,
/// arrows). v0 paints fill + outline + centered text. Unknown presets
/// fall back to a plain rectangle — Excel's chrome-shape vocabulary is
/// vast (~200 presets) and most workbook chrome is rounded-rect or
/// arrow; we'll grow this as fixtures demand.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    /// Leaf shapes in z-order (first painted first; later ones overlay).
    /// Groups are flattened — children inherit the accumulated transform.
    pub nodes: Vec<ShapeNode>,
}

/// One leaf `<xdr:sp>` (or `<xdr:cxnSp>`) positioned inside the drawing
/// anchor's bbox via fractional coordinates (0..1). The renderer maps
/// these to the anchor's resolved pixel rect.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ShapeNode {
    /// Fractional bbox inside the drawing-anchor's resolved pixel rect.
    pub rel_x: f32,
    pub rel_y: f32,
    pub rel_w: f32,
    pub rel_h: f32,
    /// `<a:prstGeom prst="..."/>` token (`rect`, `roundRect`, `ellipse`,
    /// `leftArrow`, …). None ⇒ no preset (custom geometry); renderer
    /// falls back to a plain rectangle.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// Resolved `#RRGGBB` fill color from `<a:solidFill>`. `None` when
    /// `<a:noFill/>` is authored or no fill is present.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Resolved `#RRGGBB` outline color. `None` when `<a:noFill/>` or
    /// `<a:ln>` is absent.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_color: Option<String>,
    /// Outline stroke width in EMU. `None` ⇒ Excel default (~9525 EMU
    /// = 1pt). 0 ⇒ hairline.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_width_emu: Option<i32>,
    /// `<a:bodyPr anchor="..."/>` vertical anchor (`t`/`ctr`/`b`).
    /// Default `t` (top).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_anchor: Option<String>,
    /// Rotation in 1/60000 degree units (OOXML's `rot` attr unit).
    /// `None` ⇒ 0.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<i32>,
    /// Text paragraphs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paragraphs: Vec<ShapeParagraph>,
    /// `<a:bodyPr wrap="..."/>` token (`square` ⇒ wrap on word
    /// boundaries; `none` ⇒ no wrap). Default `square` (Excel's
    /// implicit default when the attr is absent).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_wrap: Option<String>,
    /// Inline picture node — when this is `Some(...)`, the node
    /// renders an embedded raster image (`<xdr:pic>` nested inside
    /// `<xdr:grpSp>`) instead of a `prstGeom` rect. The string is a
    /// `data:<mime>;base64,...` URI (same encoding the top-level
    /// `Image.dataUri` uses, sharing the renderer image cache).
    /// Fill / outline / paragraphs are ignored when this is set.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_data_uri: Option<String>,
    /// `<a:srcRect l="" t="" r="" b=""/>` crop, in 1/1000 percent of
    /// the source image dimensions. Length-4 vec: [left, top, right,
    /// bottom]. Renderer uses these as fractional crop insets when
    /// painting `image_data_uri`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_src_rect: Option<Vec<i32>>,
}

/// One `<a:p>` paragraph inside a shape's text body.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ShapeParagraph {
    /// `<a:pPr algn="..."/>` — `l`/`ctr`/`r`/`just`. None ⇒ `l`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// Plain-text runs. We reuse the SST `TextRun` shape since the
    /// rendered properties overlap (text, bold/italic, size, color,
    /// font name).
    pub runs: Vec<crate::schema::TextRun>,
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
    /// Radar style: `standard`, `marker`, `filled`. Only meaningful for
    /// chart_type == radar. ECMA-376 §21.2.2.176; defaults to
    /// `standard` (line, no markers) per the schema, though Excel's UI
    /// default for new radar charts is `marker`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radar_style: Option<String>,
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
    /// `<c:majorUnit val="N"/>` on the primary value axis — the explicit
    /// tick step. When present, the renderer generates ticks at
    /// `min + k*majorUnit` instead of using its niceTicks heuristic, so
    /// axis labels match Excel's authored cadence (e.g. NWC line chart
    /// authored `<c:majorUnit val="9000"/>` with `<c:max val="45000"/>`
    /// produces 0/9/18/27/36/45 once `dispUnits=1000` scales tick labels).
    /// Stored in source units (before any `dispUnits` divisor).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major_unit: Option<f64>,
    /// Same as `major_unit`, but for the secondary value axis.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major_unit_secondary: Option<f64>,
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
    /// Stock-chart decoration toggles (ECMA-376 §21.2.2.207).
    /// `hiLowLines` connect each category's high+low values with a
    /// vertical line; xlsxwriter emits `<c:hiLowLines/>` by default
    /// for HLC/OHLC stock. `upDownBars` paint a column between
    /// open and close (white-fill for up days, black-fill for down)
    /// — only meaningful when series count >= 4 (OHLC). `dropLines`
    /// connect each value point down to the category axis; rarely
    /// authored on stock charts but legal.
    #[serde(default, skip_serializing_if = "is_false")]
    pub stock_hi_low_lines: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stock_up_down_bars: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stock_drop_lines: bool,
    /// chartEx (`cx:`) series layout id: `waterfall`, `funnel`,
    /// `treemap`, `sunburst`, `boxWhisker`, `paretoLine`, `regionMap`,
    /// `clusteredColumn` (histogram). When set, `chart_type` is
    /// `chartex` and the renderer dispatches on this field. Today
    /// only `waterfall` is rendered; others fall back to the placeholder
    /// chart box.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx_layout: Option<String>,
    /// Per-point subtotal flags for `cx_layout == "waterfall"` (and
    /// future pareto/histogram variants). 0-based category indices
    /// flagged as subtotal/total bars — the renderer paints them
    /// from the floor (not stacked on the previous cumulative).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cx_subtotal_indices: Vec<u32>,
    /// Multi-level categories for hierarchical chartEx layouts
    /// (treemap, sunburst). Outer Vec = depth (column 0 = outermost
    /// parent, last column = leaf label). Each inner Vec is parallel
    /// to `series[0].values` (one entry per data point). Populated by
    /// the chart-ref resolver when `categories_ref` spans more than
    /// one column. When this is non-empty the (1D) `categories`
    /// field holds the innermost (leaf) column for backward compat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cx_category_levels: Vec<Vec<String>>,
    /// Waterfall fill colors: `[increment, decrement, subtotal]`.
    /// Each entry is a CSS color string, or empty when authored
    /// `<cx:layoutPr><cx:{increment,decrement,subtotal}>` is absent
    /// — in which case the renderer uses Office defaults (green /
    /// red / blue).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx_waterfall_increment_color: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx_waterfall_decrement_color: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx_waterfall_subtotal_color: Option<String>,
    /// RegionMap (`cx_layout == "regionMap"`) color scale stops.
    /// Excel authors a 2-stop palette via `<cx:valueColors>` with just
    /// `<cx:minColor>` + `<cx:maxColor>`, or a 3-stop diverging palette
    /// with all three. Each entry is a CSS color string resolved through
    /// the workbook theme; absent slots stay `None` and the renderer
    /// substitutes its default ramp (near-white → accent1).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx_region_map_min_color: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx_region_map_mid_color: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cx_region_map_max_color: Option<String>,
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
