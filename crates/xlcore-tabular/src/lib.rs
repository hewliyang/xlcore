//! CSV / Parquet → `WorkbookLayout` adapters.
//!
//! The goal is to feed the existing `xlsx-preview` canvas renderer with a
//! one-sheet, **unstyled** workbook for any tabular file: same JSON IR, same
//! renderer code path, no Excel styling machinery involved beyond a single
//! default font.
//!
//! Public surface:
//! - [`extract_csv`] — always available
//! - [`extract_parquet`] — behind the `parquet` cargo feature
//!
//! Both return `(WorkbookLayout, LoadReport)` to mirror the xlsx pipeline.

use xlcore_export::{Cell, Col, ColumnarCells, RowMetaBlob, Sheet, Styles, WorkbookLayout};

mod csv_reader;

#[cfg(feature = "parquet")]
mod parquet_reader;

pub use csv_reader::{extract_csv, extract_csv_reader, CsvOptions};

#[cfg(feature = "parquet")]
pub use parquet_reader::{extract_parquet, ParquetOptions};

// ─── shared helpers ────────────────────────────────────────────────────────

/// Default column width when a column has no content (≈ Excel's 8.43 chars).
const DEFAULT_COL_WIDTH_PX: f32 = 64.0;
/// Default row height (Excel: 15 pt ≈ 20 px @ 96 dpi).
const DEFAULT_ROW_HEIGHT_PX: f32 = 20.0;

/// Heuristic average character width for Calibri 11 at 96 dpi. Real font
/// metrics would need a measure pass on canvas; this constant is biased
/// slightly wide so capitals (M / W / cap-heavy headers) don't get clipped.
const AVG_CHAR_PX: f32 = 8.5;
/// Horizontal padding inside a cell (left + right).
const CELL_PADDING_PX: f32 = 10.0;
/// Auto-width clamps. Below 50 px headers get clipped; above 400 px a single
/// pathological row would stretch the whole preview.
const MIN_COL_WIDTH_PX: f32 = 50.0;
const MAX_COL_WIDTH_PX: f32 = 400.0;

/// How many rows we sample for the auto-width pass. Bounded so a 10M-row file
/// still measures in milliseconds.
const AUTO_WIDTH_SAMPLE_ROWS: usize = 2000;

/// Build the minimal `Styles` block the renderer needs: one default font,
/// nothing else. The renderer's `styleIndex` lookup falls through to defaults
/// when fills/borders/cellXfs are empty.
fn empty_styles() -> Styles {
    Styles {
        fonts: Vec::new(),
        fills: Vec::new(),
        borders: Vec::new(),
        cell_xfs: Vec::new(),
        num_fmts: Vec::new(),
        default_font: "Calibri".to_string(),
        default_font_size: 11.0,
    }
}

/// Build a `WorkbookLayout` wrapping a single already-populated sheet. The
/// sheet's `rows` field is consumed by `compactify` into the columnar blobs;
/// callers must populate `rows` (not `cells`/`row_meta`/`value_pool`/…) before
/// calling this.
fn finalize_layout(mut sheet: Sheet) -> WorkbookLayout {
    // compactify() expects the columnar/pool fields to start empty.
    sheet.cells = ColumnarCells::default();
    sheet.row_meta = RowMetaBlob::default();
    sheet.value_pool = Vec::new();
    sheet.formula_pool = Vec::new();
    sheet.inline_runs = Vec::new();

    let mut layout = WorkbookLayout {
        sheets: vec![sheet],
        styles: empty_styles(),
        shared_strings: Vec::new(),
        shared_string_runs: Vec::new(),
        dxfs: Vec::new(),
        table_styles: Vec::new(),
        theme: None,
        defined_names: Vec::new(),
        active_sheet_index: Some(0),
    };
    xlcore_export::compactify(&mut layout);
    layout
}

/// Empty sheet skeleton with sensible non-styling defaults.
fn empty_sheet(name: String) -> Sheet {
    Sheet {
        index: 0,
        name,
        state: None,
        tab_color: None,
        max_row: 0,
        max_col: 0,
        default_col_width_px: DEFAULT_COL_WIDTH_PX,
        default_row_height_px: DEFAULT_ROW_HEIGHT_PX,
        cols: Vec::new(),
        rows: Vec::new(),
        merges: Vec::new(),
        auto_filter_range: None,
        freeze: None,
        show_grid_lines: true,
        conditional_formats: Vec::new(),
        drawings: Vec::new(),
        tables: Vec::new(),
        cells: ColumnarCells::default(),
        row_meta: RowMetaBlob::default(),
        value_pool: Vec::new(),
        formula_pool: Vec::new(),
        inline_runs: Vec::new(),
        hyperlinks: Vec::new(),
        comments: Vec::new(),
        pivots: Vec::new(),
        outline_pr: None,
        sparkline_groups: Vec::new(),
    }
}

/// Compute auto column widths from the longest sampled string per column.
/// Inputs are widths in **characters**; conversion to px happens here.
fn cols_from_char_widths(char_widths: &[usize]) -> Vec<Col> {
    char_widths
        .iter()
        .enumerate()
        .map(|(i, chars)| {
            let px = (*chars as f32) * AVG_CHAR_PX + CELL_PADDING_PX;
            let width = px.clamp(MIN_COL_WIDTH_PX, MAX_COL_WIDTH_PX);
            Col {
                min: (i as u32) + 1,
                max: (i as u32) + 1,
                width_px: width,
                style_index: None,
                hidden: false,
                outline_level: 0,
            }
        })
        .collect()
}

/// One cell, built positionally (1-based row/col like the rest of the
/// pipeline). `kind` is one of `"n"`, `"b"`, `"str"`, `"s"`. Empty/None
/// callers should simply not emit a cell.
fn make_cell(row: u32, col: u32, kind: &str, value: String) -> Cell {
    Cell {
        r: row,
        c: col,
        kind: kind.to_string(),
        value: Some(value),
        formula: None,
        style_index: None,
        runs: Vec::new(),
    }
}

/// Inferred cell kind/value for a raw CSV string token.
#[derive(Debug)]
enum InferredCell {
    /// Empty token — caller should drop the cell.
    Empty,
    Number(f64),
    Bool(bool),
    /// String fallback (preserves leading zeros, units, dates, etc.).
    Str,
}

/// Lightweight per-token type inference. Conservative: anything ambiguous
/// stays a string so the user never silently loses leading zeros (ZIPs,
/// phone numbers) or has dates re-rendered as serials.
fn infer_csv_token(raw: &str) -> InferredCell {
    let s = raw.trim();
    if s.is_empty() {
        return InferredCell::Empty;
    }
    // Booleans: case-insensitive only for the two canonical spellings.
    match s {
        "true" | "TRUE" | "True" => return InferredCell::Bool(true),
        "false" | "FALSE" | "False" => return InferredCell::Bool(false),
        _ => {}
    }
    if looks_numeric(s) {
        if let Ok(n) = s.parse::<f64>() {
            if n.is_finite() && n.abs() < 9_007_199_254_740_992.0 {
                return InferredCell::Number(n);
            }
        }
    }
    InferredCell::Str
}

/// Loose check before paying for `f64::from_str`. Also rejects strings with
/// leading zeros (e.g. `"00123"`, `"0123"`) so identifier-shaped tokens stay
/// strings. A bare `"0"`, `"0.5"`, or `"-0.5"` is still numeric.
fn looks_numeric(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let (sign_len, rest) = match bytes[0] {
        b'+' | b'-' => (1usize, &bytes[1..]),
        _ => (0usize, bytes),
    };
    if rest.is_empty() {
        return false;
    }
    // Reject leading zeros: "0123" stays a string, "0" / "0.5" do not.
    if rest.len() > 1 && rest[0] == b'0' && rest[1].is_ascii_digit() {
        return false;
    }
    // Must contain at least one digit and only the digit/./e/E/+/- charset.
    let mut saw_digit = false;
    for &b in &bytes[sign_len..] {
        match b {
            b'0'..=b'9' => saw_digit = true,
            b'.' | b'e' | b'E' | b'+' | b'-' => {}
            _ => return false,
        }
    }
    saw_digit
}

/// Update the running max-chars-per-column tracker.
fn track_col_width(char_widths: &mut Vec<usize>, col_idx: usize, token: &str) {
    if char_widths.len() <= col_idx {
        char_widths.resize(col_idx + 1, 0);
    }
    let len = token.chars().count();
    if len > char_widths[col_idx] {
        char_widths[col_idx] = len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_numeric_rejects_leading_zeros() {
        assert!(looks_numeric("0"));
        assert!(looks_numeric("0.5"));
        assert!(looks_numeric("-0.5"));
        assert!(looks_numeric("1234"));
        assert!(looks_numeric("-1.2e3"));
        assert!(!looks_numeric("0123"));
        assert!(!looks_numeric("007"));
        assert!(!looks_numeric("12a"));
        assert!(!looks_numeric(""));
        assert!(!looks_numeric("-"));
    }

    #[test]
    fn infer_token_classifies_basics() {
        assert!(matches!(infer_csv_token(""), InferredCell::Empty));
        assert!(matches!(infer_csv_token(" "), InferredCell::Empty));
        assert!(matches!(infer_csv_token("42"), InferredCell::Number(_)));
        assert!(matches!(infer_csv_token("TRUE"), InferredCell::Bool(true)));
        assert!(matches!(
            infer_csv_token("false"),
            InferredCell::Bool(false)
        ));
        assert!(matches!(infer_csv_token("00123"), InferredCell::Str));
        assert!(matches!(infer_csv_token("hello"), InferredCell::Str));
        // NaN-shaped strings shouldn't be reported as numbers.
        assert!(matches!(infer_csv_token("NaN"), InferredCell::Str));
    }

    #[test]
    fn auto_widths_clamp() {
        let cols = cols_from_char_widths(&[3, 100, 0]);
        assert_eq!(cols.len(), 3);
        assert!(cols[0].width_px >= MIN_COL_WIDTH_PX);
        assert!(cols[1].width_px <= MAX_COL_WIDTH_PX);
        // Empty column still gets a sensible default.
        assert!(cols[2].width_px >= MIN_COL_WIDTH_PX);
        assert_eq!(cols[0].min, 1);
        assert_eq!(cols[1].min, 2);
    }
}

// Re-export the things downstream crates (`xlcore-wasm`, `xlcore-cli`) need.
pub use xlcore_export::WorkbookLayout as Layout;

/// Errors surfaced from the tabular adapters. Distinct from `XlsxLoadError`
/// because the failure modes don't overlap (no OOXML schema, no zip parse).
#[derive(Debug, thiserror::Error)]
pub enum TabularError {
    #[error("csv parse error: {0}")]
    Csv(#[from] csv::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "parquet")]
    #[error("parquet error: {0}")]
    Parquet(String),

    #[error("parquet support not compiled in (enable the `parquet` feature)")]
    ParquetDisabled,

    #[error("empty file: no rows or columns to render")]
    Empty,
}
