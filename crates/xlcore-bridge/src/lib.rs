use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::simple_type::BooleanValue;
use xlcore_engine::{CellValue, WorkbookEngine};
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

pub struct ResidentEngine {
    engine: WorkbookEngine<'static>,
    fallbacks: HashMap<CellKey, FormulaFallback>,
}

impl ResidentEngine {
    pub fn sheet_index(&self, name: &str) -> Option<u32> {
        self.engine
            .sheet_names()
            .iter()
            .position(|n| n == name)
            .map(|i| i as u32)
    }

    pub fn set_cell_value(
        &mut self,
        sheet: u32,
        row: i32,
        column: i32,
        value: &xlcore_types::ApiCellValue,
    ) -> Result<()> {
        use xlcore_types::ApiCellValue;
        let input = match value {
            ApiCellValue::Blank | ApiCellValue::Error(_) => String::new(),
            ApiCellValue::String(v) => format!("'{v}"),
            ApiCellValue::Number(v) => v.to_string(),
            ApiCellValue::Boolean(v) => v.to_string(),
        };
        self.engine.set_input(sheet, row, column, input)?;
        self.fallbacks.remove(&CellKey {
            sheet,
            r: row as u32,
            c: column as u32,
        });
        Ok(())
    }

    pub fn set_cell_formula(&mut self, sheet: u32, row: i32, column: i32, formula: &str) {
        let key = CellKey {
            sheet,
            r: row as u32,
            c: column as u32,
        };
        match self.engine.set_formula(sheet, row, column, formula) {
            Ok(()) => {
                self.fallbacks.remove(&key);
            }
            Err(err) => {
                self.fallbacks.insert(
                    key,
                    FormulaFallback {
                        kind: "#ERROR!".to_string(),
                        message: err.to_string(),
                    },
                );
            }
        }
    }
}

pub fn recalculate_doc(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<RecalcWorkbook> {
    let mut resident = None;
    recalculate_doc_with_resident(doc, &mut resident)
}

pub fn recalculate_doc_with_resident(
    doc: &mut xlcore_io::SpreadsheetDocument,
    resident: &mut Option<ResidentEngine>,
) -> Result<RecalcWorkbook> {
    let shared_strings = load_shared_strings(doc);
    let (harvested, harvested_defined_names) = harvest_workbook(doc, &shared_strings)?;

    if resident.is_none() {
        let mut engine = WorkbookEngine::new("xlcore-bridge")?;
        let fallbacks = load_engine(&mut engine, &harvested, &harvested_defined_names)?;
        *resident = Some(ResidentEngine { engine, fallbacks });
    }
    let resident = resident.as_mut().expect("resident engine present");
    resident.engine.evaluate();

    build_report(&resident.engine, &harvested, &resident.fallbacks)
}

fn harvest_workbook(
    doc: &mut xlcore_io::SpreadsheetDocument,
    shared_strings: &[String],
) -> Result<(Vec<HarvestedSheet>, Vec<HarvestedDefinedName>)> {
    let (workbook_sheets, harvested_defined_names) = {
        let wb_part = doc.workbook_part()?;
        let wb = wb_part.root_element(doc)?;
        let sheets = wb.sheets.sheet.clone();
        let defined_names = wb
            .defined_names
            .as_ref()
            .map(|dns| {
                dns.defined_name
                    .iter()
                    .map(|dn| HarvestedDefinedName {
                        name: dn.name.as_str().to_string(),
                        scope: dn.local_sheet_id,
                        formula: dn
                            .xml_content
                            .as_ref()
                            .map(|s| s.as_str().to_string())
                            .unwrap_or_default(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        (sheets, defined_names)
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
            cells: harvest_sheet_cells(ws, shared_strings),
        });
    }

    Ok((harvested, harvested_defined_names))
}

fn build_report(
    engine: &WorkbookEngine<'_>,
    harvested: &[HarvestedSheet],
    fallbacks: &HashMap<CellKey, FormulaFallback>,
) -> Result<RecalcWorkbook> {
    let mut sheets = Vec::with_capacity(harvested.len());
    for sheet in harvested {
        let mut cells = Vec::new();
        for cell in &sheet.cells {
            if let Some(formula) = &cell.formula {
                let key = CellKey {
                    sheet: sheet.index,
                    r: cell.r,
                    c: cell.c,
                };
                let mut fallback = fallbacks.get(&key).cloned();
                let value = if fallback.is_some() {
                    cell.cached_value.clone().unwrap_or(CellValue::Blank)
                } else {
                    evaluated_formula_value(engine, key, cell.cached_value.as_ref(), &mut fallback)
                };
                cells.push(RecalcCell {
                    r: cell.r,
                    c: cell.c,
                    formula: formula.clone(),
                    cached_value: cell.cached_value.clone(),
                    value,
                    fallback,
                });
            }
        }
        sheets.push(RecalcSheet {
            index: sheet.index,
            name: sheet.name.clone(),
            cells,
        });
    }

    Ok(RecalcWorkbook { sheets })
}

pub fn recalculate_doc_with_writeback(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> Result<RecalcWorkbook> {
    let mut resident = None;
    recalculate_doc_with_writeback_resident(doc, &mut resident)
}

pub fn recalculate_doc_with_writeback_resident(
    doc: &mut xlcore_io::SpreadsheetDocument,
    resident: &mut Option<ResidentEngine>,
) -> Result<RecalcWorkbook> {
    let recalculated = recalculate_doc_with_resident(doc, resident)?;
    let resident_ref = resident.as_ref().expect("resident engine present");
    write_cached_formula_values(doc, &recalculated, &resident_ref.engine)?;
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
struct HarvestedDefinedName {
    name: String,
    scope: Option<u32>,
    formula: String,
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
    defined_names: &[HarvestedDefinedName],
) -> Result<HashMap<CellKey, FormulaFallback>> {
    if let Some(first) = sheets.first() {
        engine.rename_sheet("Sheet1", &first.name)?;
    }
    for sheet in sheets.iter().skip(1) {
        engine.add_sheet(&sheet.name)?;
    }

    for dn in defined_names {
        let _ = engine.add_defined_name(&dn.name, dn.scope, &dn.formula);
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
        Ok(Some(error)) => match cached_value {
            Some(cv) if !matches!(cv, CellValue::Blank | CellValue::Error(_)) => {
                *fallback = Some(FormulaFallback {
                    kind: error.kind,
                    message: error.message,
                });
                cv.clone()
            }
            _ => CellValue::Error(error.kind),
        },
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
        CellValue::Error(_) => {}
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
    engine: &WorkbookEngine<'_>,
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

    let mut has_dynamic_array = false;

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
        let mut array_ranges = collect_array_ranges(ws);
        mark_engine_spill_anchors(ws, sheet_index, engine, &mut array_ranges);
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
        write_spilled_cells(ws, sheet_index, &array_ranges, engine);
        for range in &array_ranges {
            let cell = ensure_cell(ws, range.anchor.0, range.anchor.1);
            cell.cell_meta_index = Some(DYNAMIC_ARRAY_CM_INDEX);
            has_dynamic_array = true;
        }
    }

    if has_dynamic_array {
        emit_dynamic_array_metadata(doc)?;
    }

    Ok(())
}

const DYNAMIC_ARRAY_CM_INDEX: u32 = 1;

const SPREADSHEETML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const XDA_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray";
const XLRD_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2017/richdata";
const XLDAPR_EXT_URI: &str = "{bdbb8cdc-fa1e-496e-a857-3c3f30c029c3}";

fn metadata_namespace(prefix: &str, uri: &str) -> ooxmlsdk::common::XmlNamespace {
    let mut raw = Vec::with_capacity(prefix.len() + 1 + uri.len());
    raw.extend_from_slice(prefix.as_bytes());
    raw.push(0);
    raw.extend_from_slice(uri.as_bytes());
    ooxmlsdk::common::XmlNamespace::Raw(raw.into_boxed_slice())
}

fn emit_dynamic_array_metadata(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<()> {
    let wb_part = doc.workbook_part()?;
    if wb_part.cell_metadata_part(doc).is_some() {
        return Ok(());
    }
    let metadata = build_xldapr_metadata();
    let part: ooxmlsdk::parts::cell_metadata_part::CellMetadataPart =
        wb_part.add_new_part_auto_id(doc)?;
    part.set_root_element(doc, metadata)?;
    Ok(())
}

fn build_xldapr_metadata() -> x::Metadata {
    let yes = || Some(BooleanValue::from_bool(true));
    let metadata_type = x::MetadataType {
        name: "XLDAPR".to_string(),
        min_supported_version: 120000,
        copy: yes(),
        paste_all: yes(),
        paste_values: yes(),
        merge: yes(),
        split_first: yes(),
        row_column_shift: yes(),
        clear_formats: yes(),
        clear_comments: yes(),
        assign: yes(),
        coerce: yes(),
        cell_meta: yes(),
        ..Default::default()
    };
    let dynamic_array_props =
        b"<xda:dynamicArrayProperties fDynamic=\"1\" fCollapsed=\"0\"/>".to_vec();
    let ext = x::Extension {
        xmlns: Vec::new(),
        uri: XLDAPR_EXT_URI.to_string(),
        xml_children: vec![dynamic_array_props.into_boxed_slice()],
    };
    let future_block = x::FutureMetadataBlock {
        extension_list: Some(x::ExtensionList {
            xmlns: Vec::new(),
            extension: vec![ext],
        }),
    };
    x::Metadata {
        xmlns: vec![
            metadata_namespace("", SPREADSHEETML_NS),
            metadata_namespace("xda", XDA_NS),
            metadata_namespace("xlrd", XLRD_NS),
        ],
        xml_header: ooxmlsdk::common::XmlHeaderType::Standalone,
        metadata_types: Some(x::MetadataTypes {
            count: Some(1),
            metadata_type: vec![metadata_type],
        }),
        future_metadata: vec![x::FutureMetadata {
            name: "XLDAPR".to_string(),
            count: Some(1),
            future_metadata_block: vec![future_block],
            extension_list: None,
        }],
        cell_metadata: Some(x::CellMetadata {
            count: Some(1),
            metadata_block: vec![x::MetadataBlock {
                metadata_record: vec![x::MetadataRecord {
                    type_index: 1,
                    val: 0,
                }],
            }],
        }),
        ..Default::default()
    }
}

fn write_spilled_cells(
    ws: &mut x::Worksheet,
    sheet_index: u32,
    array_ranges: &[ArrayRange],
    engine: &WorkbookEngine<'_>,
) {
    for range in array_ranges {
        for r in range.start.0..=range.end.0 {
            for c in range.start.1..=range.end.1 {
                if (r, c) == range.anchor {
                    continue;
                }
                let Ok(value) = engine.cell_value(sheet_index, r as i32, c as i32) else {
                    continue;
                };
                let cell = ensure_cell(ws, r, c);
                set_cached_formula_value(cell, &value);
            }
        }
    }
}

fn ensure_cell(ws: &mut x::Worksheet, r: u32, c: u32) -> &mut x::Cell {
    let row_pos = match ws
        .sheet_data
        .row
        .iter()
        .position(|row| row.row_index == Some(r))
    {
        Some(pos) => pos,
        None => {
            let insert_at = ws
                .sheet_data
                .row
                .iter()
                .position(|row| row.row_index.unwrap_or(0) > r)
                .unwrap_or(ws.sheet_data.row.len());
            let mut row = x::Row::default();
            row.row_index = Some(r);
            ws.sheet_data.row.insert(insert_at, row);
            insert_at
        }
    };
    let row = &mut ws.sheet_data.row[row_pos];
    let reference = format!("{}{}", xlcore_io::col_label(c), r);
    let cell_pos = match row.cell.iter().position(|cell| {
        cell.cell_reference
            .as_deref()
            .and_then(xlcore_io::parse_a1)
            .map(|(_, col)| col)
            == Some(c)
    }) {
        Some(pos) => pos,
        None => {
            let insert_at = row
                .cell
                .iter()
                .position(|cell| {
                    cell.cell_reference
                        .as_deref()
                        .and_then(xlcore_io::parse_a1)
                        .map(|(_, col)| col)
                        .unwrap_or(0)
                        > c
                })
                .unwrap_or(row.cell.len());
            let mut cell = x::Cell::default();
            cell.cell_reference = Some(reference);
            row.cell.insert(insert_at, cell);
            insert_at
        }
    };
    &mut row.cell[cell_pos]
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
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(format_number(*value)),
                ..Default::default()
            });
        }
        CellValue::Boolean(value) => {
            cell.data_type = Some(x::CellValues::Boolean);
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(if *value { "1" } else { "0" }.to_string()),
                ..Default::default()
            });
        }
        CellValue::String(value) => {
            cell.data_type = Some(x::CellValues::String);
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(value.clone()),
                ..Default::default()
            });
        }
        CellValue::Error(kind) => {
            cell.data_type = Some(x::CellValues::Error);
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(kind.clone()),
                ..Default::default()
            });
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
    let array_ranges = collect_array_ranges(ws);
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
                .and_then(|v| v.xml_content.as_deref());
            let value = cell_value(cell, raw_v, shared_strings);
            let (literal, cached_value) = if formula.is_some() {
                (None, value)
            } else {
                (value, None)
            };
            if formula.is_none() && is_spilled_target(&array_ranges, r, c) {
                continue;
            }
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

#[derive(Clone, Copy, Debug)]
struct ArrayRange {
    anchor: (u32, u32),
    start: (u32, u32),
    end: (u32, u32),
}

fn collect_array_ranges(ws: &x::Worksheet) -> Vec<ArrayRange> {
    let mut ranges = Vec::new();
    for row in &ws.sheet_data.row {
        for cell in &row.cell {
            let Some(formula) = cell.cell_formula.as_ref() else {
                continue;
            };
            if formula.formula_type != Some(x::CellFormulaValues::Array) {
                continue;
            }
            let Some(reference) = formula.reference.as_ref() else {
                continue;
            };
            let Some(r_attr) = cell.cell_reference.as_ref() else {
                continue;
            };
            let Some(anchor) = xlcore_io::parse_a1(r_attr.as_str()) else {
                continue;
            };
            let Some((start, end)) = xlcore_io::parse_range(reference.as_str()) else {
                continue;
            };
            ranges.push(ArrayRange { anchor, start, end });
        }
    }
    ranges
}

fn mark_engine_spill_anchors(
    ws: &mut x::Worksheet,
    sheet_index: u32,
    engine: &WorkbookEngine<'_>,
    array_ranges: &mut Vec<ArrayRange>,
) {
    for ((row, column), range) in engine.spill_ranges(sheet_index) {
        let anchor = (row as u32, column as u32);
        if array_ranges.iter().any(|existing| existing.anchor == anchor) {
            continue;
        }
        let Some((start, end)) = xlcore_io::parse_range(range.as_str()) else {
            continue;
        };
        let cell = ensure_cell(ws, anchor.0, anchor.1);
        let Some(formula) = cell.cell_formula.as_mut() else {
            continue;
        };
        formula.formula_type = Some(x::CellFormulaValues::Array);
        formula.reference = Some(range.clone());
        array_ranges.push(ArrayRange { anchor, start, end });
    }
}

fn is_spilled_target(ranges: &[ArrayRange], r: u32, c: u32) -> bool {
    ranges.iter().any(|range| {
        (r, c) != range.anchor
            && r >= range.start.0
            && r <= range.end.0
            && c >= range.start.1
            && c <= range.end.1
    })
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
            return Some(CellValue::Error(raw_v.unwrap_or("#ERROR!").to_string()));
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

    let mut strings = Vec::with_capacity(sst.shared_string_item.len());
    for item in &sst.shared_string_item {
        if let Some(text) = &item.text {
            strings.push(text.xml_content.as_deref().unwrap_or("").to_string());
            continue;
        }

        let mut out = String::new();
        for run in &item.run {
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
    fn reuses_resident_engine_across_recalcs() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/engine/basic-formulas.xlsx");
        let mut doc = xlcore_io::open(&fixture).unwrap();
        let mut resident: Option<ResidentEngine> = None;

        let first = recalculate_doc_with_resident(&mut doc, &mut resident).unwrap();
        assert_eq!(
            first.cell("Sheet1", "C1").map(|cell| &cell.value),
            Some(&CellValue::Number(30.0))
        );
        assert!(resident.is_some());

        resident
            .as_mut()
            .unwrap()
            .set_cell_value(0, 1, 1, &xlcore_types::ApiCellValue::Number(100.0))
            .unwrap();

        let second = recalculate_doc_with_resident(&mut doc, &mut resident).unwrap();
        assert_eq!(
            second.cell("Sheet1", "C1").map(|cell| &cell.value),
            Some(&CellValue::Number(120.0)),
            "resident engine must be reused (not rebuilt from DOM) so the engine-only A1 edit persists"
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

    #[test]
    fn round_trips_dynamic_array_spill() {
        for fixture_name in ["transpose.xlsx", "transpose_precached.xlsx"] {
            let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/spill")
                .join(fixture_name);
            let out = temp_xlsx_path("xlcore-bridge-spill");
            let _ = std::fs::remove_file(&out);

            let wb = recalculate_and_save(&fixture, &out)
                .with_context(|| format!("recalculate {fixture_name}"))
                .unwrap();
            assert_number(&wb, "A4", 1.0);
            assert_number(&wb, "A8", 50.0);
            assert!(
                wb.cell("Sheet1", "A4").unwrap().fallback.is_none(),
                "{fixture_name}: anchor must not be #SPILL!"
            );

            let saved = recalculate(&out)
                .with_context(|| format!("reopen {fixture_name}"))
                .unwrap();
            assert_eq!(
                saved
                    .cell("Sheet1", "A4")
                    .and_then(|cell| cell.cached_value.as_ref()),
                Some(&CellValue::Number(1.0)),
                "{fixture_name}: anchor cached value"
            );
            assert!(
                saved.cell("Sheet1", "A4").unwrap().fallback.is_none(),
                "{fixture_name}: reopened anchor must not be #SPILL!"
            );

            let doc = xlcore_io::open(&out).unwrap();
            let xml = unzip_sheet(&out);
            for (cell_ref, expected) in [
                ("B4", "4"),
                ("A5", "2"),
                ("B5", "5"),
                ("A6", "3"),
                ("B6", "6"),
            ] {
                assert!(
                    xml.contains(&format!("<c r=\"{cell_ref}\"><v>{expected}</v></c>")),
                    "{fixture_name}: expected spilled {cell_ref}={expected} in {xml}"
                );
            }
            assert!(
                xml.contains("<f t=\"array\" ref=\"A4:B6\">"),
                "{fixture_name}: anchor array formula must survive"
            );
            assert!(
                !xml.contains("t=\"e\""),
                "{fixture_name}: no error cells expected, got {xml}"
            );
            drop(doc);
            let _ = std::fs::remove_file(out);
        }
    }

    fn unzip_sheet(path: &Path) -> String {
        let output = std::process::Command::new("unzip")
            .arg("-p")
            .arg(path)
            .arg("xl/worksheets/sheet1.xml")
            .output()
            .expect("unzip sheet");
        String::from_utf8(output.stdout).expect("utf-8 sheet xml")
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
