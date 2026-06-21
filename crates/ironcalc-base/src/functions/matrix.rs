use crate::expressions::parser::{ArrayNode, Node};
use crate::expressions::token::Error;
use crate::expressions::types::CellReferenceIndex;
use crate::{calc_result::CalcResult, model::Model};

fn calc_result_to_array_node(value: CalcResult) -> ArrayNode {
    match value {
        CalcResult::Number(f) => ArrayNode::Number(f),
        CalcResult::String(s) => ArrayNode::String(s),
        CalcResult::Boolean(b) => ArrayNode::Boolean(b),
        CalcResult::Error { error, .. } => ArrayNode::Error(error),
        CalcResult::EmptyCell | CalcResult::EmptyArg => ArrayNode::Number(0.0),
        CalcResult::Range { .. } | CalcResult::Array(_) => ArrayNode::Error(Error::VALUE),
    }
}

impl<'a> Model<'a> {
    fn read_square_matrix(
        &mut self,
        arg: &Node,
        cell: CellReferenceIndex,
    ) -> Result<Vec<Vec<f64>>, CalcResult> {
        let result = self.evaluate_node_in_context(arg, cell);
        let matrix = match result {
            CalcResult::Number(value) => vec![vec![value]],
            CalcResult::Range { left, right } => {
                if left.sheet != right.sheet {
                    return Err(CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Ranges are in different sheets".to_string(),
                    ));
                }
                let mut rows = Vec::new();
                for row in left.row..=right.row {
                    let mut current = Vec::new();
                    for column in left.column..=right.column {
                        let cell_ref = CellReferenceIndex {
                            sheet: left.sheet,
                            row,
                            column,
                        };
                        match self.evaluate_cell(cell_ref) {
                            CalcResult::Number(v) => current.push(v),
                            error @ CalcResult::Error { .. } => return Err(error),
                            _ => {
                                return Err(CalcResult::new_error(
                                    Error::VALUE,
                                    cell,
                                    "MDETERM requires numeric values".to_string(),
                                ));
                            }
                        }
                    }
                    rows.push(current);
                }
                rows
            }
            CalcResult::Array(array) => {
                let mut rows = Vec::new();
                for row in array {
                    let mut current = Vec::new();
                    for item in row {
                        match item {
                            ArrayNode::Number(v) => current.push(v),
                            ArrayNode::Error(error) => {
                                return Err(CalcResult::new_error(
                                    error,
                                    cell,
                                    "MDETERM error in array".to_string(),
                                ));
                            }
                            _ => {
                                return Err(CalcResult::new_error(
                                    Error::VALUE,
                                    cell,
                                    "MDETERM requires numeric values".to_string(),
                                ));
                            }
                        }
                    }
                    rows.push(current);
                }
                rows
            }
            error @ CalcResult::Error { .. } => return Err(error),
            _ => {
                return Err(CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "MDETERM requires a numeric matrix".to_string(),
                ));
            }
        };

        let n = matrix.len();
        if n == 0 {
            return Err(CalcResult::new_error(
                Error::VALUE,
                cell,
                "MDETERM requires a square matrix".to_string(),
            ));
        }
        for row in &matrix {
            if row.len() != n {
                return Err(CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "MDETERM requires a square matrix".to_string(),
                ));
            }
        }
        Ok(matrix)
    }

    pub(crate) fn fn_transpose(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let result = self.evaluate_node_in_context(&args[0], cell);
        let grid: Vec<Vec<ArrayNode>> = match result {
            CalcResult::Range { left, right } => {
                if left.sheet != right.sheet {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Ranges are in different sheets".to_string(),
                    );
                }
                let mut rows = Vec::new();
                for row in left.row..=right.row {
                    let mut current = Vec::new();
                    for column in left.column..=right.column {
                        let cell_ref = CellReferenceIndex {
                            sheet: left.sheet,
                            row,
                            column,
                        };
                        current.push(calc_result_to_array_node(
                            self.evaluate_cell(cell_ref),
                        ));
                    }
                    rows.push(current);
                }
                rows
            }
            CalcResult::Array(array) => array,
            error @ CalcResult::Error { .. } => return error,
            other => vec![vec![calc_result_to_array_node(other)]],
        };
        let n_rows = grid.len();
        if n_rows == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "TRANSPOSE: empty array".to_string());
        }
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        if n_cols == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "TRANSPOSE: empty array".to_string());
        }
        let mut transposed: Vec<Vec<ArrayNode>> = Vec::with_capacity(n_cols);
        for c in 0..n_cols {
            let mut new_row = Vec::with_capacity(n_rows);
            for row in grid.iter() {
                let node = row.get(c).cloned().unwrap_or(ArrayNode::Number(0.0));
                new_row.push(node);
            }
            transposed.push(new_row);
        }
        CalcResult::Array(transposed)
    }

    pub(crate) fn fn_sequence(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let rows = match self.get_number(&args[0], cell) {
            Ok(f) => f.trunc() as i64,
            Err(e) => return e,
        };
        let columns = if args.len() >= 2 {
            match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc() as i64,
                Err(e) => return e,
            }
        } else {
            1
        };
        let start = if args.len() >= 3 {
            match self.get_number(&args[2], cell) {
                Ok(f) => f,
                Err(e) => return e,
            }
        } else {
            1.0
        };
        let step = if args.len() >= 4 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f,
                Err(e) => return e,
            }
        } else {
            1.0
        };
        if rows < 1 || columns < 1 {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "SEQUENCE requires positive dimensions".to_string(),
            );
        }
        let mut grid: Vec<Vec<ArrayNode>> = Vec::with_capacity(rows as usize);
        let mut counter = 0.0;
        for _ in 0..rows {
            let mut new_row = Vec::with_capacity(columns as usize);
            for _ in 0..columns {
                new_row.push(ArrayNode::Number(start + counter * step));
                counter += 1.0;
            }
            grid.push(new_row);
        }
        CalcResult::Array(grid)
    }

    pub(crate) fn fn_mdeterm(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let mut matrix = match self.read_square_matrix(&args[0], cell) {
            Ok(m) => m,
            Err(e) => return e,
        };

        let n = matrix.len();
        let mut det = 1.0;
        for column in 0..n {
            let mut pivot = column;
            let mut pivot_value = matrix[column][column].abs();
            for row in (column + 1)..n {
                let value = matrix[row][column].abs();
                if value > pivot_value {
                    pivot_value = value;
                    pivot = row;
                }
            }
            if matrix[pivot][column] == 0.0 {
                return CalcResult::Number(0.0);
            }
            if pivot != column {
                matrix.swap(pivot, column);
                det = -det;
            }
            det *= matrix[column][column];
            let pivot_diag = matrix[column][column];
            for row in (column + 1)..n {
                let factor = matrix[row][column] / pivot_diag;
                if factor != 0.0 {
                    for col in column..n {
                        matrix[row][col] -= factor * matrix[column][col];
                    }
                }
            }
        }

        CalcResult::Number(det)
    }
}
