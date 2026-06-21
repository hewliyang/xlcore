use crate::constants::{LAST_COLUMN, LAST_ROW};
use crate::expressions::parser::ArrayNode;
use crate::expressions::types::CellReferenceIndex;
use crate::{
    calc_result::CalcResult, expressions::parser::Node, expressions::token::Error, model::Model,
};

impl<'a> Model<'a> {
    pub(crate) fn fn_mode_sngl(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }

        let mut values: Vec<f64> = Vec::new();

        for arg in args {
            match self.evaluate_node_in_context(arg, cell) {
                CalcResult::Number(value) => {
                    values.push(value);
                }
                CalcResult::Range { left, right } => {
                    if left.sheet != right.sheet {
                        return CalcResult::new_error(
                            Error::VALUE,
                            cell,
                            "Ranges are in different sheets".to_string(),
                        );
                    }

                    let row1 = left.row;
                    let mut row2 = right.row;
                    let column1 = left.column;
                    let mut column2 = right.column;

                    if row1 == 1 && row2 == LAST_ROW {
                        row2 = match self.workbook.worksheet(left.sheet) {
                            Ok(s) => s.dimension().max_row,
                            Err(_) => {
                                return CalcResult::new_error(
                                    Error::ERROR,
                                    cell,
                                    format!("Invalid worksheet index: '{}'", left.sheet),
                                );
                            }
                        };
                    }
                    if column1 == 1 && column2 == LAST_COLUMN {
                        column2 = match self.workbook.worksheet(left.sheet) {
                            Ok(s) => s.dimension().max_column,
                            Err(_) => {
                                return CalcResult::new_error(
                                    Error::ERROR,
                                    cell,
                                    format!("Invalid worksheet index: '{}'", left.sheet),
                                );
                            }
                        };
                    }

                    for row in row1..row2 + 1 {
                        for column in column1..(column2 + 1) {
                            match self.evaluate_cell(CellReferenceIndex {
                                sheet: left.sheet,
                                row,
                                column,
                            }) {
                                CalcResult::Number(value) => {
                                    values.push(value);
                                }
                                error @ CalcResult::Error { .. } => return error,
                                _ => {
                                    // ignore non-numeric
                                }
                            }
                        }
                    }
                }
                CalcResult::Array(array) => {
                    for row in array {
                        for value in row {
                            match value {
                                ArrayNode::Number(value) => {
                                    values.push(value);
                                }
                                ArrayNode::Error(error) => {
                                    return CalcResult::Error {
                                        error,
                                        origin: cell,
                                        message: "Error in array".to_string(),
                                    }
                                }
                                _ => {
                                    // ignore non-numeric
                                }
                            }
                        }
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                _ => {
                    // ignore non-numeric
                }
            }
        }

        let mut best_value: Option<f64> = None;
        let mut best_count: usize = 0;

        for &candidate in &values {
            let count = values.iter().filter(|&&v| v == candidate).count();
            if count < 2 {
                continue;
            }
            if count > best_count {
                best_count = count;
                best_value = Some(candidate);
            }
        }

        match best_value {
            Some(value) => CalcResult::Number(value),
            None => CalcResult::new_error(
                Error::NA,
                cell,
                "MODE found no repeated value".to_string(),
            ),
        }
    }
}
