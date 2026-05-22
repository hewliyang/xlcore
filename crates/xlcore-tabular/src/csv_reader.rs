//! CSV → `WorkbookLayout` adapter.
//!
//! Single-pass over a buffered reader. We buffer once so delimiter sniffing and
//! `csv::Reader` can consume the same bytes, then build per-column max-char
//! trackers as records are read. Type inference is intentionally conservative
//! — see `infer_csv_token` in `lib.rs`.

use std::io::Read;

use xlcore_export::{Row, WorkbookLayout};
use xlcore_io::LoadReport;

use super::{
    cols_from_char_widths, empty_sheet, finalize_layout, infer_csv_token, make_cell,
    track_col_width, InferredCell, TabularError, AUTO_WIDTH_SAMPLE_ROWS,
};

/// Knobs for CSV ingestion. Matches the conventions Excel uses on Text Import.
#[derive(Debug, Clone)]
pub struct CsvOptions {
    /// Field delimiter. `None` triggers a tiny heuristic sniff over the first
    /// line (comma / tab / semicolon / pipe by frequency).
    pub delimiter: Option<u8>,
    /// Hard cap on rendered rows. Files larger than this are truncated and a
    /// warning is appended to the returned `LoadReport`.
    pub max_rows: usize,
    /// Quote character. Defaults to `"`.
    pub quote: u8,
    /// Sheet name shown in the renderer's tab strip.
    pub sheet_name: String,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            delimiter: None,
            max_rows: 100_000,
            quote: b'"',
            sheet_name: "data".to_string(),
        }
    }
}

/// Convenience: parse from an in-memory byte slice.
pub fn extract_csv(
    bytes: &[u8],
    options: &CsvOptions,
) -> Result<(WorkbookLayout, LoadReport), TabularError> {
    extract_csv_reader(std::io::Cursor::new(bytes), options)
}

/// Entry-point for any `Read`; buffers the input once for delimiter sniffing.
pub fn extract_csv_reader<R: Read>(
    mut reader: R,
    options: &CsvOptions,
) -> Result<(WorkbookLayout, LoadReport), TabularError> {
    let mut report = LoadReport::default();

    // Sniff the delimiter from the first ~4 KiB of input if not specified.
    // We have to buffer to sniff anyway; just buffer the whole thing here. The
    // `csv` crate is already fully buffered internally, so this trades a bit
    // of peak RSS for simpler code.
    let mut all = Vec::new();
    reader.read_to_end(&mut all)?;
    let delimiter = options.delimiter.unwrap_or_else(|| sniff_delimiter(&all));

    let mut builder = csv::ReaderBuilder::new();
    builder
        .delimiter(delimiter)
        .has_headers(false) // render every record as a worksheet row
        .quote(options.quote)
        .flexible(true);
    let mut rdr = builder.from_reader(std::io::Cursor::new(&all));

    // Tracking state.
    let mut rows: Vec<Row> = Vec::new();
    let mut char_widths: Vec<usize> = Vec::new();
    let mut max_col: u32 = 0;
    let mut record = csv::ByteRecord::new();
    let mut emitted: usize = 0;
    let mut total_seen: usize = 0;
    let mut truncated = false;
    let max_rows = options.max_rows.max(1);

    while rdr.read_byte_record(&mut record)? {
        total_seen += 1;
        if emitted >= max_rows {
            truncated = true;
            // Keep counting to report the true total in the warning.
            continue;
        }
        let row_index = (emitted as u32) + 1;
        let mut row_cells = Vec::with_capacity(record.len());
        for (col_idx, field) in record.iter().enumerate() {
            let token = String::from_utf8_lossy(field);
            let col_num = (col_idx as u32) + 1;
            if col_num > max_col {
                max_col = col_num;
            }
            // Width sampling: only over the auto-width window (cheap on huge
            // files; the rest of the column rarely changes the bound).
            if emitted < AUTO_WIDTH_SAMPLE_ROWS {
                track_col_width(&mut char_widths, col_idx, token.as_ref());
            }
            match infer_csv_token(token.as_ref()) {
                InferredCell::Empty => {} // omit cell entirely (matches xlsx)
                InferredCell::Number(n) => {
                    row_cells.push(make_cell(row_index, col_num, "n", format_number(n)));
                }
                InferredCell::Bool(b) => {
                    row_cells.push(make_cell(
                        row_index,
                        col_num,
                        "b",
                        if b { "1" } else { "0" }.to_string(),
                    ));
                }
                InferredCell::Str => {
                    row_cells.push(make_cell(row_index, col_num, "str", token.into_owned()));
                }
            }
        }
        rows.push(Row {
            index: row_index,
            height_px: None,
            cells: row_cells,
            style_index: None,
            hidden: false,
            outline_level: 0,
        });
        emitted += 1;
    }

    if rows.is_empty() && max_col == 0 {
        return Err(TabularError::Empty);
    }

    if truncated {
        report.warnings.push(format!(
            "csv truncated: rendered {} of {} rows (max_rows={})",
            emitted, total_seen, max_rows
        ));
    }
    let mut sheet = empty_sheet(options.sheet_name.clone());
    sheet.max_row = emitted as u32;
    sheet.max_col = max_col;
    sheet.cols = cols_from_char_widths(&char_widths);
    sheet.rows = rows;

    Ok((finalize_layout(sheet), report))
}

/// Format an `f64` for storage in the value pool. Whole-valued floats render
/// as integers (Excel's behaviour with the General format).
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        // Use Rust's default `{}` which already trims trailing zeros and
        // switches to scientific only at extreme magnitudes.
        format!("{n}")
    }
}

/// Pick the most plausible delimiter from `,`, `\t`, `;`, `|` based on
/// occurrence counts in the first non-empty line. Defaults to `,` when the
/// first line contains none of them.
fn sniff_delimiter(bytes: &[u8]) -> u8 {
    let head_len = bytes.len().min(4096);
    let head = &bytes[..head_len];
    // First newline-terminated line, ignoring CR.
    let line_end = head.iter().position(|&b| b == b'\n').unwrap_or(head.len());
    let line = &head[..line_end];

    let mut best = b',';
    let mut best_count: usize = 0;
    for &cand in b",\t;|" {
        let count = line.iter().filter(|&&b| b == cand).count();
        if count > best_count {
            best_count = count;
            best = cand;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingests_basic_csv() {
        let bytes = b"name,age\nAda,36\nGrace,85\n";
        let (layout, report) = extract_csv(bytes, &CsvOptions::default()).expect("csv extract");
        assert!(report.is_clean());
        let sheet = &layout.sheets[0];
        assert_eq!(sheet.max_row, 3);
        assert_eq!(sheet.max_col, 2);
        assert_eq!(sheet.cols.len(), 2);
        // Cells were compactified into the columnar blob.
        assert_eq!(sheet.cells.count, 6);
        // Value pool should hold the unique strings plus number strings.
        assert!(sheet.value_pool.iter().any(|v| v == "Ada"));
        assert!(sheet.value_pool.iter().any(|v| v == "36"));
    }

    #[test]
    fn sniffs_tab_delimiter() {
        let bytes = b"a\tb\tc\n1\t2\t3\n";
        let (layout, _) = extract_csv(bytes, &CsvOptions::default()).unwrap();
        assert_eq!(layout.sheets[0].max_col, 3);
    }

    #[test]
    fn sniffs_semicolon_delimiter() {
        let bytes = b"a;b\nx;y\n";
        let (layout, _) = extract_csv(bytes, &CsvOptions::default()).unwrap();
        assert_eq!(layout.sheets[0].max_col, 2);
    }

    #[test]
    fn truncates_with_warning() {
        let mut bytes = String::from("a,b\n");
        for i in 0..50 {
            bytes.push_str(&format!("{i},x\n"));
        }
        let opts = CsvOptions {
            max_rows: 10,
            ..CsvOptions::default()
        };
        let (layout, report) = extract_csv(bytes.as_bytes(), &opts).unwrap();
        assert_eq!(layout.sheets[0].max_row, 10);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("truncated"));
    }

    #[test]
    fn zero_max_rows_still_renders_one_row() {
        let opts = CsvOptions {
            max_rows: 0,
            ..CsvOptions::default()
        };
        let (layout, report) = extract_csv(b"a,b\n1,2\n", &opts).unwrap();
        assert_eq!(layout.sheets[0].max_row, 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("max_rows=1"));
    }

    #[test]
    fn preserves_leading_zeros_as_strings() {
        let bytes = b"zip\n00123\n";
        let (layout, _) = extract_csv(bytes, &CsvOptions::default()).unwrap();
        assert!(layout.sheets[0].value_pool.iter().any(|v| v == "00123"));
    }

    #[test]
    fn invalid_utf8_is_replaced_not_dropped() {
        let bytes = b"name\nbad-\xff\n";
        let (layout, _) = extract_csv(bytes, &CsvOptions::default()).unwrap();
        assert!(layout.sheets[0]
            .value_pool
            .iter()
            .any(|v| v == "bad-\u{fffd}"));
    }

    #[test]
    fn empty_file_errors() {
        let err = extract_csv(b"", &CsvOptions::default()).err().unwrap();
        assert!(matches!(err, TabularError::Empty));
    }

    #[test]
    fn ragged_rows_extend_max_col() {
        // Row 2 has more fields than row 1. We should report max_col=3.
        let bytes = b"a,b\nx,y,z\n";
        let (layout, _) = extract_csv(bytes, &CsvOptions::default()).unwrap();
        assert_eq!(layout.sheets[0].max_col, 3);
    }

    #[test]
    fn whole_floats_render_as_ints() {
        let bytes = b"v\n2.0\n3.5\n";
        let (layout, _) = extract_csv(bytes, &CsvOptions::default()).unwrap();
        let pool = &layout.sheets[0].value_pool;
        assert!(pool.iter().any(|v| v == "2"), "got {:?}", pool);
        assert!(pool.iter().any(|v| v == "3.5"), "got {:?}", pool);
    }
}
