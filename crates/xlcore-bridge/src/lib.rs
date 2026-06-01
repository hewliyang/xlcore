use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use ooxmlsdk::simple_type::BooleanValue;
use ooxmlsdk::sdk::SdkPart;
use xlcore_engine::{CellValue, FormulaError, WorkbookEngine};
use xlcore_io::spreadsheetml as x;

pub use xlcore_types::{FormulaFallback, RecalcCell, RecalcSheet, RecalcWorkbook};

pub fn recalculate<P: AsRef<Path>>(path: P) -> Result<RecalcWorkbook> {
    let mut doc = xlcore_io::open(path)?;
    recalculate_doc(&mut doc)
}

pub fn recalculate_and_save<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
) -> Result<RecalcWorkbook> {
    let mut doc = xlcore_io::open(input)?;
    let recalculated = recalculate_doc_with_writeback(&mut doc)?;
    xlcore_io::save(&mut doc, output)?;
    Ok(recalculated)
}

pub fn recalculate_layout<P: AsRef<Path>>(
    path: P,
) -> Result<(RecalcWorkbook, xlcore_export::WorkbookLayout)> {
    let mut doc = xlcore_io::open(path)?;
    recalculate_layout_doc(&mut doc)
}

pub fn recalculate_doc(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<RecalcWorkbook> {
    let shared_strings = load_shared_strings(doc);
    let workbook_sheets = {
        let wb_part = doc.workbook_part()?;
        wb_part.root_element(doc)?.sheets.sheet.clone()
    };

    let ws_parts_by_rel_id = {
        let wb_part = doc.workbook_part()?;
        wb_part
            .worksheet_parts(doc)
            .filter_map(|part| {
                part.relationship_id()
                    .map(|id| (id.to_string(), part.clone()))
            })
            .collect::<HashMap<_, _>>()
    };

    let mut harvested = Vec::with_capacity(workbook_sheets.len());
    for (idx, wb_sheet) in workbook_sheets.iter().enumerate() {
        let Some(ws_part) = ws_parts_by_rel_id.get(wb_sheet.id.as_str()).cloned() else {
            continue;
        };
        let ws = ws_part.root_element(doc)?;
        harvested.push(HarvestedSheet {
            index: idx as u32,
            name: wb_sheet.name.as_str().to_string(),
            cells: harvest_sheet_cells(ws, &shared_strings),
        });
    }

    let mut engine = WorkbookEngine::new("xlcore-bridge")?;
    let mut fallbacks = load_engine(&mut engine, &harvested)?;
    engine.evaluate();

    let mut sheets = Vec::with_capacity(harvested.len());
    for sheet in harvested {
        let mut cells = Vec::new();
        for cell in sheet.cells {
            if let Some(formula) = cell.formula {
                let key = CellKey {
                    sheet: sheet.index,
                    r: cell.r,
                    c: cell.c,
                };
                let mut fallback = fallbacks.remove(&key);
                let value = if fallback.is_some() {
                    cell.cached_value.clone().unwrap_or(CellValue::Blank)
                } else {
                    evaluated_formula_value(&engine, key, cell.cached_value.as_ref(), &mut fallback)
                };
                cells.push(RecalcCell {
                    r: cell.r,
                    c: cell.c,
                    formula,
                    cached_value: cell.cached_value,
                    value,
                    fallback,
                });
            }
        }
        sheets.push(RecalcSheet {
            index: sheet.index,
            name: sheet.name,
            cells,
        });
    }

    Ok(RecalcWorkbook { sheets })
}

pub fn recalculate_doc_with_writeback(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> Result<RecalcWorkbook> {
    let recalculated = recalculate_doc(doc)?;
    write_cached_formula_values(doc, &recalculated)?;
    mark_cached_formula_values_current(doc)?;
    Ok(recalculated)
}

pub fn recalculate_layout_doc(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> Result<(RecalcWorkbook, xlcore_export::WorkbookLayout)> {
    let recalculated = recalculate_doc_with_writeback(doc)?;
    let layout = xlcore_export::extract_doc(doc)?;
    Ok((recalculated, layout))
}

#[derive(Clone, Debug)]
struct HarvestedSheet {
    index: u32,
    name: String,
    cells: Vec<HarvestedCell>,
}

#[derive(Clone, Debug)]
struct HarvestedCell {
    r: u32,
    c: u32,
    formula: Option<String>,
    literal: Option<CellValue>,
    cached_value: Option<CellValue>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CellKey {
    sheet: u32,
    r: u32,
    c: u32,
}

fn load_engine(
    engine: &mut WorkbookEngine<'_>,
    sheets: &[HarvestedSheet],
) -> Result<HashMap<CellKey, FormulaFallback>> {
    if let Some(first) = sheets.first() {
        engine.rename_sheet("Sheet1", &first.name)?;
    }
    for sheet in sheets.iter().skip(1) {
        engine.add_sheet(&sheet.name)?;
    }

    let mut fallbacks = HashMap::new();
    for sheet in sheets {
        for cell in &sheet.cells {
            let sheet_index = sheet.index;
            let row = cell.r as i32;
            let column = cell.c as i32;
            if let Some(formula) = &cell.formula {
                if let Err(err) = engine.set_formula(sheet_index, row, column, formula) {
                    fallbacks.insert(
                        CellKey {
                            sheet: sheet_index,
                            r: cell.r,
                            c: cell.c,
                        },
                        FormulaFallback {
                            kind: "#ERROR!".to_string(),
                            message: err.to_string(),
                        },
                    );
                }
            } else if let Some(value) = &cell.literal {
                set_literal(engine, sheet_index, row, column, value)?;
            }
        }
    }

    Ok(fallbacks)
}

fn evaluated_formula_value(
    engine: &WorkbookEngine<'_>,
    key: CellKey,
    cached_value: Option<&CellValue>,
    fallback: &mut Option<FormulaFallback>,
) -> CellValue {
    match engine.formula_error(key.sheet, key.r as i32, key.c as i32) {
        Ok(Some(error)) => {
            if let Some(formula_fallback) = fallback_for_formula_error(error.clone()) {
                *fallback = Some(formula_fallback);
                cached_value.cloned().unwrap_or(CellValue::Blank)
            } else {
                CellValue::String(error.kind)
            }
        }
        Ok(None) => match engine.cell_value(key.sheet, key.r as i32, key.c as i32) {
            Ok(value) => value,
            Err(err) => {
                *fallback = Some(FormulaFallback {
                    kind: "#ERROR!".to_string(),
                    message: err.to_string(),
                });
                cached_value.cloned().unwrap_or(CellValue::Blank)
            }
        },
        Err(err) => {
            *fallback = Some(FormulaFallback {
                kind: "#ERROR!".to_string(),
                message: err.to_string(),
            });
            cached_value.cloned().unwrap_or(CellValue::Blank)
        }
    }
}

fn fallback_for_formula_error(error: FormulaError) -> Option<FormulaFallback> {
    matches!(
        error.kind.as_str(),
        "#NAME?" | "#N/IMPL" | "#N/IMPL!" | "#ERROR!"
    )
    .then_some(FormulaFallback {
        kind: error.kind,
        message: error.message,
    })
}

fn set_literal(
    engine: &mut WorkbookEngine<'_>,
    sheet: u32,
    row: i32,
    column: i32,
    value: &CellValue,
) -> Result<()> {
    match value {
        CellValue::Blank => {}
        CellValue::String(value) => engine.set_input(sheet, row, column, format!("'{value}"))?,
        CellValue::Number(value) => engine.set_input(sheet, row, column, value.to_string())?,
        CellValue::Boolean(value) => engine.set_input(sheet, row, column, value.to_string())?,
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct SharedFormula {
    r: u32,
    c: u32,
    formula: String,
}

fn write_cached_formula_values(
    doc: &mut xlcore_io::SpreadsheetDocument,
    recalculated: &RecalcWorkbook,
) -> Result<()> {
    let workbook_sheets = {
        let wb_part = doc.workbook_part()?;
        wb_part.root_element(doc)?.sheets.sheet.clone()
    };
    let ws_parts = {
        let wb_part = doc.workbook_part()?;
        wb_part.worksheet_parts(doc).collect::<Vec<_>>()
    };
    let ws_parts_by_rel_id = ws_parts
        .iter()
        .filter_map(|part| {
            part.relationship_id()
                .map(|id| (id.to_string(), part.clone()))
        })
        .collect::<HashMap<_, _>>();
    let recalc_by_sheet_index = recalculated
        .sheets
        .iter()
        .map(|sheet| (sheet.index, sheet))
        .collect::<HashMap<_, _>>();

    for (idx, wb_sheet) in workbook_sheets.iter().enumerate() {
        let sheet_index = idx as u32;
        let Some(recalc_sheet) = recalc_by_sheet_index.get(&sheet_index) else {
            continue;
        };
        let Some(ws_part) = ws_parts_by_rel_id
            .get(wb_sheet.id.as_str())
            .or_else(|| ws_parts.get(idx))
            .cloned()
        else {
            continue;
        };
        let updates = recalc_sheet
            .cells
            .iter()
            .map(|cell| ((cell.r, cell.c), cell))
            .collect::<HashMap<_, _>>();

        let ws = ws_part.root_element_mut(doc)?;
        for row in &mut ws.sheet_data.row {
            for cell in &mut row.cell {
                if cell.cell_formula.is_none() {
                    continue;
                }
                let Some(r_attr) = cell.cell_reference.as_ref() else {
                    continue;
                };
                let Some((r, c)) = xlcore_io::parse_a1(r_attr.as_str()) else {
                    continue;
                };
                let Some(update) = updates.get(&(r, c)) else {
                    continue;
                };
                if update.fallback.is_some() {
                    continue;
                }
                set_cached_formula_value(cell, &update.value);
            }
        }
    }

    Ok(())
}

fn set_cached_formula_value(cell: &mut x::Cell, value: &CellValue) {
    cell.inline_string = None;
    match value {
        CellValue::Blank => {
            cell.data_type = None;
            cell.cell_value = None;
        }
        CellValue::Number(value) => {
            cell.data_type = None;
            cell.cell_value = Some(x::CellValue(x::XstringType { xml_content: Some(format_number(*value)), ..Default::default() }));
        }
        CellValue::Boolean(value) => {
            cell.data_type = Some(x::CellValues::Boolean);
            cell.cell_value = Some(x::CellValue(x::XstringType { xml_content: Some(if *value { "1" } else { "0" }.to_string()), ..Default::default() }));
        }
        CellValue::String(value) => {
            cell.data_type = Some(x::CellValues::String);
            cell.cell_value = Some(x::CellValue(x::XstringType { xml_content: Some(value.clone()), ..Default::default() }));
        }
    }
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

fn mark_cached_formula_values_current(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<()> {
    let wb_part = doc.workbook_part()?;
    let workbook = wb_part.root_element_mut(doc)?;
    let calc = workbook
        .calculation_properties
        .get_or_insert_with(Default::default);
    calc.full_calculation_on_load = Some(BooleanValue::from_bool(false));
    calc.force_full_calculation = Some(BooleanValue::from_bool(false));
    calc.calculation_completed = Some(BooleanValue::from_bool(true));
    Ok(())
}

fn harvest_sheet_cells(ws: &x::Worksheet, shared_strings: &[String]) -> Vec<HarvestedCell> {
    let shared_formulas = collect_shared_formulas(ws);
    let mut cells = Vec::new();
    for row in &ws.sheet_data.row {
        for cell in &row.cell {
            let Some(r_attr) = cell.cell_reference.as_ref() else {
                continue;
            };
            let Some((r, c)) = xlcore_io::parse_a1(r_attr.as_str()) else {
                continue;
            };
            let formula = expanded_formula(cell, r, c, &shared_formulas);
            let raw_v = cell
                .cell_value
                .as_ref()
                .and_then(|v| v.0.xml_content.as_deref());
            let value = cell_value(cell, raw_v, shared_strings);
            let (literal, cached_value) = if formula.is_some() {
                (None, value)
            } else {
                (value, None)
            };
            if formula.is_none() && literal.is_none() {
                continue;
            }
            cells.push(HarvestedCell {
                r,
                c,
                formula,
                literal,
                cached_value,
            });
        }
    }
    cells
}

fn collect_shared_formulas(ws: &x::Worksheet) -> HashMap<u32, SharedFormula> {
    let mut formulas = HashMap::new();
    for row in &ws.sheet_data.row {
        for cell in &row.cell {
            let Some(formula) = cell.cell_formula.as_ref() else {
                continue;
            };
            if !is_shared_formula(formula) {
                continue;
            }
            let Some(shared_index) = formula.shared_index else {
                continue;
            };
            let Some(formula_text) = formula.xml_content.as_deref() else {
                continue;
            };
            let Some(cell_ref) = cell.cell_reference.as_ref() else {
                continue;
            };
            let Some((r, c)) = xlcore_io::parse_a1(cell_ref.as_str()) else {
                continue;
            };
            formulas
                .entry(shared_index)
                .or_insert_with(|| SharedFormula {
                    r,
                    c,
                    formula: formula_text.to_string(),
                });
        }
    }
    formulas
}

fn expanded_formula(
    cell: &x::Cell,
    r: u32,
    c: u32,
    shared_formulas: &HashMap<u32, SharedFormula>,
) -> Option<String> {
    let formula = cell.cell_formula.as_ref()?;
    let formula_text = formula.xml_content.as_deref();
    if !is_shared_formula(formula) {
        return formula_text.map(str::to_string);
    }

    if let Some(formula_text) = formula_text {
        return Some(formula_text.to_string());
    }

    let shared_index = formula.shared_index?;
    let base = shared_formulas.get(&shared_index)?;
    Some(translate_shared_formula(
        &base.formula,
        r as i32 - base.r as i32,
        c as i32 - base.c as i32,
    ))
}

fn is_shared_formula(formula: &x::CellFormula) -> bool {
    formula.formula_type == Some(x::CellFormulaValues::Shared)
}

fn translate_shared_formula(formula: &str, row_delta: i32, column_delta: i32) -> String {
    let bytes = formula.as_bytes();
    let mut out = String::with_capacity(formula.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let end = consume_double_quoted(bytes, i);
                out.push_str(&formula[i..end]);
                i = end;
            }
            b'\'' => {
                let end = consume_single_quoted(bytes, i);
                out.push_str(&formula[i..end]);
                i = end;
            }
            b'[' => {
                let end = consume_bracketed(bytes, i);
                out.push_str(&formula[i..end]);
                i = end;
            }
            _ => {
                if let Some((end, translated)) =
                    translate_a1_reference_at(formula, i, row_delta, column_delta)
                {
                    out.push_str(&translated);
                    i = end;
                } else {
                    let ch = formula[i..].chars().next().expect("valid utf-8 boundary");
                    out.push(ch);
                    i += ch.len_utf8();
                }
            }
        }
    }
    out
}

fn consume_double_quoted(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                i += 2;
            } else {
                return i + 1;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

fn consume_single_quoted(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                i += 2;
            } else {
                return i + 1;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

fn consume_bracketed(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b']' {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn translate_a1_reference_at(
    formula: &str,
    start: usize,
    row_delta: i32,
    column_delta: i32,
) -> Option<(usize, String)> {
    let bytes = formula.as_bytes();
    if start > 0 && is_reference_name_char(bytes[start - 1]) {
        return None;
    }

    let mut i = start;
    let absolute_column = consume_byte(bytes, &mut i, b'$');
    let column_start = i;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == column_start {
        return None;
    }
    let column_end = i;
    let absolute_row = consume_byte(bytes, &mut i, b'$');
    let row_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == row_start {
        return None;
    }
    if i < bytes.len() && is_reference_name_char(bytes[i]) {
        return None;
    }
    if i < bytes.len() && matches!(bytes[i], b'!' | b'[') {
        return None;
    }

    let column_label = &formula[column_start..column_end];
    let row_text = &formula[row_start..i];
    let column = column_label_to_number(column_label)?;
    let row = row_text.parse::<u32>().ok()?;

    let translated_column = if absolute_column {
        column
    } else {
        checked_offset(column, column_delta)?
    };
    let translated_row = if absolute_row {
        row
    } else {
        checked_offset(row, row_delta)?
    };

    let mut translated = String::new();
    if absolute_column {
        translated.push('$');
    }
    translated.push_str(&xlcore_io::col_label(translated_column));
    if absolute_row {
        translated.push('$');
    }
    translated.push_str(&translated_row.to_string());

    Some((i, translated))
}

fn consume_byte(bytes: &[u8], i: &mut usize, byte: u8) -> bool {
    if *i < bytes.len() && bytes[*i] == byte {
        *i += 1;
        true
    } else {
        false
    }
}

fn is_reference_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')
}

fn column_label_to_number(label: &str) -> Option<u32> {
    let mut column = 0u32;
    for ch in label.bytes() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        column = column
            .checked_mul(26)?
            .checked_add((ch.to_ascii_uppercase() - b'A' + 1) as u32)?;
    }
    (column > 0).then_some(column)
}

fn checked_offset(value: u32, delta: i32) -> Option<u32> {
    let shifted = value as i64 + delta as i64;
    if shifted < 1 || shifted > u32::MAX as i64 {
        None
    } else {
        Some(shifted as u32)
    }
}

fn cell_value(cell: &x::Cell, raw_v: Option<&str>, shared_strings: &[String]) -> Option<CellValue> {
    let dt_dbg = cell
        .data_type
        .as_ref()
        .map(|d| format!("{d:?}").to_ascii_lowercase());

    if let Some(dt) = &dt_dbg {
        if dt.contains("sharedstring") {
            let idx = raw_v?.parse::<usize>().ok()?;
            return Some(CellValue::String(
                shared_strings.get(idx).cloned().unwrap_or_default(),
            ));
        }
        if dt.contains("inlinestring") {
            return Some(CellValue::String(inline_string_text(cell)));
        }
        if dt.contains("boolean") {
            return Some(CellValue::Boolean(matches!(raw_v, Some("1"))));
        }
        if dt.contains("error") {
            return Some(CellValue::String(raw_v.unwrap_or("#ERROR!").to_string()));
        }
        if dt.contains("str") {
            return Some(CellValue::String(raw_v.unwrap_or("").to_string()));
        }
    }

    raw_v
        .and_then(|value| value.parse::<f64>().ok())
        .map(CellValue::Number)
}

fn inline_string_text(cell: &x::Cell) -> String {
    let Some(inline) = cell.inline_string.as_ref() else {
        return String::new();
    };
    if !inline.run.is_empty() {
        let mut out = String::new();
        for run in &inline.run {
            out.push_str(run.text.0.xml_content.as_deref().unwrap_or(""));
        }
        return out;
    }
    inline
        .text
        .as_ref()
        .and_then(|text| text.0.xml_content.as_deref())
        .unwrap_or("")
        .to_string()
}

fn load_shared_strings(doc: &mut xlcore_io::SpreadsheetDocument) -> Vec<String> {
    let Ok(wb_part) = doc.workbook_part() else {
        return Vec::new();
    };
    let Some(sst_part) = wb_part.shared_string_table_part(doc) else {
        return Vec::new();
    };
    let Ok(sst) = sst_part.root_element(doc) else {
        return Vec::new();
    };

    let mut strings = Vec::with_capacity(sst.shared_string_item.len());
    for item in &sst.shared_string_item {
        if let Some(text) = &item.text {
            strings.push(text.0.xml_content.as_deref().unwrap_or("").to_string());
            continue;
        }

        let mut out = String::new();
        for run in &item.run {
            out.push_str(run.text.0.xml_content.as_deref().unwrap_or(""));
        }
        strings.push(out);
    }
    strings
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn recalculates_basic_formula_fixture() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/engine/basic-formulas.xlsx");
        let wb = recalculate(fixture).context("recalculate fixture").unwrap();

        assert_eq!(
            wb.cell("Sheet1", "C1").map(|cell| &cell.value),
            Some(&CellValue::Number(30.0))
        );
        assert_eq!(
            wb.cell("Sheet1", "C2").map(|cell| &cell.value),
            Some(&CellValue::Number(80.0))
        );
    }

    #[test]
    fn writes_cached_formula_values_and_layout_uses_them() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/engine/stale-formulas.xlsx");
        let out = temp_xlsx_path("xlcore-bridge-writeback");
        let _ = std::fs::remove_file(&out);

        let wb = recalculate_and_save(&fixture, &out)
            .context("recalculate and save fixture")
            .unwrap();
        assert_eq!(
            wb.cell("Sheet1", "C2").map(|cell| &cell.value),
            Some(&CellValue::Number(80.0))
        );

        let saved = recalculate(&out).context("reopen saved fixture").unwrap();
        assert_eq!(
            saved
                .cell("Sheet1", "C1")
                .and_then(|cell| cell.cached_value.as_ref()),
            Some(&CellValue::Number(30.0))
        );
        assert_eq!(
            saved
                .cell("Sheet1", "C2")
                .and_then(|cell| cell.cached_value.as_ref()),
            Some(&CellValue::Number(80.0))
        );

        let (_recalc, layout) = recalculate_layout(&out)
            .context("recalculate saved fixture into layout")
            .unwrap();
        let sheet = layout
            .sheets
            .iter()
            .find(|sheet| sheet.name == "Sheet1")
            .expect("Sheet1 layout");
        assert!(sheet.value_pool.iter().any(|value| value == "80"));
        assert!(sheet
            .formula_pool
            .iter()
            .any(|formula| formula == "SUMPRODUCT(A1:A2,B1:B2)"));

        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn expands_shared_formulas_and_writes_each_cache() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/engine/shared-formulas.xlsx");
        let out = temp_xlsx_path("xlcore-bridge-shared");
        let _ = std::fs::remove_file(&out);

        let wb = recalculate_and_save(&fixture, &out)
            .context("recalculate shared formula fixture")
            .unwrap();
        assert_number(&wb, "C1", 11.0);
        assert_number(&wb, "C2", 22.0);
        assert_number(&wb, "C3", 33.0);
        assert_number(&wb, "D3", 33.0);
        assert_number(&wb, "E3", 31.0);
        assert_eq!(
            wb.cell("Sheet1", "C2").map(|cell| cell.formula.as_str()),
            Some("A2+B2")
        );
        assert_eq!(
            wb.cell("Sheet1", "D3").map(|cell| cell.formula.as_str()),
            Some("SUM(A3:B3)")
        );
        assert_eq!(
            wb.cell("Sheet1", "E3").map(|cell| cell.formula.as_str()),
            Some("$A3+B$1")
        );

        let saved = recalculate(&out).context("reopen shared output").unwrap();
        assert_eq!(
            saved
                .cell("Sheet1", "E3")
                .and_then(|cell| cell.cached_value.as_ref()),
            Some(&CellValue::Number(31.0))
        );

        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn preserves_cached_values_for_unsupported_formulas() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/engine/unsupported-formulas.xlsx");
        let out = temp_xlsx_path("xlcore-bridge-unsupported");
        let _ = std::fs::remove_file(&out);

        let wb = recalculate_and_save(&fixture, &out)
            .context("recalculate unsupported formula fixture")
            .unwrap();
        let unsupported = wb.cell("Sheet1", "B1").expect("B1");
        assert_eq!(unsupported.value, CellValue::Number(123.0));
        assert_eq!(unsupported.cached_value, Some(CellValue::Number(123.0)));
        assert_eq!(
            unsupported
                .fallback
                .as_ref()
                .map(|fallback| fallback.kind.as_str()),
            Some("#NAME?")
        );
        assert!(unsupported
            .fallback
            .as_ref()
            .is_some_and(|fallback| fallback.message.contains("Invalid function")));

        let supported = wb.cell("Sheet1", "C1").expect("C1");
        assert_eq!(supported.value, CellValue::Number(15.0));
        assert_eq!(supported.fallback, None);

        let saved = recalculate(&out)
            .context("reopen unsupported output")
            .unwrap();
        assert_eq!(
            saved
                .cell("Sheet1", "B1")
                .and_then(|cell| cell.cached_value.as_ref()),
            Some(&CellValue::Number(123.0))
        );
        assert_eq!(
            saved
                .cell("Sheet1", "C1")
                .and_then(|cell| cell.cached_value.as_ref()),
            Some(&CellValue::Number(15.0))
        );

        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn translates_shared_formula_a1_references() {
        assert_eq!(translate_shared_formula("A1+B1", 2, 1), "B3+C3");
        assert_eq!(
            translate_shared_formula("SUM(A1:B1,$A1+B$1)", 2, 1),
            "SUM(B3:C3,$A3+C$1)"
        );
        assert_eq!(
            translate_shared_formula(r#""A1"&'Q1'!A1&Sheet1!A1&Table1[Col]"#, 2, 1),
            r#""A1"&'Q1'!B3&Sheet1!B3&Table1[Col]"#
        );
    }

    fn assert_number(wb: &RecalcWorkbook, cell_ref: &str, expected: f64) {
        assert_eq!(
            wb.cell("Sheet1", cell_ref).map(|cell| &cell.value),
            Some(&CellValue::Number(expected)),
            "{cell_ref}"
        );
    }

    fn temp_xlsx_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.xlsx", std::process::id()))
    }
}
