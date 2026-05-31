use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use ooxmlsdk::sdk::SdkPart;
use xlcore_engine::{CellValue, WorkbookEngine};
use xlcore_io::spreadsheetml as x;

#[derive(Clone, Debug, PartialEq)]
pub struct RecalcWorkbook {
    pub sheets: Vec<RecalcSheet>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecalcSheet {
    pub index: u32,
    pub name: String,
    pub cells: Vec<RecalcCell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecalcCell {
    pub r: u32,
    pub c: u32,
    pub formula: String,
    pub cached_value: Option<CellValue>,
    pub value: CellValue,
}

impl RecalcWorkbook {
    pub fn sheet(&self, name: &str) -> Option<&RecalcSheet> {
        self.sheets.iter().find(|sheet| sheet.name == name)
    }

    pub fn cell(&self, sheet_name: &str, cell_ref: &str) -> Option<&RecalcCell> {
        let (r, c) = xlcore_io::parse_a1(cell_ref)?;
        self.sheet(sheet_name)?.cell(r, c)
    }
}

impl RecalcSheet {
    pub fn cell(&self, r: u32, c: u32) -> Option<&RecalcCell> {
        self.cells.iter().find(|cell| cell.r == r && cell.c == c)
    }
}

pub fn recalculate<P: AsRef<Path>>(path: P) -> Result<RecalcWorkbook> {
    let mut doc = xlcore_io::open(path)?;
    recalculate_doc(&mut doc)
}

pub fn recalculate_doc(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<RecalcWorkbook> {
    let shared_strings = load_shared_strings(doc);
    let workbook_sheets = {
        let wb_part = doc.workbook_part()?;
        wb_part.root_element(doc)?.sheets.x_sheet.clone()
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
    load_engine(&mut engine, &harvested)?;
    engine.evaluate();

    let mut sheets = Vec::with_capacity(harvested.len());
    for sheet in harvested {
        let mut cells = Vec::new();
        for cell in sheet.cells {
            if let Some(formula) = cell.formula {
                let value = engine
                    .cell_value(sheet.index, cell.r as i32, cell.c as i32)
                    .unwrap_or_else(|_| cell.cached_value.clone().unwrap_or(CellValue::Blank));
                cells.push(RecalcCell {
                    r: cell.r,
                    c: cell.c,
                    formula,
                    cached_value: cell.cached_value,
                    value,
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

fn load_engine(engine: &mut WorkbookEngine<'_>, sheets: &[HarvestedSheet]) -> Result<()> {
    if let Some(first) = sheets.first() {
        engine.rename_sheet("Sheet1", &first.name)?;
    }
    for sheet in sheets.iter().skip(1) {
        engine.add_sheet(&sheet.name)?;
    }

    for sheet in sheets {
        for cell in &sheet.cells {
            let sheet_index = sheet.index;
            let row = cell.r as i32;
            let column = cell.c as i32;
            if let Some(formula) = &cell.formula {
                engine.set_formula(sheet_index, row, column, formula)?;
            } else if let Some(value) = &cell.literal {
                set_literal(engine, sheet_index, row, column, value)?;
            }
        }
    }

    Ok(())
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

fn harvest_sheet_cells(ws: &x::Worksheet, shared_strings: &[String]) -> Vec<HarvestedCell> {
    let mut cells = Vec::new();
    for row in &ws.x_sheet_data.x_row {
        for cell in &row.x_c {
            let Some(r_attr) = cell.cell_reference.as_ref() else {
                continue;
            };
            let Some((r, c)) = xlcore_io::parse_a1(r_attr.as_str()) else {
                continue;
            };
            let formula = cell
                .cell_formula
                .as_ref()
                .and_then(|f| f.xml_content.as_deref().map(str::to_string));
            let raw_v = cell
                .cell_value
                .as_ref()
                .and_then(|v| v.xml_content.as_deref());
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
    if !inline.x_r.is_empty() {
        let mut out = String::new();
        for run in &inline.x_r {
            out.push_str(run.text.xml_content.as_deref().unwrap_or(""));
        }
        return out;
    }
    inline
        .text
        .as_ref()
        .and_then(|text| text.xml_content.as_deref())
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

    let mut strings = Vec::with_capacity(sst.x_si.len());
    for item in &sst.x_si {
        if let Some(text) = &item.text {
            strings.push(text.xml_content.as_deref().unwrap_or("").to_string());
            continue;
        }

        let mut out = String::new();
        for run in &item.x_r {
            out.push_str(run.text.xml_content.as_deref().unwrap_or(""));
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
}
