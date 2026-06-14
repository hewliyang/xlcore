use crate::schema::{Merge, Table, TableFilterArrow};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use std::collections::HashSet;
use xlcore_io::{col_label, parse_a1};

pub fn extract(
    ws: &x::Worksheet,
    sheet_name: &str,
    auto_filter_range: &Option<Merge>,
    tables: &[Table],
    shared_strings: &[String],
) -> Vec<TableFilterArrow> {
    let mut out = Vec::new();
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    let prefix = sheet_ref_prefix(sheet_name);

    if let Some(range) = auto_filter_range {
        push_range(&mut out, &mut seen, ws, &prefix, range, shared_strings, None);
    }

    for table in tables {
        if table.has_auto_filter {
            push_range(
                &mut out,
                &mut seen,
                ws,
                &prefix,
                &table.range,
                shared_strings,
                Some(table),
            );
        }
    }

    out
}

fn sheet_ref_prefix(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    let simple = !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    if simple {
        format!("{name}!")
    } else {
        format!("'{}'!", name.replace('\'', "''"))
    }
}

fn push_range(
    out: &mut Vec<TableFilterArrow>,
    seen: &mut HashSet<(u32, u32)>,
    ws: &x::Worksheet,
    prefix: &str,
    range: &Merge,
    shared_strings: &[String],
    table: Option<&Table>,
) {
    let range_ref = format!(
        "{}{}{}:{}{}",
        prefix,
        col_label(range.c1),
        range.r1,
        col_label(range.c2),
        range.r2
    );

    for c in range.c1..=range.c2 {
        if !seen.insert((range.r1, c)) {
            continue;
        }
        let column_offset = c - range.c1;
        let column_name = table
            .and_then(|t| t.columns.get(column_offset as usize))
            .map(|col| col.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| header_cell_text(ws, range.r1, c, shared_strings));
        out.push(TableFilterArrow {
            r: range.r1,
            c,
            column_offset,
            column_name,
            range_ref: range_ref.clone(),
        });
    }
}

fn header_cell_text(ws: &x::Worksheet, r: u32, c: u32, shared_strings: &[String]) -> String {
    for row in &ws.sheet_data.row {
        for cell in &row.cell {
            let Some((rr, cc)) = cell
                .cell_reference
                .as_ref()
                .and_then(|rf| parse_a1(rf.as_str()))
            else {
                continue;
            };
            if rr != r || cc != c {
                continue;
            }
            return cell_text(cell, shared_strings);
        }
    }
    String::new()
}

fn cell_text(cell: &x::Cell, shared_strings: &[String]) -> String {
    let raw = cell
        .cell_value
        .as_ref()
        .and_then(|v| v.xml_content.as_deref());
    let dt = cell
        .data_type
        .as_ref()
        .map(|d| format!("{d:?}").to_ascii_lowercase());

    if let Some(dt) = &dt {
        if dt.contains("sharedstring") {
            return raw
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|i| shared_strings.get(i))
                .cloned()
                .unwrap_or_default();
        }
        if dt.contains("inlinestring") {
            return cell
                .inline_string
                .as_ref()
                .map(|is| inline_text(is))
                .unwrap_or_default();
        }
    }

    raw.map(str::to_string).unwrap_or_default()
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
