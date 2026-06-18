use ooxmlsdk::parts::theme_part::ThemePart;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::sdk::{SdkPart, SpreadsheetDocumentType};
use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_io::SpreadsheetDocument;
use xlcore_types::{ApiCellValue as CellValue, CellInfo, ClearMode};

use crate::errors::sdk_err_to_api;
use crate::ooxml_header;
use crate::{ApiError, ApiErrorCode, Result};

pub(crate) fn empty_worksheet() -> x::Worksheet {
    x::Worksheet {
        xmlns: ooxml_header::spreadsheetml_default(),
        xml_header: ooxml_header::STANDALONE,
        sheet_data: Box::new(x::SheetData::default()),
        ..Default::default()
    }
}

pub(crate) fn ensure_cell(ws: &mut x::Worksheet, row: u32, column: u32) -> &mut x::Cell {
    let row_pos = match ws
        .sheet_data
        .row
        .binary_search_by_key(&row, |existing| existing.row_index.unwrap_or(u32::MAX))
    {
        Ok(pos) => pos,
        Err(pos) => {
            ws.sheet_data.row.insert(
                pos,
                x::Row {
                    row_index: Some(row),
                    ..Default::default()
                },
            );
            pos
        }
    };

    let row_ref = &mut ws.sheet_data.row[row_pos];
    let cell_ref = format!("{}{}", xlcore_io::col_label(column), row);
    let cell_pos = match row_ref.cell.binary_search_by_key(&column, |existing| {
        existing
            .cell_reference
            .as_ref()
            .and_then(|r| xlcore_io::parse_a1(r.as_str()))
            .map(|(_, c)| c)
            .unwrap_or(u32::MAX)
    }) {
        Ok(pos) => pos,
        Err(pos) => {
            row_ref.cell.insert(
                pos,
                x::Cell {
                    cell_reference: Some(cell_ref),
                    ..Default::default()
                },
            );
            pos
        }
    };
    &mut row_ref.cell[cell_pos]
}

pub(crate) fn apply_clear_mode(cell: &mut x::Cell, mode: ClearMode) {
    match mode {
        ClearMode::All => {
            cell.data_type = None;
            cell.inline_string = None;
            cell.cell_value = None;
            cell.cell_formula = None;
            cell.style_index = None;
        }
        ClearMode::Values => {
            cell.data_type = None;
            cell.inline_string = None;
            cell.cell_value = None;
        }
        ClearMode::Formulas => {
            cell.cell_formula = None;
        }
        ClearMode::Styles => {
            cell.style_index = None;
        }
    }
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
            style: None,
            rich_text: None,
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
        style: None,
        rich_text: crate::rich_text::rich_text_from_cell(cell),
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

pub(crate) fn sheet_dimensions(
    doc: &mut xlcore_io::SpreadsheetDocument,
    part: &WorksheetPart,
) -> Result<(u32, u32)> {
    let ws = part.root_element(doc).map_err(sdk_err_to_api)?;
    let mut rows = 0;
    let mut cols = 0;
    for row in &ws.sheet_data.row {
        let row_idx = row.row_index.unwrap_or(0);
        rows = rows.max(row_idx);
        for cell in &row.cell {
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
        x::SheetStateValues::Visible | x::SheetStateValues::Show => None,
        x::SheetStateValues::Hidden => Some("hidden".to_string()),
        x::SheetStateValues::VeryHidden => Some("veryHidden".to_string()),
    }
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
    calc.full_calculation_on_load = Some(BooleanValue::from_bool(true));
    calc.force_full_calculation = Some(BooleanValue::from_bool(true));
    calc.calculation_completed = Some(BooleanValue::from_bool(false));
    Ok(())
}

pub(crate) fn blank_workbook() -> Result<SpreadsheetDocument> {
    let mut doc = SpreadsheetDocument::create(SpreadsheetDocumentType::Workbook);

    let wb_part = doc.add_workbook_part().map_err(sdk_err_to_api)?;

    let ws_part: WorksheetPart = wb_part
        .add_new_part_auto_id(&mut doc)
        .map_err(sdk_err_to_api)?;
    ws_part
        .set_root_element(&mut doc, empty_worksheet())
        .map_err(sdk_err_to_api)?;
    let sheet_rid = ws_part
        .relationship_id()
        .ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::Other,
                "new worksheet is missing relationship id",
            )
        })?
        .to_string();

    let theme_part: ThemePart = wb_part
        .add_new_part_auto_id(&mut doc)
        .map_err(sdk_err_to_api)?;
    theme_part
        .set_root_element(&mut doc, default_theme()?)
        .map_err(sdk_err_to_api)?;

    wb_part
        .set_root_element(&mut doc, empty_workbook(sheet_rid))
        .map_err(sdk_err_to_api)?;

    Ok(doc)
}

fn empty_workbook(sheet_rid: String) -> x::Workbook {
    x::Workbook {
        xmlns: ooxml_header::spreadsheetml_default(),
        xml_header: ooxml_header::STANDALONE,
        book_views: Some(x::BookViews {
            workbook_view: vec![x::WorkbookView {
                active_tab: Some(0),
                ..Default::default()
            }],
        }),
        sheets: Box::new(x::Sheets {
            sheet: vec![x::Sheet {
                name: "Sheet1".to_string(),
                sheet_id: 1,
                id: sheet_rid,
                ..Default::default()
            }],
        }),
        calculation_properties: Some(x::CalculationProperties {
            calculation_mode: Some(x::CalculateModeValues::Auto),
            full_calculation_on_load: Some(BooleanValue::from_bool(true)),
            force_full_calculation: Some(BooleanValue::from_bool(true)),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn default_theme() -> Result<a::Theme> {
    THEME1_XML.parse::<a::Theme>().map_err(sdk_err_to_api)
}

const THEME1_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="0F1011"/></a:dk2><a:lt2><a:srgbClr val="EAEAEA"/></a:lt2><a:accent1><a:srgbClr val="156082"/></a:accent1><a:accent2><a:srgbClr val="E97132"/></a:accent2><a:accent3><a:srgbClr val="196B24"/></a:accent3><a:accent4><a:srgbClr val="0F9ED5"/></a:accent4><a:accent5><a:srgbClr val="A02B93"/></a:accent5><a:accent6><a:srgbClr val="4EA72E"/></a:accent6><a:hlink><a:srgbClr val="467886"/></a:hlink><a:folHlink><a:srgbClr val="96607D"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Aptos Display" panose="02110004020202020204"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Aptos" panose="02110004020202020204"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:lumMod val="110000"/><a:satMod val="105000"/><a:tint val="67000"/></a:schemeClr></a:gs><a:gs pos="50000"><a:schemeClr val="phClr"><a:lumMod val="105000"/><a:satMod val="103000"/><a:tint val="73000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:lumMod val="105000"/><a:satMod val="109000"/><a:tint val="81000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:satMod val="103000"/><a:lumMod val="102000"/><a:tint val="94000"/></a:schemeClr></a:gs><a:gs pos="50000"><a:schemeClr val="phClr"><a:satMod val="110000"/><a:lumMod val="100000"/><a:shade val="100000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:lumMod val="99000"/><a:satMod val="120000"/><a:shade val="78000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill></a:fillStyleLst><a:lnStyleLst><a:ln w="12700" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/><a:miter lim="800000"/></a:ln><a:ln w="19050" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/><a:miter lim="800000"/></a:ln><a:ln w="25400" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/><a:miter lim="800000"/></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="57150" dist="19050" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="63000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"><a:tint val="95000"/><a:satMod val="170000"/></a:schemeClr></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="93000"/><a:satMod val="150000"/><a:shade val="98000"/><a:lumMod val="102000"/></a:schemeClr></a:gs><a:gs pos="50000"><a:schemeClr val="phClr"><a:tint val="98000"/><a:satMod val="130000"/><a:shade val="90000"/><a:lumMod val="103000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="63000"/><a:satMod val="120000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>"#;
