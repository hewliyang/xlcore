//! xlcore-export: xlsx → `WorkbookLayout` (structured JSON for the TS canvas renderer).
//!
//! v0 covers: shared strings, default + custom column widths, default + custom row
//! heights, fonts (name/size/bold/italic/color), pattern-solid fills, per-side borders,
//! basic alignment, merged cells, freeze panes, number formats (built-ins + custom),
//! cell values (numeric / shared string / inline string / boolean / error / formula).
//!
//! Out of scope (engine preserves XML, renderer skips): charts, drawings, CF,
//! pivot tables, slicers, theme color resolution, rich-text runs, gradient fills.
//! Those land milestone-by-milestone.
//!
//! No formulas are recomputed here — we emit the source-cached `<v>`. Recalc is
//! `xlcore-bridge`'s job (future).

mod annotations;
mod chart_colors;
mod charts;
mod charts_helpers;
mod columnar;
mod pivots;
mod refs;
mod schema;
mod shapes;
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

/// One-shot: read an xlsx file, extract its `WorkbookLayout`.
pub fn extract<P: AsRef<Path>>(path: P) -> Result<WorkbookLayout> {
    let mut doc = xlcore_io::open(path)?;
    extract_doc(&mut doc)
}

#[derive(Clone, Debug, Default)]
pub struct ExtractOptions {
    pub sheet_index: Option<usize>,
    pub sheet_name: Option<String>,
}

/// Extract from an already-open document.
pub fn extract_doc(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<WorkbookLayout> {
    extract_doc_with_options(doc, &ExtractOptions::default())
}

/// Extract from an already-open document, optionally narrowing to one sheet.
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
            // Theme XML can fail to parse (rare — corrupt drawingml); fall
            // back to the default palette rather than failing the whole
            // extract.
            tp.root_element(doc).ok().map(theme::extract)
        } else {
            None
        }
    };

    let wb_part = doc.workbook_part()?;
    let workbook = wb_part.root_element(doc)?.clone();
    let workbook_sheets = workbook.sheets.x_sheet.clone();
    // `<bookViews><workbookView activeTab="N"/></bookViews>`. Excel only
    // ever writes one `workbookView`; if there are multiple we just take
    // the first (matches what `hsx`/Office do on open).
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
            // Fallback for malformed packages or SDKs that don't expose child
            // relationship ids; preserves the old behavior rather than
            // dropping a sheet.
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
        // Hyperlinks need both the worksheet XML (for the `<hyperlinks>`
        // block) and the worksheet part (to resolve `r:id` rels). Borrow
        // the XML once, then drop before annotations::extract_comments
        // mutates `doc` again. We do hyperlinks last so the &Worksheet
        // borrow doesn't outlive the comments call above.
        let ws_clone = ws_part.root_element(doc)?.clone();
        let hyperlinks = annotations::extract_hyperlinks(doc, &ws_part, &ws_clone);
        let ws = ws_part.root_element(doc)?;
        let name = wb_sheet.name.as_str().to_string();
        let mut sheet = sheet::extract(ws, idx, name, &shared_strings.0, &styles);
        // Sheet visibility lives in workbook.xml, not the worksheet part.
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
    // Collect workbook-level `<definedName>` entries so chart refs that
    // use opaque aliases (Excel's `_xlchart.vN.X` placeholders) can be
    // dereferenced to their real `Sheet!$A$1:$B$2` ranges before the
    // resolver chases them.
    let defined_names: std::collections::HashMap<String, String> = defined_names_vec
        .iter()
        .map(|d| (d.name.clone(), d.formula.clone()))
        .collect();
    refs::resolve_chart_refs(&mut layout, &defined_names);
    refs::resolve_sparkline_refs(&mut layout);
    // Final pass: collapse `Sheet.rows: Vec<Row>` (the ergonomic shape
    // every other extractor pass uses) into the columnar typed-array
    // blobs that actually ship in the JSON. After this point the
    // `rows` field is empty and must not be read.
    columnar::compactify(&mut layout);
    Ok(layout)
}
