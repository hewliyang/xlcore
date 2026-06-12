use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::opc_relationships::TargetMode;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, HyperlinkInfo, HyperlinkPatch};

use xlcore_types::ApiCellValue as CellValue;

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_a1, qualify_ref, ranges_overlap, ResolvedRangeRef};
use crate::xml::{ensure_cell, mark_formulas_stale, set_cell_value};
use crate::{Result, Workbook};

impl Workbook {
    pub fn hyperlinks(&mut self, sheet: impl AsRef<str>) -> Result<Vec<HyperlinkInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let rels: Vec<(String, String)> = ws_part
            .hyperlink_relationships(&self.doc)
            .map(|rel| (rel.id().to_string(), rel.target().to_string()))
            .collect();
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let mut out = Vec::new();
        if let Some(block) = ws.hyperlinks.as_ref() {
            for h in &block.hyperlink {
                let Some((r1, c1, r2, c2)) = parse_range_a1(h.reference.as_str()) else {
                    continue;
                };
                let target = h.id.as_ref().and_then(|rid| {
                    rels.iter()
                        .find(|(id, _)| id == rid.as_str())
                        .map(|(_, t)| t.clone())
                });
                out.push(HyperlinkInfo {
                    sheet: sheet.clone(),
                    reference: format!(
                        "{}{}:{}{}",
                        xlcore_io::col_label(c1),
                        r1,
                        xlcore_io::col_label(c2),
                        r2,
                    ),
                    start_row: r1,
                    start_column: c1,
                    end_row: r2,
                    end_column: c2,
                    target,
                    location: h.location.as_ref().map(|s| s.as_str().to_string()),
                    tooltip: h.tooltip.as_ref().map(|s| s.as_str().to_string()),
                    display: h.display.as_ref().map(|s| s.as_str().to_string()),
                });
            }
        }
        Ok(out)
    }

    pub fn set_hyperlink(
        &mut self,
        sheet: impl AsRef<str>,
        reference: impl AsRef<str>,
        patch: HyperlinkPatch,
    ) -> Result<HyperlinkInfo> {
        let reference = qualify_ref(sheet.as_ref(), reference.as_ref())?;
        let reference = reference.as_str();
        let range_ref = self.resolve_range_ref(reference)?;
        validate_patch(&patch, reference)?;

        let new_ref = range_ref.range_reference();
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;

        let rid = if let Some(target) = patch.target.as_ref() {
            Some(ensure_hyperlink_rel(&mut self.doc, &ws_part, target)?)
        } else {
            None
        };

        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let block = ws.hyperlinks.get_or_insert_with(x::Hyperlinks::default);
        let mut orphaned_rids: Vec<String> = Vec::new();
        block.hyperlink.retain(|h| {
            let Some((r1, c1, r2, c2)) = parse_range_a1(h.reference.as_str()) else {
                return true;
            };
            let overlaps = ranges_overlap(
                range_ref.start_row,
                range_ref.start_column,
                range_ref.end_row,
                range_ref.end_column,
                r1,
                c1,
                r2,
                c2,
            );
            if overlaps {
                if let Some(rid) = h.id.as_ref() {
                    orphaned_rids.push(rid.as_str().to_string());
                }
            }
            !overlaps
        });
        block.hyperlink.push(x::Hyperlink {
            reference: new_ref.clone(),
            id: rid.clone().map(Into::into),
            location: patch.location.clone().map(Into::into),
            tooltip: patch.tooltip.clone().map(Into::into),
            display: patch.display.clone().map(Into::into),
            ..Default::default()
        });
        let still_used: std::collections::HashSet<String> = block
            .hyperlink
            .iter()
            .filter_map(|h| h.id.as_ref().map(|s| s.as_str().to_string()))
            .collect();
        let mut populated_display = false;
        if let Some(display) = patch.display.as_deref() {
            let cell = ensure_cell(ws, range_ref.start_row, range_ref.start_column);
            if cell.cell_value.is_none()
                && cell.inline_string.is_none()
                && cell.cell_formula.is_none()
            {
                set_cell_value(cell, &CellValue::String(display.to_string()));
                populated_display = true;
            }
        }
        for orphan in orphaned_rids {
            if !still_used.contains(&orphan) {
                let _ = ws_part.delete_reference_relationship(&mut self.doc, &orphan);
            }
        }
        if populated_display {
            mark_formulas_stale(&mut self.doc)?;
        }
        Ok(hyperlink_info(&range_ref, &patch))
    }

    pub fn remove_hyperlink(
        &mut self,
        sheet: impl AsRef<str>,
        reference: impl AsRef<str>,
    ) -> Result<Vec<HyperlinkInfo>> {
        let reference = qualify_ref(sheet.as_ref(), reference.as_ref())?;
        let range_ref = self.resolve_range_ref(&reference)?;
        let sheet = range_ref.sheet.clone();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let rels: Vec<(String, String)> = ws_part
            .hyperlink_relationships(&self.doc)
            .map(|rel| (rel.id().to_string(), rel.target().to_string()))
            .collect();
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(block) = ws.hyperlinks.as_mut() else {
            return Ok(Vec::new());
        };
        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(block.hyperlink.len());
        for h in block.hyperlink.drain(..) {
            let hit = match parse_range_a1(h.reference.as_str()) {
                Some((r1, c1, r2, c2)) => ranges_overlap(
                    range_ref.start_row,
                    range_ref.start_column,
                    range_ref.end_row,
                    range_ref.end_column,
                    r1,
                    c1,
                    r2,
                    c2,
                ),
                None => false,
            };
            if hit {
                let (r1, c1, r2, c2) = parse_range_a1(h.reference.as_str()).unwrap();
                let target = h.id.as_ref().and_then(|rid| {
                    rels.iter()
                        .find(|(id, _)| id == rid.as_str())
                        .map(|(_, t)| t.clone())
                });
                removed.push(HyperlinkInfo {
                    sheet: sheet.clone(),
                    reference: h.reference.as_str().to_string(),
                    start_row: r1,
                    start_column: c1,
                    end_row: r2,
                    end_column: c2,
                    target,
                    location: h.location.as_ref().map(|s| s.as_str().to_string()),
                    tooltip: h.tooltip.as_ref().map(|s| s.as_str().to_string()),
                    display: h.display.as_ref().map(|s| s.as_str().to_string()),
                });
            } else {
                kept.push(h);
            }
        }
        let still_used: std::collections::HashSet<String> = kept
            .iter()
            .filter_map(|h| h.id.as_ref().map(|s| s.as_str().to_string()))
            .collect();
        if kept.is_empty() {
            ws.hyperlinks = None;
        } else {
            block.hyperlink = kept;
        }
        for info in &removed {
            let Some(rid) = info_rid(info, &rels) else {
                continue;
            };
            if !still_used.contains(&rid) {
                let _ = ws_part.delete_reference_relationship(&mut self.doc, &rid);
            }
        }
        Ok(removed)
    }
}

fn info_rid(info: &HyperlinkInfo, rels: &[(String, String)]) -> Option<String> {
    let target = info.target.as_deref()?;
    rels.iter()
        .find(|(_, t)| t == target)
        .map(|(id, _)| id.clone())
}

fn validate_patch(patch: &HyperlinkPatch, reference: &str) -> Result<()> {
    if patch.target.is_none() && patch.location.is_none() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidHyperlink,
            "hyperlink requires at least one of target or location",
        )
        .with_ref(reference));
    }
    if let Some(t) = patch.target.as_deref() {
        if t.trim().is_empty() {
            return Err(
                ApiError::new(ApiErrorCode::InvalidHyperlink, "hyperlink target is empty")
                    .with_ref(reference),
            );
        }
    }
    Ok(())
}

fn ensure_hyperlink_rel(
    doc: &mut ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument,
    ws_part: &WorksheetPart,
    target: &str,
) -> Result<String> {
    if let Some(existing) = ws_part
        .hyperlink_relationships(doc)
        .find(|rel| rel.target() == target)
    {
        return Ok(existing.id().to_string());
    }
    let rel = ws_part
        .add_hyperlink_relationship_auto_id(doc, target.to_string(), TargetMode::External)
        .map_err(sdk_err_to_api)?;
    Ok(rel.id().to_string())
}

fn hyperlink_info(range_ref: &ResolvedRangeRef, patch: &HyperlinkPatch) -> HyperlinkInfo {
    HyperlinkInfo {
        sheet: range_ref.sheet.clone(),
        reference: range_ref.range_reference(),
        start_row: range_ref.start_row,
        start_column: range_ref.start_column,
        end_row: range_ref.end_row,
        end_column: range_ref.end_column,
        target: patch.target.clone(),
        location: patch.location.clone(),
        tooltip: patch.tooltip.clone(),
        display: patch.display.clone(),
    }
}
