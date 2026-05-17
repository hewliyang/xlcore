//! `WorkbookLayout` — the JSON contract between the Rust extractor and the
//! TS canvas renderer. Stable-ish, semver inside this crate.
//!
//! Coordinate units: pixels at 96 DPI. The renderer maps these to the canvas
//! 1:1 by default.
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
    /// Custom user-defined table styles (`<tableStyles><tableStyle>`).
    /// Excel's *built-in* styles (`TableStyleMedium2`, `TableStyleLight1`,
    /// …) are not enumerated here — the renderer derives those from
    /// the trailing integer in the style name. Custom styles need the
    /// per-element `dxfId` references because the fill/font live in
    /// the workbook's `dxfs` table, not the style name. Look up by
    /// `TableStyle.name` from `Table.style`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_styles: Vec<CustomTableStyle>,
    /// Workbook theme (`xl/theme/theme1.xml`) — color palette + font scheme.
    /// `Color { theme: N }` references resolve against `theme.colors[N]`.
    /// `None` when the workbook ships no theme part (rare); renderer falls
    /// back to the Office 2007+ default palette.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Workbook and sheet-scoped `<definedName>` entries. Used by
    /// in-workbook hyperlinks whose `location` is a bare name (e.g. `Top`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defined_names: Vec<DefinedName>,
    /// Index into `sheets` of the tab that should be focused on initial
    /// render. Mirrors `xl/workbook.xml`'s `<workbookView activeTab="N"/>`.
    /// `None` (or omitted) → renderer defaults to sheet 0.
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
    /// Target formula / reference text, e.g. `'Sheet 1'!$A$1`.
    pub formula: String,
    /// Zero-based sheet index for sheet-scoped names. Omitted for workbook
    /// scope.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_sheet_id: Option<u32>,
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
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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
    /// OOXML `<u val="..."/>` variant when not the default `single`.
    /// One of `"single"` / `"double"` / `"singleAccounting"` /
    /// `"doubleAccounting"`. Absent = `single` (matches the OOXML default).
    /// Renderer paints `double*` as two parallel strokes; the
    /// accounting variants currently fall through to single/double
    /// (the "line extends across the full cell width" semantics are
    /// not honored yet — tracked in PARITY.md).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_style: Option<String>,
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
    /// OOXML `<vertAlign val="..."/>` — `"superscript"` or
    /// `"subscript"`. `"baseline"` (the default) is omitted. Renderer
    /// draws sup/sub at ~58% of the run's font size, shifted ±33%/+14% of
    /// the base font's em above/below the baseline.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vert_align: Option<String>,
    /// OOXML `<family val="N"/>` — numeric font-family hint (0..5):
    /// 0=N/A, 1=Roman (serif), 2=Swiss (sans-serif), 3=Modern (monospace),
    /// 4=Script (cursive), 5=Decorative (fantasy). Renderer uses this to
    /// pick a richer CSS fallback so a workbook authored in a serif
    /// typeface that's not installed locally still falls back to a serif
    /// (not the generic sans-serif default).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<u8>,
    /// OOXML `<scheme val="major|minor"/>` — theme-font reference. When
    /// present, the run logically references the workbook's theme major /
    /// minor font; the `<rFont>` cache may be stale if a different theme
    /// document has been swapped in. Renderer prefers the resolved theme
    /// font over `font_name` when this is set. `"none"` is omitted.
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
    /// Workbook sheet visibility. `None` means visible.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// `<sheetPr><tabColor/>` as unresolved `Color`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_color: Option<Color>,
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
    /// Worksheet-level `<autoFilter ref="...">` range. Table-scoped
    /// autoFilters live on `Table.has_auto_filter`; this captures the
    /// plain sheet autoFilter used by Data → Filter. Renderer paints
    /// header dropdown chevrons and relies on serialized row `hidden`
    /// flags for the saved filtered-row result.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_filter_range: Option<Merge>,
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
    /// Sparkline groups from the worksheet's `<extLst>` (x14 ext URI
    /// `{05C60535-1F16-4fd2-B633-F4F36F0B64E0}`). Each group shares
    /// type/colors/axis settings across N anchored sparklines.
    /// Renderer paints one mini-chart per anchor cell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sparkline_groups: Vec<SparklineGroup>,
}

/// One `<x14:sparklineGroup>` — shared chrome across N `<x14:sparkline>`
/// children. All booleans default false unless noted (matches OOXML).
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct SparklineGroup {
    /// `"line"` (default), `"column"`, or `"stacked"` (win/loss).
    pub spark_type: String,
    /// Default 0.75pt — matches Excel's UI default for new sparklines.
    pub line_weight: f32,
    pub markers: bool,
    pub high: bool,
    pub low: bool,
    pub first: bool,
    pub last: bool,
    pub negative: bool,
    /// `displayXAxis=1` paints a horizontal axis line at zero when the
    /// data crosses zero. (Excel calls this "Show Axis".)
    pub display_x_axis: bool,
    pub right_to_left: bool,
    /// `"gap"` (default), `"zero"`, or `"span"` — controls how empty
    /// cells in the data range are treated.
    pub display_empty_cells_as: String,
    /// `"individual"` (default), `"group"`, or `"custom"`.
    pub min_axis_type: String,
    pub max_axis_type: String,
    /// Set when `min_axis_type == "custom"`.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_min: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_max: Option<f64>,
    /// Resolved when `min_axis_type == "group"` — the renderer should
    /// use this as both the per-cell min and max so the entire group
    /// shares one y-scale. `None` when not in group mode (or no data).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_min: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_max: Option<f64>,
    /// Series fill / line color (hex `RRGGBB`). `None` ⇒ renderer
    /// falls back to a sensible default (theme accent1).
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
    /// Anchored sparklines that share this group's chrome.
    pub sparklines: Vec<Sparkline>,
}

/// One `<x14:sparkline>` — anchored at one cell, drawing values from
/// `formula`. Values are resolved post-extract (see
/// `lib.rs::resolve_sparkline_refs`); preserve `None` for empty/text
/// cells so the renderer can honor `displayEmptyCellsAs`.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct Sparkline {
    /// 1-based anchor cell row.
    pub r: u32,
    /// 1-based anchor cell column.
    pub c: u32,
    /// Source-data formula, e.g. `"Sheet1!B2:G2"`. Kept for debugging.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// Resolved data values in source order. `None` entries indicate
    /// empty / non-numeric source cells; the renderer interprets them
    /// according to the group's `displayEmptyCellsAs`.
    pub values: Vec<Option<f64>>,
}

/// `<outlinePr>` defaults from `<sheetPr>`. Both fields default to true
/// when `<outlinePr>` is absent (matches Excel/OOXML spec).
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

/// One `<pivotTableDefinition>` — just enough to paint the cosmetic
/// chrome. The actual values are already in `Sheet.rows` (Excel/SpreadJS
/// always materialize pivot output cells into the sheet's `<sheetData>`
/// with explicit cell xfs).
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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
/// Custom user table styles (`<tableStyles>` in styles.xml) ARE
/// resolved — they're surfaced on `WorkbookLayout.table_styles` and
/// the renderer looks them up by `name`. When neither a built-in nor
/// a custom match is found the renderer falls back to the default
/// Medium2 accent.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
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

/// One `<tableStyle name="…">` entry from `xl/styles.xml`'s
/// `<tableStyles>` block. Each style is a bag of
/// `<tableStyleElement type="…" dxfId="N"/>` references; the renderer
/// uses the dxf at index N as an overlay for that band of the table
/// (header row, first/second row stripe, total row, etc.).
///
/// We surface the elements we actually paint today. Bands we don't
/// implement (column stripes, subtotal rows, page-field cells) are
/// dropped on the floor — add fields as the renderer learns more
/// bands. ECMA-376 §18.8.40 has the full enumeration.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CustomTableStyle {
    pub name: String,
    /// `dxfId` for the whole-table overlay (border defaults, base fill).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whole_table: Option<u32>,
    /// `dxfId` for the header row band (fill + bold/colored font).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_row: Option<u32>,
    /// `dxfId` for the totals row band.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_row: Option<u32>,
    /// `dxfId` for the first row stripe (the tint applied to alternating
    /// data rows; in Excel's Medium styles this is the accent-tinted band).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_row_stripe: Option<u32>,
    /// `dxfId` for the second row stripe (the *other* alternating band;
    /// usually unset — implied as the default "no fill" complement).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second_row_stripe: Option<u32>,
    /// `dxfId` for the first column band (typically bold).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_column: Option<u32>,
    /// `dxfId` for the last column band (typically bold).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_column: Option<u32>,
}

mod charts;
pub use charts::*;

mod tail;
pub use tail::*;
