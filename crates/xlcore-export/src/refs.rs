use crate::*;

/// excluded so they don't silently coerce to 0/1.
fn chart_cell_to_number(target: &Cell) -> Option<f64> {
    match target.kind.as_str() {
        "n" | "f" => target.value.as_ref().and_then(|v| v.parse::<f64>().ok()),
        _ => None,
    }
}

/// Trim trailing `None`s off a value vector. Common with Google
/// Sheets-style array formulas (`IF(B28:B2218="","",...)`) that pad an
/// unbounded chart reference with empty strings; without this the
/// renderer sees `[v1, ..., vN, 0, 0, ..., 0]` and flatlines.
fn trim_trailing_empties<T>(mut values: Vec<Option<T>>) -> Vec<Option<T>> {
    if let Some(last) = values.iter().rposition(Option::is_some) {
        values.truncate(last + 1);
        values
    } else {
        Vec::new()
    }
}

/// Resolve the effective number-format code for a cell, walking
/// `cell.style_index -> cell_xfs[i].num_fmt_id -> num_fmts[].format_code`
/// and falling back to the built-in OOXML format table (ids 0..49).
fn cell_format_code(cell: &Cell, styles: &Styles) -> Option<String> {
    let style_idx = cell.style_index? as usize;
    let xf = styles.cell_xfs.get(style_idx)?;
    let fmt_id = xf.num_fmt_id?;
    if let Some(nf) = styles.num_fmts.iter().find(|f| f.id == fmt_id) {
        return Some(nf.format_code.clone());
    }
    builtin_num_fmt(fmt_id).map(str::to_string)
}

/// ECMA-376 Part 1 §18.8.30 — built-in number-format ids. Only the
/// subset that's actually useful for chart axis labels; unknown ids
/// fall back to `None` and the renderer renders the raw value.
fn builtin_num_fmt(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        14 => "m/d/yyyy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yyyy h:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

/// After all sheets are extracted, resolve any `Sheet!$A$1:$B$2`-style
/// references in chart series/categories that didn't come with cached
/// numbers. Office writes the cache most of the time, but not always --
/// without this, fresh chartsheets render empty.
pub(crate) fn resolve_chart_refs(layout: &mut WorkbookLayout) {
    // Snapshot sheet name -> index so we can lookup cells without aliasing.
    let name_to_idx: std::collections::HashMap<String, usize> = layout
        .sheets
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.clone(), i))
        .collect();

    let read_string = |sheets: &[Sheet], target: &Cell, sst: &[String]| -> Option<String> {
        match target.kind.as_str() {
            "s" => target
                .value
                .as_ref()
                .and_then(|v| v.parse::<usize>().ok())
                .and_then(|idx| sst.get(idx).cloned()),
            "inline" | "str" => target.value.clone(),
            "n" | "f" => target.value.clone(),
            _ => target.value.clone(),
        }
        .or_else(|| {
            // suppress unused warning on `sheets`
            let _ = sheets;
            None
        })
    };

    let read_number = chart_cell_to_number;

    // Helpers: resolve range -> Vec of cells in row-major order.
    let collect_cells = |sheets: &[Sheet],
                         sheet_name: &str,
                         r1: u32,
                         c1: u32,
                         r2: u32,
                         c2: u32|
     -> Vec<Option<Cell>> {
        let Some(&idx) = name_to_idx.get(sheet_name) else {
            return Vec::new();
        };
        let sheet = &sheets[idx];
        let mut out = Vec::with_capacity(((r2 - r1 + 1) * (c2 - c1 + 1)) as usize);
        for r in r1..=r2 {
            for c in c1..=c2 {
                let cell = sheet
                    .rows
                    .iter()
                    .find(|row| row.index == r)
                    .and_then(|row| row.cells.iter().find(|cc| cc.r == r && cc.c == c))
                    .cloned();
                out.push(cell);
            }
        }
        out
    };

    let snapshot_sheets = layout.sheets.clone();
    let sst = layout.shared_strings.clone();
    let styles_snapshot = layout.styles.clone();

    for sheet in layout.sheets.iter_mut() {
        for drawing in sheet.drawings.iter_mut() {
            let Some(chart) = drawing.chart.as_mut() else {
                continue;
            };

            // categories
            if let Some(formula) = &chart.categories_ref {
                if let Some((sheet_name, r1, c1, r2, c2)) = parse_chart_ref(formula) {
                    let cells = collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r2, c2);
                    // Pick up the format string from the first populated
                    // referenced cell even when chart XML already supplied a
                    // value cache. Producers often cache numeric category
                    // values but omit <c:formatCode>, and date serial labels
                    // need the source-cell style to render correctly.
                    if chart.categories_format.is_none() {
                        if let Some(Some(cell)) = cells.iter().find(|c| c.is_some()) {
                            chart.categories_format = cell_format_code(cell, &styles_snapshot);
                        }
                    }
                    if chart.categories.is_empty() {
                        chart.categories = cells
                            .into_iter()
                            .map(|cell| {
                                cell.as_ref()
                                    .and_then(|cc| read_string(&snapshot_sheets, cc, &sst))
                                    .unwrap_or_default()
                            })
                            .collect();
                    }
                }
            }

            // series name + values
            for ser in chart.series.iter_mut() {
                if ser.name.is_empty() {
                    if let Some(formula) = &ser.name_ref {
                        if let Some((sheet_name, r1, c1, _, _)) = parse_chart_ref(formula) {
                            // Series name is a single-cell ref; just read (r1,c1).
                            let cells =
                                collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r1, c1);
                            if let Some(Some(cell)) = cells.first() {
                                if let Some(s) = read_string(&snapshot_sheets, cell, &sst) {
                                    ser.name = s;
                                }
                            }
                        }
                    }
                }
                if ser.values.is_empty() {
                    if let Some(formula) = &ser.values_ref {
                        if let Some((sheet_name, r1, c1, r2, c2)) = parse_chart_ref(formula) {
                            let cells =
                                collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r2, c2);
                            let opt: Vec<Option<f64>> = cells
                                .into_iter()
                                .map(|cell| cell.as_ref().and_then(read_number))
                                .collect();
                            let trimmed = trim_trailing_empties(opt);
                            ser.values = trimmed.into_iter().map(|v| v.unwrap_or(0.0)).collect();
                        }
                    }
                }
                if ser.x_values.is_empty() {
                    if let Some(formula) = &ser.x_values_ref {
                        if let Some((sheet_name, r1, c1, r2, c2)) = parse_chart_ref(formula) {
                            let cells =
                                collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r2, c2);
                            let opt: Vec<Option<f64>> = cells
                                .into_iter()
                                .map(|cell| cell.as_ref().and_then(read_number))
                                .collect();
                            let trimmed = trim_trailing_empties(opt);
                            ser.x_values = trimmed.into_iter().map(|v| v.unwrap_or(0.0)).collect();
                        }
                    }
                }
                if ser.bubble_sizes.is_empty() {
                    if let Some(formula) = &ser.bubble_sizes_ref {
                        if let Some((sheet_name, r1, c1, r2, c2)) = parse_chart_ref(formula) {
                            let cells =
                                collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r2, c2);
                            let opt: Vec<Option<f64>> = cells
                                .into_iter()
                                .map(|cell| cell.as_ref().and_then(read_number))
                                .collect();
                            let trimmed = trim_trailing_empties(opt);
                            ser.bubble_sizes =
                                trimmed.into_iter().map(|v| v.unwrap_or(0.0)).collect();
                        }
                    }
                }
            }

            // Categories are read 1:1 from their ref but the parallel
            // value series may have been trimmed. Excel pairs them by
            // index, so trim categories down to the longest series so
            // we don't render N extra ghost points along the x-axis.
            let max_series_len = chart
                .series
                .iter()
                .map(|s| s.values.len())
                .max()
                .unwrap_or(0);
            if chart.categories.len() > max_series_len {
                chart.categories.truncate(max_series_len);
            }
        }
    }
}

/// Walk every sparkline group, resolve `formula` -> numeric value
/// vector against the (already-extracted) sheet data, and compute the
/// shared `group_min`/`group_max` when `min/maxAxisType == "group"`.
/// Falls back to using the host sheet's name when the formula has no
/// explicit sheet prefix (Excel's UI defaults to the anchor sheet).
pub(crate) fn resolve_sparkline_refs(layout: &mut WorkbookLayout) {
    let name_to_idx: std::collections::HashMap<String, usize> = layout
        .sheets
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.clone(), i))
        .collect();
    let snapshot = layout.sheets.clone();

    let read_number = |sheets: &[Sheet], sheet_name: &str, r: u32, c: u32| -> Option<f64> {
        let &idx = name_to_idx.get(sheet_name)?;
        let sheet = sheets.get(idx)?;
        let row = sheet.rows.iter().find(|row| row.index == r)?;
        let cell = row.cells.iter().find(|cc| cc.r == r && cc.c == c)?;
        // Only numeric kinds count for sparkline plotting; text / errors
        // / booleans become None ("empty") so the renderer can honor
        // displayEmptyCellsAs.
        match cell.kind.as_str() {
            "n" | "f" => cell.value.as_ref().and_then(|v| v.parse::<f64>().ok()),
            _ => None,
        }
    };

    for sheet_idx in 0..layout.sheets.len() {
        let host_name = layout.sheets[sheet_idx].name.clone();
        let groups_len = layout.sheets[sheet_idx].sparkline_groups.len();
        for gi in 0..groups_len {
            let mut all_values: Vec<f64> = Vec::new();
            let spark_count = layout.sheets[sheet_idx].sparkline_groups[gi]
                .sparklines
                .len();
            for si in 0..spark_count {
                let formula = layout.sheets[sheet_idx].sparkline_groups[gi].sparklines[si]
                    .formula
                    .clone();
                let Some(formula) = formula else { continue };
                let Some((sheet_name, r1, c1, r2, c2)) = parse_sparkline_ref(&formula, &host_name)
                else {
                    continue;
                };
                let mut vals: Vec<Option<f64>> = Vec::new();
                // Walk row-major (rows then cols). For typical 1xN or Nx1
                // ranges this is just the source order.
                for r in r1..=r2 {
                    for c in c1..=c2 {
                        let v = read_number(&snapshot, &sheet_name, r, c);
                        if let Some(v) = v {
                            all_values.push(v);
                        }
                        vals.push(v);
                    }
                }
                layout.sheets[sheet_idx].sparkline_groups[gi].sparklines[si].values = vals;
            }
            // Group-axis resolution.
            let g = &mut layout.sheets[sheet_idx].sparkline_groups[gi];
            if !all_values.is_empty() {
                if g.min_axis_type == "group" {
                    g.group_min = Some(all_values.iter().cloned().fold(f64::INFINITY, f64::min));
                }
                if g.max_axis_type == "group" {
                    g.group_max =
                        Some(all_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
                }
            }
        }
    }
}

/// Parse a sparkline data ref. Accepts:
///   - `Sheet1!B2:G2`
///   - `'My Sheet'!$A$1:$F$1`
///   - `B2:G2` (no sheet prefix ⇒ use the anchor sheet)
///   - `B2` (single cell)
fn parse_sparkline_ref(formula: &str, host_sheet: &str) -> Option<(String, u32, u32, u32, u32)> {
    let (sheet, range_part) = if let Some((s, r)) = formula.split_once('!') {
        (s.trim_matches('\'').to_string(), r)
    } else {
        (host_sheet.to_string(), formula)
    };
    let cleaned: String = range_part.chars().filter(|c| *c != '$').collect();
    let (a, b) = cleaned
        .split_once(':')
        .unwrap_or((cleaned.as_str(), cleaned.as_str()));
    let (r1, c1) = xlcore_io::parse_a1(a)?;
    let (r2, c2) = xlcore_io::parse_a1(b)?;
    Some((sheet, r1.min(r2), c1.min(c2), r1.max(r2), c1.max(c2)))
}

/// Parse a chart-style reference like `Sheet1!$B$2:$E$2` or
/// `'My Sheet'!$A$1` into (sheet, r1, c1, r2, c2).
fn parse_chart_ref(formula: &str) -> Option<(String, u32, u32, u32, u32)> {
    let (sheet_part, range_part) = formula.split_once('!')?;
    // Strip surrounding quotes if present.
    let sheet = sheet_part.trim_matches('\'').to_string();
    let cleaned: String = range_part.chars().filter(|c| *c != '$').collect();
    let (a, b) = cleaned
        .split_once(':')
        .unwrap_or((cleaned.as_str(), cleaned.as_str()));
    let (r1, c1) = xlcore_io::parse_a1(a)?;
    let (r2, c2) = xlcore_io::parse_a1(b)?;
    let (r1, r2) = (r1.min(r2), r1.max(r2));
    let (c1, c2) = (c1.min(c2), c1.max(c2));
    Some((sheet, r1, c1, r2, c2))
}

/// Returns `(plain_text_per_si, runs_per_si)`, the two parallel arrays
/// indexed by SharedStringItem position. `runs_per_si[i]` is empty when
/// the SST entry is plain text (single `<t>`); otherwise it carries the

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `Cell` containing the fields used by the chart helpers.
    fn cell(kind: &str, value: Option<&str>, style_index: Option<u32>) -> Cell {
        Cell {
            r: 1,
            c: 1,
            kind: kind.to_string(),
            value: value.map(str::to_string),
            formula: None,
            style_index,
            runs: Vec::new(),
        }
    }

    #[test]
    fn chart_cell_to_number_accepts_only_numeric_cells() {
        assert_eq!(
            chart_cell_to_number(&cell("n", Some("42.5"), None)),
            Some(42.5)
        );
        assert_eq!(
            chart_cell_to_number(&cell("f", Some("-3"), None)),
            Some(-3.0)
        );

        // Regression: `t="s"` cells store the SST index in <v>; treating
        // that as a number turns header strings into bogus data points
        // (notably the doughnut header that came back as a 50% slice).
        assert_eq!(chart_cell_to_number(&cell("s", Some("23"), None)), None);
        assert_eq!(
            chart_cell_to_number(&cell("inline", Some("12"), None)),
            None
        );
        assert_eq!(chart_cell_to_number(&cell("str", Some("7"), None)), None);

        assert_eq!(chart_cell_to_number(&cell("b", Some("1"), None)), None);
        assert_eq!(
            chart_cell_to_number(&cell("e", Some("#DIV/0!"), None)),
            None
        );
        assert_eq!(
            chart_cell_to_number(&cell("n", Some("not numeric"), None)),
            None
        );
    }

    #[test]
    fn trim_trailing_empties_preserves_interior_gaps() {
        assert_eq!(
            trim_trailing_empties(vec![Some(1.0), None, Some(3.0), None, None]),
            vec![Some(1.0), None, Some(3.0)]
        );
        assert_eq!(
            trim_trailing_empties(vec![Some(1.0), Some(2.0), None, None]),
            vec![Some(1.0), Some(2.0)]
        );
        assert_eq!(
            trim_trailing_empties(vec![None, None]),
            Vec::<Option<f64>>::new()
        );
    }

    fn styles_with(num_fmts: Vec<(u32, &str)>, cell_xfs: Vec<Option<u32>>) -> Styles {
        Styles {
            num_fmts: num_fmts
                .into_iter()
                .map(|(id, code)| NumberFormat {
                    id,
                    format_code: code.to_string(),
                })
                .collect(),
            cell_xfs: cell_xfs
                .into_iter()
                .map(|nf| CellFormat {
                    num_fmt_id: nf,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn cell_format_code_resolves_custom_and_builtin_formats() {
        let styles = styles_with(vec![(164, "mmm d, yyyy")], vec![Some(164)]);
        let c = cell("n", Some("45974.66"), Some(0));
        assert_eq!(
            cell_format_code(&c, &styles).as_deref(),
            Some("mmm d, yyyy")
        );

        let styles = styles_with(vec![], vec![Some(14)]);
        let c = cell("n", Some("45974"), Some(0));
        assert_eq!(cell_format_code(&c, &styles).as_deref(), Some("m/d/yyyy"));
    }

    #[test]
    fn cell_format_code_returns_none_without_a_supported_format() {
        let styles = styles_with(vec![], vec![Some(14)]);
        assert_eq!(cell_format_code(&cell("n", Some("1"), None), &styles), None);

        let styles = styles_with(vec![], vec![Some(999)]);
        assert_eq!(
            cell_format_code(&cell("n", Some("1"), Some(0)), &styles),
            None
        );
    }
}
