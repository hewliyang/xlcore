use crate::constants::{LAST_COLUMN, LAST_ROW};
use crate::expressions::parser::ArrayNode;
use crate::expressions::types::CellReferenceIndex;
use crate::{
    calc_result::CalcResult, expressions::parser::Node, expressions::token::Error, model::Model,
};

impl<'a> Model<'a> {
    fn collect_numbers(
        &mut self,
        arg: &Node,
        cell: CellReferenceIndex,
    ) -> Result<Vec<f64>, CalcResult> {
        let mut values: Vec<f64> = Vec::new();
        match self.evaluate_node_in_context(arg, cell) {
            CalcResult::Number(value) => {
                values.push(value);
            }
            CalcResult::Range { left, right } => {
                if left.sheet != right.sheet {
                    return Err(CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Ranges are in different sheets".to_string(),
                    ));
                }

                let row1 = left.row;
                let mut row2 = right.row;
                let column1 = left.column;
                let mut column2 = right.column;

                if row1 == 1 && row2 == LAST_ROW {
                    row2 = match self.workbook.worksheet(left.sheet) {
                        Ok(s) => s.dimension().max_row,
                        Err(_) => {
                            return Err(CalcResult::new_error(
                                Error::ERROR,
                                cell,
                                format!("Invalid worksheet index: '{}'", left.sheet),
                            ));
                        }
                    };
                }
                if column1 == 1 && column2 == LAST_COLUMN {
                    column2 = match self.workbook.worksheet(left.sheet) {
                        Ok(s) => s.dimension().max_column,
                        Err(_) => {
                            return Err(CalcResult::new_error(
                                Error::ERROR,
                                cell,
                                format!("Invalid worksheet index: '{}'", left.sheet),
                            ));
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
                            error @ CalcResult::Error { .. } => return Err(error),
                            _ => {}
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
                                return Err(CalcResult::Error {
                                    error,
                                    origin: cell,
                                    message: "Error in array".to_string(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            error @ CalcResult::Error { .. } => return Err(error),
            _ => {}
        }
        Ok(values)
    }

    fn percentile_inc(values: &[f64], k: f64) -> Option<f64> {
        if values.is_empty() || !(0.0..=1.0).contains(&k) {
            return None;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        let rank = k * ((n - 1) as f64);
        let lower = rank.floor() as usize;
        let frac = rank - rank.floor();
        if lower + 1 < n {
            Some(sorted[lower] + frac * (sorted[lower + 1] - sorted[lower]))
        } else {
            Some(sorted[lower])
        }
    }

    fn percentile_exc(values: &[f64], k: f64) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        let nf = n as f64;
        if k < 1.0 / (nf + 1.0) || k > nf / (nf + 1.0) {
            return None;
        }
        let rank = k * (nf + 1.0) - 1.0;
        let lower = rank.floor() as usize;
        let frac = rank - rank.floor();
        if lower + 1 < n {
            Some(sorted[lower] + frac * (sorted[lower + 1] - sorted[lower]))
        } else {
            Some(sorted[lower])
        }
    }

    fn percentrank_position(sorted: &[f64], x: f64) -> Option<f64> {
        let n = sorted.len();
        if n == 0 || x < sorted[0] || x > sorted[n - 1] {
            return None;
        }
        for (i, value) in sorted.iter().enumerate() {
            if *value == x {
                return Some(i as f64);
            }
        }
        for i in 0..n - 1 {
            if sorted[i] < x && x < sorted[i + 1] {
                return Some(i as f64 + (x - sorted[i]) / (sorted[i + 1] - sorted[i]));
            }
        }
        None
    }

    fn percentrank_truncate(value: f64, significance: i32) -> f64 {
        if value == 0.0 {
            return 0.0;
        }
        let digits = value.abs().log10().floor() as i32 + 1;
        let factor = 10f64.powi(significance - digits);
        (value * factor).trunc() / factor
    }

    fn percentrank(values: &[f64], x: f64, significance: i32, exclusive: bool) -> Option<f64> {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let pos = Model::percentrank_position(&sorted, x)?;
        let n = sorted.len() as f64;
        let result = if exclusive {
            (pos + 1.0) / (n + 1.0)
        } else {
            pos / (n - 1.0)
        };
        Some(Model::percentrank_truncate(result, significance))
    }

    fn fn_percentrank_impl(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
        exclusive: bool,
    ) -> CalcResult {
        if args.len() != 2 && args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.collect_numbers(&args[0], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if values.is_empty() {
            return CalcResult::new_error(Error::NA, cell, "Empty array".to_string());
        }
        let x = match self.get_number(&args[1], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let significance = if args.len() == 3 {
            match self.get_number(&args[2], cell) {
                Ok(v) => v.trunc() as i32,
                Err(e) => return e,
            }
        } else {
            3
        };
        if significance < 1 {
            return CalcResult::new_error(Error::NUM, cell, "Invalid significance".to_string());
        }
        match Model::percentrank(&values, x, significance, exclusive) {
            Some(value) => CalcResult::Number(value),
            None => CalcResult::new_error(Error::NA, cell, "x is out of range".to_string()),
        }
    }

    pub(crate) fn fn_percentrank_inc(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        self.fn_percentrank_impl(args, cell, false)
    }

    pub(crate) fn fn_percentrank_exc(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        self.fn_percentrank_impl(args, cell, true)
    }

    pub(crate) fn fn_percentile_inc(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.collect_numbers(&args[0], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let k = match self.get_number(&args[1], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match Model::percentile_inc(&values, k) {
            Some(value) => CalcResult::Number(value),
            None => CalcResult::new_error(Error::NUM, cell, "Invalid percentile".to_string()),
        }
    }

    pub(crate) fn fn_percentile_exc(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.collect_numbers(&args[0], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let k = match self.get_number(&args[1], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match Model::percentile_exc(&values, k) {
            Some(value) => CalcResult::Number(value),
            None => CalcResult::new_error(Error::NUM, cell, "Invalid percentile".to_string()),
        }
    }

    pub(crate) fn fn_quartile_inc(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.collect_numbers(&args[0], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let quart = match self.get_number(&args[1], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let quart = quart.trunc();
        if !(0.0..=4.0).contains(&quart) {
            return CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string());
        }
        match Model::percentile_inc(&values, quart / 4.0) {
            Some(value) => CalcResult::Number(value),
            None => CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string()),
        }
    }

    pub(crate) fn fn_quartile_exc(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.collect_numbers(&args[0], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let quart = match self.get_number(&args[1], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let quart = quart.trunc();
        if !(0.0..=4.0).contains(&quart) {
            return CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string());
        }
        match Model::percentile_exc(&values, quart / 4.0) {
            Some(value) => CalcResult::Number(value),
            None => CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string()),
        }
    }
}
