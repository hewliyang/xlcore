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
mod charts;
mod columnar;
mod pivots;
mod schema;
mod sheet;
mod styles;
mod tables;
mod theme;

pub use schema::*;

use anyhow::Result;
use ooxmlsdk::sdk::SdkPart;
use std::collections::HashMap;
use std::path::Path;

/// One-shot: read an xlsx file, extract its `WorkbookLayout`.
pub fn extract<P: AsRef<Path>>(path: P) -> Result<WorkbookLayout> {
    let mut doc = xlcore_io::open(path)?;
    extract_doc(&mut doc)
}

/// Extract from an already-open document.
pub fn extract_doc(doc: &mut xlcore_io::SpreadsheetDocument) -> Result<WorkbookLayout> {
    let shared_strings = preload_shared_strings(doc);

    let (styles, dxfs) = {
        let wb_part = doc.workbook_part()?;
        if let Some(sp) = wb_part.workbook_styles_part(doc) {
            let sp = sp.clone();
            let s = sp.root_element(doc)?;
            let dxfs = styles::extract_dxfs(s);
            (styles::extract(s), dxfs)
        } else {
            (Styles::default(), Vec::new())
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

    let mut sheets = Vec::with_capacity(workbook_sheets.len());
    for (idx, wb_sheet) in workbook_sheets.iter().enumerate() {
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
        sheet.drawings = drawings;
        sheet.tables = tables;
        sheet.pivots = pivots;
        sheet.hyperlinks = hyperlinks;
        sheet.comments = comments;
        sheets.push(sheet);
    }

    let mut layout = WorkbookLayout {
        sheets,
        styles,
        shared_strings: shared_strings.0,
        shared_string_runs: shared_strings.1,
        dxfs,
        theme,
        active_sheet_index,
    };
    resolve_chart_refs(&mut layout);
    // Final pass: collapse `Sheet.rows: Vec<Row>` (the ergonomic shape
    // every other extractor pass uses) into the columnar typed-array
    // blobs that actually ship in the JSON. After this point the
    // `rows` field is empty and must not be read.
    columnar::compactify(&mut layout);
    Ok(layout)
}

/// After all sheets are extracted, resolve any `Sheet!$A$1:$B$2`-style
/// references in chart series/categories that didn't come with cached
/// numbers. Office writes the cache most of the time, but not always --
/// without this, fresh chartsheets render empty.
fn resolve_chart_refs(layout: &mut WorkbookLayout) {
    // Snapshot sheet name -> index so we can lookup cells without aliasing.
    let name_to_idx: std::collections::HashMap<String, usize> = layout
        .sheets
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.clone(), i))
        .collect();

    let read_string = |sheets: &[Sheet], target: &Cell, sst: &[String]| -> Option<String> {
        match target.kind.as_str() {
            "s" => target
                .value
                .as_ref()
                .and_then(|v| v.parse::<usize>().ok())
                .and_then(|idx| sst.get(idx).cloned()),
            "inline" | "str" => target.value.clone(),
            "n" | "f" => target.value.clone(),
            _ => target.value.clone(),
        }
        .or_else(|| {
            // suppress unused warning on `sheets`
            let _ = sheets;
            None
        })
    };

    let read_number = |target: &Cell| -> Option<f64> {
        target.value.as_ref().and_then(|v| v.parse::<f64>().ok())
    };

    // Helpers: resolve range -> Vec of cells in row-major order.
    let collect_cells = |sheets: &[Sheet],
                         sheet_name: &str,
                         r1: u32,
                         c1: u32,
                         r2: u32,
                         c2: u32|
     -> Vec<Option<Cell>> {
        let Some(&idx) = name_to_idx.get(sheet_name) else {
            return Vec::new();
        };
        let sheet = &sheets[idx];
        let mut out = Vec::with_capacity(((r2 - r1 + 1) * (c2 - c1 + 1)) as usize);
        for r in r1..=r2 {
            for c in c1..=c2 {
                let cell = sheet
                    .rows
                    .iter()
                    .find(|row| row.index == r)
                    .and_then(|row| row.cells.iter().find(|cc| cc.r == r && cc.c == c))
                    .cloned();
                out.push(cell);
            }
        }
        out
    };

    let snapshot_sheets = layout.sheets.clone();
    let sst = layout.shared_strings.clone();

    for sheet in layout.sheets.iter_mut() {
        for drawing in sheet.drawings.iter_mut() {
            let Some(chart) = drawing.chart.as_mut() else {
                continue;
            };

            // categories
            if chart.categories.is_empty() {
                if let Some(formula) = &chart.categories_ref {
                    if let Some((sheet_name, r1, c1, r2, c2)) = parse_chart_ref(formula) {
                        let cells = collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r2, c2);
                        chart.categories = cells
                            .into_iter()
                            .map(|cell| {
                                cell.as_ref()
                                    .and_then(|cc| read_string(&snapshot_sheets, cc, &sst))
                                    .unwrap_or_default()
                            })
                            .collect();
                    }
                }
            }

            // series name + values
            for ser in chart.series.iter_mut() {
                if ser.name.is_empty() {
                    if let Some(formula) = &ser.name_ref {
                        if let Some((sheet_name, r1, c1, _, _)) = parse_chart_ref(formula) {
                            // Series name is a single-cell ref; just read (r1,c1).
                            let cells =
                                collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r1, c1);
                            if let Some(Some(cell)) = cells.first() {
                                if let Some(s) = read_string(&snapshot_sheets, cell, &sst) {
                                    ser.name = s;
                                }
                            }
                        }
                    }
                }
                if ser.values.is_empty() {
                    if let Some(formula) = &ser.values_ref {
                        if let Some((sheet_name, r1, c1, r2, c2)) = parse_chart_ref(formula) {
                            let cells =
                                collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r2, c2);
                            ser.values = cells
                                .into_iter()
                                .map(|cell| cell.as_ref().and_then(read_number).unwrap_or(0.0))
                                .collect();
                        }
                    }
                }
                if ser.x_values.is_empty() {
                    if let Some(formula) = &ser.x_values_ref {
                        if let Some((sheet_name, r1, c1, r2, c2)) = parse_chart_ref(formula) {
                            let cells =
                                collect_cells(&snapshot_sheets, &sheet_name, r1, c1, r2, c2);
                            ser.x_values = cells
                                .into_iter()
                                .map(|cell| cell.as_ref().and_then(read_number).unwrap_or(0.0))
                                .collect();
                        }
                    }
                }
            }
        }
    }
}

/// Parse a chart-style reference like `Sheet1!$B$2:$E$2` or
/// `'My Sheet'!$A$1` into (sheet, r1, c1, r2, c2).
fn parse_chart_ref(formula: &str) -> Option<(String, u32, u32, u32, u32)> {
    let (sheet_part, range_part) = formula.split_once('!')?;
    // Strip surrounding quotes if present.
    let sheet = sheet_part.trim_matches('\'').to_string();
    let cleaned: String = range_part.chars().filter(|c| *c != '$').collect();
    let (a, b) = cleaned
        .split_once(':')
        .unwrap_or((cleaned.as_str(), cleaned.as_str()));
    let (r1, c1) = xlcore_io::parse_a1(a)?;
    let (r2, c2) = xlcore_io::parse_a1(b)?;
    let (r1, r2) = (r1.min(r2), r1.max(r2));
    let (c1, c2) = (c1.min(c2), c1.max(c2));
    Some((sheet, r1, c1, r2, c2))
}

/// Returns `(plain_text_per_si, runs_per_si)`, the two parallel arrays
/// indexed by SharedStringItem position. `runs_per_si[i]` is empty when
/// the SST entry is plain text (single `<t>`); otherwise it carries the
/// rich-text runs so the renderer can preserve per-run bold/italic/color.
fn preload_shared_strings(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> (Vec<String>, Vec<Vec<TextRun>>) {
    let wb_part = match doc.workbook_part() {
        Ok(p) => p,
        Err(_) => return (Vec::new(), Vec::new()),
    };
    let sst_part = match wb_part.shared_string_table_part(doc) {
        Some(p) => p.clone(),
        None => return (Vec::new(), Vec::new()),
    };
    let sst = match sst_part.root_element(doc) {
        Ok(s) => s,
        Err(_) => return (Vec::new(), Vec::new()),
    };
    let mut texts = Vec::with_capacity(sst.x_si.len());
    let mut runs = Vec::with_capacity(sst.x_si.len());
    for item in &sst.x_si {
        // Plain `<t>` form -> no runs.
        if let Some(t) = &item.text {
            texts.push(t.xml_content.as_deref().unwrap_or("").to_string());
            runs.push(Vec::new());
            continue;
        }
        // `<r>` form -> build flat string + parallel TextRun list.
        let mut s = String::new();
        let mut rs: Vec<TextRun> = Vec::with_capacity(item.x_r.len());
        for r in &item.x_r {
            let txt = r.text.xml_content.as_deref().unwrap_or("").to_string();
            s.push_str(&txt);
            rs.push(text_run_from(r, txt));
        }
        // Collapse trivially-styled run lists (e.g. one run with no rPr) so
        // we don't bloat the JSON for plain SST entries.
        if rs.iter().all(is_unstyled_run) {
            rs.clear();
        }
        texts.push(s);
        runs.push(rs);
    }
    (texts, runs)
}

use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as xspread;

/// Convert one OOXML `<r>` element into our `TextRun`. Properties that
/// aren't set leave the field as `None`/`false` so the renderer can
/// inherit from the cell's own font.
pub(crate) fn text_run_from(r: &xspread::Run, text: String) -> TextRun {
    let mut tr = TextRun {
        text,
        ..Default::default()
    };
    let Some(rpr) = &r.run_properties else {
        return tr;
    };
    // CT_BooleanProperty: element present + no `val` attr defaults to true,
    // but `val="0"` explicitly unsets the property. Same pattern as Font.
    if let Some(b) = rpr.x_b.first() {
        tr.bold = b.val.unwrap_or(true);
    }
    if let Some(i) = rpr.x_i.first() {
        tr.italic = i.val.unwrap_or(true);
    }
    if !rpr.x_u.is_empty() {
        tr.underline = true;
    }
    if let Some(s) = rpr.x_strike.first() {
        tr.strike = s.val.unwrap_or(true);
    }
    if let Some(sz) = rpr.x_sz.first() {
        tr.size = Some(sz.val as f32);
    }
    if let Some(rf) = rpr.x_r_font.first() {
        tr.font_name = Some(rf.val.as_str().to_string());
    }
    if let Some(c) = rpr.x_color.first() {
        let any = c.rgb.is_some() || c.theme.is_some() || c.indexed.is_some();
        if any {
            tr.color = Some(Color {
                rgb: c.rgb.as_ref().map(|s| s.as_str().to_string()),
                theme: c.theme,
                indexed: c.indexed,
                tint: c.tint,
            });
        }
    }
    tr
}

fn is_unstyled_run(r: &TextRun) -> bool {
    !r.bold
        && !r.italic
        && !r.underline
        && !r.strike
        && r.size.is_none()
        && r.font_name.is_none()
        && r.color.is_none()
}
