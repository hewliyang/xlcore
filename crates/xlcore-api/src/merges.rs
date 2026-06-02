use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, MergeInfo};

use crate::errors::sdk_err_to_api;
use crate::refs::{
    parse_cell_address, parse_range_a1, parse_range_reference, quote_sheet_name, ranges_overlap,
    split_sheet_reference, ResolvedRangeRef,
};
use crate::{Result, Workbook};

impl Workbook {
    pub fn merges(&mut self, sheet: impl AsRef<str>) -> Result<Vec<MergeInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let mut out = Vec::new();
        if let Some(mc) = ws.merge_cells.as_ref() {
            for m in &mc.merge_cell {
                if let Some(info) = merge_info_from_ref(&sheet, m.reference.as_str()) {
                    out.push(info);
                }
            }
        }
        Ok(out)
    }

    pub fn add_merge(&mut self, reference: impl AsRef<str>) -> Result<MergeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let new_ref = range_ref.range_reference();
        let merges = ws.merge_cells.get_or_insert_with(x::MergeCells::default);
        for existing in &merges.merge_cell {
            let Some((r1, c1, r2, c2)) = parse_range_a1(existing.reference.as_str()) else {
                continue;
            };
            if ranges_overlap(
                range_ref.start_row,
                range_ref.start_column,
                range_ref.end_row,
                range_ref.end_column,
                r1,
                c1,
                r2,
                c2,
            ) {
                return Err(ApiError::new(
                    ApiErrorCode::MergeOverlap,
                    format!(
                        "merge {new_ref} overlaps existing merge {}",
                        existing.reference.as_str()
                    ),
                )
                .with_sheet(&range_ref.sheet)
                .with_ref(&new_ref));
            }
        }
        merges.merge_cell.push(x::MergeCell {
            reference: new_ref.clone(),
        });
        merges.count = Some(merges.merge_cell.len() as u32);
        Ok(merge_info(&range_ref.sheet, &range_ref))
    }

    pub fn remove_merge(&mut self, reference: impl AsRef<str>) -> Result<Option<MergeInfo>> {
        let reference = reference.as_ref();
        let (sheet, body) = match split_sheet_reference(reference)? {
            (Some(s), body) => (s, body.to_string()),
            (None, body) => (self.default_sheet_name()?, body.to_string()),
        };
        let is_range = body.contains(':');
        let target_range = if is_range {
            Some(parse_range_reference(&format!(
                "{}!{}",
                quote_sheet_name(&sheet),
                body
            ))?)
        } else {
            None
        };
        let (target_row, target_col) = if !is_range {
            parse_cell_address(&body).ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::InvalidRef,
                    format!("invalid cell reference: {reference}"),
                )
                .with_ref(reference)
            })?
        } else {
            (0, 0)
        };

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(merges) = ws.merge_cells.as_mut() else {
            return Ok(None);
        };
        let mut found: Option<usize> = None;
        for (idx, existing) in merges.merge_cell.iter().enumerate() {
            let Some((r1, c1, r2, c2)) = parse_range_a1(existing.reference.as_str()) else {
                continue;
            };
            let hit = if let Some(tr) = &target_range {
                tr.start_row == r1
                    && tr.start_column == c1
                    && tr.end_row == r2
                    && tr.end_column == c2
            } else {
                target_row >= r1 && target_row <= r2 && target_col >= c1 && target_col <= c2
            };
            if hit {
                found = Some(idx);
                break;
            }
        }
        let Some(idx) = found else { return Ok(None) };
        let removed = merges.merge_cell.remove(idx);
        if merges.merge_cell.is_empty() {
            ws.merge_cells = None;
        } else {
            let len = merges.merge_cell.len() as u32;
            merges.count = Some(len);
        }
        Ok(merge_info_from_ref(&sheet, removed.reference.as_str()))
    }
}

fn merge_info(sheet: &str, range_ref: &ResolvedRangeRef) -> MergeInfo {
    MergeInfo {
        sheet: sheet.to_string(),
        reference: range_ref.range_reference(),
        start_row: range_ref.start_row,
        start_column: range_ref.start_column,
        end_row: range_ref.end_row,
        end_column: range_ref.end_column,
        rows: range_ref.end_row - range_ref.start_row + 1,
        columns: range_ref.end_column - range_ref.start_column + 1,
    }
}

fn merge_info_from_ref(sheet: &str, reference: &str) -> Option<MergeInfo> {
    let (r1, c1, r2, c2) = parse_range_a1(reference)?;
    Some(MergeInfo {
        sheet: sheet.to_string(),
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
        rows: r2 - r1 + 1,
        columns: c2 - c1 + 1,
    })
}
