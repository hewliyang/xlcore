use crate::expressions::types::CellReferenceIndex;
use crate::{
    calc_result::CalcResult, expressions::parser::Node, expressions::token::Error, model::Model,
};

impl<'a> Model<'a> {
    pub(crate) fn fn_prob(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 && args.len() != 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let x_range = match self.get_array_of_numbers(&args[0], &cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let prob_range = match self.get_array_of_numbers(&args[1], &cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if x_range.len() != prob_range.len() || x_range.is_empty() {
            return CalcResult::new_error(Error::NA, cell, "Ranges must be same size".to_string());
        }
        let mut sum = 0.0;
        for p in &prob_range {
            if *p < 0.0 || *p > 1.0 {
                return CalcResult::new_error(Error::NUM, cell, "Invalid probability".to_string());
            }
            sum += *p;
        }
        if (sum - 1.0).abs() > 1e-7 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "Probabilities must sum to 1".to_string(),
            );
        }
        let lower = match self.get_number(&args[2], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let upper = if args.len() == 4 {
            match self.get_number(&args[3], cell) {
                Ok(v) => v,
                Err(e) => return e,
            }
        } else {
            lower
        };
        let (lo, hi) = if lower <= upper {
            (lower, upper)
        } else {
            (upper, lower)
        };
        let mut result = 0.0;
        for (x, p) in x_range.iter().zip(prob_range.iter()) {
            if *x >= lo && *x <= hi {
                result += *p;
            }
        }
        CalcResult::Number(result)
    }

    pub(crate) fn fn_trimmean(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let mut values = match self.get_array_of_numbers(&args[0], &cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let percent = match self.get_number(&args[1], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if !(0.0..1.0).contains(&percent) {
            return CalcResult::new_error(Error::NUM, cell, "Invalid percent".to_string());
        }
        if values.is_empty() {
            return CalcResult::new_error(Error::NUM, cell, "Empty array".to_string());
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = values.len();
        let n_trim = (((n as f64) * percent / 2.0).trunc() as usize) * 2;
        let half = n_trim / 2;
        let kept = &values[half..n - half];
        let sum: f64 = kept.iter().sum();
        CalcResult::Number(sum / (kept.len() as f64))
    }
}
