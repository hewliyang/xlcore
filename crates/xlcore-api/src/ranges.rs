use std::collections::HashMap;

use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiCellValue as CellValue, ApiError, ApiErrorCode, ClearMode, RangeInfo, StylePatch,
};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_reference, qualify_ref, validate_matrix_shape, ResolvedRangeRef};
use crate::styles;
use crate::xml::{
    apply_clear_mode, ensure_cell, load_shared_strings, mark_formulas_stale, normalize_formula,
    read_cell_value, set_cell_value,
};
use crate::{Result, Workbook};

impl Workbook {
    pub fn get_range_in(&mut self, sheet: &str, reference: &str) -> Result<RangeInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.get_range(reference)
    }

    pub fn set_range_values_in(
        &mut self,
        sheet: &str,
        reference: &str,
        values: Vec<Vec<CellValue>>,
    ) -> Result<RangeInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.set_range_values(reference, values)
    }

    pub fn set_range_formulas_in(
        &mut self,
        sheet: &str,
        reference: &str,
        formulas: Vec<Vec<Option<String>>>,
    ) -> Result<RangeInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.set_range_formulas(reference, formulas)
    }

    pub fn set_style_in(
        &mut self,
        sheet: &str,
        reference: &str,
        patch: StylePatch,
    ) -> Result<RangeInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.set_style(reference, patch)
    }

    pub fn clear_range_in(&mut self, sheet: &str, reference: &str) -> Result<RangeInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.clear_range(reference)
    }

    pub fn clear_range_with_in(
        &mut self,
        sheet: &str,
        reference: &str,
        mode: ClearMode,
    ) -> Result<RangeInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.clear_range_with(reference, mode)
    }

    pub fn get_range(&mut self, reference: impl AsRef<str>) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        self.read_range(&range_ref)
    }

    pub fn set_range_values(
        &mut self,
        reference: impl AsRef<str>,
        values: Vec<Vec<CellValue>>,
    ) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        validate_matrix_shape(&values, &range_ref, "values")?;

        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for (r_off, row) in values.iter().enumerate() {
            let row_idx = range_ref.start_row + r_off as u32;
            for (c_off, value) in row.iter().enumerate() {
                let col_idx = range_ref.start_column + c_off as u32;
                let cell = ensure_cell(ws, row_idx, col_idx);
                set_cell_value(cell, value);
            }
        }
        mark_formulas_stale(&mut self.doc)?;
        self.read_range(&range_ref)
    }

    pub fn append_row(&mut self, sheet: &str, values: Vec<CellValue>) -> Result<RangeInfo> {
        self.append_rows(sheet, vec![values])
    }

    pub fn append_rows(&mut self, sheet: &str, rows: Vec<Vec<CellValue>>) -> Result<RangeInfo> {
        if rows.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::ShapeMismatch,
                "append_rows: no rows provided",
            ));
        }
        let sheet = if sheet.is_empty() {
            self.default_sheet_name()?
        } else {
            sheet.to_string()
        };
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let max_row = ws
            .sheet_data
            .row
            .iter()
            .filter(|row| !row.cell.is_empty())
            .filter_map(|row| row.row_index)
            .max()
            .unwrap_or(0);
        let start_row = max_row + 1;
        let mut max_cols = 0u32;
        for (r_off, row) in rows.iter().enumerate() {
            let row_idx = start_row + r_off as u32;
            max_cols = max_cols.max(row.len() as u32);
            for (c_off, value) in row.iter().enumerate() {
                let col_idx = 1 + c_off as u32;
                let cell = ensure_cell(ws, row_idx, col_idx);
                set_cell_value(cell, value);
            }
        }
        mark_formulas_stale(&mut self.doc)?;
        let range_ref = ResolvedRangeRef {
            sheet,
            start_row,
            start_column: 1,
            end_row: start_row + rows.len() as u32 - 1,
            end_column: max_cols.max(1),
        };
        self.read_range(&range_ref)
    }

    pub fn set_range_formulas(
        &mut self,
        reference: impl AsRef<str>,
        formulas: Vec<Vec<Option<String>>>,
    ) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        validate_matrix_shape(&formulas, &range_ref, "formulas")?;

        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for (r_off, row) in formulas.iter().enumerate() {
            let row_idx = range_ref.start_row + r_off as u32;
            for (c_off, formula) in row.iter().enumerate() {
                let col_idx = range_ref.start_column + c_off as u32;
                let cell = ensure_cell(ws, row_idx, col_idx);
                match formula {
                    Some(text) => {
                        let normalized = normalize_formula(text.as_str());
                        cell.data_type = None;
                        cell.inline_string = None;
                        cell.cell_value = None;
                        cell.cell_formula = Some(x::CellFormula {
                            xml_content: Some(normalized),
                            ..Default::default()
                        });
                    }
                    None => {
                        cell.data_type = None;
                        cell.inline_string = None;
                        cell.cell_value = None;
                        cell.cell_formula = None;
                    }
                }
            }
        }
        mark_formulas_stale(&mut self.doc)?;
        self.read_range(&range_ref)
    }

    pub fn set_style(
        &mut self,
        reference: impl AsRef<str>,
        patch: StylePatch,
    ) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        {
            let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
            let mut existing: HashMap<(u32, u32), Option<u32>> = HashMap::new();
            {
                let ws = ws_part
                    .root_element(&mut self.doc)
                    .map_err(sdk_err_to_api)?;
                for row in &ws.sheet_data.row {
                    let Some(row_idx) = row.row_index else {
                        continue;
                    };
                    if row_idx < range_ref.start_row || row_idx > range_ref.end_row {
                        continue;
                    }
                    for cell in &row.cell {
                        if let Some((r, c)) = cell
                            .cell_reference
                            .as_ref()
                            .and_then(|r| xlcore_io::parse_a1(r.as_str()))
                        {
                            if c >= range_ref.start_column && c <= range_ref.end_column {
                                existing.insert((r, c), cell.style_index);
                            }
                        }
                    }
                }
            }

            let mut resolved_indexes: HashMap<(u32, u32), u32> = HashMap::new();
            for row_idx in range_ref.start_row..=range_ref.end_row {
                for col_idx in range_ref.start_column..=range_ref.end_column {
                    let current = existing.get(&(row_idx, col_idx)).copied().flatten();
                    let new_idx = styles::resolve_style_index(&mut self.doc, current, &patch)?;
                    resolved_indexes.insert((row_idx, col_idx), new_idx);
                }
            }

            let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
            let ws = ws_part
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            for row_idx in range_ref.start_row..=range_ref.end_row {
                for col_idx in range_ref.start_column..=range_ref.end_column {
                    let cell = ensure_cell(ws, row_idx, col_idx);
                    if let Some(idx) = resolved_indexes.get(&(row_idx, col_idx)) {
                        cell.style_index = Some(*idx);
                    }
                }
            }
        }
        self.read_range(&range_ref)
    }

    pub fn clear_range(&mut self, reference: impl AsRef<str>) -> Result<RangeInfo> {
        self.clear_range_with(reference, ClearMode::All)
    }

    pub fn clear_range_with(
        &mut self,
        reference: impl AsRef<str>,
        mode: ClearMode,
    ) -> Result<RangeInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        let touches_formulas = matches!(
            mode,
            ClearMode::All | ClearMode::Formulas | ClearMode::Values
        );
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for row_idx in range_ref.start_row..=range_ref.end_row {
            for col_idx in range_ref.start_column..=range_ref.end_column {
                let cell = ensure_cell(ws, row_idx, col_idx);
                apply_clear_mode(cell, mode);
            }
        }
        if touches_formulas {
            mark_formulas_stale(&mut self.doc)?;
        }
        self.read_range(&range_ref)
    }

    pub(crate) fn read_range(&mut self, range_ref: &ResolvedRangeRef) -> Result<RangeInfo> {
        let shared_strings = load_shared_strings(&mut self.doc);
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let rows = (range_ref.end_row - range_ref.start_row + 1) as usize;
        let cols = (range_ref.end_column - range_ref.start_column + 1) as usize;
        let mut values = vec![vec![CellValue::Blank; cols]; rows];
        let mut formulas: Vec<Vec<Option<String>>> = vec![vec![None; cols]; rows];
        for row in &ws.sheet_data.row {
            let Some(row_idx) = row.row_index else {
                continue;
            };
            if row_idx < range_ref.start_row || row_idx > range_ref.end_row {
                continue;
            }
            let r_off = (row_idx - range_ref.start_row) as usize;
            for cell in &row.cell {
                let Some((cr, cc)) = cell
                    .cell_reference
                    .as_ref()
                    .and_then(|r| xlcore_io::parse_a1(r.as_str()))
                else {
                    continue;
                };
                if cr != row_idx || cc < range_ref.start_column || cc > range_ref.end_column {
                    continue;
                }
                let c_off = (cc - range_ref.start_column) as usize;
                let raw_v = cell
                    .cell_value
                    .as_ref()
                    .and_then(|value| value.xml_content.as_deref());
                values[r_off][c_off] = read_cell_value(cell, raw_v, &shared_strings);
                formulas[r_off][c_off] = cell
                    .cell_formula
                    .as_ref()
                    .and_then(|formula| formula.xml_content.as_deref().map(str::to_string));
            }
        }
        Ok(RangeInfo {
            sheet: range_ref.sheet.clone(),
            reference: range_ref.range_reference(),
            start_row: range_ref.start_row,
            start_column: range_ref.start_column,
            end_row: range_ref.end_row,
            end_column: range_ref.end_column,
            rows: rows as u32,
            columns: cols as u32,
            values,
            formulas,
        })
    }

    pub(crate) fn resolve_range_ref(&mut self, reference: &str) -> Result<ResolvedRangeRef> {
        let parsed = parse_range_reference(reference)?;
        let sheet = match parsed.sheet {
            Some(sheet) => sheet,
            None => self.default_sheet_name()?,
        };
        Ok(ResolvedRangeRef {
            sheet,
            start_row: parsed.start_row,
            start_column: parsed.start_column,
            end_row: parsed.end_row,
            end_column: parsed.end_column,
        })
    }
}
