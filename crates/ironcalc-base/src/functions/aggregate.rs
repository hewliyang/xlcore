use crate::{
    calc_result::CalcResult,
    expressions::{parser::Node, token::Error, types::CellReferenceIndex},
    functions::{subtotal::CellTableStatus, Function},
    model::Model,
};

struct AggregateData {
    numbers: Vec<f64>,
    counta: usize,
}

impl<'a> Model<'a> {
    fn aggregate_collect(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
        ignore_errors: bool,
        ignore_hidden: bool,
    ) -> Result<AggregateData, CalcResult> {
        let mut numbers: Vec<f64> = Vec::new();
        let mut counta: usize = 0;
        for arg in args {
            if let Node::FunctionKind {
                kind: Function::Subtotal | Function::Aggregate,
                args: _,
            } = arg
            {
                continue;
            }
            match self.evaluate_node_with_reference(arg, cell) {
                CalcResult::Number(value) => {
                    numbers.push(value);
                    counta += 1;
                }
                CalcResult::String(_) | CalcResult::Boolean(_) => {
                    counta += 1;
                }
                error @ CalcResult::Error { .. } => {
                    if !ignore_errors {
                        return Err(error);
                    }
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
                    let row2 = right.row;
                    let column1 = left.column;
                    let column2 = right.column;
                    for row in row1..=row2 {
                        if ignore_hidden {
                            let status = self
                                .cell_hidden_status(left.sheet, row, column1)
                                .map_err(|message| {
                                    CalcResult::new_error(Error::ERROR, cell, message)
                                })?;
                            if status != CellTableStatus::Normal {
                                continue;
                            }
                        }
                        for column in column1..=column2 {
                            match self.evaluate_cell(CellReferenceIndex {
                                sheet: left.sheet,
                                row,
                                column,
                            }) {
                                CalcResult::Number(value) => {
                                    numbers.push(value);
                                    counta += 1;
                                }
                                CalcResult::String(_) | CalcResult::Boolean(_) => {
                                    counta += 1;
                                }
                                error @ CalcResult::Error { .. } => {
                                    if !ignore_errors {
                                        return Err(error);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                CalcResult::Array(_) => {
                    return Err(CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    });
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => {}
            }
        }
        Ok(AggregateData { numbers, counta })
    }

    pub(crate) fn fn_aggregate(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let function_num = match self.get_number(&args[0], cell) {
            Ok(f) => f.trunc() as i32,
            Err(e) => return e,
        };
        let options = match self.get_number(&args[1], cell) {
            Ok(f) => f.trunc() as i32,
            Err(e) => return e,
        };
        if !(1..=19).contains(&function_num) || !(0..=7).contains(&options) {
            return CalcResult::new_error(Error::VALUE, cell, "Invalid AGGREGATE args".to_string());
        }
        let ignore_errors = matches!(options, 2 | 3 | 6 | 7);
        let ignore_hidden = matches!(options, 1 | 3 | 5 | 7);

        let (ref_args, k_arg) = if function_num >= 14 {
            if args.len() != 4 {
                return CalcResult::new_args_number_error(cell);
            }
            (&args[2..3], Some(&args[3]))
        } else {
            (&args[2..], None)
        };

        let k = match k_arg {
            Some(node) => match self.get_number(node, cell) {
                Ok(v) => v,
                Err(e) => return e,
            },
            None => 0.0,
        };

        let data =
            match self.aggregate_collect(ref_args, cell, ignore_errors, ignore_hidden) {
                Ok(d) => d,
                Err(e) => return e,
            };
        let values = data.numbers;

        match function_num {
            1 => {
                if values.is_empty() {
                    return CalcResult::new_error(Error::DIV, cell, "Empty range".to_string());
                }
                CalcResult::Number(values.iter().sum::<f64>() / values.len() as f64)
            }
            2 => CalcResult::Number(values.len() as f64),
            3 => CalcResult::Number(data.counta as f64),
            4 => {
                if values.is_empty() {
                    return CalcResult::Number(0.0);
                }
                CalcResult::Number(values.iter().copied().fold(f64::NEG_INFINITY, f64::max))
            }
            5 => {
                if values.is_empty() {
                    return CalcResult::Number(0.0);
                }
                CalcResult::Number(values.iter().copied().fold(f64::INFINITY, f64::min))
            }
            6 => CalcResult::Number(values.iter().product::<f64>()),
            7 => match aggregate_variance(&values, true) {
                Some(v) => CalcResult::Number(v.sqrt()),
                None => CalcResult::new_error(Error::DIV, cell, "Not enough values".to_string()),
            },
            8 => match aggregate_variance(&values, false) {
                Some(v) => CalcResult::Number(v.sqrt()),
                None => CalcResult::new_error(Error::DIV, cell, "Empty range".to_string()),
            },
            9 => CalcResult::Number(values.iter().sum::<f64>()),
            10 => match aggregate_variance(&values, true) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::DIV, cell, "Not enough values".to_string()),
            },
            11 => match aggregate_variance(&values, false) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::DIV, cell, "Empty range".to_string()),
            },
            12 => match aggregate_median(&values) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::NUM, cell, "Empty range".to_string()),
            },
            13 => match aggregate_mode(&values) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::NA, cell, "No repeated value".to_string()),
            },
            14 => match aggregate_large(&values, k, true) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::NUM, cell, "Invalid k".to_string()),
            },
            15 => match aggregate_large(&values, k, false) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::NUM, cell, "Invalid k".to_string()),
            },
            16 => match Model::percentile_inc(&values, k) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::NUM, cell, "Invalid percentile".to_string()),
            },
            17 => {
                let q = k.trunc();
                if !(0.0..=4.0).contains(&q) {
                    return CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string());
                }
                match Model::percentile_inc(&values, q / 4.0) {
                    Some(v) => CalcResult::Number(v),
                    None => CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string()),
                }
            }
            18 => match Model::percentile_exc(&values, k) {
                Some(v) => CalcResult::Number(v),
                None => CalcResult::new_error(Error::NUM, cell, "Invalid percentile".to_string()),
            },
            19 => {
                let q = k.trunc();
                if !(0.0..=4.0).contains(&q) {
                    return CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string());
                }
                match Model::percentile_exc(&values, q / 4.0) {
                    Some(v) => CalcResult::Number(v),
                    None => CalcResult::new_error(Error::NUM, cell, "Invalid quartile".to_string()),
                }
            }
            _ => CalcResult::new_error(Error::VALUE, cell, "Invalid function_num".to_string()),
        }
    }
}

fn aggregate_variance(values: &[f64], sample: bool) -> Option<f64> {
    let n = values.len();
    if (sample && n < 2) || (!sample && n == 0) {
        return None;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let ss: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
    let denom = if sample { n as f64 - 1.0 } else { n as f64 };
    Some(ss / denom)
}

fn aggregate_median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        Some(sorted[n / 2])
    } else {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

fn aggregate_mode(values: &[f64]) -> Option<f64> {
    let mut best_value: Option<f64> = None;
    let mut best_count = 0;
    for &candidate in values {
        let count = values.iter().filter(|&&v| v == candidate).count();
        if count < 2 {
            continue;
        }
        if count > best_count {
            best_count = count;
            best_value = Some(candidate);
        }
    }
    best_value
}

fn aggregate_large(values: &[f64], k: f64, largest: bool) -> Option<f64> {
    let n = values.len();
    let ki = k.trunc();
    if ki < 1.0 || ki as usize > n || n == 0 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ki as usize - 1;
    if largest {
        Some(sorted[n - 1 - idx])
    } else {
        Some(sorted[idx])
    }
}
