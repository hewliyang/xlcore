use std::collections::HashMap;

use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, RangeInfo};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_reference, ResolvedRangeRef};
use crate::structural::{translate_formula_refs, MAX_COLUMN, MAX_ROW};
use crate::xml::{ensure_cell, mark_formulas_stale};
use crate::{Result, Workbook};

#[derive(Clone, Default)]
struct CellSnapshot {
    data_type: Option<x::CellValues>,
    cell_value: Option<x::CellValue>,
    cell_formula: Option<x::CellFormula>,
    inline_string: Option<Box<x::InlineString>>,
    style_index: Option<u32>,
}

impl Workbook {
    pub fn copy_range(
        &mut self,
        src_reference: impl AsRef<str>,
        dst_reference: impl AsRef<str>,
    ) -> Result<RangeInfo> {
        let src = self.resolve_range_ref(src_reference.as_ref())?;
        let dst_parsed = parse_range_reference(dst_reference.as_ref())?;
        let dst_sheet = match dst_parsed.sheet {
            Some(s) => s,
            None => src.sheet.clone(),
        };
        let src_rows = src.end_row - src.start_row + 1;
        let src_cols = src.end_column - src.start_column + 1;
        let dst_rows_in = dst_parsed.end_row - dst_parsed.start_row + 1;
        let dst_cols_in = dst_parsed.end_column - dst_parsed.start_column + 1;

        let (dst_end_row, dst_end_column) = if dst_rows_in == 1 && dst_cols_in == 1 {
            (
                dst_parsed.start_row + src_rows - 1,
                dst_parsed.start_column + src_cols - 1,
            )
        } else if dst_rows_in % src_rows == 0 && dst_cols_in % src_cols == 0 {
            (dst_parsed.end_row, dst_parsed.end_column)
        } else {
            return Err(ApiError::new(
                ApiErrorCode::ShapeMismatch,
                format!(
                    "copy destination must be a single cell or a whole multiple of source shape ({src_rows}x{src_cols})",
                ),
            )
            .with_ref(dst_reference.as_ref()));
        };
        check_bounds(dst_end_row, dst_end_column, dst_reference.as_ref())?;

        let dst = ResolvedRangeRef {
            sheet: dst_sheet,
            start_row: dst_parsed.start_row,
            start_column: dst_parsed.start_column,
            end_row: dst_end_row,
            end_column: dst_end_column,
        };

        let snapshot = self.snapshot_range(&src)?;
        let mut r = dst.start_row;
        while r <= dst.end_row {
            let mut c = dst.start_column;
            while c <= dst.end_column {
                let dr = r as i64 - src.start_row as i64;
                let dc = c as i64 - src.start_column as i64;
                self.write_snapshot_at(&dst.sheet, r, c, &snapshot, dr, dc)?;
                c += src_cols;
            }
            r += src_rows;
        }
        mark_formulas_stale(&mut self.doc)?;
        self.read_range(&dst)
    }

    pub fn fill_range(
        &mut self,
        src_reference: impl AsRef<str>,
        dst_reference: impl AsRef<str>,
    ) -> Result<RangeInfo> {
        let src = self.resolve_range_ref(src_reference.as_ref())?;
        let dst = self.resolve_range_ref(dst_reference.as_ref())?;
        if dst.sheet != src.sheet {
            return Err(ApiError::new(
                ApiErrorCode::InvalidRef,
                "fill source and destination must be on the same sheet",
            )
            .with_ref(dst_reference.as_ref()));
        }
        if src.start_row < dst.start_row
            || src.end_row > dst.end_row
            || src.start_column < dst.start_column
            || src.end_column > dst.end_column
        {
            return Err(ApiError::new(
                ApiErrorCode::ShapeMismatch,
                "fill destination must contain the source range",
            )
            .with_ref(dst_reference.as_ref()));
        }
        let src_rows = src.end_row - src.start_row + 1;
        let src_cols = src.end_column - src.start_column + 1;
        let dst_rows = dst.end_row - dst.start_row + 1;
        let dst_cols = dst.end_column - dst.start_column + 1;
        if dst_rows % src_rows != 0 || dst_cols % src_cols != 0 {
            return Err(ApiError::new(
                ApiErrorCode::ShapeMismatch,
                format!(
                    "fill destination ({dst_rows}x{dst_cols}) must be a whole multiple of source ({src_rows}x{src_cols})",
                ),
            )
            .with_ref(dst_reference.as_ref()));
        }
        check_bounds(dst.end_row, dst.end_column, dst_reference.as_ref())?;

        let snapshot = self.snapshot_range(&src)?;
        let mut r = dst.start_row;
        while r <= dst.end_row {
            let mut c = dst.start_column;
            while c <= dst.end_column {
                if r == src.start_row && c == src.start_column {
                    c += src_cols;
                    continue;
                }
                let dr = r as i64 - src.start_row as i64;
                let dc = c as i64 - src.start_column as i64;
                self.write_snapshot_at(&dst.sheet, r, c, &snapshot, dr, dc)?;
                c += src_cols;
            }
            r += src_rows;
        }
        mark_formulas_stale(&mut self.doc)?;
        self.read_range(&dst)
    }

    fn snapshot_range(
        &mut self,
        range_ref: &ResolvedRangeRef,
    ) -> Result<HashMap<(u32, u32), CellSnapshot>> {
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let mut out: HashMap<(u32, u32), CellSnapshot> = HashMap::new();
        for row in &ws.x_sheet_data.x_row {
            let Some(row_idx) = row.row_index else { continue };
            if row_idx < range_ref.start_row || row_idx > range_ref.end_row {
                continue;
            }
            for cell in &row.x_c {
                let Some((cr, cc)) = cell
                    .cell_reference
                    .as_ref()
                    .and_then(|r| xlcore_io::parse_a1(r.as_str()))
                else {
                    continue;
                };
                if cr != row_idx
                    || cc < range_ref.start_column
                    || cc > range_ref.end_column
                {
                    continue;
                }
                let r_off = cr - range_ref.start_row;
                let c_off = cc - range_ref.start_column;
                out.insert(
                    (r_off, c_off),
                    CellSnapshot {
                        data_type: cell.data_type.clone(),
                        cell_value: cell.cell_value.clone(),
                        cell_formula: cell.cell_formula.clone(),
                        inline_string: cell.inline_string.clone(),
                        style_index: cell.style_index,
                    },
                );
            }
        }
        Ok(out)
    }

    fn write_snapshot_at(
        &mut self,
        dst_sheet: &str,
        dst_start_row: u32,
        dst_start_column: u32,
        snapshot: &HashMap<(u32, u32), CellSnapshot>,
        dr: i64,
        dc: i64,
    ) -> Result<()> {
        let max_r = snapshot.keys().map(|(r, _)| *r).max().unwrap_or(0);
        let max_c = snapshot.keys().map(|(_, c)| *c).max().unwrap_or(0);
        let ws_part = self.worksheet_part_for_sheet(dst_sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for r_off in 0..=max_r {
            for c_off in 0..=max_c {
                let row_idx = dst_start_row + r_off;
                let col_idx = dst_start_column + c_off;
                let cell = ensure_cell(ws, row_idx, col_idx);
                match snapshot.get(&(r_off, c_off)) {
                    Some(snap) => {
                        cell.data_type = snap.data_type.clone();
                        cell.cell_value = snap.cell_value.clone();
                        cell.inline_string = snap.inline_string.clone();
                        cell.style_index = snap.style_index;
                        cell.cell_formula = snap.cell_formula.as_ref().map(|f| x::CellFormula {
                            xml_content: f
                                .xml_content
                                .as_deref()
                                .map(|src| translate_formula_refs(src, dr, dc)),
                            ..Default::default()
                        });
                        if cell.cell_formula.is_some() {
                            cell.cell_value = None;
                            cell.data_type = None;
                            cell.inline_string = None;
                        }
                    }
                    None => {
                        cell.data_type = None;
                        cell.cell_value = None;
                        cell.cell_formula = None;
                        cell.inline_string = None;
                        cell.style_index = None;
                    }
                }
            }
        }
        Ok(())
    }
}

fn check_bounds(end_row: u32, end_col: u32, reference: &str) -> Result<()> {
    if end_row > MAX_ROW || end_col > MAX_COLUMN {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            "destination exceeds sheet bounds",
        )
        .with_ref(reference));
    }
    Ok(())
}
