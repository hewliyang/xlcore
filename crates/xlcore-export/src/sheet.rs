//! Per-worksheet extraction.

use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::Cell as XCell;
use xlcore_io::parse_a1;

// Unit conversions. Excel's exact column-width formula is
// `truncate((char_width * w + 5) / char_width * 256) / 256 * char_width`,
// where `char_width` is the maximum digit width (MDW) of the workbook's
// default font in pixels. We approximate MDW from the default font size:
// Calibri/Aptos at 11pt give MDW ≈ 7px, and digit width scales roughly
// linearly with font size for sans-serif fonts of that family.
//
// Without this scaling, an Aptos-20pt workbook (the default in Office
// 2024+) renders every column at ~half its true pixel width, which makes
// the whole sheet look squashed compared to SpreadJS / real Excel.
const PT_PER_PX: f64 = 72.0 / 96.0;
const DEFAULT_COL_WIDTH_CHARS: f64 = 8.43;
const COL_PADDING_PX: f64 = 5.0;
const DEFAULT_ROW_HEIGHT_PT: f64 = 15.0;

fn px_per_char(default_font_size_pt: f32) -> f64 {
    // Calibri 11pt baseline: MDW = 7px. Scale linearly with size.
    // Clamp to sane bounds so a corrupt font entry can't blow up layout.
    let scaled = (default_font_size_pt as f64) * 7.0 / 11.0;
    scaled.clamp(5.0, 24.0)
}

pub fn extract(
    ws: &x::Worksheet,
    index: usize,
    name: String,
    _shared_strings: &[String],
    styles: &Styles,
) -> Sheet {
    let px_per_char = px_per_char(styles.default_font_size);
    let default_col_width_px_const: f32 = (DEFAULT_COL_WIDTH_CHARS * px_per_char + COL_PADDING_PX) as f32;
    // Compute used range
    let mut max_row = 0u32;
    let mut max_col = 0u32;
    for row in &ws.x_sheet_data.x_row {
        for cell in &row.x_c {
            if let Some(r) = cell.cell_reference.as_ref() {
                if let Some((rr, cc)) = parse_a1(r.as_str()) {
                    max_row = max_row.max(rr);
                    max_col = max_col.max(cc);
                }
            }
        }
    }
    if let Some(mc) = &ws.x_merge_cells {
        for m in &mc.x_merge_cell {
            if let Some(((r1, c1), (r2, c2))) = xlcore_io::parse_range(m.reference.as_str()) {
                max_row = max_row.max(r2.max(r1));
                max_col = max_col.max(c2.max(c1));
            }
        }
    }

    // Sheet format defaults
    let mut default_col_width_px = default_col_width_px_const;
    let mut default_row_height_pt = DEFAULT_ROW_HEIGHT_PT;
    if let Some(fmt) = &ws.sheet_format_properties {
        if let Some(w) = fmt.default_column_width {
            default_col_width_px = (w * px_per_char + COL_PADDING_PX) as f32;
        }
        // default_row_height is non-optional in the schema (DoubleValue).
        if fmt.default_row_height > 0.0 {
            default_row_height_pt = fmt.default_row_height;
        }
    }
    let default_row_height_px = (default_row_height_pt / PT_PER_PX) as f32;

    // Columns
    let mut cols: Vec<Col> = Vec::new();
    if !ws.x_cols.is_empty() {
        for c in &ws.x_cols[0].x_col {
            let width_px = c
                .width
                .map(|w| (w * px_per_char + COL_PADDING_PX) as f32)
                .unwrap_or(default_col_width_px);
            cols.push(Col {
                min: c.min,
                max: c.max,
                width_px,
                style_index: c.style,
                hidden: c.hidden.unwrap_or(false),
            });
        }
    }

    // Rows
    let mut rows: Vec<Row> = Vec::with_capacity(ws.x_sheet_data.x_row.len());
    for r in &ws.x_sheet_data.x_row {
        let row_index = r.row_index.unwrap_or(0);
        if row_index == 0 { continue; }
        let mut cells = Vec::with_capacity(r.x_c.len());
        for cell in &r.x_c {
            if let Some(c) = extract_cell(cell) {
                cells.push(c);
            }
        }
        rows.push(Row {
            index: row_index,
            height_px: r.height.map(|h| (h / PT_PER_PX) as f32),
            cells,
            style_index: r.style_index,
            hidden: r.hidden.unwrap_or(false),
        });
    }

    // Merges
    let merges: Vec<Merge> = ws
        .x_merge_cells
        .as_ref()
        .map(|mc| {
            mc.x_merge_cell
                .iter()
                .filter_map(|m| {
                    let ((r1, c1), (r2, c2)) = xlcore_io::parse_range(m.reference.as_str())?;
                    Some(Merge { r1, c1, r2, c2 })
                })
                .collect()
        })
        .unwrap_or_default();

    // Freeze
    let mut freeze: Option<Freeze> = None;
    let mut show_grid_lines = true;
    if let Some(sv) = &ws.sheet_views {
        for view in &sv.x_sheet_view {
            if let Some(g) = view.show_grid_lines { show_grid_lines = g; }
            if let Some(p) = &view.pane {
                let split_col = p.horizontal_split.unwrap_or(0.0) as u32;
                let split_row = p.vertical_split.unwrap_or(0.0) as u32;
                if split_col > 0 || split_row > 0 {
                    freeze = Some(Freeze {
                        top_row: split_row + 1,
                        left_col: split_col + 1,
                    });
                }
            }
        }
    }

    let conditional_formats = extract_conditional_formats(ws);

    Sheet {
        index: index as u32,
        name,
        max_row,
        max_col,
        default_col_width_px,
        default_row_height_px,
        cols,
        rows,
        merges,
        freeze,
        show_grid_lines,
        conditional_formats,
        drawings: Vec::new(),    // populated by lib.rs after sheet extract
        tables: Vec::new(),      // populated by lib.rs after sheet extract
        pivots: Vec::new(),      // populated by lib.rs after sheet extract
        hyperlinks: Vec::new(),  // populated by lib.rs after sheet extract
        comments: Vec::new(),    // populated by lib.rs after sheet extract
    }
}

fn extract_conditional_formats(ws: &x::Worksheet) -> Vec<ConditionalFormat> {
    let mut out = Vec::new();
    for cf in &ws.x_conditional_formatting {
        // sqref is a space-separated list of A1 ranges.
        let mut ranges: Vec<Merge> = Vec::new();
        if let Some(sqref) = &cf.sequence_of_references {
            // ListValue<StringValue>: stringify and split. Just take the
            // textual representation; ooxmlsdk gives us the inner Vec via
            // .items in some versions, but we go through Debug/Display safely.
            let s = format!("{}", sqref);
            for part in s.split_whitespace() {
                if let Some(((r1, c1), (r2, c2))) = xlcore_io::parse_range(part) {
                    ranges.push(Merge { r1, c1, r2, c2 });
                } else if let Some((r, c)) = xlcore_io::parse_a1(part) {
                    ranges.push(Merge { r1: r, c1: c, r2: r, c2: c });
                }
            }
        }
        if ranges.is_empty() { continue; }

        let mut rules = Vec::new();
        for rule in &cf.x_cf_rule {
            let kind = format!("{:?}", rule.r#type);
            let kind_norm = normalize_cf_kind(&kind);
            let color_scale = rule.x_color_scale.as_ref().and_then(extract_color_scale);
            let data_bar = rule.x_data_bar.as_ref().and_then(|db| extract_data_bar(db));
            let icon_set = rule.x_icon_set.as_ref().and_then(extract_icon_set);
            let operator = rule.operator.as_ref()
                .map(|o| normalize_cf_operator(&format!("{o:?}")));
            let operands: Vec<String> = rule.x_formula.iter()
                .filter_map(|f| f.xml_content.as_deref().map(str::to_string))
                .collect();
            let time_period = rule.time_period.as_ref()
                .map(|tp| normalize_time_period(&format!("{tp:?}")));
            rules.push(CfRule {
                priority: rule.priority,
                kind: kind_norm,
                color_scale,
                data_bar,
                icon_set,
                operator,
                operands,
                dxf_id: rule.format_id,
                stop_if_true: rule.stop_if_true.unwrap_or(false),
                rank: rule.rank.map(|v| v as u32),
                bottom: rule.bottom.unwrap_or(false),
                percent: rule.percent.unwrap_or(false),
                above_average: rule.above_average,
                equal_average: rule.equal_average.unwrap_or(false),
                std_dev: rule.std_dev,
                text: rule.text.clone(),
                time_period,
            });
        }
        out.push(ConditionalFormat { ranges, rules });
    }
    out
}

fn normalize_cf_operator(dbg: &str) -> String {
    // ooxmlsdk's enum Debug repr is e.g. `GreaterThan` / `LessThanOrEqual`.
    // Lowercase first letter to match the ECMA-376 attribute spelling.
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("greaterthanorequal") { "greaterThanOrEqual" }
    else if lower.contains("lessthanorequal") { "lessThanOrEqual" }
    else if lower.contains("greaterthan") { "greaterThan" }
    else if lower.contains("lessthan") { "lessThan" }
    else if lower.contains("notbetween") { "notBetween" }
    else if lower.contains("between") { "between" }
    else if lower.contains("notequal") { "notEqual" }
    else if lower.contains("equal") { "equal" }
    else if lower.contains("beginswith") { "beginsWith" }
    else if lower.contains("endswith") { "endsWith" }
    else if lower.contains("containing") || lower.contains("containstext") { "containsText" }
    else if lower.contains("notcontains") { "notContains" }
    else { "unknown" }.to_string()
}

fn normalize_cf_kind(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("colorscale") { "colorScale" }
    else if lower.contains("databar") { "dataBar" }
    else if lower.contains("iconset") { "iconSet" }
    else if lower.contains("cellis") { "cellIs" }
    else if lower.contains("expression") { "expression" }
    else if lower.contains("top10") { "top10" }
    else if lower.contains("aboveaverage") { "aboveAverage" }
    // Order matters: `notContainsText` must precede `containsText`.
    else if lower.contains("notcontainstext") { "notContainsText" }
    else if lower.contains("containstext") { "containsText" }
    else if lower.contains("beginswith") { "beginsWith" }
    else if lower.contains("endswith") { "endsWith" }
    else if lower.contains("duplicatevalues") { "duplicateValues" }
    else if lower.contains("uniquevalues") { "uniqueValues" }
    else if lower.contains("timeperiod") { "timePeriod" }
    else { "unknown" }.to_string()
}

fn normalize_time_period(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("yesterday") { "yesterday" }
    else if lower.contains("tomorrow") { "tomorrow" }
    else if lower.contains("last7days") { "last7Days" }
    else if lower.contains("thisweek") { "thisWeek" }
    else if lower.contains("lastweek") { "lastWeek" }
    else if lower.contains("nextweek") { "nextWeek" }
    else if lower.contains("thismonth") { "thisMonth" }
    else if lower.contains("lastmonth") { "lastMonth" }
    else if lower.contains("nextmonth") { "nextMonth" }
    else { "today" }.to_string()
}

fn cfvo_type_norm(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("automin") { "automin" }
    else if lower.contains("automax") { "automax" }
    else if lower.contains("min") { "min" }
    else if lower.contains("max") { "max" }
    else if lower.contains("percentile") { "percentile" }
    else if lower.contains("percent") { "percent" }
    else if lower.contains("formula") { "formula" }
    else { "num" }.to_string()
}

fn extract_data_bar(db: &x::DataBar) -> Option<CfDataBar> {
    if db.x_cfvo.len() < 2 { return None; }
    let mk_stop = |cfvo: &x::ConditionalFormatValueObject| CfvoStop {
        cfvo_type: cfvo_type_norm(&format!("{:?}", cfvo.r#type)),
        val: cfvo.val.as_ref().map(|s| s.as_str().to_string()),
    };
    let min = mk_stop(&db.x_cfvo[0]);
    let max = mk_stop(&db.x_cfvo[1]);
    // Legacy `<dataBar>` always has the bar fill color. Some writers
    // (notably SpreadJS / hsx) round-trip the CF rule without the
    // `<color>` child — the canonical color lives in the x14 extension
    // we don't parse yet. Fall back to Excel's default blue so the
    // renderer can paint something coherent regardless.
    let color = {
        let c = &db.x_color;
        if c.rgb.is_none() && c.theme.is_none() && c.indexed.is_none() {
            Color { rgb: Some("FF638EC6".to_string()), theme: None, indexed: None, tint: None }
        } else {
            Color {
                rgb: c.rgb.as_ref().map(|s| s.as_str().to_string()),
                theme: c.theme,
                indexed: c.indexed,
                tint: c.tint,
            }
        }
    };
    Some(CfDataBar {
        min, max, color,
        negative_color: None,
        // The OOXML legacy schema defaults are `minLength=10` / `maxLength=90`,
        // but every real-world writer (Excel, SpreadJS, LibreOffice) emits an
        // x14 extension with `0`/`100` and ignores the legacy attrs. Until we
        // parse the x14 ext, default to `0`/`100` so renders match what users
        // see in Excel. The bug-for-bug spec defaults are tracked in TRIAGE.
        min_length_pct: db.min_length.unwrap_or(0),
        max_length_pct: db.max_length.unwrap_or(100),
        show_value: db.show_value.unwrap_or(true),
    })
}

fn extract_icon_set(is: &x::IconSet) -> Option<CfIconSet> {
    // Default ooxml `iconSet` enum value is `3Arrows`. Some writers
    // omit the attribute and rely on the default.
    let icon_set_name = match &is.icon_set_value {
        Some(v) => normalize_icon_set_name(&format!("{v:?}")),
        None => "3Arrows".to_string(),
    };
    let cfvos: Vec<CfvoStop> = is.x_cfvo.iter().map(|cfvo| CfvoStop {
        cfvo_type: cfvo_type_norm(&format!("{:?}", cfvo.r#type)),
        val: cfvo.val.as_ref().map(|s| s.as_str().to_string()),
    }).collect();
    if cfvos.len() < 3 { return None; }
    Some(CfIconSet {
        icon_set: icon_set_name,
        cfvos,
        show_value: is.show_value.unwrap_or(true),
        reverse: is.reverse.unwrap_or(false),
    })
}

/// `ooxmlsdk` Debug for `IconSetValues` looks like `ThreeTrafficLights1`,
/// `FourArrowsGray`, etc. Translate back to the spec spellings
/// (`3TrafficLights1`, `4ArrowsGray`).
fn normalize_icon_set_name(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    let prefix = if lower.starts_with("three") { "3" }
        else if lower.starts_with("four") { "4" }
        else if lower.starts_with("five") { "5" }
        else { return dbg.to_string(); };
    let rest = &dbg[match prefix { "3" => 5, "4" => 4, "5" => 4, _ => 0 }..];
    format!("{prefix}{rest}")
}

fn extract_color_scale(cs: &x::ColorScale) -> Option<CfColorScale> {
    if cs.x_cfvo.len() != cs.x_color.len() || cs.x_cfvo.is_empty() {
        return None;
    }
    let mut stops = Vec::with_capacity(cs.x_cfvo.len());
    for (cfvo, color) in cs.x_cfvo.iter().zip(cs.x_color.iter()) {
        let cfvo_type = format!("{:?}", cfvo.r#type).to_ascii_lowercase();
        let cfvo_type = if cfvo_type.contains("min") { "min" }
            else if cfvo_type.contains("max") { "max" }
            else if cfvo_type.contains("percentile") { "percentile" }
            else if cfvo_type.contains("percent") { "percent" }
            else if cfvo_type.contains("formula") { "formula" }
            else { "num" }.to_string();
        let val = cfvo.val.as_ref().map(|s| s.as_str().to_string());
        let col = Color {
            rgb: color.rgb.as_ref().map(|s| s.as_str().to_string()),
            theme: color.theme,
            indexed: color.indexed,
            tint: color.tint,
        };
        // Skip stops without any color info.
        if col.rgb.is_none() && col.theme.is_none() && col.indexed.is_none() {
            continue;
        }
        stops.push(CfColorScaleStop { cfvo_type, val, color: col });
    }
    if stops.is_empty() { None } else { Some(CfColorScale { stops }) }
}

fn extract_cell(cell: &XCell) -> Option<Cell> {
    let r_attr = cell.cell_reference.as_ref()?;
    let (r, c) = parse_a1(r_attr.as_str())?;

    let formula = cell
        .cell_formula
        .as_ref()
        .and_then(|f| f.xml_content.as_deref().map(str::to_string));

    let raw_v = cell
        .cell_value
        .as_ref()
        .and_then(|v| v.xml_content.as_deref());

    let dt_dbg = cell
        .data_type
        .as_ref()
        .map(|d| format!("{d:?}").to_ascii_lowercase());

    // Rich-text runs for inline strings (set below if cell is `inline` and
    // carries `<r>` children). Shared-string runs live on the SST table.
    let mut inline_runs: Vec<TextRun> = Vec::new();

    let (kind, value): (&str, Option<String>) = if formula.is_some() {
        ("f", raw_v.map(str::to_string))
    } else if let Some(dt) = &dt_dbg {
        if dt.contains("sharedstring") {
            ("s", raw_v.map(str::to_string))
        } else if dt.contains("inlinestring") {
            // Two encodings: a single `<t>` (plain text) or a sequence of
            // `<r>` runs (rich text). When runs are present we concatenate
            // their `<t>` content for the flat `value`, *and* preserve the
            // per-run styling for the renderer.
            let (s, runs) = if let Some(is) = cell.inline_string.as_ref() {
                if !is.x_r.is_empty() {
                    let mut s = String::new();
                    let mut rs: Vec<TextRun> = Vec::with_capacity(is.x_r.len());
                    for r in &is.x_r {
                        let txt = r.text.xml_content.as_deref().unwrap_or("").to_string();
                        s.push_str(&txt);
                        rs.push(crate::text_run_from(r, txt));
                    }
                    (s, rs)
                } else {
                    let s = is.text.as_ref()
                        .and_then(|t| t.xml_content.as_deref())
                        .unwrap_or("")
                        .to_string();
                    (s, Vec::new())
                }
            } else {
                (String::new(), Vec::new())
            };
            inline_runs = runs;
            ("inline", Some(s))
        } else if dt.contains("boolean") {
            ("b", raw_v.map(str::to_string))
        } else if dt.contains("error") {
            ("e", raw_v.map(str::to_string))
        } else if dt.contains("str") {
            ("str", raw_v.map(str::to_string))
        } else {
            ("n", raw_v.map(str::to_string))
        }
    } else if let Some(v) = raw_v {
        ("n", Some(v.to_string()))
    } else if cell.style_index.is_some() {
        // Empty styled cell — keep so the renderer paints background/borders.
        ("n", None)
    } else {
        return None;
    };

    Some(Cell {
        r,
        c,
        kind: kind.to_string(),
        value,
        formula,
        style_index: cell.style_index,
        runs: inline_runs,
    })
}
