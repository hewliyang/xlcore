//! Parquet → `WorkbookLayout` adapter.
//!
//! Streams record batches via `parquet::arrow::ParquetRecordBatchReaderBuilder`,
//! downcasts each column to its concrete arrow array type, and stringifies
//! values per row into the same columnar IR the xlsx pipeline emits.
//!
//! Types are mapped conservatively. Anything we don't recognise falls back to
//! Arrow's display formatter so the user at least sees *something* rather
//! than an empty cell. This is intentional v0 behaviour — we'd rather render
//! a quirky preview than refuse the file.

use arrow_array::{
    cast::AsArray,
    types::{
        Date32Type, Date64Type, Decimal128Type, Decimal256Type, Float16Type, Float32Type,
        Float64Type, Int16Type, Int32Type, Int64Type, Int8Type, Time32MillisecondType,
        Time32SecondType, Time64MicrosecondType, Time64NanosecondType, TimestampMicrosecondType,
        TimestampMillisecondType, TimestampNanosecondType, TimestampSecondType, UInt16Type,
        UInt32Type, UInt64Type, UInt8Type,
    },
    Array, RecordBatch,
};
use arrow_schema::{DataType, TimeUnit};
use bytes::Bytes;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use xlcore_export::{Row, WorkbookLayout};
use xlcore_io::LoadReport;

use super::{
    cols_from_char_widths, empty_sheet, finalize_layout, make_cell, track_col_width, TabularError,
    AUTO_WIDTH_SAMPLE_ROWS,
};

/// Tuning knobs for the parquet adapter. Mirrors `CsvOptions` for symmetry.
#[derive(Debug, Clone)]
pub struct ParquetOptions {
    /// Hard cap on rendered rows, including the synthetic header row.
    /// Files larger than this are truncated and reported via `LoadReport`.
    pub max_rows: usize,
    pub sheet_name: String,
}

impl Default for ParquetOptions {
    fn default() -> Self {
        Self {
            max_rows: 100_000,
            sheet_name: "data".to_string(),
        }
    }
}

impl From<parquet::errors::ParquetError> for TabularError {
    fn from(e: parquet::errors::ParquetError) -> Self {
        TabularError::Parquet(e.to_string())
    }
}

impl From<arrow_schema::ArrowError> for TabularError {
    fn from(e: arrow_schema::ArrowError) -> Self {
        TabularError::Parquet(e.to_string())
    }
}

/// Read a parquet file from an in-memory byte slice.
pub fn extract_parquet(
    bytes: &[u8],
    options: &ParquetOptions,
) -> Result<(WorkbookLayout, LoadReport), TabularError> {
    let mut report = LoadReport::default();
    let buf = Bytes::copy_from_slice(bytes);
    let builder = ParquetRecordBatchReaderBuilder::try_new(buf)?;
    let arrow_schema = builder.schema().clone();
    let reader = builder.build()?;

    let column_names: Vec<String> = arrow_schema
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();
    let col_count = column_names.len();
    if col_count == 0 {
        return Err(TabularError::Empty);
    }

    // Row 1 is the header. We always emit it as plain strings (unstyled).
    let mut rows: Vec<Row> = Vec::new();
    let mut char_widths: Vec<usize> = vec![0; col_count];
    {
        let mut header_cells = Vec::with_capacity(col_count);
        for (i, name) in column_names.iter().enumerate() {
            track_col_width(&mut char_widths, i, name);
            header_cells.push(make_cell(1, (i as u32) + 1, "str", name.clone()));
        }
        rows.push(Row {
            index: 1,
            height_px: None,
            cells: header_cells,
            style_index: None,
            hidden: false,
            outline_level: 0,
        });
    }

    let mut emitted: usize = 1; // header counted in max_row
    let mut total_data_rows: usize = 0;
    let mut truncated = false;
    let max_rows = options.max_rows.max(1);

    for batch in reader {
        let batch: RecordBatch = batch?;
        total_data_rows += batch.num_rows();
        if truncated {
            // Keep counting to report an accurate total in the warning.
            continue;
        }
        let take = batch.num_rows().min(max_rows.saturating_sub(emitted));
        if take == 0 {
            truncated = true;
            continue;
        }
        for row_idx in 0..take {
            let row_num = (emitted as u32) + 1;
            let mut cells = Vec::with_capacity(col_count);
            for (col_idx, col) in batch.columns().iter().enumerate() {
                if col.is_null(row_idx) {
                    continue;
                }
                let (kind, value) = stringify_cell(col.as_ref(), row_idx);
                if emitted < AUTO_WIDTH_SAMPLE_ROWS {
                    track_col_width(&mut char_widths, col_idx, &value);
                }
                cells.push(make_cell(row_num, (col_idx as u32) + 1, kind, value));
            }
            rows.push(Row {
                index: row_num,
                height_px: None,
                cells,
                style_index: None,
                hidden: false,
                outline_level: 0,
            });
            emitted += 1;
        }
        if take < batch.num_rows() {
            truncated = true;
        }
    }

    if truncated {
        report.warnings.push(format!(
            "parquet truncated: rendered {} of {} rows including header (max_rows={})",
            emitted,
            total_data_rows + 1,
            max_rows
        ));
    }

    let mut sheet = empty_sheet(options.sheet_name.clone());
    sheet.max_row = emitted as u32;
    sheet.max_col = col_count as u32;
    sheet.cols = cols_from_char_widths(&char_widths);
    sheet.rows = rows;

    Ok((finalize_layout(sheet), report))
}

/// Translate one (array, row) pair into a cell `(kind, value)` pair.
///
/// `kind` follows the same single-letter convention used elsewhere:
/// `"n"` = number, `"b"` = bool, `"str"` = inline string.
fn stringify_cell(array: &dyn Array, row: usize) -> (&'static str, String) {
    use DataType::*;
    match array.data_type() {
        Boolean => {
            let v = array.as_boolean().value(row);
            ("b", if v { "1" } else { "0" }.to_string())
        }
        Int8 => num_cell(array.as_primitive::<Int8Type>().value(row) as f64),
        Int16 => num_cell(array.as_primitive::<Int16Type>().value(row) as f64),
        Int32 => num_cell(array.as_primitive::<Int32Type>().value(row) as f64),
        Int64 => int64_cell(array.as_primitive::<Int64Type>().value(row)),
        UInt8 => num_cell(array.as_primitive::<UInt8Type>().value(row) as f64),
        UInt16 => num_cell(array.as_primitive::<UInt16Type>().value(row) as f64),
        UInt32 => num_cell(array.as_primitive::<UInt32Type>().value(row) as f64),
        UInt64 => uint64_cell(array.as_primitive::<UInt64Type>().value(row)),
        Float16 => num_cell(array.as_primitive::<Float16Type>().value(row).to_f32() as f64),
        Float32 => num_cell(array.as_primitive::<Float32Type>().value(row) as f64),
        Float64 => num_cell(array.as_primitive::<Float64Type>().value(row)),
        Utf8 => ("str", array.as_string::<i32>().value(row).to_string()),
        LargeUtf8 => ("str", array.as_string::<i64>().value(row).to_string()),
        Date32 => (
            "str",
            arrow_array::temporal_conversions::as_date::<Date32Type>(
                array.as_primitive::<Date32Type>().value(row) as i64,
            )
            .map(|d| d.to_string())
            .unwrap_or_default(),
        ),
        Date64 => (
            "str",
            arrow_array::temporal_conversions::as_date::<Date64Type>(
                array.as_primitive::<Date64Type>().value(row),
            )
            .map(|d| d.to_string())
            .unwrap_or_default(),
        ),
        Timestamp(unit, _tz) => ("str", format_timestamp(array, row, *unit)),
        Time32(unit) => ("str", format_time32(array, row, *unit)),
        Time64(unit) => ("str", format_time64(array, row, *unit)),
        Decimal128(_, scale) => {
            let v = array.as_primitive::<Decimal128Type>().value(row);
            ("str", format_decimal_i128(v, *scale))
        }
        Decimal256(_, scale) => {
            // Decimal256 holds an i256; render via its Display.
            let v = array.as_primitive::<Decimal256Type>().value(row);
            ("str", format!("{} (scale {})", v, scale))
        }
        Binary => bytes_cell(array.as_binary::<i32>().value(row)),
        LargeBinary => bytes_cell(array.as_binary::<i64>().value(row)),
        // List / Struct / Map / Union / Dictionary / FixedSizeBinary / … —
        // delegate to arrow's `ArrayFormatter`, which produces compact
        // single-line representations (`[1, 2, 3]`, `{a: 1, b: 2}`, etc.).
        // Falling back to `format!("{:?}", ...)` instead spews multi-line
        // arrow Debug output that mangles the cell layout.
        _ => ("str", format_via_arrow(array, row)),
    }
}

/// Last-resort stringifier for types we don't translate by hand. Arrow's
/// formatter handles every type uniformly, but we keep the hand-written
/// fast paths above to (a) control number formatting (whole floats as ints,
/// f32 precision), (b) bypass formatter setup cost per cell on the common
/// primitive columns.
fn format_via_arrow(array: &dyn Array, row: usize) -> String {
    arrow_cast::display::ArrayFormatter::try_new(array, &Default::default())
        .map(|fmt| fmt.value(row).to_string())
        .unwrap_or_else(|_| String::new())
}

fn num_cell(n: f64) -> (&'static str, String) {
    if !n.is_finite() {
        return ("str", n.to_string());
    }
    if n.fract() == 0.0 && n.abs() < 1e16 {
        ("n", format!("{}", n as i64))
    } else {
        ("n", format!("{n}"))
    }
}

/// i64 values outside ±2^53 cannot round-trip through JS `number`, so keep
/// them as strings to preserve precision. Cell kind degrades to `"str"`
/// (loses right-alignment), which is the correct tradeoff for IDs.
fn int64_cell(v: i64) -> (&'static str, String) {
    if v.abs() < (1_i64 << 53) {
        ("n", v.to_string())
    } else {
        ("str", v.to_string())
    }
}

fn uint64_cell(v: u64) -> (&'static str, String) {
    if v < (1_u64 << 53) {
        ("n", v.to_string())
    } else {
        ("str", v.to_string())
    }
}

fn bytes_cell(b: &[u8]) -> (&'static str, String) {
    // Try utf-8; fall back to a compact length marker for true binary blobs.
    match std::str::from_utf8(b) {
        Ok(s) => ("str", s.to_string()),
        Err(_) => ("str", format!("<{} bytes>", b.len())),
    }
}

fn format_timestamp(array: &dyn Array, row: usize, unit: TimeUnit) -> String {
    let ndt = match unit {
        TimeUnit::Second => arrow_array::temporal_conversions::as_datetime::<TimestampSecondType>(
            array.as_primitive::<TimestampSecondType>().value(row),
        ),
        TimeUnit::Millisecond => {
            arrow_array::temporal_conversions::as_datetime::<TimestampMillisecondType>(
                array.as_primitive::<TimestampMillisecondType>().value(row),
            )
        }
        TimeUnit::Microsecond => {
            arrow_array::temporal_conversions::as_datetime::<TimestampMicrosecondType>(
                array.as_primitive::<TimestampMicrosecondType>().value(row),
            )
        }
        TimeUnit::Nanosecond => {
            arrow_array::temporal_conversions::as_datetime::<TimestampNanosecondType>(
                array.as_primitive::<TimestampNanosecondType>().value(row),
            )
        }
    };
    ndt.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn format_time32(array: &dyn Array, row: usize, unit: TimeUnit) -> String {
    match unit {
        TimeUnit::Second => arrow_array::temporal_conversions::as_time::<Time32SecondType>(
            array.as_primitive::<Time32SecondType>().value(row) as i64,
        ),
        TimeUnit::Millisecond => {
            arrow_array::temporal_conversions::as_time::<Time32MillisecondType>(
                array.as_primitive::<Time32MillisecondType>().value(row) as i64,
            )
        }
        _ => None,
    }
    .map(|t| t.to_string())
    .unwrap_or_default()
}

fn format_time64(array: &dyn Array, row: usize, unit: TimeUnit) -> String {
    match unit {
        TimeUnit::Microsecond => {
            arrow_array::temporal_conversions::as_time::<Time64MicrosecondType>(
                array.as_primitive::<Time64MicrosecondType>().value(row),
            )
        }
        TimeUnit::Nanosecond => arrow_array::temporal_conversions::as_time::<Time64NanosecondType>(
            array.as_primitive::<Time64NanosecondType>().value(row),
        ),
        _ => None,
    }
    .map(|t| t.to_string())
    .unwrap_or_default()
}

/// Format an i128 with a fixed scale (decimal exponent). Hand-rolled to avoid
/// pulling in `rust_decimal` or `bigdecimal`.
fn format_decimal_i128(v: i128, scale: i8) -> String {
    if scale == 0 {
        return v.to_string();
    }
    if scale < 0 {
        // Negative scale = the value is `v * 10^|scale|`; render with trailing zeros.
        let zeros = "0".repeat((-scale) as usize);
        return format!("{v}{zeros}");
    }
    let neg = v < 0;
    let abs = v.unsigned_abs();
    let s = abs.to_string();
    let scale = scale as usize;
    let int_part;
    let frac_part;
    if s.len() > scale {
        let (i, f) = s.split_at(s.len() - scale);
        int_part = i.to_string();
        frac_part = f.to_string();
    } else {
        int_part = "0".to_string();
        frac_part = "0".repeat(scale - s.len()) + &s;
    }
    let mut out = if neg {
        String::from("-")
    } else {
        String::new()
    };
    out.push_str(&int_part);
    out.push('.');
    out.push_str(&frac_part);
    // Trim trailing zeros for the fractional part, but keep at least one digit.
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{BooleanArray, Float64Array, Int64Array, StringArray};
    use arrow_schema::{Field, Schema};
    use parquet::arrow::ArrowWriter;

    use super::*;

    fn build_basic_parquet() -> Vec<u8> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("age", DataType::Int64, false),
            Field::new("score", DataType::Float64, true),
            Field::new("active", DataType::Boolean, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["Ada", "Grace", "Linus"])),
                Arc::new(Int64Array::from(vec![36, 85, 54])),
                Arc::new(Float64Array::from(vec![Some(0.95), None, Some(0.5)])),
                Arc::new(BooleanArray::from(vec![true, false, true])),
            ],
        )
        .unwrap();
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = ArrowWriter::try_new(&mut buf, schema, None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }
        buf
    }

    /// Build a tiny parquet file in memory and verify it round-trips through
    /// `extract_parquet` into a sane `WorkbookLayout`.
    #[test]
    fn round_trips_basic_parquet() {
        let buf = build_basic_parquet();
        let (layout, report) =
            extract_parquet(&buf, &ParquetOptions::default()).expect("parquet extract");
        assert!(report.is_clean(), "unexpected report: {report:?}");
        let sheet = &layout.sheets[0];
        // 1 header + 3 data rows.
        assert_eq!(sheet.max_row, 4);
        assert_eq!(sheet.max_col, 4);
        assert_eq!(sheet.cols.len(), 4);
        // Column headers landed in the value pool.
        assert!(sheet.value_pool.iter().any(|v| v == "name"));
        assert!(sheet.value_pool.iter().any(|v| v == "Ada"));
        assert!(sheet.value_pool.iter().any(|v| v == "36"));
        assert!(sheet.value_pool.iter().any(|v| v == "0.95"));
        // The null score cell should be omitted, not stringified.
        // 4 header + (3 names + 3 ages + 2 scores + 3 bools) = 4 + 11 = 15.
        assert_eq!(sheet.cells.count, 15);
    }

    #[test]
    fn max_rows_counts_rendered_rows_including_header() {
        let buf = build_basic_parquet();
        let opts = ParquetOptions {
            max_rows: 2,
            ..ParquetOptions::default()
        };
        let (layout, report) = extract_parquet(&buf, &opts).unwrap();
        assert_eq!(layout.sheets[0].max_row, 2);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("rendered 2 of 4 rows"));
    }

    #[test]
    fn zero_max_rows_still_renders_header() {
        let buf = build_basic_parquet();
        let opts = ParquetOptions {
            max_rows: 0,
            ..ParquetOptions::default()
        };
        let (layout, report) = extract_parquet(&buf, &opts).unwrap();
        assert_eq!(layout.sheets[0].max_row, 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("max_rows=1"));
    }

    #[test]
    fn decimal_formatter_handles_common_cases() {
        assert_eq!(format_decimal_i128(12345, 2), "123.45");
        assert_eq!(format_decimal_i128(-12345, 2), "-123.45");
        assert_eq!(format_decimal_i128(5, 3), "0.005");
        assert_eq!(format_decimal_i128(123, 0), "123");
        assert_eq!(format_decimal_i128(10000, 2), "100");
        assert_eq!(format_decimal_i128(12, -2), "1200");
    }

    #[test]
    fn int64_cell_keeps_precision_for_large_ids() {
        let big = 1_i64 << 60;
        let (kind, val) = int64_cell(big);
        assert_eq!(kind, "str");
        assert_eq!(val, big.to_string());
        let small = 42_i64;
        let (kind, val) = int64_cell(small);
        assert_eq!(kind, "n");
        assert_eq!(val, "42");
    }
}
