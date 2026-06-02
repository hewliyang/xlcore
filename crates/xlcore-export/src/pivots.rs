use crate::pivot_engine::PivotStyleIndices;
use crate::schema::{Cell, Merge, Pivot, PivotFilterArrow, PivotFilterAxis, Styles};
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::Cell as XCell;
use std::collections::HashMap;
use xlcore_io::{parse_a1, parse_range, SpreadsheetDocument};

pub fn extract(
    doc: &mut SpreadsheetDocument,
    ws_part: &WorksheetPart,
    styles: &mut Styles,
    style_memo: &mut Option<PivotStyleIndices>,
    shared_strings: &[String],
    sheet_parts: &HashMap<String, WorksheetPart>,
) -> (Vec<Pivot>, Vec<Cell>) {
    let mut out = Vec::new();
    let mut cells = Vec::new();
    let pivot_parts: Vec<_> = ws_part.pivot_table_parts(doc).collect();

    for pp in &pivot_parts {
        let pt = match pp.root_element(doc) {
            Ok(pt) => pt.clone(),
            Err(_) => continue,
        };

        let cache = pp
            .pivot_table_cache_definition_part(doc)
            .and_then(|def_part| {
                let rec_part = def_part.pivot_table_cache_records_part(doc)?;
                let cache_def = def_part.root_element(doc).ok()?.clone();
                let records = rec_part.root_element(doc).ok()?.clone();
                Some((cache_def, records))
            });
        let mut field_names: Vec<String> = Vec::new();
        if let Some((cache_def, mut records)) = cache {
            field_names = cache_def
                .cache_fields
                .cache_field
                .iter()
                .map(|f| f.name.clone())
                .collect();
            if records.pivot_cache_record.is_empty() {
                if let Some(synth) =
                    synthesize_records(doc, &cache_def, shared_strings, sheet_parts)
                {
                    records.pivot_cache_record = synth;
                }
            }
            cells.extend(crate::pivot_engine::compute_cells(
                &pt, &cache_def, &records, styles, style_memo,
            ));
        }

        let field_name = |idx: i32| -> String {
            usize::try_from(idx)
                .ok()
                .and_then(|i| field_names.get(i))
                .cloned()
                .unwrap_or_default()
        };
        let row_field_name = pt
            .row_fields
            .as_ref()
            .and_then(|rf| rf.field.first())
            .map(|f| field_name(f.index))
            .unwrap_or_default();
        let col_field_name = pt
            .column_fields
            .as_ref()
            .and_then(|cf| cf.field.iter().find(|f| f.index >= 0))
            .map(|f| field_name(f.index))
            .unwrap_or_default();

        let Some(((r1, c1), (r2, c2))) = parse_range(pt.location.reference.as_str()) else {
            continue;
        };

        let first_header_row = pt.location.first_header_row;
        let first_data_row = pt.location.first_data_row;
        let first_data_col = pt.location.first_data_column;
        let has_row_fields = pt
            .row_fields
            .as_ref()
            .map(|rf| !rf.field.is_empty())
            .unwrap_or(false);
        let has_col_fields = pt
            .column_fields
            .as_ref()
            .map(|cf| !cf.field.is_empty())
            .unwrap_or(false);

        let mut filter_arrow_cells = Vec::new();

        if has_row_fields && first_data_row >= 1 && first_data_col >= 1 {
            let r = r1 + first_data_row - 1;
            let c = c1 + (first_data_col - 1);
            if r >= r1 && r <= r2 && c >= c1 && c <= c2 {
                filter_arrow_cells.push(PivotFilterArrow {
                    r,
                    c,
                    field: row_field_name.clone(),
                    axis: PivotFilterAxis::Row,
                });
            }
        }

        if has_col_fields && first_header_row >= 1 {
            let r = r1 + (first_header_row - 1);
            let c = c1 + first_data_col;
            if r >= r1 && r <= r2 && c >= c1 && c <= c2 {
                filter_arrow_cells.push(PivotFilterArrow {
                    r,
                    c,
                    field: col_field_name.clone(),
                    axis: PivotFilterAxis::Column,
                });
            }
        }

        out.push(Pivot {
            name: pt.name.as_str().to_string(),
            range: Merge { r1, c1, r2, c2 },
            filter_arrow_cells,
        });
    }
    (out, cells)
}

fn synthesize_records(
    doc: &mut SpreadsheetDocument,
    cache_def: &x::PivotCacheDefinition,
    shared_strings: &[String],
    sheet_parts: &HashMap<String, WorksheetPart>,
) -> Option<Vec<x::PivotCacheRecord>> {
    let ws_src = match &cache_def.cache_source.cache_source_choice {
        Some(x::CacheSourceChoice::WorksheetSource(w)) => w,
        _ => return None,
    };
    let sheet_name = ws_src.sheet.as_deref()?;
    let reference = ws_src.reference.as_deref()?;
    let ((r1, c1), (r2, c2)) = parse_range(reference)?;
    if r2 <= r1 {
        return None;
    }
    let ncols = cache_def.cache_fields.cache_field.len();
    if ncols == 0 {
        return None;
    }
    let part = sheet_parts.get(sheet_name)?;
    let ws = part.root_element(doc).ok()?.clone();

    let mut grid: HashMap<(u32, u32), x::PivotCacheRecordChoice> = HashMap::new();
    for row in &ws.sheet_data.row {
        for cell in &row.cell {
            let Some((rr, cc)) = cell
                .cell_reference
                .as_ref()
                .and_then(|r| parse_a1(r.as_str()))
            else {
                continue;
            };
            if rr <= r1 || rr > r2 || cc < c1 || cc > c2 {
                continue;
            }
            grid.insert((rr, cc), cell_choice(cell, shared_strings));
        }
    }

    let records = (r1 + 1..=r2)
        .map(|rr| x::PivotCacheRecord {
            pivot_cache_record_choice: (0..ncols as u32)
                .map(|i| grid.remove(&(rr, c1 + i)).unwrap_or_else(missing_item))
                .collect(),
        })
        .collect();
    Some(records)
}

fn cell_choice(cell: &XCell, shared_strings: &[String]) -> x::PivotCacheRecordChoice {
    let raw = cell.cell_value.as_ref().and_then(|v| v.xml_content.as_deref());
    let dt = cell
        .data_type
        .as_ref()
        .map(|d| format!("{d:?}").to_ascii_lowercase());

    if let Some(dt) = &dt {
        if dt.contains("sharedstring") {
            return raw
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|i| shared_strings.get(i))
                .map(|s| string_item(s.clone()))
                .unwrap_or_else(missing_item);
        }
        if dt.contains("inlinestring") {
            return cell
                .inline_string
                .as_ref()
                .map(|is| string_item(inline_text(is)))
                .unwrap_or_else(missing_item);
        }
        if dt.contains("boolean") {
            return raw
                .map(|v| bool_item(v == "1" || v.eq_ignore_ascii_case("true")))
                .unwrap_or_else(missing_item);
        }
        if dt.contains("error") {
            return missing_item();
        }
        if dt.contains("str") {
            return raw
                .map(|v| string_item(v.to_string()))
                .unwrap_or_else(missing_item);
        }
    }

    raw.and_then(|v| v.parse::<f64>().ok())
        .map(number_item)
        .unwrap_or_else(missing_item)
}

fn inline_text(is: &x::InlineString) -> String {
    if let Some(t) = is.text.as_ref().and_then(|t| t.xml_content.as_deref()) {
        return t.to_string();
    }
    is.run
        .iter()
        .map(|r| r.text.xml_content.as_deref().unwrap_or(""))
        .collect()
}

fn number_item(n: f64) -> x::PivotCacheRecordChoice {
    x::PivotCacheRecordChoice::NumberItem(Box::new(x::NumberItem {
        val: n,
        ..Default::default()
    }))
}

fn string_item(s: String) -> x::PivotCacheRecordChoice {
    x::PivotCacheRecordChoice::StringItem(Box::new(x::StringItem {
        val: s,
        ..Default::default()
    }))
}

fn bool_item(b: bool) -> x::PivotCacheRecordChoice {
    x::PivotCacheRecordChoice::BooleanItem(Box::new(x::BooleanItem {
        val: b.into(),
        ..Default::default()
    }))
}

fn missing_item() -> x::PivotCacheRecordChoice {
    x::PivotCacheRecordChoice::MissingItem(Box::new(Default::default()))
}
