use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::Cell as XCell;
use xlcore_io::parse_a1;

const PT_PER_PX: f64 = 72.0 / 96.0;
const DEFAULT_COL_WIDTH_CHARS: f64 = 8.43;
const COL_PADDING_PX: f64 = 5.0;
const DEFAULT_ROW_HEIGHT_PT: f64 = 15.0;

fn px_per_char(default_font_size_pt: f32) -> f64 {
    let scaled = (default_font_size_pt as f64) * 7.0 / 11.0;
    scaled.clamp(5.0, 24.0)
}

fn explicit_width_attr_to_px(width: f64, default_font_size_pt: f32) -> f32 {
    let mdw = px_per_char(default_font_size_pt);
    (((256.0 * width + (128.0 / mdw).trunc()) / 256.0) * mdw).trunc() as f32
}

pub fn extract(
    ws: &x::Worksheet,
    index: usize,
    name: String,
    _shared_strings: &[String],
    styles: &Styles,
) -> Sheet {
    let px_per_char = px_per_char(styles.default_font_size);
    let width_attr_to_px =
        |w: f64| -> f32 { explicit_width_attr_to_px(w, styles.default_font_size) };

    let default_col_width_px_const: f32 =
        (DEFAULT_COL_WIDTH_CHARS * px_per_char + COL_PADDING_PX) as f32;

    let mut max_row = 0u32;
    let mut max_col = 0u32;
    for row in &ws.sheet_data.row {
        for cell in &row.cell {
            if let Some(r) = cell.cell_reference.as_ref() {
                if let Some((rr, cc)) = parse_a1(r.as_str()) {
                    max_row = max_row.max(rr);
                    max_col = max_col.max(cc);
                }
            }
        }
    }
    if let Some(mc) = &ws.merge_cells {
        for m in &mc.merge_cell {
            if let Some(((r1, c1), (r2, c2))) = xlcore_io::parse_range(m.reference.as_str()) {
                max_row = max_row.max(r2.max(r1));
                max_col = max_col.max(c2.max(c1));
            }
        }
    }

    let mut default_col_width_px = default_col_width_px_const;
    let mut default_row_height_pt = DEFAULT_ROW_HEIGHT_PT;
    if let Some(fmt) = &ws.sheet_format_properties {
        if let Some(w) = fmt.default_column_width {
            default_col_width_px = width_attr_to_px(w);
        }

        if fmt.default_row_height > 0.0 {
            default_row_height_pt = fmt.default_row_height;
        }
    }
    let default_row_height_px = (default_row_height_pt / PT_PER_PX) as f32;

    let mut cols: Vec<Col> = Vec::new();
    if !ws.columns.is_empty() {
        for c in &ws.columns[0].column {
            let width_px = c
                .width
                .map(width_attr_to_px)
                .unwrap_or(default_col_width_px);
            cols.push(Col {
                min: c.min,
                max: c.max,
                width_px,
                style_index: c.style,
                hidden: c.hidden.map(bool::from).unwrap_or(false),
                outline_level: c.outline_level.unwrap_or(0).min(7),
            });
        }
    }

    let mut rows: Vec<Row> = Vec::with_capacity(ws.sheet_data.row.len());
    for r in &ws.sheet_data.row {
        let row_index = r.row_index.unwrap_or(0);
        if row_index == 0 {
            continue;
        }
        let mut cells = Vec::with_capacity(r.cell.len());
        for cell in &r.cell {
            if let Some(c) = extract_cell(cell) {
                cells.push(c);
            }
        }
        rows.push(Row {
            index: row_index,
            height_px: r.height.map(|h| (h / PT_PER_PX) as f32),
            cells,
            style_index: r.style_index,
            hidden: r.hidden.map(bool::from).unwrap_or(false),
            outline_level: r.outline_level.unwrap_or(0).min(7),
        });
    }

    let merges: Vec<Merge> = ws
        .merge_cells
        .as_ref()
        .map(|mc| {
            mc.merge_cell
                .iter()
                .filter_map(|m| {
                    let ((r1, c1), (r2, c2)) = xlcore_io::parse_range(m.reference.as_str())?;
                    Some(Merge { r1, c1, r2, c2 })
                })
                .collect()
        })
        .unwrap_or_default();

    let auto_filter_range = ws
        .auto_filter
        .as_ref()
        .and_then(|af| af.reference.as_ref())
        .and_then(|r| {
            if let Some(((r1, c1), (r2, c2))) = xlcore_io::parse_range(r.as_str()) {
                Some(Merge { r1, c1, r2, c2 })
            } else {
                xlcore_io::parse_a1(r.as_str()).map(|(r, c)| Merge {
                    r1: r,
                    c1: c,
                    r2: r,
                    c2: c,
                })
            }
        });

    let mut freeze: Option<Freeze> = None;
    let mut show_grid_lines = true;
    if let Some(sv) = &ws.sheet_views {
        for view in &sv.sheet_view {
            if let Some(g) = view.show_grid_lines {
                show_grid_lines = g.into();
            }
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

    let tab_color = ws
        .sheet_properties
        .as_ref()
        .and_then(|sp| sp.tab_color.as_ref())
        .and_then(|tc| {
            let rgb = tc.rgb.as_ref().map(|s| s.as_str().to_string());
            let theme = tc.theme;
            let indexed = tc.indexed;
            if rgb.is_none() && theme.is_none() && indexed.is_none() {
                None
            } else {
                Some(Color {
                    rgb,
                    theme,
                    indexed,
                    tint: tc.tint,
                })
            }
        });

    let outline_pr = ws
        .sheet_properties
        .as_ref()
        .and_then(|sp| sp.outline_properties.as_ref())
        .and_then(|op| {
            let sb = op.summary_below.unwrap_or(true.into());
            let sr = op.summary_right.unwrap_or(true.into());
            if sb.into() && sr.into() {
                None
            } else {
                Some(OutlinePr {
                    summary_below: sb.into(),
                    summary_right: sr.into(),
                })
            }
        });

    Sheet {
        index: index as u32,
        name,
        state: None,
        tab_color,
        max_row,
        max_col,
        default_col_width_px,
        default_row_height_px,
        cols,
        rows,
        merges,
        auto_filter_range,
        freeze,
        show_grid_lines,
        conditional_formats,
        outline_pr,
        drawings: Vec::new(),
        tables: Vec::new(),
        pivots: Vec::new(),
        hyperlinks: Vec::new(),
        comments: Vec::new(),
        sparkline_groups: Vec::new(),

        cells: Default::default(),
        row_meta: Default::default(),
        value_pool: Vec::new(),
        formula_pool: Vec::new(),
        inline_runs: Vec::new(),
    }
}

fn extract_conditional_formats(ws: &x::Worksheet) -> Vec<ConditionalFormat> {
    let mut out = Vec::new();
    for cf in &ws.conditional_formatting {
        let mut ranges: Vec<Merge> = Vec::new();
        if let Some(sqref) = &cf.sequence_of_references {
            let s = sqref.iter().cloned().collect::<Vec<_>>().join(" ");
            for part in s.split_whitespace() {
                if let Some(((r1, c1), (r2, c2))) = xlcore_io::parse_range(part) {
                    ranges.push(Merge { r1, c1, r2, c2 });
                } else if let Some((r, c)) = xlcore_io::parse_a1(part) {
                    ranges.push(Merge {
                        r1: r,
                        c1: c,
                        r2: r,
                        c2: c,
                    });
                }
            }
        }
        if ranges.is_empty() {
            continue;
        }

        let mut rules = Vec::new();
        for rule in &cf.conditional_formatting_rule {
            let kind = format!("{:?}", rule.r#type);
            let kind_norm = normalize_cf_kind(&kind);
            let color_scale = rule.color_scale.as_ref().and_then(extract_color_scale);
            let data_bar = rule.data_bar.as_ref().and_then(|db| extract_data_bar(db));
            let icon_set = rule.icon_set.as_ref().and_then(extract_icon_set);
            let operator = rule
                .operator
                .as_ref()
                .map(|o| normalize_cf_operator(&format!("{o:?}")));
            let operands: Vec<String> = rule
                .formula
                .iter()
                .filter_map(|f| f.xml_content.as_deref().map(str::to_string))
                .collect();
            let time_period = rule
                .time_period
                .as_ref()
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
                stop_if_true: rule.stop_if_true.map(bool::from).unwrap_or(false),
                rank: rule.rank,
                bottom: rule.bottom.map(bool::from).unwrap_or(false),
                percent: rule.percent.map(bool::from).unwrap_or(false),
                above_average: rule.above_average.map(bool::from),
                equal_average: rule.equal_average.map(bool::from).unwrap_or(false),
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
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("greaterthanorequal") {
        "greaterThanOrEqual"
    } else if lower.contains("lessthanorequal") {
        "lessThanOrEqual"
    } else if lower.contains("greaterthan") {
        "greaterThan"
    } else if lower.contains("lessthan") {
        "lessThan"
    } else if lower.contains("notbetween") {
        "notBetween"
    } else if lower.contains("between") {
        "between"
    } else if lower.contains("notequal") {
        "notEqual"
    } else if lower.contains("equal") {
        "equal"
    } else if lower.contains("beginswith") {
        "beginsWith"
    } else if lower.contains("endswith") {
        "endsWith"
    } else if lower.contains("containing") || lower.contains("containstext") {
        "containsText"
    } else if lower.contains("notcontains") {
        "notContains"
    } else {
        "unknown"
    }
    .to_string()
}

fn normalize_cf_kind(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("colorscale") {
        "colorScale"
    } else if lower.contains("databar") {
        "dataBar"
    } else if lower.contains("iconset") {
        "iconSet"
    } else if lower.contains("cellis") {
        "cellIs"
    } else if lower.contains("expression") {
        "expression"
    } else if lower.contains("top10") {
        "top10"
    } else if lower.contains("aboveaverage") {
        "aboveAverage"
    } else if lower.contains("notcontainstext") {
        "notContainsText"
    } else if lower.contains("containstext") {
        "containsText"
    } else if lower.contains("beginswith") {
        "beginsWith"
    } else if lower.contains("endswith") {
        "endsWith"
    } else if lower.contains("duplicatevalues") {
        "duplicateValues"
    } else if lower.contains("uniquevalues") {
        "uniqueValues"
    } else if lower.contains("timeperiod") {
        "timePeriod"
    } else {
        "unknown"
    }
    .to_string()
}

fn normalize_time_period(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("yesterday") {
        "yesterday"
    } else if lower.contains("tomorrow") {
        "tomorrow"
    } else if lower.contains("last7days") {
        "last7Days"
    } else if lower.contains("thisweek") {
        "thisWeek"
    } else if lower.contains("lastweek") {
        "lastWeek"
    } else if lower.contains("nextweek") {
        "nextWeek"
    } else if lower.contains("thismonth") {
        "thisMonth"
    } else if lower.contains("lastmonth") {
        "lastMonth"
    } else if lower.contains("nextmonth") {
        "nextMonth"
    } else {
        "today"
    }
    .to_string()
}

fn cfvo_type_norm(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    if lower.contains("automin") {
        "automin"
    } else if lower.contains("automax") {
        "automax"
    } else if lower.contains("min") {
        "min"
    } else if lower.contains("max") {
        "max"
    } else if lower.contains("percentile") {
        "percentile"
    } else if lower.contains("percent") {
        "percent"
    } else if lower.contains("formula") {
        "formula"
    } else {
        "num"
    }
    .to_string()
}

fn extract_data_bar(db: &x::DataBar) -> Option<CfDataBar> {
    if db.conditional_format_value_object.len() < 2 {
        return None;
    }
    let mk_stop = |cfvo: &x::ConditionalFormatValueObject| CfvoStop {
        cfvo_type: cfvo_type_norm(&format!("{:?}", cfvo.r#type)),
        val: cfvo.val.as_ref().map(|s| s.as_str().to_string()),
    };
    let min = mk_stop(&db.conditional_format_value_object[0]);
    let max = mk_stop(&db.conditional_format_value_object[1]);

    let color = {
        let c = &db.color;
        if c.rgb.is_none() && c.theme.is_none() && c.indexed.is_none() {
            Color {
                rgb: Some("FF638EC6".to_string()),
                theme: None,
                indexed: None,
                tint: None,
            }
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
        min,
        max,
        color,
        negative_color: None,

        min_length_pct: db.min_length.unwrap_or(0),
        max_length_pct: db.max_length.unwrap_or(100),
        show_value: db.show_value.unwrap_or(true.into()).into(),

        gradient: true,
    })
}

fn extract_icon_set(is: &x::IconSet) -> Option<CfIconSet> {
    let icon_set_name = match &is.icon_set_value {
        Some(v) => normalize_icon_set_name(&format!("{v:?}")),
        None => "3Arrows".to_string(),
    };
    let cfvos: Vec<CfvoStop> = is
        .conditional_format_value_object
        .iter()
        .map(|cfvo| CfvoStop {
            cfvo_type: cfvo_type_norm(&format!("{:?}", cfvo.r#type)),
            val: cfvo.val.as_ref().map(|s| s.as_str().to_string()),
        })
        .collect();
    if cfvos.len() < 3 {
        return None;
    }
    Some(CfIconSet {
        icon_set: icon_set_name,
        cfvos,
        show_value: is.show_value.unwrap_or(true.into()).into(),
        reverse: is.reverse.unwrap_or(false.into()).into(),
    })
}

fn normalize_icon_set_name(dbg: &str) -> String {
    let lower = dbg.to_ascii_lowercase();
    let prefix = if lower.starts_with("three") {
        "3"
    } else if lower.starts_with("four") {
        "4"
    } else if lower.starts_with("five") {
        "5"
    } else {
        return dbg.to_string();
    };
    let rest = &dbg[match prefix {
        "3" => 5,
        "4" => 4,
        "5" => 4,
        _ => 0,
    }..];
    format!("{prefix}{rest}")
}

fn extract_color_scale(cs: &x::ColorScale) -> Option<CfColorScale> {
    if cs.conditional_format_value_object.len() != cs.color.len()
        || cs.conditional_format_value_object.is_empty()
    {
        return None;
    }
    let mut stops = Vec::with_capacity(cs.conditional_format_value_object.len());
    for (cfvo, color) in cs
        .conditional_format_value_object
        .iter()
        .zip(cs.color.iter())
    {
        let cfvo_type = format!("{:?}", cfvo.r#type).to_ascii_lowercase();
        let cfvo_type = if cfvo_type.contains("min") {
            "min"
        } else if cfvo_type.contains("max") {
            "max"
        } else if cfvo_type.contains("percentile") {
            "percentile"
        } else if cfvo_type.contains("percent") {
            "percent"
        } else if cfvo_type.contains("formula") {
            "formula"
        } else {
            "num"
        }
        .to_string();
        let val = cfvo.val.as_ref().map(|s| s.as_str().to_string());
        let col = Color {
            rgb: color.rgb.as_ref().map(|s| s.as_str().to_string()),
            theme: color.theme,
            indexed: color.indexed,
            tint: color.tint,
        };

        if col.rgb.is_none() && col.theme.is_none() && col.indexed.is_none() {
            continue;
        }
        stops.push(CfColorScaleStop {
            cfvo_type,
            val,
            color: col,
        });
    }
    if stops.is_empty() {
        None
    } else {
        Some(CfColorScale { stops })
    }
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

    let mut inline_runs: Vec<TextRun> = Vec::new();

    let (kind, value): (&str, Option<String>) = if formula.is_some() {
        ("f", raw_v.map(str::to_string))
    } else if let Some(dt) = &dt_dbg {
        if dt.contains("sharedstring") {
            ("s", raw_v.map(str::to_string))
        } else if dt.contains("inlinestring") {
            let (s, runs) = if let Some(is) = cell.inline_string.as_ref() {
                if !is.run.is_empty() {
                    let mut s = String::new();
                    let mut rs: Vec<TextRun> = Vec::with_capacity(is.run.len());
                    for r in &is.run {
                        let txt = r.text.xml_content.as_deref().unwrap_or("").to_string();
                        s.push_str(&txt);
                        rs.push(crate::text_run_from(r, txt));
                    }
                    (s, rs)
                } else {
                    let s = is
                        .text
                        .as_ref()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_ooxml_widths_use_excel_mdw_formula() {
        assert_eq!(explicit_width_attr_to_px(23.421875, 11.0), 164.0);
        assert_eq!(explicit_width_attr_to_px(8.00390625, 11.0), 56.0);
        assert_eq!(explicit_width_attr_to_px(8.43, 11.0), 59.0);
    }
}
