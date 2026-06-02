mod annotations;
mod chart_colors;
mod charts;
mod charts_ex;
mod charts_helpers;
mod charts_legacy;
mod columnar;
mod fmt_scheme;
mod font_flat;
mod pivot_engine;
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

pub use columnar::compactify;
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
            (styles::empty_styles(), Vec::new(), Vec::new())
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
    let workbook_sheets = workbook.sheets.sheet.clone();

    let active_sheet_index = workbook
        .book_views
        .as_ref()
        .and_then(|bv| bv.workbook_view.first())
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
    let mut selected_original_sheet_index: Option<u32> = None;
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
        let (pivots, pivot_cells) = pivots::extract(doc, &ws_part);
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
        merge_pivot_cells(&mut sheet, pivot_cells);
        sheet.hyperlinks = hyperlinks;
        sheet.comments = comments;
        sheet.sparkline_groups = sparkline_groups;
        if options.sheet_index.is_some() || options.sheet_name.is_some() {
            selected_original_sheet_index = Some(idx as u32);
        }
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

    let mut defined_names_vec: Vec<DefinedName> = workbook
        .defined_names
        .as_ref()
        .map(|dn| {
            dn.defined_name
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
    normalize_defined_names_for_single_sheet(&mut defined_names_vec, selected_original_sheet_index);

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

    let defined_names =
        defined_name_lookup(&defined_names_vec, selected_original_sheet_index.is_some());
    refs::resolve_chart_refs(&mut layout, &defined_names);
    refs::resolve_sparkline_refs(&mut layout);

    columnar::compactify(&mut layout);
    Ok(layout)
}

fn merge_pivot_cells(sheet: &mut Sheet, cells: Vec<Cell>) {
    if cells.is_empty() {
        return;
    }
    let mut by_row: HashMap<u32, usize> = HashMap::new();
    for (i, row) in sheet.rows.iter().enumerate() {
        by_row.insert(row.index, i);
    }
    for cell in cells {
        sheet.max_row = sheet.max_row.max(cell.r);
        sheet.max_col = sheet.max_col.max(cell.c);
        match by_row.get(&cell.r) {
            Some(&i) => sheet.rows[i].cells.push(cell),
            None => {
                by_row.insert(cell.r, sheet.rows.len());
                sheet.rows.push(Row {
                    index: cell.r,
                    height_px: None,
                    cells: vec![cell],
                    style_index: None,
                    hidden: false,
                    outline_level: 0,
                });
            }
        }
    }
}

fn defined_name_lookup(
    defined_names: &[DefinedName],
    single_sheet: bool,
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for name in defined_names {
        if single_sheet && name.local_sheet_id == Some(0) {
            out.insert(name.name.clone(), name.formula.clone());
        } else {
            out.entry(name.name.clone())
                .or_insert_with(|| name.formula.clone());
        }
    }
    out
}

fn normalize_defined_names_for_single_sheet(
    defined_names: &mut Vec<DefinedName>,
    selected_original_sheet_index: Option<u32>,
) {
    let Some(selected_idx) = selected_original_sheet_index else {
        return;
    };
    *defined_names = std::mem::take(defined_names)
        .into_iter()
        .filter_map(|mut name| match name.local_sheet_id {
            Some(local_idx) if local_idx == selected_idx => {
                name.local_sheet_id = Some(0);
                Some(name)
            }
            Some(_) => None,
            None => Some(name),
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defined_name(name: &str, local_sheet_id: Option<u32>) -> DefinedName {
        DefinedName {
            name: name.to_string(),
            formula: "A1".to_string(),
            local_sheet_id,
        }
    }

    #[test]
    fn single_sheet_defined_names_are_filtered_and_remapped() {
        let mut names = vec![
            defined_name("global", None),
            defined_name("selected_local", Some(9)),
            defined_name("other_local", Some(3)),
        ];

        normalize_defined_names_for_single_sheet(&mut names, Some(9));

        assert_eq!(names.len(), 2);
        assert_eq!(names[0].name, "global");
        assert_eq!(names[0].local_sheet_id, None);
        assert_eq!(names[1].name, "selected_local");
        assert_eq!(names[1].local_sheet_id, Some(0));
    }

    #[test]
    fn full_workbook_defined_names_keep_original_scope() {
        let mut names = vec![defined_name("local", Some(2))];
        normalize_defined_names_for_single_sheet(&mut names, None);
        assert_eq!(names[0].local_sheet_id, Some(2));
    }

    #[test]
    fn single_sheet_defined_name_lookup_prefers_selected_local_scope() {
        let names = vec![
            DefinedName {
                name: "SeriesValues".to_string(),
                formula: "Sheet1!A1:A2".to_string(),
                local_sheet_id: None,
            },
            DefinedName {
                name: "SeriesValues".to_string(),
                formula: "Sheet2!A1:A2".to_string(),
                local_sheet_id: Some(0),
            },
        ];

        let lookup = defined_name_lookup(&names, true);
        assert_eq!(lookup["SeriesValues"], "Sheet2!A1:A2");
    }
}
