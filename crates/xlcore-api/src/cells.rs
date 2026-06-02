use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiCellValue as CellValue, CellInfo, ClearMode};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_cell_reference, ResolvedCellRef};
use crate::xml::{
    apply_clear_mode, cell_info_from_cell, ensure_cell, load_shared_strings, mark_formulas_stale,
    normalize_formula, set_cell_value,
};
use crate::{Result, Workbook};

impl Workbook {
    pub fn get_cell(&mut self, reference: impl AsRef<str>) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let shared_strings = load_shared_strings(&mut self.doc);
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ws
            .sheet_data
            .row
            .iter()
            .find(|row| row.row_index == Some(cell_ref.row))
            .and_then(|row| {
                row.cell.iter().find(|cell| {
                    cell.cell_reference
                        .as_ref()
                        .and_then(|r| xlcore_io::parse_a1(r.as_str()))
                        == Some((cell_ref.row, cell_ref.column))
                })
            });
        Ok(cell_info_from_cell(
            &cell_ref.sheet,
            cell_ref.row,
            cell_ref.column,
            cell,
            &shared_strings,
        ))
    }

    pub fn set_value(
        &mut self,
        reference: impl AsRef<str>,
        value: impl Into<CellValue>,
    ) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let value = value.into();
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ensure_cell(ws, cell_ref.row, cell_ref.column);
        set_cell_value(cell, &value);
        mark_formulas_stale(&mut self.doc)?;
        self.get_cell(cell_ref.full_reference())
    }

    pub fn set_formula(
        &mut self,
        reference: impl AsRef<str>,
        formula: impl AsRef<str>,
    ) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let formula = normalize_formula(formula.as_ref());
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ensure_cell(ws, cell_ref.row, cell_ref.column);
        cell.data_type = None;
        cell.inline_string = None;
        cell.cell_value = None;
        cell.cell_formula = Some(x::CellFormula {
            xml_content: Some(formula),
            ..Default::default()
        });
        mark_formulas_stale(&mut self.doc)?;
        self.get_cell(cell_ref.full_reference())
    }

    pub fn clear(&mut self, reference: impl AsRef<str>) -> Result<CellInfo> {
        self.clear_with(reference, ClearMode::All)
    }

    pub fn clear_with(&mut self, reference: impl AsRef<str>, mode: ClearMode) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let touches_formulas = matches!(
            mode,
            ClearMode::All | ClearMode::Formulas | ClearMode::Values
        );
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ensure_cell(ws, cell_ref.row, cell_ref.column);
        apply_clear_mode(cell, mode);
        if touches_formulas {
            mark_formulas_stale(&mut self.doc)?;
        }
        self.get_cell(cell_ref.full_reference())
    }

    pub(crate) fn resolve_cell_ref(&mut self, reference: &str) -> Result<ResolvedCellRef> {
        let parsed = parse_cell_reference(reference)?;
        let sheet = match parsed.sheet {
            Some(sheet) => sheet,
            None => self.default_sheet_name()?,
        };
        Ok(ResolvedCellRef {
            sheet,
            row: parsed.row,
            column: parsed.column,
        })
    }
}
