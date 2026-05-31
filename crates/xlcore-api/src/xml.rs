use std::io::{Cursor, Write};

use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiCellValue as CellValue, CellInfo};

use crate::errors::{sdk_err_to_api, zip_err};
use crate::Result;

pub(crate) fn empty_worksheet() -> x::Worksheet {
    x::Worksheet {
        x_sheet_data: Box::new(x::SheetData::default()),
        ..Default::default()
    }
}

pub(crate) fn ensure_cell(ws: &mut x::Worksheet, row: u32, column: u32) -> &mut x::Cell {
    let row_pos = match ws
        .x_sheet_data
        .x_row
        .binary_search_by_key(&row, |existing| existing.row_index.unwrap_or(u32::MAX))
    {
        Ok(pos) => pos,
        Err(pos) => {
            ws.x_sheet_data.x_row.insert(
                pos,
                x::Row {
                    row_index: Some(row),
                    ..Default::default()
                },
            );
            pos
        }
    };

    let row_ref = &mut ws.x_sheet_data.x_row[row_pos];
    let cell_ref = format!("{}{}", xlcore_io::col_label(column), row);
    let cell_pos = match row_ref.x_c.binary_search_by_key(&column, |existing| {
        existing
            .cell_reference
            .as_ref()
            .and_then(|r| xlcore_io::parse_a1(r.as_str()))
            .map(|(_, c)| c)
            .unwrap_or(u32::MAX)
    }) {
        Ok(pos) => pos,
        Err(pos) => {
            row_ref.x_c.insert(
                pos,
                x::Cell {
                    cell_reference: Some(cell_ref),
                    ..Default::default()
                },
            );
            pos
        }
    };
    &mut row_ref.x_c[cell_pos]
}

pub(crate) fn set_cell_value(cell: &mut x::Cell, value: &CellValue) {
    cell.cell_formula = None;
    cell.inline_string = None;
    match value {
        CellValue::Blank => {
            cell.data_type = None;
            cell.cell_value = None;
        }
        CellValue::String(value) => {
            cell.data_type = Some(x::CellValues::InlineString);
            cell.cell_value = None;
            cell.inline_string = Some(Box::new(x::InlineString {
                text: Some(x::Text {
                    xml_content: Some(value.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }));
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
        CellValue::Error(value) => {
            cell.data_type = Some(x::CellValues::Error);
            cell.cell_value = Some(x::CellValue {
                xml_content: Some(value.clone()),
                ..Default::default()
            });
        }
    }
}

pub(crate) fn cell_info_from_cell(
    sheet: &str,
    row: u32,
    column: u32,
    cell: Option<&x::Cell>,
    shared_strings: &[String],
) -> CellInfo {
    let reference = format!("{}{}", xlcore_io::col_label(column), row);
    let Some(cell) = cell else {
        return CellInfo {
            sheet: sheet.to_string(),
            reference,
            row,
            column,
            value: CellValue::Blank,
            formula: None,
            style_index: None,
        };
    };
    let raw_v = cell
        .cell_value
        .as_ref()
        .and_then(|value| value.xml_content.as_deref());
    CellInfo {
        sheet: sheet.to_string(),
        reference,
        row,
        column,
        value: read_cell_value(cell, raw_v, shared_strings),
        formula: cell
            .cell_formula
            .as_ref()
            .and_then(|formula| formula.xml_content.as_deref().map(str::to_string)),
        style_index: cell.style_index,
    }
}

pub(crate) fn read_cell_value(
    cell: &x::Cell,
    raw_v: Option<&str>,
    shared_strings: &[String],
) -> CellValue {
    match cell.data_type {
        Some(x::CellValues::SharedString) => raw_v
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|idx| shared_strings.get(idx).cloned())
            .map(CellValue::String)
            .unwrap_or_else(|| CellValue::String(String::new())),
        Some(x::CellValues::InlineString) => CellValue::String(inline_string_text(cell)),
        Some(x::CellValues::Boolean) => {
            CellValue::Boolean(matches!(raw_v, Some("1" | "true" | "TRUE")))
        }
        Some(x::CellValues::Error) => CellValue::Error(raw_v.unwrap_or("#ERROR!").to_string()),
        Some(x::CellValues::String) => CellValue::String(raw_v.unwrap_or("").to_string()),
        _ => raw_v
            .and_then(|value| value.parse::<f64>().ok())
            .map(CellValue::Number)
            .unwrap_or(CellValue::Blank),
    }
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

pub(crate) fn load_shared_strings(doc: &mut xlcore_io::SpreadsheetDocument) -> Vec<String> {
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

pub(crate) fn sheet_dimensions(
    doc: &mut xlcore_io::SpreadsheetDocument,
    part: &WorksheetPart,
) -> Result<(u32, u32)> {
    let ws = part.root_element(doc).map_err(sdk_err_to_api)?;
    let mut rows = 0;
    let mut cols = 0;
    for row in &ws.x_sheet_data.x_row {
        let row_idx = row.row_index.unwrap_or(0);
        rows = rows.max(row_idx);
        for cell in &row.x_c {
            if let Some((_, c)) = cell
                .cell_reference
                .as_ref()
                .and_then(|reference| xlcore_io::parse_a1(reference.as_str()))
            {
                cols = cols.max(c);
            }
        }
    }
    Ok((rows, cols))
}

pub(crate) fn sheet_state_name(state: &x::SheetStateValues) -> Option<String> {
    match state {
        x::SheetStateValues::Visible => None,
        x::SheetStateValues::Hidden => Some("hidden".to_string()),
        x::SheetStateValues::VeryHidden => Some("veryHidden".to_string()),
    }
}

pub(crate) fn normalize_formula(formula: &str) -> String {
    formula
        .trim()
        .strip_prefix('=')
        .unwrap_or(formula.trim())
        .to_string()
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

pub(crate) fn mark_formulas_stale(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<()> {
    let wb_part = doc.workbook_part().map_err(sdk_err_to_api)?.clone();
    let workbook = wb_part.root_element_mut(doc).map_err(sdk_err_to_api)?;
    let calc = workbook
        .calculation_properties
        .get_or_insert_with(Default::default);
    calc.full_calculation_on_load = Some(true);
    calc.force_full_calculation = Some(true);
    calc.calculation_completed = Some(false);
    Ok(())
}

pub(crate) fn blank_workbook_bytes() -> Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        zip.start_file("[Content_Types].xml", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("_rels", options).map_err(zip_err)?;
        zip.start_file("_rels/.rels", options).map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("xl", options).map_err(zip_err)?;
        zip.start_file("xl/workbook.xml", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <bookViews><workbookView activeTab="0"/></bookViews>
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
  <calcPr calcMode="auto" fullCalcOnLoad="1" forceFullCalc="1"/>
</workbook>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("xl/_rels", options).map_err(zip_err)?;
        zip.start_file("xl/_rels/workbook.xml.rels", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#,
        )
        .map_err(zip_err)?;

        zip.add_directory("xl/worksheets", options)
            .map_err(zip_err)?;
        zip.start_file("xl/worksheets/sheet1.xml", options)
            .map_err(zip_err)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
</worksheet>"#,
        )
        .map_err(zip_err)?;

        zip.finish().map_err(zip_err)?;
    }
    Ok(buffer.into_inner())
}
