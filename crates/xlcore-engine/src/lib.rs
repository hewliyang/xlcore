mod formula;

use ironcalc_base::{cell::CellValue as IronCellValue, types::Cell, Model};

pub use formula::prepare_formula_for_ironcalc;
pub use xlcore_types::EngineCellValue as CellValue;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("{0}")]
    IronCalc(String),
}

impl From<String> for EngineError {
    fn from(value: String) -> Self {
        Self::IronCalc(value)
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FormulaError {
    pub kind: String,
    pub message: String,
}

fn cell_value_from_iron(value: IronCellValue) -> CellValue {
    match value {
        IronCellValue::None => CellValue::Blank,
        IronCellValue::String(v) => CellValue::String(v),
        IronCellValue::Number(v) => CellValue::Number(v),
        IronCellValue::Boolean(v) => CellValue::Boolean(v),
    }
}

/// Thin recalc facade over IronCalc.
///
/// Coordinates are zero-based for sheets and one-based for rows/columns, matching
/// IronCalc and SpreadsheetML cell coordinates.
pub struct WorkbookEngine<'a> {
    model: Model<'a>,
}

impl<'a> WorkbookEngine<'a> {
    pub fn new(name: &'a str) -> Result<Self> {
        Ok(Self {
            model: Model::new_empty(name, "en", "UTC", "en")?,
        })
    }

    pub fn sheet_count(&self) -> usize {
        self.model.workbook.worksheets.len()
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.model.workbook.get_worksheet_names()
    }

    pub fn add_sheet(&mut self, name: &str) -> Result<u32> {
        let index = self.model.workbook.worksheets.len() as u32;
        self.model.add_sheet(name)?;
        Ok(index)
    }

    pub fn rename_sheet(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        self.model.rename_sheet(old_name, new_name)?;
        Ok(())
    }

    pub fn add_defined_name(
        &mut self,
        name: &str,
        scope: Option<u32>,
        formula: &str,
    ) -> Result<()> {
        let trimmed = formula.trim();
        let trimmed = trimmed.strip_prefix('=').unwrap_or(trimmed);
        self.model.new_defined_name(name, scope, trimmed)?;
        Ok(())
    }

    pub fn set_input(
        &mut self,
        sheet: u32,
        row: i32,
        column: i32,
        value: impl Into<String>,
    ) -> Result<()> {
        self.model
            .set_user_input(sheet, row, column, value.into())?;
        Ok(())
    }

    pub fn set_formula(
        &mut self,
        sheet: u32,
        row: i32,
        column: i32,
        formula: impl AsRef<str>,
    ) -> Result<()> {
        let formula = prepare_formula_for_ironcalc(formula.as_ref());
        let formula = if formula.starts_with('=') {
            formula
        } else {
            format!("={formula}")
        };
        self.set_input(sheet, row, column, formula)
    }

    pub fn evaluate(&mut self) {
        self.model.evaluate();
    }

    pub fn cell_value(&self, sheet: u32, row: i32, column: i32) -> Result<CellValue> {
        Ok(cell_value_from_iron(
            self.model.get_cell_value_by_index(sheet, row, column)?,
        ))
    }

    pub fn spill_ranges(&self, sheet: u32) -> Vec<((i32, i32), String)> {
        let Ok(worksheet) = self.model.workbook.worksheet(sheet) else {
            return Vec::new();
        };
        let mut anchors = Vec::new();
        for (row, columns) in &worksheet.sheet_data {
            for (column, cell) in columns {
                if let Cell::CellFormulaArray { range, .. } = cell {
                    anchors.push(((*row, *column), range.clone()));
                }
            }
        }
        anchors
    }

    pub fn formula_error(&self, sheet: u32, row: i32, column: i32) -> Result<Option<FormulaError>> {
        let cell = self
            .model
            .workbook
            .worksheet(sheet)?
            .cell(row, column)
            .cloned()
            .unwrap_or_default();
        Ok(match cell {
            Cell::CellFormulaError { ei, m, .. } => Some(FormulaError {
                kind: ei.to_string(),
                message: m,
            }),
            Cell::CellFormula { .. } => Some(FormulaError {
                kind: "#ERROR!".to_string(),
                message: "Unevaluated formula".to_string(),
            }),
            _ => None,
        })
    }

    pub fn inner(&self) -> &Model<'a> {
        &self.model
    }

    pub fn inner_mut(&mut self) -> &mut Model<'a> {
        &mut self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recalculates_arithmetic_and_ranges() {
        let mut engine = WorkbookEngine::new("basic").unwrap();
        engine.rename_sheet("Sheet1", "Data").unwrap();
        engine.set_input(0, 1, 1, "10").unwrap();
        engine.set_input(0, 2, 1, "15").unwrap();
        engine.set_formula(0, 3, 1, "SUM(A1:A2) * 2").unwrap();

        engine.evaluate();

        assert_eq!(engine.cell_value(0, 3, 1).unwrap(), CellValue::Number(50.0));
        assert_eq!(engine.sheet_names(), vec!["Data".to_string()]);
    }

    #[test]
    fn recalculates_cross_sheet_references() {
        let mut engine = WorkbookEngine::new("cross-sheet").unwrap();
        engine.rename_sheet("Sheet1", "Inputs").unwrap();
        let summary = engine.add_sheet("Summary").unwrap();
        engine.set_input(0, 1, 1, "7").unwrap();
        engine.set_formula(summary, 1, 1, "Inputs!A1 + 5").unwrap();

        engine.evaluate();

        assert_eq!(
            engine.cell_value(summary, 1, 1).unwrap(),
            CellValue::Number(12.0)
        );
    }

    #[test]
    fn accepts_formula_with_or_without_leading_equals() {
        let mut engine = WorkbookEngine::new("formula-prefix").unwrap();
        engine.set_formula(0, 1, 1, "=1+1").unwrap();
        engine.set_formula(0, 1, 2, "A1+1").unwrap();

        engine.evaluate();

        assert_eq!(engine.cell_value(0, 1, 1).unwrap(), CellValue::Number(2.0));
        assert_eq!(engine.cell_value(0, 1, 2).unwrap(), CellValue::Number(3.0));
    }

    #[test]
    fn recalculates_scalar_let_formula_with_compat_shim() {
        let mut engine = WorkbookEngine::new("let-shim").unwrap();
        engine.set_input(0, 1, 1, "10").unwrap();
        engine.set_input(0, 2, 1, "15").unwrap();
        engine
            .set_formula(0, 3, 1, "LET(total,SUM(A1:A2), total * 2)")
            .unwrap();

        engine.evaluate();

        assert_eq!(engine.cell_value(0, 3, 1).unwrap(), CellValue::Number(50.0));
    }

    #[test]
    fn let_bindings_can_reference_prior_bindings() {
        let mut engine = WorkbookEngine::new("let-bindings").unwrap();
        engine.set_input(0, 1, 1, "10").unwrap();
        engine.set_input(0, 2, 1, "15").unwrap();
        engine
            .set_formula(0, 3, 1, "LET(a,SUM(A1:A2), b, a*2, b+1)")
            .unwrap();

        engine.evaluate();

        assert_eq!(engine.cell_value(0, 3, 1).unwrap(), CellValue::Number(51.0));
    }

    #[test]
    fn recalculates_sumproduct_from_forked_engine() {
        let mut engine = WorkbookEngine::new("sumproduct").unwrap();
        engine.set_input(0, 1, 1, "1").unwrap();
        engine.set_input(0, 2, 1, "2").unwrap();
        engine.set_input(0, 3, 1, "3").unwrap();
        engine.set_input(0, 1, 2, "10").unwrap();
        engine.set_input(0, 2, 2, "20").unwrap();
        engine.set_input(0, 3, 2, "30").unwrap();
        engine
            .set_formula(0, 4, 1, "SUMPRODUCT(A1:A3,B1:B3)")
            .unwrap();

        engine.evaluate();

        assert_eq!(
            engine.cell_value(0, 4, 1).unwrap(),
            CellValue::Number(140.0)
        );
    }

    #[test]
    fn exposes_formula_errors_after_evaluation() {
        let mut engine = WorkbookEngine::new("errors").unwrap();
        engine.set_formula(0, 1, 1, "NO_SUCH_FUNCTION(1)").unwrap();

        engine.evaluate();

        let error = engine.formula_error(0, 1, 1).unwrap().unwrap();
        assert_eq!(error.kind, "#NAME?");
        assert!(error.message.contains("Invalid function"));
    }
}
