use crate::schema::ValidationDropdown;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use std::collections::HashMap;
use xlcore_io::{parse_a1, parse_range};

pub fn extract(
    ws: &x::Worksheet,
    sheet_name: &str,
    shared_strings: &[String],
) -> (Vec<ValidationDropdown>, Vec<Vec<String>>) {
    let mut dropdowns = Vec::new();
    let mut lists: Vec<Vec<String>> = Vec::new();
    let mut list_index: HashMap<Vec<String>, u32> = HashMap::new();
    let mut seen: HashMap<(u32, u32), ()> = HashMap::new();
    let mut cell_text: Option<HashMap<(u32, u32), String>> = None;

    let Some(block) = ws.data_validations.as_ref() else {
        return (dropdowns, lists);
    };

    for dv in &block.data_validation {
        if !matches!(dv.r#type, Some(x::DataValidationValues::List)) {
            continue;
        }
        if dv
            .show_drop_down
            .map(ooxmlsdk::simple_type::BooleanValue::into)
            .unwrap_or(false)
        {
            continue;
        }
        let formula1 = dv
            .formula1
            .as_ref()
            .and_then(|f| f.xml_content.as_deref())
            .map(str::trim)
            .unwrap_or("");
        let options = resolve_options(formula1, sheet_name, ws, shared_strings, &mut cell_text);
        let idx = *list_index.entry(options.clone()).or_insert_with(|| {
            lists.push(options.clone());
            (lists.len() - 1) as u32
        });
        for sqref in &dv.sequence_of_references {
            let raw = sqref.as_str();
            let bounds = parse_range(raw).or_else(|| parse_a1(raw).map(|cell| (cell, cell)));
            let Some(((r1, c1), (r2, c2))) = bounds else {
                continue;
            };
            for r in r1..=r2 {
                for c in c1..=c2 {
                    if seen.insert((r, c), ()).is_none() {
                        dropdowns.push(ValidationDropdown { r, c, list: idx });
                    }
                }
            }
        }
    }

    (dropdowns, lists)
}

fn resolve_options(
    formula1: &str,
    sheet_name: &str,
    ws: &x::Worksheet,
    shared_strings: &[String],
    cell_text: &mut Option<HashMap<(u32, u32), String>>,
) -> Vec<String> {
    if formula1.is_empty() {
        return Vec::new();
    }
    if formula1.starts_with('"') && formula1.ends_with('"') && formula1.len() >= 2 {
        return formula1[1..formula1.len() - 1]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
    }
    let Some(range) = resolve_range_ref(formula1, sheet_name) else {
        return Vec::new();
    };
    let map = cell_text.get_or_insert_with(|| build_cell_text(ws, shared_strings));
    let ((r1, c1), (r2, c2)) = range;
    let mut out = Vec::new();
    for r in r1..=r2 {
        for c in c1..=c2 {
            if let Some(t) = map.get(&(r, c)) {
                if !t.is_empty() {
                    out.push(t.clone());
                }
            }
        }
    }
    out
}

fn resolve_range_ref(formula1: &str, sheet_name: &str) -> Option<((u32, u32), (u32, u32))> {
    let body = if let Some(idx) = formula1.rfind('!') {
        let prefix = &formula1[..idx];
        let target = prefix.trim_matches('\'').replace("''", "'");
        if target != sheet_name {
            return None;
        }
        &formula1[idx + 1..]
    } else {
        formula1
    };
    let clean = body.replace('$', "");
    parse_range(&clean).or_else(|| parse_a1(&clean).map(|cell| (cell, cell)))
}

fn build_cell_text(ws: &x::Worksheet, shared_strings: &[String]) -> HashMap<(u32, u32), String> {
    let mut map = HashMap::new();
    for row in &ws.sheet_data.row {
        for cell in &row.cell {
            let Some((r, c)) = cell
                .cell_reference
                .as_ref()
                .and_then(|rf| parse_a1(rf.as_str()))
            else {
                continue;
            };
            map.insert((r, c), cell_text(cell, shared_strings));
        }
    }
    map
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
