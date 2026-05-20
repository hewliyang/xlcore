mod annotations;
mod chart_colors;
mod charts;
mod charts_ex;
mod charts_helpers;
mod charts_legacy;
mod columnar;
mod fmt_scheme;
mod pivots;
mod refs;
mod schema;
mod shapes;
mod shapes_connector;
mod shapes_fill;
mod shapes_style;
mod shapes_text;
mod shared_strings;
mod sheet;
mod sparklines;
mod styles;
mod tables;
mod theme;

pub use schema::*;
pub(crate) use shared_strings::{
    font_scheme_variant, text_run_from, underline_variant, vert_align_variant,
};

use anyhow::Result;
use ooxmlsdk::sdk::SdkPart;
use std::collections::HashMap;
use std::path::Path;

pub fn extract<P: AsRef<Path>>(path: P) -> Result<WorkbookLayout> {
    let mut doc = xlcore_io::open(path)?;
    extract_doc(&mut doc)
}

#[derive(Clone, Debug, Default)]
pub struct ExtractOptions {
    pub sheet_index: Option<usize>,
    pub sheet_name: Option<String>,
}

pub fn extract_doc(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<WorkbookLayout> {
    extract_doc_with_options(doc, &ExtractOptions::default())
}

pub fn extract_doc_with_options(
    doc: &mut xlcore_io::SpreadsheetDocument,
    options: &ExtractOptions,
) -> Result<WorkbookLayout> {
    let shared_strings = shared_strings::preload(doc);

    let (styles, dxfs, table_styles) = {
        let wb_part = doc.workbook_part()?;
        if let Some(sp) = wb_part.workbook_styles_part(doc) {
            let sp = sp.clone();
            let s = sp.root_element(doc)?;
            let dxfs = styles::extract_dxfs(s);
            let table_styles = styles::extract_table_styles(s);
            (styles::extract(s), dxfs, table_styles)
        } else {
            (Styles::default(), Vec::new(), Vec::new())
        }
    };

    let theme = {
        let wb_part = doc.workbook_part()?;
        if let Some(tp) = wb_part.theme_part(doc) {
            let tp = tp.clone();

            tp.root_element(doc).ok().map(theme::extract)
        } else {
            None
        }
    };

    let wb_part = doc.workbook_part()?;
    let workbook = wb_part.root_element(doc)?.clone();
    let workbook_sheets = workbook.sheets.x_sheet.clone();

    let active_sheet_index = workbook
        .book_views
        .as_ref()
        .and_then(|bv| bv.x_workbook_view.first())
        .and_then(|wv| wv.active_tab);

    let wb_part = doc.workbook_part()?;
    let ws_parts: Vec<_> = wb_part.worksheet_parts(doc).collect();
    let ws_parts_by_rel_id: HashMap<String, _> = ws_parts
        .iter()
        .filter_map(|p| p.relationship_id().map(|id| (id.to_string(), p.clone())))
        .collect();

    let sheet_capacity = if options.sheet_index.is_some() || options.sheet_name.is_some() {
        1
    } else {
        workbook_sheets.len()
    };
    let mut sheets = Vec::with_capacity(sheet_capacity);
    for (idx, wb_sheet) in workbook_sheets.iter().enumerate() {
        if let Some(wanted_idx) = options.sheet_index {
            if idx != wanted_idx {
                continue;
            }
        }
        if let Some(wanted_name) = options.sheet_name.as_deref() {
            if wb_sheet.name.as_str() != wanted_name {
                continue;
            }
        }
        let ws_part = ws_parts_by_rel_id
            .get(wb_sheet.id.as_str())
            .or_else(|| ws_parts.get(idx))
            .cloned();
        let Some(ws_part) = ws_part else {
            continue;
        };

        let drawings = charts::extract(doc, &ws_part, theme.as_ref());
        let tables = tables::extract(doc, &ws_part);
        let pivots = pivots::extract(doc, &ws_part);
        let comments = annotations::extract_comments(doc, &ws_part);
        let ws_for_sparks = ws_part.root_element(doc)?.clone();
        let sparkline_groups = sparklines::extract(&ws_for_sparks);

        let ws_clone = ws_part.root_element(doc)?.clone();
        let hyperlinks = annotations::extract_hyperlinks(doc, &ws_part, &ws_clone);
        let ws = ws_part.root_element(doc)?;
        let name = wb_sheet.name.as_str().to_string();
        let mut sheet = sheet::extract(ws, idx, name, &shared_strings.0, &styles);

        sheet.state = wb_sheet.state.as_ref().and_then(|s| {
            let d = format!("{s:?}").to_ascii_lowercase();
            if d.contains("veryhidden") {
                Some("veryHidden".to_string())
            } else if d.contains("hidden") {
                Some("hidden".to_string())
            } else {
                None
            }
        });
        sheet.drawings = drawings;
        sheet.tables = tables;
        sheet.pivots = pivots;
        sheet.hyperlinks = hyperlinks;
        sheet.comments = comments;
        sheet.sparkline_groups = sparkline_groups;
        sheets.push(sheet);
    }

    if sheets.is_empty() {
        if let Some(name) = options.sheet_name.as_deref() {
            anyhow::bail!("sheet not found: {name}");
        }
        if let Some(index) = options.sheet_index {
            anyhow::bail!("sheet index out of range: {index}");
        }
    }

    let defined_names_vec: Vec<DefinedName> = workbook
        .defined_names
        .as_ref()
        .map(|dn| {
            dn.x_defined_name
                .iter()
                .filter_map(|d| {
                    let formula = d.xml_content.as_ref()?.clone();
                    Some(DefinedName {
                        name: d.name.as_str().to_string(),
                        formula,
                        local_sheet_id: d.local_sheet_id,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut layout = WorkbookLayout {
        sheets,
        styles,
        shared_strings: shared_strings.0,
        shared_string_runs: shared_strings.1,
        dxfs,
        table_styles,
        theme,
        defined_names: defined_names_vec.clone(),
        active_sheet_index: if options.sheet_index.is_some() || options.sheet_name.is_some() {
            Some(0)
        } else {
            active_sheet_index
        },
    };

    let defined_names: std::collections::HashMap<String, String> = defined_names_vec
        .iter()
        .map(|d| (d.name.clone(), d.formula.clone()))
        .collect();
    refs::resolve_chart_refs(&mut layout, &defined_names);
    refs::resolve_sparkline_refs(&mut layout);

    columnar::compactify(&mut layout);
    Ok(layout)
}
