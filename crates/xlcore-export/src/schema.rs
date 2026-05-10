//! `WorkbookLayout` — the JSON contract between the Rust extractor and the
//! TS canvas renderer. Stable-ish, semver inside this crate.
//!
//! Coordinate units: pixels at 96 DPI. The renderer maps these to the canvas
//! 1:1 by default.
use serde::{Deserialize, Serialize};
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookLayout {
    pub sheets: Vec<Sheet>,
    pub styles: Styles,
    pub shared_strings: Vec<String>,
    /// Rich-text runs aligned with `shared_strings` by index. Inner `Vec`
    /// is empty when the SST entry is plain text (no `<r>` runs); the
    /// renderer falls back to the cell's base font in that case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_string_runs: Vec<Vec<TextRun>>,
    /// Differential formats (`<dxfs>`) — overlay style records that
    /// conditional-formatting rules and table styles reference by index.
    /// Indexed by `CfRule.dxf_id`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dxfs: Vec<Dxf>,
    /// Workbook theme (`xl/theme/theme1.xml`) — color palette + font scheme.
    /// `Color { theme: N }` references resolve against `theme.colors[N]`.
    /// `None` when the workbook ships no theme part (rare); renderer falls
    /// back to the Office 2007+ default palette.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Index into `sheets` of the tab that should be focused on initial
    /// render. Mirrors `xl/workbook.xml`'s `<workbookView activeTab="N"/>`.
    /// `None` (or omitted) → renderer defaults to sheet 0.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_sheet_index: Option<u32>,
}
/// Workbook theme: a 12-entry color palette plus font scheme names.
///
/// `colors` is pre-remapped to the spreadsheet `theme="N"` indexing
/// convention (the OOXML quirk where lt1/dk1 and lt2/dk2 are swapped
/// relative to the XML element order in `<a:clrScheme>`):
///
/// | idx | role     | xml element |
/// |-----|----------|-------------|
/// | 0   | lt1      | `<a:lt1>`   |
/// | 1   | dk1      | `<a:dk1>`   |
/// | 2   | lt2      | `<a:lt2>`   |
/// | 3   | dk2      | `<a:dk2>`   |
/// | 4-9 | accent1-6| `<a:accent1>`..`<a:accent6>` |
/// | 10  | hlink    | `<a:hlink>` |
/// | 11  | folHlink | `<a:folHlink>` |
///
/// Each entry is a 6-char hex string `"RRGGBB"` (no leading `#`,
/// matching the rest of `Color.rgb`). When the source theme uses an
/// `<a:sysClr>` with a `lastClr` fallback we resolve to that; if a
/// color can't be resolved (preset names, scrgb percentages we don't
/// handle yet) we fall back to the Office default for that slot.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    /// 12 hex colors, indexed by `Color.theme`.
    pub colors: Vec<String>,
    /// `<a:majorFont>` Latin typeface (e.g. "Calibri Light"). Used for
    /// chart titles when the chart references the major theme font.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major_font: Option<String>,
    /// `<a:minorFont>` Latin typeface (e.g. "Calibri"). The default body
    /// font; cells with no explicit `<rFont>` inherit this.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minor_font: Option<String>,
}
/// One styled span inside a rich-text cell. Mirrors `<r><rPr/><t/></r>` in
/// OOXML. Properties left as `None`/`false` mean "inherit from the cell's
/// own font".
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,
    /// Font size in points (matches `Font.size`).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub index: u32,
    pub name: String,
    /// 1-based, inclusive. (0,0) when sheet is empty.
    pub max_row: u32,
    pub max_col: u32,
    /// Default column width in px.
    pub default_col_width_px: f32,
    /// Default row height in px.
    pub default_row_height_px: f32,
    /// Custom column widths (sparse). Each entry covers cols `min..=max`.
    pub cols: Vec<Col>,
    /// **Wire-invisible**: the extractor populates this Vec, then a
    /// post-pass collapses it into the columnar blobs below and clears
    /// it. Always empty in serialized JSON. Hidden from TS bindings.
    #[cfg_attr(feature = "typescript", ts(skip))]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Row>,
    pub merges: Vec<Merge>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub freeze: Option<Freeze>,
    pub show_grid_lines: bool,
    /// Conditional formatting blocks, in source order. Renderer applies
    /// highest-`priority` rule per cell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditional_formats: Vec<ConditionalFormat>,
    /// Drawings (charts, images) anchored on this sheet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drawings: Vec<Drawing>,
    /// `<table>` ListObjects on this sheet (one entry per
    /// `xl/tables/tableN.xml` part referenced by the worksheet).
    /// Renderer paints the table's visual chrome (header band, banded
    /// rows, filter-arrow glyphs); no filtering interactivity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<Table>,

    // ---------------------------------------------------------------
    // Columnar cell storage. The extractor still builds `rows` (above)
    // for ergonomic post-processing (chart-ref resolution, etc.); a
    // post-pass in `lib.rs::compactify_sheets` converts that into the
    // typed-array blobs below and clears `rows`. The wire format only
    // ships the columnar form. See `crates/xlcore-export/src/columnar.rs`.
    // ---------------------------------------------------------------
    /// Non-empty cell records, sorted (row asc, col asc within row).
    /// All inner blobs are base64-encoded little-endian typed arrays
    /// of length `count`. The renderer decodes these once at load time
    /// into Uint32Array/Int32Array/Uint8Array views.
    #[serde(default)]
    pub cells: ColumnarCells,
    /// Per-row metadata for rows that carry custom height/style/hidden
    /// flags or simply have any cells. Sorted by `index` ascending.
    #[serde(default)]
    pub row_meta: RowMetaBlob,
    /// Deduplicated string pool for `cells.valueIdx`. `valueIdx[i] >= 0`
    /// means "look up `value_pool[valueIdx[i]]`"; `-1` means the cell
    /// has no cached value (rare; mostly empty formula cells).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_pool: Vec<String>,
    /// Deduplicated formula pool for `cells.formulaIdx`. Most cells
    /// have no formula (`formulaIdx[i] == -1`); pool is small.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formula_pool: Vec<String>,
    /// Inline rich-text run lists for cells that carry `<r>` children
    /// directly (i.e. `kind == "inline"` with explicit runs). Indexed
    /// by `cells.runsIdx`; `-1` means "no inline runs on this cell".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inline_runs: Vec<Vec<TextRun>>,
    /// `<hyperlink>` entries from the worksheet's `<hyperlinks>` block.
    /// External `r:id` rels are resolved to absolute URLs at extract
    /// time; `location` carries internal in-workbook jumps (e.g.
    /// `'Sheet 2'!A1`). Renderer paints blue+underline over the cells
    /// in `range` and (in the browser) wires a click handler.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hyperlinks: Vec<Hyperlink>,
    /// Cell comments from the worksheet's comments part. Renderer
    /// paints a small red triangle in the top-right of `r,c` and
    /// (in the browser) shows the body in a hover popover.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
    /// Pivot tables anchored on this sheet (one entry per
    /// `xl/pivotTables/pivotTableN.xml` referenced by the worksheet).
    /// We treat pivots as cosmetic chrome only — the materialized
    /// result cells already live in `Sheet.rows` with explicit
    /// styling, so the renderer just paints filter-arrow chevrons on
    /// the row/column field-header cells. No filtering / refresh /
    /// expand-collapse interactivity. ("Cheap path" in PARITY.md.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pivots: Vec<Pivot>,
    /// OOXML `<sheetPr><outlinePr summaryBelow="..."
    /// summaryRight="..."/></sheetPr>`. Tells the renderer where the
    /// summary row/col sits relative to a group (default: below + right,
    /// matching Excel's UI default). `None` = use those defaults.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_pr: Option<OutlinePr>,
}

/// `<outlinePr>` defaults from `<sheetPr>`. Both fields default to true
/// when `<outlinePr>` is absent (matches Excel/OOXML spec).
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct OutlinePr {
    pub summary_below: bool,
    pub summary_right: bool,
}

/// One `<pivotTableDefinition>` — just enough to paint the cosmetic
/// chrome. The actual values are already in `Sheet.rows` (Excel/SpreadJS
/// always materialize pivot output cells into the sheet's `<sheetData>`
/// with explicit cell xfs).
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Pivot {
    /// Internal name (`name=` attr on `<pivotTableDefinition>`).
    pub name: String,
    /// Bounding range covered by the pivot, including header strip
    /// and grand-total row/column. Mirrors `<location ref=".">`.
    pub range: Merge,
    /// Cells (1-based r,c) that should get a filter-dropdown chevron.
    /// Computed by the extractor from the pivot's `<location>` +
    /// `<rowFields>` / `<colFields>` so the renderer doesn't need to
    /// re-derive them. Empty for pivots with no axis fields.
    pub filter_arrow_cells: Vec<CellRef>,
}

/// 1-based cell address. Used by `Pivot.filter_arrow_cells`.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CellRef {
    pub r: u32,
    pub c: u32,
}

/// One `<hyperlink>` entry from the worksheet `<hyperlinks>` block.
/// At least one of `target` / `location` is set.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Hyperlink {
    /// Range covered by this hyperlink (often a single cell, but the
    /// schema allows e.g. `A1:B3`).
    pub range: Merge,
    /// External absolute target — the `Target` of the `r:id` rel.
    /// `None` for in-workbook (`location`) links.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// In-workbook bookmark, e.g. `'Sheet 2'!A1`. Mutually-not-exclusive
    /// with `target` — Excel sometimes emits both.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Hover tooltip text.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    /// Display-string override (rare; the cell's own value is the
    /// usual visible text).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// One `<comment>` entry from the worksheet's comments part.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    /// 1-based row.
    pub r: u32,
    /// 1-based column.
    pub c: u32,
    /// Resolved author name (looked up in the comments part's
    /// `<authors>` table). Empty when `authorId` is out-of-range.
    pub author: String,
    /// Concatenated plain-text body (matches `runs` joined).
    pub text: String,
    /// Per-run styled spans, mirroring the SST rich-text shape. Empty
    /// for plain-text comments — renderer falls back to a default
    /// font in that case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<TextRun>,
}

/// One `<table>` ListObject — a named range with banded styling, an
/// optional header row, and an optional totals row. Mirrors
/// `xl/tables/tableN.xml` (CT_Table). The renderer treats this as
/// pure cosmetic chrome; the cell values themselves stay in `Sheet.rows`.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    /// Internal name (`name` attr; falls back to `displayName`).
    pub name: String,
    /// User-visible name shown in the formula bar's name box.
    pub display_name: String,
    /// Inclusive range covered by the table, including header + totals.
    pub range: Merge,
    /// 0 or 1. Excel's default is 1; some `headerRowCount="0"` tables
    /// (used as ranges-with-style) skip the header band.
    pub header_row_count: u32,
    /// 0 or 1.
    pub totals_row_count: u32,
    /// Per-column metadata (header label, totals-row label/function).
    /// `columns.len()` always equals `range.c2 - range.c1 + 1`.
    pub columns: Vec<TableColumn>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<TableStyle>,
    /// True when the table has an `<autoFilter>` child — drives whether
    /// the renderer paints filter-arrow glyphs in the header cells.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_auto_filter: bool,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    pub name: String,
    /// `sum`, `average`, `count`, `countNums`, `min`, `max`, `stdDev`,
    /// `var`, `custom`. None ⇒ no totals function for this column.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totals_row_function: Option<String>,
    /// Literal label shown in the totals row (e.g. "Total").
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totals_row_label: Option<String>,
}

/// `<tableStyleInfo>` — picks one of Excel's built-in table styles
/// (e.g. `TableStyleMedium2`) and toggles the four banding axes.
/// Custom user table styles (`<customTableStyles>` in styles.xml) are
/// NOT resolved; the renderer falls back to the default Medium2 look
/// when it doesn't recognize the style name.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct TableStyle {
    /// Built-in style name, e.g. `TableStyleMedium2`. The trailing
    /// integer indexes into the workbook's accent colors
    /// (`(N-1) % 6 → accent{1..6}`).
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
/// One drawing object placed on the sheet, with its xlsx cell-anchor.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    /// Fill foreground color (solid pattern). Background is rare in dxfs.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    /// Override number-format code, e.g. `"0.00%"`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_fmt: Option<String>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    /// Gradient stop colors in source order. v0 renderer uses the first and
    /// last stop for a linear left→right fill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gradient_stops: Vec<Color>,
}
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
    ts(export, export_to = "../../../render-ts/src/schema/")
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
