use chrono::Datelike;

use crate::{
    calc_result::CalcResult,
    constants::{LAST_COLUMN, LAST_ROW, MAXIMUM_DATE_SERIAL_NUMBER, MINIMUM_DATE_SERIAL_NUMBER},
    expressions::{parser::Node, token::Error, types::CellReferenceIndex},
    formatter::dates::{date_to_serial_number, from_excel_date},
    model::Model,
};

use super::financial_util::{compute_irr, compute_npv, compute_rate, compute_xirr, compute_xnpv};

// See:
// https://github.com/apache/openoffice/blob/c014b5f2b55cff8d4b0c952d5c16d62ecde09ca1/main/scaddins/source/analysis/financial.cxx

fn is_less_than_one_year(start_date: i64, end_date: i64) -> Result<bool, String> {
    let end = from_excel_date(end_date)?;
    let start = from_excel_date(start_date)?;
    if end_date - start_date < 365 {
        return Ok(true);
    }
    let end_year = end.year();
    let start_year = start.year();
    if end_year == start_year {
        return Ok(true);
    }
    if end_year != start_year + 1 {
        return Ok(false);
    }
    let start_month = start.month();
    let end_month = end.month();
    if end_month < start_month {
        return Ok(true);
    }
    if end_month > start_month {
        return Ok(false);
    }
    // we are one year later same month
    let start_day = start.day();
    let end_day = end.day();
    Ok(end_day <= start_day)
}

fn vdb_ddb(cost: f64, salvage: f64, life: f64, period: f64, factor: f64) -> f64 {
    let mut rate = factor / life;
    if rate > 1.0 {
        rate = 1.0;
    }
    let value = if rate == 1.0 {
        if period == 1.0 {
            cost
        } else {
            0.0
        }
    } else {
        cost * (1.0 - rate).powf(period - 1.0)
    };
    let new_value = cost * (1.0 - rate).powf(period);
    f64::max(value - f64::max(salvage, new_value), 0.0)
}

fn vdb_total(cost: f64, salvage: f64, life: f64, period: f64, factor: f64, no_switch: bool) -> f64 {
    let int_end = period.ceil();
    let n = int_end as i64;
    if n <= 0 {
        return 0.0;
    }
    let mut vdb = 0.0;
    if no_switch {
        for i in 1..=n {
            let gda = vdb_ddb(cost, salvage, life, i as f64, factor);
            let term = if i == n {
                gda * (period + 1.0 - int_end)
            } else {
                gda
            };
            vdb += term;
        }
    } else {
        let mut restwert = cost - salvage;
        let mut now_lia = false;
        let mut lia = 0.0;
        for i in 1..=n {
            let mut term;
            if !now_lia {
                let gda = vdb_ddb(cost, salvage, life, i as f64, factor);
                lia = restwert / (life - (i as f64 - 1.0));
                if lia > gda {
                    term = lia;
                    now_lia = true;
                } else {
                    term = gda;
                    restwert -= gda;
                }
            } else {
                term = lia;
            }
            if i == n {
                term *= period + 1.0 - int_end;
            }
            vdb += term;
        }
    }
    vdb
}

fn compute_payment(
    rate: f64,
    nper: f64,
    pv: f64,
    fv: f64,
    period_start: bool,
) -> Result<f64, (Error, String)> {
    if rate == 0.0 {
        if nper == 0.0 {
            return Err((Error::NUM, "Period count must be non zero".to_string()));
        }
        return Ok(-(pv + fv) / nper);
    }
    if rate <= -1.0 {
        return Err((Error::NUM, "Rate must be > -1".to_string()));
    };
    let rate_nper = if nper == 0.0 {
        1.0
    } else {
        (1.0 + rate).powf(nper)
    };
    let result = if period_start {
        // type = 1
        (fv + pv * rate_nper) * rate / ((1.0 + rate) * (1.0 - rate_nper))
    } else {
        (fv * rate + pv * rate * rate_nper) / (1.0 - rate_nper)
    };
    if result.is_nan() || result.is_infinite() {
        return Err((Error::NUM, "Invalid result".to_string()));
    }
    Ok(result)
}

fn compute_future_value(
    rate: f64,
    nper: f64,
    pmt: f64,
    pv: f64,
    period_start: bool,
) -> Result<f64, (Error, String)> {
    if rate == 0.0 {
        return Ok(-pv - pmt * nper);
    }
    if rate == -1.0 && nper < 0.0 {
        return Err((Error::DIV, "Divide by zero".to_string()));
    }

    let rate_nper = (1.0 + rate).powf(nper);
    let fv = if period_start {
        // type = 1
        -pv * rate_nper - pmt * (1.0 + rate) * (rate_nper - 1.0) / rate
    } else {
        -pv * rate_nper - pmt * (rate_nper - 1.0) / rate
    };
    if fv.is_nan() {
        return Err((Error::NUM, "Invalid result".to_string()));
    }
    if !fv.is_finite() {
        return Err((Error::DIV, "Divide by zero".to_string()));
    }
    Ok(fv)
}

fn compute_ipmt(
    rate: f64,
    period: f64,
    period_count: f64,
    present_value: f64,
    future_value: f64,
    period_start: bool,
) -> Result<f64, (Error, String)> {
    // http://www.staff.city.ac.uk/o.s.kerr/CompMaths/WSheet4.pdf
    // https://www.experts-exchange.com/articles/1948/A-Guide-to-the-PMT-FV-IPMT-and-PPMT-Functions.html
    // type = 0 (end of period)
    // impt = -[(1+rate)^(period-1)*(pv*rate+pmt)-pmt]
    // ipmt = FV(rate, period-1, payment, pv, type) * rate
    // type = 1 (beginning of period)
    // ipmt = (FV(rate, period-2, payment, pv, type) - payment) * rate
    let payment = compute_payment(
        rate,
        period_count,
        present_value,
        future_value,
        period_start,
    )?;
    if period < 1.0 || period >= period_count + 1.0 {
        return Err((
            Error::NUM,
            format!("Period must be between 1 and {}", period_count + 1.0),
        ));
    }
    if period == 1.0 && period_start {
        Ok(0.0)
    } else {
        let p = if period_start {
            period - 2.0
        } else {
            period - 1.0
        };
        let c = if period_start { -payment } else { 0.0 };
        let fv = compute_future_value(rate, p, payment, present_value, period_start)?;
        Ok((fv + c) * rate)
    }
}

fn compute_ppmt(
    rate: f64,
    period: f64,
    period_count: f64,
    present_value: f64,
    future_value: f64,
    period_start: bool,
) -> Result<f64, (Error, String)> {
    let payment = compute_payment(
        rate,
        period_count,
        present_value,
        future_value,
        period_start,
    )?;
    // It's a bit unfortunate that the first thing compute_ipmt does is compute_payment again
    let ipmt = compute_ipmt(
        rate,
        period,
        period_count,
        present_value,
        future_value,
        period_start,
    )?;
    Ok(payment - ipmt)
}

// These formulas revolve around compound interest and annuities.
// The financial functions pv, rate, nper, pmt and fv:
// rate = interest rate per period
// nper (number of periods) = loan term
// pv (present value) = loan amount
// fv (future value) = cash balance after last payment. Default is 0
// type = the annuity type indicates when payments are due
//         * 0 (default) Payments are made at the end of the period
//         * 1 Payments are made at the beginning of the period (like a lease or rent)
// The variable period_start is true if type is 1
// They are linked by the formulas:
// If rate != 0
//   $pv*(1+rate)^nper+pmt*(1+rate*type)*((1+rate)^nper-1)/rate+fv=0$
// If rate = 0
//   $pmt*nper+pv+fv=0$
// All, except for rate are easily solvable in terms of the others.
// In these formulas the payment (pmt) is normally negative

impl<'a> Model<'a> {
    fn get_array_of_numbers_generic(
        &mut self,
        arg: &Node,
        cell: &CellReferenceIndex,
        accept_number_node: bool,
        handle_empty_cell: impl Fn() -> Result<Option<f64>, CalcResult>,
        handle_non_number_cell: impl Fn() -> Result<Option<f64>, CalcResult>,
    ) -> Result<Vec<f64>, CalcResult> {
        let mut values = Vec::new();
        match self.evaluate_node_in_context(arg, *cell) {
            CalcResult::Number(value) if accept_number_node => values.push(value),
            CalcResult::Number(_) => {
                return Err(CalcResult::new_error(
                    Error::VALUE,
                    *cell,
                    "Expected range of numbers".to_string(),
                ));
            }
            CalcResult::Range { left, right } => {
                if left.sheet != right.sheet {
                    return Err(CalcResult::new_error(
                        Error::VALUE,
                        *cell,
                        "Ranges are in different sheets".to_string(),
                    ));
                }
                let sheet = left.sheet;
                let row1 = left.row;
                let mut row2 = right.row;
                let column1 = left.column;
                let mut column2 = right.column;
                if row1 == 1 && row2 == LAST_ROW {
                    row2 = self
                        .workbook
                        .worksheet(sheet)
                        .map_err(|_| {
                            CalcResult::new_error(
                                Error::ERROR,
                                *cell,
                                format!("Invalid worksheet index: '{sheet}'"),
                            )
                        })?
                        .dimension()
                        .max_row;
                }
                if column1 == 1 && column2 == LAST_COLUMN {
                    column2 = self
                        .workbook
                        .worksheet(sheet)
                        .map_err(|_| {
                            CalcResult::new_error(
                                Error::ERROR,
                                *cell,
                                format!("Invalid worksheet index: '{sheet}'"),
                            )
                        })?
                        .dimension()
                        .max_column;
                }
                for row in row1..=row2 {
                    for column in column1..=column2 {
                        let cell_ref = CellReferenceIndex { sheet, row, column };
                        match self.evaluate_cell(cell_ref) {
                            CalcResult::Number(value) => values.push(value),
                            error @ CalcResult::Error { .. } => return Err(error),
                            CalcResult::EmptyCell => {
                                if let Some(value) = handle_empty_cell()? {
                                    values.push(value);
                                }
                            }
                            _ => {
                                if let Some(value) = handle_non_number_cell()? {
                                    values.push(value);
                                }
                            }
                        }
                    }
                }
            }
            error @ CalcResult::Error { .. } => return Err(error),
            _ => {
                handle_non_number_cell()?;
            }
        }
        Ok(values)
    }

    pub(crate) fn get_array_of_numbers(
        &mut self,
        arg: &Node,
        cell: &CellReferenceIndex,
    ) -> Result<Vec<f64>, CalcResult> {
        self.get_array_of_numbers_generic(
            arg,
            cell,
            true,        // accept_number_node
            || Ok(None), // Ignore empty cells
            || Ok(None), // Ignore non-number cells
        )
    }

    fn get_array_of_numbers_xpnv(
        &mut self,
        arg: &Node,
        cell: &CellReferenceIndex,
        error: Error,
    ) -> Result<Vec<f64>, CalcResult> {
        self.get_array_of_numbers_generic(
            arg,
            cell,
            true, // accept_number_node
            || {
                Err(CalcResult::new_error(
                    Error::NUM,
                    *cell,
                    "Expected number".to_string(),
                ))
            },
            || {
                Err(CalcResult::new_error(
                    error.clone(),
                    *cell,
                    "Expected number".to_string(),
                ))
            },
        )
    }

    fn get_array_of_numbers_xirr(
        &mut self,
        arg: &Node,
        cell: &CellReferenceIndex,
    ) -> Result<Vec<f64>, CalcResult> {
        self.get_array_of_numbers_generic(
            arg,
            cell,
            false,            // Do not accept a single number node
            || Ok(Some(0.0)), // Treat empty cells as zero
            || {
                Err(CalcResult::new_error(
                    Error::VALUE,
                    *cell,
                    "Expected number".to_string(),
                ))
            },
        )
    }

    /// PMT(rate, nper, pv, [fv], [type])
    pub(crate) fn fn_pmt(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(3..=5).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // number of periods
        let nper = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // present value
        let pv = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // future_value
        let fv = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.0
        };
        let period_start = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f != 0.0,
                Err(s) => return s,
            }
        } else {
            // at the end of the period
            false
        };
        match compute_payment(rate, nper, pv, fv, period_start) {
            Ok(p) => CalcResult::Number(p),
            Err(error) => CalcResult::Error {
                error: error.0,
                origin: cell,
                message: error.1,
            },
        }
    }

    // PV(rate, nper, pmt, [fv], [type])
    pub(crate) fn fn_pv(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(3..=5).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // nper
        let period_count = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // pmt
        let payment = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // fv
        let future_value = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.0
        };
        let period_start = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f != 0.0,
                Err(s) => return s,
            }
        } else {
            // at the end of the period
            false
        };
        if rate == 0.0 {
            return CalcResult::Number(-future_value - payment * period_count);
        }
        if rate == -1.0 {
            return CalcResult::Error {
                error: Error::DIV,
                origin: cell,
                message: "Rate must be != -1".to_string(),
            };
        };
        let rate_nper = (1.0 + rate).powf(period_count);
        let result = if period_start {
            // type = 1
            -(future_value * rate + payment * (1.0 + rate) * (rate_nper - 1.0)) / (rate * rate_nper)
        } else {
            (-future_value * rate - payment * (rate_nper - 1.0)) / (rate * rate_nper)
        };
        if result.is_nan() || result.is_infinite() {
            return CalcResult::Error {
                error: Error::NUM,
                origin: cell,
                message: "Invalid result".to_string(),
            };
        }

        CalcResult::Number(result)
    }

    // RATE(nper, pmt, pv, [fv], [type], [guess])
    pub(crate) fn fn_rate(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(3..=5).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let nper = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pmt = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pv = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // fv
        let fv = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.0
        };
        let annuity_type = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => i32::from(f != 0.0),
                Err(s) => return s,
            }
        } else {
            // at the end of the period
            0
        };

        let guess = if arg_count > 5 {
            match self.get_number(&args[5], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.1
        };

        match compute_rate(pv, fv, nper, pmt, annuity_type, guess) {
            Ok(f) => CalcResult::Number(f),
            Err(error) => CalcResult::Error {
                error: error.0,
                origin: cell,
                message: error.1,
            },
        }
    }

    // NPER(rate,pmt,pv,[fv],[type])
    pub(crate) fn fn_nper(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(3..=5).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // pmt
        let payment = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // pv
        let present_value = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // fv
        let future_value = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.0
        };
        let period_start = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f != 0.0,
                Err(s) => return s,
            }
        } else {
            // at the end of the period
            false
        };
        if rate == 0.0 {
            if payment == 0.0 {
                return CalcResult::Error {
                    error: Error::DIV,
                    origin: cell,
                    message: "Divide by zero".to_string(),
                };
            }
            return CalcResult::Number(-(future_value + present_value) / payment);
        }
        if rate < -1.0 {
            return CalcResult::Error {
                error: Error::NUM,
                origin: cell,
                message: "Rate must be > -1".to_string(),
            };
        };
        let rate_nper = if period_start {
            // type = 1
            if payment != 0.0 {
                let term = payment * (1.0 + rate) / rate;
                (1.0 - future_value / term) / (1.0 + present_value / term)
            } else {
                -future_value / present_value
            }
        } else {
            // type = 0
            if payment != 0.0 {
                let term = payment / rate;
                (1.0 - future_value / term) / (1.0 + present_value / term)
            } else {
                -future_value / present_value
            }
        };
        if rate_nper <= 0.0 {
            return CalcResult::Error {
                error: Error::NUM,
                origin: cell,
                message: "Cannot compute.".to_string(),
            };
        }
        let result = rate_nper.ln() / (1.0 + rate).ln();
        CalcResult::Number(result)
    }

    // FV(rate, nper, pmt, [pv], [type])
    pub(crate) fn fn_fv(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(3..=5).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // number of periods
        let nper = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // payment
        let pmt = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // present value
        let pv = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.0
        };
        let period_start = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f != 0.0,
                Err(s) => return s,
            }
        } else {
            // at the end of the period
            false
        };
        match compute_future_value(rate, nper, pmt, pv, period_start) {
            Ok(f) => CalcResult::Number(f),
            Err(error) => CalcResult::Error {
                error: error.0,
                origin: cell,
                message: error.1,
            },
        }
    }

    // FVSCHEDULE(principal, schedule)
    pub(crate) fn fn_fvschedule(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let principal = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let schedule = match self.get_array_of_numbers(&args[1], &cell) {
            Ok(values) => values,
            Err(s) => return s,
        };
        let mut result = principal;
        for rate in schedule {
            result *= 1.0 + rate;
        }
        CalcResult::Number(result)
    }

    // IPMT(rate, per, nper, pv, [fv], [type])
    pub(crate) fn fn_ipmt(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(4..=6).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // per
        let period = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // nper
        let period_count = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // pv
        let present_value = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // fv
        let future_value = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.0
        };
        let period_start = if arg_count > 5 {
            match self.get_number(&args[5], cell) {
                Ok(f) => f != 0.0,
                Err(s) => return s,
            }
        } else {
            // at the end of the period
            false
        };
        let ipmt = match compute_ipmt(
            rate,
            period,
            period_count,
            present_value,
            future_value,
            period_start,
        ) {
            Ok(f) => f,
            Err(error) => {
                return CalcResult::Error {
                    error: error.0,
                    origin: cell,
                    message: error.1,
                }
            }
        };
        CalcResult::Number(ipmt)
    }

    // PPMT(rate, per, nper, pv, [fv], [type])
    pub(crate) fn fn_ppmt(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(4..=6).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // per
        let period = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // nper
        let period_count = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // pv
        let present_value = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // fv
        let future_value = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.0
        };
        let period_start = if arg_count > 5 {
            match self.get_number(&args[5], cell) {
                Ok(f) => f != 0.0,
                Err(s) => return s,
            }
        } else {
            // at the end of the period
            false
        };

        let ppmt = match compute_ppmt(
            rate,
            period,
            period_count,
            present_value,
            future_value,
            period_start,
        ) {
            Ok(f) => f,
            Err(error) => {
                return CalcResult::Error {
                    error: error.0,
                    origin: cell,
                    message: error.1,
                }
            }
        };
        CalcResult::Number(ppmt)
    }

    // NPV(rate, value1, [value2],...)
    // npv = Sum[value[i]/(1+rate)^i, {i, 1, n}]
    pub(crate) fn fn_npv(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if arg_count < 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let mut values = Vec::new();
        for arg in &args[1..] {
            match self.evaluate_node_in_context(arg, cell) {
                CalcResult::Number(value) => values.push(value),
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
                                    // We ignore booleans and strings
                                }
                            }
                        }
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                _ => {
                    // We ignore booleans and strings
                }
            };
        }
        match compute_npv(rate, &values) {
            Ok(f) => CalcResult::Number(f),
            Err(error) => CalcResult::new_error(error.0, cell, error.1),
        }
    }

    // Returns the internal rate of return for a series of cash flows represented by the numbers
    // in values.
    // These cash flows do not have to be even, as they would be for an annuity.
    // However, the cash flows must occur at regular intervals, such as monthly or annually.
    // The internal rate of return is the interest rate received for an investment consisting
    // of payments (negative values) and income (positive values) that occur at regular periods

    // IRR(values, [guess])
    pub(crate) fn fn_irr(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if arg_count > 2 || arg_count == 0 {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.get_array_of_numbers(&args[0], &cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let guess = if arg_count == 2 {
            match self.get_number(&args[1], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.1
        };
        match compute_irr(&values, guess) {
            Ok(f) => CalcResult::Number(f),
            Err(error) => CalcResult::Error {
                error: error.0,
                origin: cell,
                message: error.1,
            },
        }
    }

    // XNPV(rate, values, dates)
    pub(crate) fn fn_xnpv(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(2..=3).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let values = match self.get_array_of_numbers_xpnv(&args[1], &cell, Error::NUM) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let dates = match self.get_array_of_numbers_xpnv(&args[2], &cell, Error::VALUE) {
            Ok(s) => s,
            Err(error) => return error,
        };
        // Decimal points on dates are truncated
        let dates: Vec<f64> = dates.iter().map(|s| s.floor()).collect();
        let values_count = values.len();
        // If values and dates contain a different number of values, XNPV returns the #NUM! error value.
        if values_count != dates.len() {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "Values and dates must be the same length".to_string(),
            );
        }
        if values_count == 0 {
            return CalcResult::new_error(Error::NUM, cell, "Not enough values".to_string());
        }
        let first_date = dates[0];
        for date in &dates {
            if *date < MINIMUM_DATE_SERIAL_NUMBER as f64
                || *date > MAXIMUM_DATE_SERIAL_NUMBER as f64
            {
                // Excel docs claim that if any number in dates is not a valid date,
                // XNPV returns the #VALUE! error value, but it seems to return #VALUE!
                return CalcResult::new_error(
                    Error::NUM,
                    cell,
                    "Invalid number for date".to_string(),
                );
            }
            // If any number in dates precedes the starting date, XNPV returns the #NUM! error value.
            if date < &first_date {
                return CalcResult::new_error(
                    Error::NUM,
                    cell,
                    "Date precedes the starting date".to_string(),
                );
            }
        }
        // It seems Excel returns #NUM! if rate < 0, this is only necessary if r <= -1
        if rate <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "rate needs to be > 0".to_string());
        }
        match compute_xnpv(rate, &values, &dates) {
            Ok(f) => CalcResult::Number(f),
            Err((error, message)) => CalcResult::new_error(error, cell, message),
        }
    }

    // XIRR(values, dates, [guess])
    pub(crate) fn fn_xirr(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(2..=3).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.get_array_of_numbers_xirr(&args[0], &cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let dates = match self.get_array_of_numbers_xirr(&args[1], &cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let guess = if arg_count == 3 {
            match self.get_number(&args[2], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            0.1
        };
        // Decimal points on dates are truncated
        let dates: Vec<f64> = dates.iter().map(|s| s.floor()).collect();
        let values_count = values.len();
        // If values and dates contain a different number of values, XNPV returns the #NUM! error value.
        if values_count != dates.len() {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "Values and dates must be the same length".to_string(),
            );
        }
        if values_count == 0 {
            return CalcResult::new_error(Error::NUM, cell, "Not enough values".to_string());
        }
        let first_date = dates[0];
        for date in &dates {
            if *date < MINIMUM_DATE_SERIAL_NUMBER as f64
                || *date > MAXIMUM_DATE_SERIAL_NUMBER as f64
            {
                return CalcResult::new_error(
                    Error::NUM,
                    cell,
                    "Invalid number for date".to_string(),
                );
            }
            // If any number in dates precedes the starting date, XIRR returns the #NUM! error value.
            if date < &first_date {
                return CalcResult::new_error(
                    Error::NUM,
                    cell,
                    "Date precedes the starting date".to_string(),
                );
            }
        }
        match compute_xirr(&values, &dates, guess) {
            Ok(f) => CalcResult::Number(f),
            Err((error, message)) => CalcResult::Error {
                error,
                origin: cell,
                message,
            },
        }
    }

    //  MIRR(values, finance_rate, reinvest_rate)
    // The formula is:
    // $$ (-NPV(r1, v_p) * (1+r1)^y)/(NPV(r2, v_n)*(1+r2))^(1/y)-1$$
    // where:
    // $r1$ is the reinvest_rate, $r2$ the finance_rate
    // $v_p$ the vector of positive values
    // $v_n$ the vector of negative values
    // and $y$ is dimension of $v$ - 1 (number of years)
    pub(crate) fn fn_mirr(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let values = match self.get_array_of_numbers(&args[0], &cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let finance_rate = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let reinvest_rate = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let mut positive_values = Vec::new();
        let mut negative_values = Vec::new();
        let mut last_negative_index = -1;
        for (index, &value) in values.iter().enumerate() {
            let (p, n) = if value >= 0.0 {
                (value, 0.0)
            } else {
                last_negative_index = index as i32;
                (0.0, value)
            };
            positive_values.push(p);
            negative_values.push(n);
        }
        if last_negative_index == -1 {
            return CalcResult::new_error(
                Error::DIV,
                cell,
                "Invalid data for MIRR function".to_string(),
            );
        }
        // We do a bit of analysis if the rates are -1 as there are some cancellations
        // It is probably not important.
        let years = values.len() as f64;
        let top = if reinvest_rate == -1.0 {
            // This is finite
            match positive_values.last() {
                Some(f) => *f,
                None => 0.0,
            }
        } else {
            match compute_npv(reinvest_rate, &positive_values) {
                Ok(npv) => -npv * ((1.0 + reinvest_rate).powf(years)),
                Err((error, message)) => {
                    return CalcResult::Error {
                        error,
                        origin: cell,
                        message,
                    }
                }
            }
        };
        let bottom = if finance_rate == -1.0 {
            if last_negative_index == 0 {
                // This is still finite
                negative_values[last_negative_index as usize]
            } else {
                // or -Infinity depending of the sign in the last_negative_index coef.
                // But it is irrelevant for the calculation
                f64::INFINITY
            }
        } else {
            match compute_npv(finance_rate, &negative_values) {
                Ok(npv) => npv * (1.0 + finance_rate),
                Err((error, message)) => {
                    return CalcResult::Error {
                        error,
                        origin: cell,
                        message,
                    }
                }
            }
        };

        let result = (top / bottom).powf(1.0 / (years - 1.0)) - 1.0;
        if result.is_infinite() {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        if result.is_nan() {
            return CalcResult::new_error(Error::NUM, cell, "Invalid data for MIRR".to_string());
        }
        CalcResult::Number(result)
    }

    // ISPMT(rate, per, nper, pv)
    // Formula is:
    // $$pv*rate*\left(\frac{per}{nper}-1\right)$$
    pub(crate) fn fn_ispmt(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let per = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let nper = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pv = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        if nper == 0.0 {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        CalcResult::Number(pv * rate * (per / nper - 1.0))
    }

    // RRI(nper, pv, fv)
    // Formula is
    // $$ \left(\frac{fv}{pv}\right)^{\frac{1}{nper}}-1  $$
    pub(crate) fn fn_rri(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let nper = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pv = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let fv = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        if nper <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "nper should be >0".to_string());
        }
        if pv == 0.0 {
            // Note error is NUM not DIV/0 also bellow
            return CalcResult::new_error(Error::NUM, cell, "Division by 0".to_string());
        }
        let result = (fv / pv).powf(1.0 / nper) - 1.0;
        if result.is_infinite() {
            return CalcResult::new_error(Error::NUM, cell, "Division by 0".to_string());
        }
        if result.is_nan() {
            return CalcResult::new_error(Error::NUM, cell, "Invalid data for RRI".to_string());
        }

        CalcResult::Number(result)
    }

    // SLN(cost, salvage, life)
    // Formula is:
    // $$ \frac{cost-salvage}{life} $$
    pub(crate) fn fn_sln(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let cost = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let salvage = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let life = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        if life == 0.0 {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        let result = (cost - salvage) / life;

        CalcResult::Number(result)
    }

    // SYD(cost, salvage, life, per)
    // Formula is:
    // $$ \frac{(cost-salvage)*(life-per+1)*2}{life*(life+1)} $$
    pub(crate) fn fn_syd(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let cost = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let salvage = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let life = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let per = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        if life == 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "Division by 0".to_string());
        }
        if per > life || per <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "per should be <= life".to_string());
        }
        let result = ((cost - salvage) * (life - per + 1.0) * 2.0) / (life * (life + 1.0));

        CalcResult::Number(result)
    }

    // NOMINAL(effective_rate, npery)
    // Formula is:
    // $$ n\times\left(\left(1+r\right)^{\frac{1}{n}}-1\right) $$
    // where:
    //   $r$ is the effective interest rate
    //   $n$ is the number of periods per year
    pub(crate) fn fn_nominal(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let effect_rate = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let npery = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f.floor(),
            Err(s) => return s,
        };
        if effect_rate <= 0.0 || npery < 1.0 {
            return CalcResult::new_error(Error::NUM, cell, "Invalid arguments".to_string());
        }
        let result = ((1.0 + effect_rate).powf(1.0 / npery) - 1.0) * npery;
        if result.is_infinite() {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        if result.is_nan() {
            return CalcResult::new_error(Error::NUM, cell, "Invalid data for RRI".to_string());
        }

        CalcResult::Number(result)
    }

    // EFFECT(nominal_rate, npery)
    // Formula is:
    // $$ \left(1+\frac{r}{n}\right)^n-1 $$
    // where:
    //   $r$ is the nominal interest rate
    //   $n$ is the number of periods per year
    pub(crate) fn fn_effect(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let nominal_rate = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let npery = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f.floor(),
            Err(s) => return s,
        };
        if nominal_rate <= 0.0 || npery < 1.0 {
            return CalcResult::new_error(Error::NUM, cell, "Invalid arguments".to_string());
        }
        let result = (1.0 + nominal_rate / npery).powf(npery) - 1.0;
        if result.is_infinite() {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        if result.is_nan() {
            return CalcResult::new_error(Error::NUM, cell, "Invalid data for RRI".to_string());
        }

        CalcResult::Number(result)
    }

    // PDURATION(rate, pv, fv)
    // Formula is:
    // $$ \frac{log(fv) - log(pv)}{log(1+r)} $$
    // where:
    //   * $r$ is the interest rate per period
    //   * $pv$ is the present value of the investment
    //   * $fv$ is the desired future value of the investment
    pub(crate) fn fn_pduration(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pv = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let fv = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        if fv <= 0.0 || pv <= 0.0 || rate <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "Invalid arguments".to_string());
        }
        let result = (fv.ln() - pv.ln()) / ((1.0 + rate).ln());
        if result.is_infinite() {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        if result.is_nan() {
            return CalcResult::new_error(Error::NUM, cell, "Invalid data for RRI".to_string());
        }

        CalcResult::Number(result)
    }

    // This next three functions deal with Treasure Bills or T-Bills for short
    // They are zero-coupon that mature in one year or less.
    //  Definitions:
    //    $r$ be the discount rate
    //    $v$ the face value of the Bill
    //    $p$ the price of the Bill
    //    $d_m$ is the number of days from the settlement to maturity
    // Then:
    //   $$ p = v \times\left(1-\frac{d_m}{r}\right) $$
    // If d_m is less than 183 days the he Bond Equivalent Yield (BEY, here $y$) is given by:
    // $$ y = \frac{F - B}{M}\times \frac{365}{d_m} = \frac{365\times r}{360-r\times d_m}
    // If d_m>= 183 days things are a bit more complicated.
    // Let $d_e = d_m - 365/2$ if $d_m <= 365$ or $d_e = 183$ if $d_m = 366$.
    // $$ v = p\times \left(1+\frac{y}{2}\right)\left(1+d_e\times\frac{y}{365}\right) $$
    // Together with the previous relation of $p$ and $v$ gives us a quadratic equation for $y$.

    // TBILLEQ(settlement, maturity, discount)
    pub(crate) fn fn_tbilleq(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let settlement = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let maturity = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let discount = match self.get_number_no_bools(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let less_than_one_year = match is_less_than_one_year(settlement as i64, maturity as i64) {
            Ok(f) => f,
            Err(_) => return CalcResult::new_error(Error::NUM, cell, "Invalid date".to_string()),
        };
        if settlement > maturity {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "settlement should be <= maturity".to_string(),
            );
        }
        if !less_than_one_year {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "maturity <= settlement + year".to_string(),
            );
        }
        if discount <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "discount should be >0".to_string());
        }
        // days to maturity
        let d_m = maturity - settlement;
        let result = if d_m < 183.0 {
            365.0 * discount / (360.0 - discount * d_m)
        } else {
            // Equation here is:
            // (1-days*rate/360)*(1+y/2)*(1+d_extra*y/year)=1
            let year = if d_m == 366.0 { 366.0 } else { 365.0 };
            let d_extra = d_m - year / 2.0;
            let alpha = 1.0 - d_m * discount / 360.0;
            let beta = 0.5 + d_extra / year;
            // ay^2+by+c=0
            let a = d_extra * alpha / (year * 2.0);
            let b = alpha * beta;
            let c = alpha - 1.0;
            (-b + (b * b - 4.0 * a * c).sqrt()) / (2.0 * a)
        };
        if result.is_infinite() {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        if result.is_nan() {
            return CalcResult::new_error(Error::NUM, cell, "Invalid data for RRI".to_string());
        }

        CalcResult::Number(result)
    }

    // TBILLPRICE(settlement, maturity, discount)
    pub(crate) fn fn_tbillprice(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let settlement = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let maturity = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let discount = match self.get_number_no_bools(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let less_than_one_year = match is_less_than_one_year(settlement as i64, maturity as i64) {
            Ok(f) => f,
            Err(_) => return CalcResult::new_error(Error::NUM, cell, "Invalid date".to_string()),
        };
        if settlement > maturity {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "settlement should be <= maturity".to_string(),
            );
        }
        if !less_than_one_year {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "maturity <= settlement + year".to_string(),
            );
        }
        if discount <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "discount should be >0".to_string());
        }
        // days to maturity
        let d_m = maturity - settlement;
        let result = 100.0 * (1.0 - discount * d_m / 360.0);
        if result.is_infinite() {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        if result.is_nan() || result < 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "Invalid data for RRI".to_string());
        }

        CalcResult::Number(result)
    }

    // TBILLYIELD(settlement, maturity, pr)
    pub(crate) fn fn_tbillyield(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let settlement = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let maturity = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pr = match self.get_number_no_bools(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let less_than_one_year = match is_less_than_one_year(settlement as i64, maturity as i64) {
            Ok(f) => f,
            Err(_) => return CalcResult::new_error(Error::NUM, cell, "Invalid date".to_string()),
        };
        if settlement > maturity {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "settlement should be <= maturity".to_string(),
            );
        }
        if !less_than_one_year {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "maturity <= settlement + year".to_string(),
            );
        }
        if pr <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "discount should be >0".to_string());
        }
        let days = maturity - settlement;
        let result = (100.0 - pr) * 360.0 / (pr * days);

        CalcResult::Number(result)
    }

    // DOLLARDE(fractional_dollar, fraction)
    pub(crate) fn fn_dollarde(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let fractional_dollar = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let mut fraction = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        if fraction < 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "fraction should be >= 1".to_string());
        }
        if fraction < 1.0 {
            // this is not necessarily DIV/0
            return CalcResult::new_error(Error::DIV, cell, "fraction should be >= 1".to_string());
        }
        fraction = fraction.trunc();
        while fraction > 10.0 {
            fraction /= 10.0;
        }
        let t = fractional_dollar.trunc();
        let result = t + (fractional_dollar - t) * 10.0 / fraction;
        CalcResult::Number(result)
    }

    // DOLLARFR(decimal_dollar, fraction)
    pub(crate) fn fn_dollarfr(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let decimal_dollar = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let mut fraction = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        if fraction < 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "fraction should be >= 1".to_string());
        }
        if fraction < 1.0 {
            // this is not necessarily DIV/0
            return CalcResult::new_error(Error::DIV, cell, "fraction should be >= 1".to_string());
        }
        fraction = fraction.trunc();
        while fraction > 10.0 {
            fraction /= 10.0;
        }
        let t = decimal_dollar.trunc();
        let result = t + (decimal_dollar - t) * fraction / 10.0;
        CalcResult::Number(result)
    }

    // CUMIPMT(rate, nper, pv, start_period, end_period, type)
    pub(crate) fn fn_cumipmt(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 6 {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let nper = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pv = match self.get_number_no_bools(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let start_period = match self.get_number_no_bools(&args[3], cell) {
            Ok(f) => f.ceil() as i32,
            Err(s) => return s,
        };
        let end_period = match self.get_number_no_bools(&args[4], cell) {
            Ok(f) => f.trunc() as i32,
            Err(s) => return s,
        };
        // 0 at the end of the period, 1 at the beginning of the period
        let period_type = match self.get_number_no_bools(&args[5], cell) {
            Ok(f) => {
                if f == 0.0 {
                    false
                } else if f == 1.0 {
                    true
                } else {
                    return CalcResult::new_error(
                        Error::NUM,
                        cell,
                        "invalid period type".to_string(),
                    );
                }
            }
            Err(s) => return s,
        };
        if start_period > end_period {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "start period should come before end period".to_string(),
            );
        }
        if rate <= 0.0 || nper <= 0.0 || pv <= 0.0 || start_period < 1 {
            return CalcResult::new_error(Error::NUM, cell, "invalid parameters".to_string());
        }
        let mut result = 0.0;
        for period in start_period..=end_period {
            result += match compute_ipmt(rate, period as f64, nper, pv, 0.0, period_type) {
                Ok(f) => f,
                Err(error) => {
                    return CalcResult::Error {
                        error: error.0,
                        origin: cell,
                        message: error.1,
                    }
                }
            }
        }
        CalcResult::Number(result)
    }

    // CUMPRINC(rate, nper, pv, start_period, end_period, type)
    pub(crate) fn fn_cumprinc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 6 {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number_no_bools(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let nper = match self.get_number_no_bools(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let pv = match self.get_number_no_bools(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let start_period = match self.get_number_no_bools(&args[3], cell) {
            Ok(f) => f.ceil() as i32,
            Err(s) => return s,
        };
        let end_period = match self.get_number_no_bools(&args[4], cell) {
            Ok(f) => f.trunc() as i32,
            Err(s) => return s,
        };
        // 0 at the end of the period, 1 at the beginning of the period
        let period_type = match self.get_number_no_bools(&args[5], cell) {
            Ok(f) => {
                if f == 0.0 {
                    false
                } else if f == 1.0 {
                    true
                } else {
                    return CalcResult::new_error(
                        Error::NUM,
                        cell,
                        "invalid period type".to_string(),
                    );
                }
            }
            Err(s) => return s,
        };
        if start_period > end_period {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "start period should come before end period".to_string(),
            );
        }
        if rate <= 0.0 || nper <= 0.0 || pv <= 0.0 || start_period < 1 {
            return CalcResult::new_error(Error::NUM, cell, "invalid parameters".to_string());
        }
        let mut result = 0.0;
        for period in start_period..=end_period {
            result += match compute_ppmt(rate, period as f64, nper, pv, 0.0, period_type) {
                Ok(f) => f,
                Err(error) => {
                    return CalcResult::Error {
                        error: error.0,
                        origin: cell,
                        message: error.1,
                    }
                }
            }
        }
        CalcResult::Number(result)
    }

    // DDB(cost, salvage, life, period, [factor])
    pub(crate) fn fn_ddb(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(4..=5).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let cost = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let salvage = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let life = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let period = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        // The rate at which the balance declines.
        let factor = if arg_count > 4 {
            match self.get_number_no_bools(&args[4], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            // If factor is omitted, it is assumed to be 2 (the double-declining balance method).
            2.0
        };
        if period > life || cost < 0.0 || salvage < 0.0 || period <= 0.0 || factor <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "invalid parameters".to_string());
        };
        // let period_trunc = period.floor() as i32;
        let mut rate = factor / life;
        if rate > 1.0 {
            rate = 1.0
        };
        let value = if rate == 1.0 {
            if period == 1.0 {
                cost
            } else {
                0.0
            }
        } else {
            cost * (1.0 - rate).powf(period - 1.0)
        };
        let new_value = cost * (1.0 - rate).powf(period);
        let result = f64::max(value - f64::max(salvage, new_value), 0.0);
        CalcResult::Number(result)
    }

    // DB(cost, salvage, life, period, [month])
    pub(crate) fn fn_db(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(4..=5).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let cost = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let salvage = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let life = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let period = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let month = if arg_count > 4 {
            match self.get_number_no_bools(&args[4], cell) {
                Ok(f) => f.trunc(),
                Err(s) => return s,
            }
        } else {
            12.0
        };
        if month == 12.0 && period > life
            || (period > life + 1.0)
            || month <= 0.0
            || month > 12.0
            || period <= 0.0
            || cost < 0.0
        {
            return CalcResult::new_error(Error::NUM, cell, "invalid parameters".to_string());
        };
        if cost == 0.0 {
            return CalcResult::Number(0.0);
        }
        // rounded to three decimal places
        // FIXME: We should have utilities for this (see to_precision)
        let rate = f64::round((1.0 - f64::powf(salvage / cost, 1.0 / life)) * 1000.0) / 1000.0;

        let mut result = cost * rate * month / 12.0;

        let period = period.floor() as i32;
        let life = life.floor() as i32;

        // Depreciation for the first and last periods is a special case.
        if period == 1 {
            return CalcResult::Number(result);
        };

        for _ in 0..period - 2 {
            result += (cost - result) * rate;
        }

        if period == life + 1 {
            // last period
            return CalcResult::Number((cost - result) * rate * (12.0 - month) / 12.0);
        }

        CalcResult::Number(rate * (cost - result))
    }

    // VDB(cost, salvage, life, start_period, end_period, [factor], [no_switch])
    pub(crate) fn fn_vdb(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(5..=7).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let cost = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let salvage = match self.get_number(&args[1], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let life = match self.get_number(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let start_period = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let end_period = match self.get_number(&args[4], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let factor = if arg_count > 5 {
            match self.get_number_no_bools(&args[5], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            2.0
        };
        let no_switch = if arg_count > 6 {
            match self.get_boolean(&args[6], cell) {
                Ok(b) => b,
                Err(s) => return s,
            }
        } else {
            false
        };
        if cost < 0.0
            || salvage < 0.0
            || life <= 0.0
            || start_period < 0.0
            || end_period < start_period
            || end_period > life
            || factor <= 0.0
        {
            return CalcResult::new_error(Error::NUM, cell, "invalid parameters".to_string());
        }
        let result = vdb_total(cost, salvage, life, end_period, factor, no_switch)
            - vdb_total(cost, salvage, life, start_period, factor, no_switch);
        CalcResult::Number(result)
    }

    // AMORLINC(cost, date_purchased, first_period, salvage, period, rate, [basis])
    pub(crate) fn fn_amorlinc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(6..=7).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let cost = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let date_purchased = match self.get_number(&args[1], cell) {
            Ok(f) => f.floor() as i64,
            Err(s) => return s,
        };
        let first_period = match self.get_number(&args[2], cell) {
            Ok(f) => f.floor() as i64,
            Err(s) => return s,
        };
        let salvage = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let period = match self.get_number(&args[4], cell) {
            Ok(f) => f.trunc(),
            Err(s) => return s,
        };
        let rate = match self.get_number(&args[5], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let basis = if arg_count > 6 {
            match self.get_number(&args[6], cell) {
                Ok(f) => f.floor() as i32,
                Err(s) => return s,
            }
        } else {
            0
        };
        if cost < 0.0 || salvage < 0.0 || rate <= 0.0 || period < 0.0 || !(0..=4).contains(&basis) {
            return CalcResult::new_error(Error::NUM, cell, "invalid parameters".to_string());
        }
        let yf = match self.yearfrac_basis(date_purchased, first_period, basis, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let one_rate = cost * rate;
        let cost_delta = cost - salvage;
        let first_rate = cost * rate * yf;
        let full_periods = ((cost - salvage - first_rate) / one_rate).floor();
        let result = if period == 0.0 {
            first_rate
        } else if period <= full_periods {
            one_rate
        } else if period == full_periods + 1.0 {
            cost_delta - one_rate * full_periods - first_rate
        } else {
            0.0
        };
        CalcResult::Number(result)
    }

    // AMORDEGRC(cost, date_purchased, first_period, salvage, period, rate, [basis])
    pub(crate) fn fn_amordegrc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(6..=7).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let mut cost = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let date_purchased = match self.get_number(&args[1], cell) {
            Ok(f) => f.floor() as i64,
            Err(s) => return s,
        };
        let first_period = match self.get_number(&args[2], cell) {
            Ok(f) => f.floor() as i64,
            Err(s) => return s,
        };
        let salvage = match self.get_number(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let period = match self.get_number(&args[4], cell) {
            Ok(f) => f.trunc(),
            Err(s) => return s,
        };
        let rate = match self.get_number(&args[5], cell) {
            Ok(f) => f,
            Err(s) => return s,
        };
        let basis = if arg_count > 6 {
            match self.get_number(&args[6], cell) {
                Ok(f) => f.floor() as i32,
                Err(s) => return s,
            }
        } else {
            0
        };
        if cost < 0.0 || salvage < 0.0 || rate <= 0.0 || period < 0.0 || !(0..=4).contains(&basis) {
            return CalcResult::new_error(Error::NUM, cell, "invalid parameters".to_string());
        }
        let use_period = 1.0 / rate;
        let amor_coeff = if use_period < 3.0 {
            1.0
        } else if use_period < 5.0 {
            1.5
        } else if use_period <= 6.0 {
            2.0
        } else {
            2.5
        };
        let rate = rate * amor_coeff;
        let yf = match self.yearfrac_basis(date_purchased, first_period, basis, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut n_rate = (yf * rate * cost).round();
        cost -= n_rate;
        let mut rest = cost - salvage;
        let n_per = period as i64;
        for n in 0..n_per {
            n_rate = (rate * cost).round();
            rest -= n_rate;
            if rest < 0.0 {
                match n_per - n {
                    0 | 1 => n_rate = (cost * 0.5).round(),
                    _ => n_rate = 0.0,
                }
            }
            cost -= n_rate;
        }
        CalcResult::Number(n_rate)
    }

    fn discount_security_args(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<(i64, i64, f64, f64, f64), CalcResult> {
        if !(4..=5).contains(&args.len()) {
            return Err(CalcResult::new_args_number_error(cell));
        }
        let settlement = match self.get_number(&args[0], cell) {
            Ok(c) => c.floor() as i64,
            Err(s) => return Err(s),
        };
        let maturity = match self.get_number(&args[1], cell) {
            Ok(c) => c.floor() as i64,
            Err(s) => return Err(s),
        };
        let value_a = match self.get_number_no_bools(&args[2], cell) {
            Ok(f) => f,
            Err(s) => return Err(s),
        };
        let value_b = match self.get_number_no_bools(&args[3], cell) {
            Ok(f) => f,
            Err(s) => return Err(s),
        };
        let basis = if args.len() == 5 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f as i32,
                Err(s) => return Err(s),
            }
        } else {
            0
        };
        if settlement >= maturity {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "settlement should be < maturity".to_string(),
            ));
        }
        let yf = self.yearfrac_basis(settlement, maturity, basis, cell)?;
        Ok((settlement, maturity, value_a, value_b, yf))
    }

    // DISC(settlement, maturity, pr, redemption, [basis])
    pub(crate) fn fn_disc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (_, _, pr, redemption, yf) = match self.discount_security_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if pr <= 0.0 || redemption <= 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "pr and redemption must be >0".to_string(),
            );
        }
        CalcResult::Number((redemption - pr) / redemption / yf)
    }

    // INTRATE(settlement, maturity, investment, redemption, [basis])
    pub(crate) fn fn_intrate(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (_, _, investment, redemption, yf) = match self.discount_security_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if investment <= 0.0 || redemption <= 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "investment and redemption must be >0".to_string(),
            );
        }
        CalcResult::Number((redemption - investment) / investment / yf)
    }

    // RECEIVED(settlement, maturity, investment, discount, [basis])
    pub(crate) fn fn_received(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (_, _, investment, discount, yf) = match self.discount_security_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if investment <= 0.0 || discount <= 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "investment and discount must be >0".to_string(),
            );
        }
        let denom = 1.0 - discount * yf;
        if denom == 0.0 {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        CalcResult::Number(investment / denom)
    }

    // PRICEDISC(settlement, maturity, discount, redemption, [basis])
    pub(crate) fn fn_pricedisc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (_, _, discount, redemption, yf) = match self.discount_security_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if discount <= 0.0 || redemption <= 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "discount and redemption must be >0".to_string(),
            );
        }
        CalcResult::Number(redemption - discount * redemption * yf)
    }

    // YIELDDISC(settlement, maturity, pr, redemption, [basis])
    pub(crate) fn fn_yielddisc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (_, _, pr, redemption, yf) = match self.discount_security_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if pr <= 0.0 || redemption <= 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "pr and redemption must be >0".to_string(),
            );
        }
        CalcResult::Number((redemption - pr) / pr / yf)
    }

    fn coupon_args(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<(i64, i64, i32, i32), CalcResult> {
        if !(3..=4).contains(&args.len()) {
            return Err(CalcResult::new_args_number_error(cell));
        }
        let settlement = match self.get_number(&args[0], cell) {
            Ok(c) => c.floor() as i64,
            Err(s) => return Err(s),
        };
        let maturity = match self.get_number(&args[1], cell) {
            Ok(c) => c.floor() as i64,
            Err(s) => return Err(s),
        };
        let frequency = match self.get_number(&args[2], cell) {
            Ok(f) => f.floor() as i32,
            Err(s) => return Err(s),
        };
        let basis = if args.len() == 4 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f.floor() as i32,
                Err(s) => return Err(s),
            }
        } else {
            0
        };
        if !matches!(frequency, 1 | 2 | 4) {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "frequency must be 1, 2 or 4".to_string(),
            ));
        }
        if !(0..=4).contains(&basis) {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "Invalid basis".to_string(),
            ));
        }
        if settlement >= maturity {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "settlement should be < maturity".to_string(),
            ));
        }
        Ok((settlement, maturity, frequency, basis))
    }

    fn coupon_pcd_ncd_num(
        &self,
        settlement: i64,
        maturity: i64,
        frequency: i32,
        cell: CellReferenceIndex,
    ) -> Result<(i64, i64, i32), CalcResult> {
        let mat = self.excel_date(maturity, cell)?;
        let mat_year = mat.year();
        let mat_month = mat.month() as i32;
        let mat_day = mat.day();
        let mat_eom = mat_day as i32 == coupon_days_in_month(mat_year, mat_month);
        let step = 12 / frequency;
        let mut ncd_serial = maturity;
        let mut k = 0;
        loop {
            let (y, m, d) = coupon_date_back(mat_year, mat_month, mat_day, mat_eom, k, step);
            let serial = match date_to_serial_number(d, m as u32, y) {
                Ok(s) => s as i64,
                Err(_) => {
                    return Err(CalcResult::new_error(
                        Error::NUM,
                        cell,
                        "date out of range".to_string(),
                    ))
                }
            };
            if serial <= settlement {
                return Ok((serial, ncd_serial, k));
            }
            ncd_serial = serial;
            k += 1;
        }
    }

    fn coupon_day_count(
        &mut self,
        start: i64,
        end: i64,
        basis: i32,
        cell: CellReferenceIndex,
    ) -> f64 {
        match basis {
            0 | 4 => {
                let method = if basis == 4 { 1.0 } else { 0.0 };
                let nodes = [
                    Node::NumberKind(start as f64),
                    Node::NumberKind(end as f64),
                    Node::NumberKind(method),
                ];
                if let CalcResult::Number(n) = self.fn_days360(&nodes, cell) {
                    n
                } else {
                    0.0
                }
            }
            _ => (end - start) as f64,
        }
    }

    // COUPPCD(settlement, maturity, frequency, [basis])
    pub(crate) fn fn_couppcd(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, frequency, _) = match self.coupon_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell) {
            Ok((pcd, _, _)) => CalcResult::Number(pcd as f64),
            Err(e) => e,
        }
    }

    // COUPNCD(settlement, maturity, frequency, [basis])
    pub(crate) fn fn_coupncd(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, frequency, _) = match self.coupon_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell) {
            Ok((_, ncd, _)) => CalcResult::Number(ncd as f64),
            Err(e) => e,
        }
    }

    // COUPNUM(settlement, maturity, frequency, [basis])
    pub(crate) fn fn_coupnum(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, frequency, _) = match self.coupon_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell) {
            Ok((_, _, num)) => CalcResult::Number(num as f64),
            Err(e) => e,
        }
    }

    // COUPDAYBS(settlement, maturity, frequency, [basis])
    pub(crate) fn fn_coupdaybs(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, frequency, basis) = match self.coupon_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let pcd = match self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell) {
            Ok((pcd, _, _)) => pcd,
            Err(e) => return e,
        };
        CalcResult::Number(self.coupon_day_count(pcd, settlement, basis, cell))
    }

    // COUPDAYS(settlement, maturity, frequency, [basis])
    pub(crate) fn fn_coupdays(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, frequency, basis) = match self.coupon_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let (pcd, ncd, _) = match self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let result = match basis {
            1 => (ncd - pcd) as f64,
            3 => 365.0 / frequency as f64,
            _ => 360.0 / frequency as f64,
        };
        CalcResult::Number(result)
    }

    // COUPDAYSNC(settlement, maturity, frequency, [basis])
    pub(crate) fn fn_coupdaysnc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, frequency, basis) = match self.coupon_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let ncd = match self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell) {
            Ok((_, ncd, _)) => ncd,
            Err(e) => return e,
        };
        CalcResult::Number(self.coupon_day_count(settlement, ncd, basis, cell))
    }

    fn coupon_price_factors(
        &mut self,
        settlement: i64,
        maturity: i64,
        frequency: i32,
        basis: i32,
        cell: CellReferenceIndex,
    ) -> Result<(f64, f64, f64, f64), CalcResult> {
        let (pcd, ncd, num) = self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell)?;
        let a = self.coupon_day_count(pcd, settlement, basis, cell);
        let dsc = self.coupon_day_count(settlement, ncd, basis, cell);
        let e = match basis {
            1 => (ncd - pcd) as f64,
            3 => 365.0 / frequency as f64,
            _ => 360.0 / frequency as f64,
        };
        Ok((num as f64, dsc, e, a))
    }

    fn coupon_price(
        rate: f64,
        yld: f64,
        redemption: f64,
        frequency: f64,
        n: f64,
        dsc: f64,
        e: f64,
        a: f64,
    ) -> f64 {
        let coupon = 100.0 * rate / frequency;
        let de = dsc / e;
        let factor = 1.0 + yld / frequency;
        let mut price = redemption / factor.powf(n - 1.0 + de);
        let count = n as i64;
        for k in 1..=count {
            price += coupon / factor.powf(k as f64 - 1.0 + de);
        }
        price - coupon * a / e
    }

    // PRICE(settlement, maturity, rate, yld, redemption, frequency, [basis])
    pub(crate) fn fn_price(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(6..=7).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[2], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let yld = match self.get_number(&args[3], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let redemption = match self.get_number(&args[4], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let coupon_args = if args.len() == 7 {
            vec![
                args[0].clone(),
                args[1].clone(),
                args[5].clone(),
                args[6].clone(),
            ]
        } else {
            vec![args[0].clone(), args[1].clone(), args[5].clone()]
        };
        let (settlement, maturity, frequency, basis) = match self.coupon_args(&coupon_args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if rate < 0.0 || yld < 0.0 || redemption <= 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "rate>=0, yld>=0, redemption>0 required".to_string(),
            );
        }
        let (n, dsc, e, a) =
            match self.coupon_price_factors(settlement, maturity, frequency, basis, cell) {
                Ok(v) => v,
                Err(err) => return err,
            };
        CalcResult::Number(Model::coupon_price(
            rate,
            yld,
            redemption,
            frequency as f64,
            n,
            dsc,
            e,
            a,
        ))
    }

    // YIELD(settlement, maturity, rate, pr, redemption, frequency, [basis])
    pub(crate) fn fn_yield(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(6..=7).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[2], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let pr = match self.get_number(&args[3], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let redemption = match self.get_number(&args[4], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let coupon_args = if args.len() == 7 {
            vec![
                args[0].clone(),
                args[1].clone(),
                args[5].clone(),
                args[6].clone(),
            ]
        } else {
            vec![args[0].clone(), args[1].clone(), args[5].clone()]
        };
        let (settlement, maturity, frequency, basis) = match self.coupon_args(&coupon_args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if rate < 0.0 || pr <= 0.0 || redemption <= 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "rate>=0, pr>0, redemption>0 required".to_string(),
            );
        }
        let (n, dsc, e, a) =
            match self.coupon_price_factors(settlement, maturity, frequency, basis, cell) {
                Ok(v) => v,
                Err(err) => return err,
            };
        let freq = frequency as f64;
        let f = |yld: f64| Model::coupon_price(rate, yld, redemption, freq, n, dsc, e, a) - pr;
        let mut lo = 0.0_f64;
        let mut hi = 1.0_f64;
        let mut f_hi = f(hi);
        let mut iterations = 0;
        while f_hi > 0.0 && iterations < 100 {
            hi *= 2.0;
            f_hi = f(hi);
            iterations += 1;
        }
        let f_lo = f(lo);
        if f_lo * f_hi > 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "YIELD did not converge".to_string());
        }
        for _ in 0..200 {
            let mid = (lo + hi) / 2.0;
            let f_mid = f(mid);
            if f_mid.abs() < 1e-10 || (hi - lo) / 2.0 < 1e-12 {
                return CalcResult::Number(mid);
            }
            if f(lo) * f_mid <= 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        CalcResult::Number((lo + hi) / 2.0)
    }

    // ACCRINTM(issue, settlement, rate, par, [basis])
    pub(crate) fn fn_accrintm(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(4..=5).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let issue = match self.get_number(&args[0], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return s,
        };
        let settlement = match self.get_number(&args[1], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return s,
        };
        let rate = match self.get_number(&args[2], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let par = match self.get_number(&args[3], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let basis = if args.len() == 5 {
            match self.get_number(&args[4], cell) {
                Ok(v) => v.floor() as i32,
                Err(s) => return s,
            }
        } else {
            0
        };
        if rate <= 0.0 || par <= 0.0 || !(0..=4).contains(&basis) || issue >= settlement {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "rate>0, par>0, basis 0-4, issue<settlement required".to_string(),
            );
        }
        let yf = match self.yearfrac_basis(issue, settlement, basis, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        CalcResult::Number(par * rate * yf)
    }

    // ACCRINT(issue, first_interest, settlement, rate, par, frequency, [basis], [calc_method])
    pub(crate) fn fn_accrint(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(6..=8).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let issue = match self.get_number(&args[0], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return s,
        };
        let _first_interest = match self.get_number(&args[1], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return s,
        };
        let settlement = match self.get_number(&args[2], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return s,
        };
        let rate = match self.get_number(&args[3], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let par = match self.get_number(&args[4], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let frequency = match self.get_number(&args[5], cell) {
            Ok(v) => v.floor() as i32,
            Err(s) => return s,
        };
        let basis = if args.len() >= 7 {
            match self.get_number(&args[6], cell) {
                Ok(v) => v.floor() as i32,
                Err(s) => return s,
            }
        } else {
            0
        };
        if args.len() == 8 {
            if let Err(s) = self.get_boolean(&args[7], cell) {
                return s;
            }
        }
        if rate <= 0.0
            || par <= 0.0
            || !matches!(frequency, 1 | 2 | 4)
            || !(0..=4).contains(&basis)
            || issue >= settlement
        {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "rate>0, par>0, freq 1/2/4, basis 0-4, issue<settlement required".to_string(),
            );
        }
        let yf = match self.yearfrac_basis(issue, settlement, basis, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        CalcResult::Number(par * rate * yf)
    }

    fn maturity_security_args(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<(f64, f64, f64), CalcResult> {
        let settlement = match self.get_number(&args[0], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return Err(s),
        };
        let maturity = match self.get_number(&args[1], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return Err(s),
        };
        let issue = match self.get_number(&args[2], cell) {
            Ok(v) => v.floor() as i64,
            Err(s) => return Err(s),
        };
        let basis = if args.len() == 6 {
            match self.get_number(&args[5], cell) {
                Ok(v) => v.floor() as i32,
                Err(s) => return Err(s),
            }
        } else {
            0
        };
        if !(0..=4).contains(&basis) || !(issue < settlement && settlement < maturity) {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "basis 0-4, issue<settlement<maturity required".to_string(),
            ));
        }
        let dsm = self.yearfrac_basis(settlement, maturity, basis, cell)?;
        let dim = self.yearfrac_basis(issue, maturity, basis, cell)?;
        let a = self.yearfrac_basis(issue, settlement, basis, cell)?;
        Ok((dsm, dim, a))
    }

    // PRICEMAT(settlement, maturity, issue, rate, yld, [basis])
    pub(crate) fn fn_pricemat(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(5..=6).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[3], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let yld = match self.get_number(&args[4], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        if rate < 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "rate>=0 required".to_string());
        }
        let (dsm, dim, a) = match self.maturity_security_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let price = (100.0 + dim * rate * 100.0) / (1.0 + dsm * yld) - a * rate * 100.0;
        CalcResult::Number(price)
    }

    // YIELDMAT(settlement, maturity, issue, rate, pr, [basis])
    pub(crate) fn fn_yieldmat(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(5..=6).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let rate = match self.get_number(&args[3], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let pr = match self.get_number(&args[4], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        if rate < 0.0 || pr <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "rate>=0, pr>0 required".to_string());
        }
        let (dsm, dim, a) = match self.maturity_security_args(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let base = pr / 100.0 + a * rate;
        if base == 0.0 || dsm == 0.0 {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        let yld = ((1.0 + dim * rate) - base) / base / dsm;
        CalcResult::Number(yld)
    }

    fn duration_value(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<f64, CalcResult> {
        if !(5..=6).contains(&args.len()) {
            return Err(CalcResult::new_args_number_error(cell));
        }
        let coupon = self.get_number(&args[2], cell).map_err(|s| s)?;
        let yld = self.get_number(&args[3], cell).map_err(|s| s)?;
        let coupon_args = if args.len() == 6 {
            vec![
                args[0].clone(),
                args[1].clone(),
                args[4].clone(),
                args[5].clone(),
            ]
        } else {
            vec![args[0].clone(), args[1].clone(), args[4].clone()]
        };
        let (settlement, maturity, frequency, basis) = self.coupon_args(&coupon_args, cell)?;
        if coupon < 0.0 || yld < 0.0 {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "coupon>=0, yld>=0 required".to_string(),
            ));
        }
        let (pcd, ncd, n) = self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell)?;
        let dsc = self.coupon_day_count(settlement, ncd, basis, cell);
        let e = match basis {
            1 => (ncd - pcd) as f64,
            3 => 365.0 / frequency as f64,
            _ => 360.0 / frequency as f64,
        };
        let freq = frequency as f64;
        let t1 = dsc / e;
        let cash = 100.0 * coupon / freq;
        let factor = 1.0 + yld / freq;
        let mut weighted = 0.0;
        let mut present = 0.0;
        for k in 1..=n {
            let time = (k as f64 - 1.0) + t1;
            let mut cf = cash;
            if k == n {
                cf += 100.0;
            }
            let df = 1.0 / factor.powf(time);
            let pv = cf * df;
            weighted += (time / freq) * pv;
            present += pv;
        }
        if present == 0.0 {
            return Err(CalcResult::new_error(
                Error::DIV,
                cell,
                "Division by 0".to_string(),
            ));
        }
        Ok(weighted / present)
    }

    // DURATION(settlement, maturity, coupon, yld, frequency, [basis])
    pub(crate) fn fn_duration(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        match self.duration_value(args, cell) {
            Ok(d) => CalcResult::Number(d),
            Err(e) => e,
        }
    }

    // MDURATION(settlement, maturity, coupon, yld, frequency, [basis])
    pub(crate) fn fn_mduration(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if !(5..=6).contains(&args.len()) {
            return CalcResult::new_args_number_error(cell);
        }
        let yld = match self.get_number(&args[3], cell) {
            Ok(v) => v,
            Err(s) => return s,
        };
        let frequency = match self.get_number(&args[4], cell) {
            Ok(f) => f.floor(),
            Err(s) => return s,
        };
        match self.duration_value(args, cell) {
            Ok(d) => CalcResult::Number(d / (1.0 + yld / frequency)),
            Err(e) => e,
        }
    }

    fn coupon_step_date(
        &self,
        anchor: i64,
        k: i32,
        step: i32,
        cell: CellReferenceIndex,
    ) -> Result<i64, CalcResult> {
        let d = self.excel_date(anchor, cell)?;
        let year = d.year();
        let month = d.month() as i32;
        let day = d.day();
        let eom = day as i32 == coupon_days_in_month(year, month);
        let (y, m, dd) = coupon_date_back(year, month, day, eom, k, step);
        match date_to_serial_number(dd, m as u32, y) {
            Ok(s) => Ok(s as i64),
            Err(_) => Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "date out of range".to_string(),
            )),
        }
    }

    fn odd_first_accrual(
        &mut self,
        first_coupon: i64,
        from: i64,
        to: i64,
        frequency: i32,
        basis: i32,
        cell: CellReferenceIndex,
    ) -> Result<f64, CalcResult> {
        let step = 12 / frequency;
        let mut acc = 0.0;
        let mut k = 0;
        loop {
            let q_end = self.coupon_step_date(first_coupon, k, step, cell)?;
            let q_start = self.coupon_step_date(first_coupon, k + 1, step, cell)?;
            if q_end <= from {
                break;
            }
            if q_start < to {
                let seg_start = q_start.max(from);
                let seg_end = q_end.min(to);
                if seg_end > seg_start {
                    let covered = self.coupon_day_count(seg_start, seg_end, basis, cell);
                    let length = match basis {
                        1 => (q_end - q_start) as f64,
                        3 => 365.0 / frequency as f64,
                        _ => 360.0 / frequency as f64,
                    };
                    if length != 0.0 {
                        acc += covered / length;
                    }
                }
            }
            k += 1;
            if k > 100_000 {
                break;
            }
        }
        Ok(acc)
    }

    #[allow(clippy::too_many_arguments)]
    fn oddf_price_value(
        &mut self,
        settlement: i64,
        maturity: i64,
        issue: i64,
        first_coupon: i64,
        rate: f64,
        yld: f64,
        redemption: f64,
        frequency: i32,
        basis: i32,
        cell: CellReferenceIndex,
    ) -> Result<f64, CalcResult> {
        let (n, dsc, e, a) =
            self.coupon_price_factors(settlement, maturity, frequency, basis, cell)?;
        let (_, ncd, _) = self.coupon_pcd_ncd_num(settlement, maturity, frequency, cell)?;
        let freq = frequency as f64;
        let coupon = 100.0 * rate / freq;
        let t1 = dsc / e;
        let factor = 1.0 + yld / freq;
        let odd_in_future = ncd == first_coupon;
        let accrual_first = if odd_in_future {
            self.odd_first_accrual(first_coupon, issue, first_coupon, frequency, basis, cell)?
        } else {
            0.0
        };
        let mut price = redemption / factor.powf(n - 1.0 + t1);
        let count = n as i64;
        for k in 1..=count {
            let cf = if k == 1 && odd_in_future {
                coupon * accrual_first
            } else {
                coupon
            };
            price += cf / factor.powf(k as f64 - 1.0 + t1);
        }
        let accrued = if odd_in_future {
            coupon
                * self.odd_first_accrual(first_coupon, issue, settlement, frequency, basis, cell)?
        } else {
            coupon * a / e
        };
        Ok(price - accrued)
    }

    fn oddf_args(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<(i64, i64, i64, i64, f64, f64, f64, i32, i32), CalcResult> {
        if !(8..=9).contains(&args.len()) {
            return Err(CalcResult::new_args_number_error(cell));
        }
        let settlement = self.get_number(&args[0], cell)?.floor() as i64;
        let maturity = self.get_number(&args[1], cell)?.floor() as i64;
        let issue = self.get_number(&args[2], cell)?.floor() as i64;
        let first_coupon = self.get_number(&args[3], cell)?.floor() as i64;
        let rate = self.get_number(&args[4], cell)?;
        let value = self.get_number(&args[5], cell)?;
        let redemption = self.get_number(&args[6], cell)?;
        let frequency = self.get_number(&args[7], cell)?.floor() as i32;
        let basis = if args.len() == 9 {
            self.get_number(&args[8], cell)?.floor() as i32
        } else {
            0
        };
        if !matches!(frequency, 1 | 2 | 4)
            || !(0..=4).contains(&basis)
            || redemption <= 0.0
            || !(issue < settlement && settlement < maturity)
            || !(issue < first_coupon && first_coupon < maturity)
        {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "invalid arguments".to_string(),
            ));
        }
        Ok((
            settlement,
            maturity,
            issue,
            first_coupon,
            rate,
            value,
            redemption,
            frequency,
            basis,
        ))
    }

    // ODDFPRICE(settlement, maturity, issue, first_coupon, rate, yld, redemption, frequency, [basis])
    pub(crate) fn fn_oddfprice(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, issue, first_coupon, rate, yld, redemption, frequency, basis) =
            match self.oddf_args(args, cell) {
                Ok(v) => v,
                Err(e) => return e,
            };
        if rate < 0.0 || yld < 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "rate>=0, yld>=0 required".to_string());
        }
        match self.oddf_price_value(
            settlement,
            maturity,
            issue,
            first_coupon,
            rate,
            yld,
            redemption,
            frequency,
            basis,
            cell,
        ) {
            Ok(p) => CalcResult::Number(p),
            Err(e) => e,
        }
    }

    // ODDFYIELD(settlement, maturity, issue, first_coupon, rate, pr, redemption, frequency, [basis])
    pub(crate) fn fn_oddfyield(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, issue, first_coupon, rate, pr, redemption, frequency, basis) =
            match self.oddf_args(args, cell) {
                Ok(v) => v,
                Err(e) => return e,
            };
        if rate < 0.0 || pr <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "rate>=0, pr>0 required".to_string());
        }
        macro_rules! eval {
            ($yld:expr) => {
                match self.oddf_price_value(
                    settlement,
                    maturity,
                    issue,
                    first_coupon,
                    rate,
                    $yld,
                    redemption,
                    frequency,
                    basis,
                    cell,
                ) {
                    Ok(p) => p - pr,
                    Err(e) => return e,
                }
            };
        }
        let mut lo = 0.0_f64;
        let mut hi = 1.0_f64;
        let mut f_hi = eval!(hi);
        let mut iterations = 0;
        while f_hi > 0.0 && iterations < 100 {
            hi *= 2.0;
            f_hi = eval!(hi);
            iterations += 1;
        }
        let f_lo = eval!(lo);
        if f_lo * f_hi > 0.0 {
            return CalcResult::new_error(
                Error::NUM,
                cell,
                "ODDFYIELD did not converge".to_string(),
            );
        }
        for _ in 0..200 {
            let mid = (lo + hi) / 2.0;
            let f_mid = eval!(mid);
            if f_mid.abs() < 1e-10 || (hi - lo) / 2.0 < 1e-12 {
                return CalcResult::Number(mid);
            }
            let f_lo_cur = eval!(lo);
            if f_lo_cur * f_mid <= 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        CalcResult::Number((lo + hi) / 2.0)
    }

    fn oddl_args(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<(i64, i64, i64, f64, f64, f64, i32, i32), CalcResult> {
        if !(7..=8).contains(&args.len()) {
            return Err(CalcResult::new_args_number_error(cell));
        }
        let settlement = self.get_number(&args[0], cell)?.floor() as i64;
        let maturity = self.get_number(&args[1], cell)?.floor() as i64;
        let last_interest = self.get_number(&args[2], cell)?.floor() as i64;
        let rate = self.get_number(&args[3], cell)?;
        let value = self.get_number(&args[4], cell)?;
        let redemption = self.get_number(&args[5], cell)?;
        let frequency = self.get_number(&args[6], cell)?.floor() as i32;
        let basis = if args.len() == 8 {
            self.get_number(&args[7], cell)?.floor() as i32
        } else {
            0
        };
        if !matches!(frequency, 1 | 2 | 4)
            || !(0..=4).contains(&basis)
            || redemption <= 0.0
            || !(last_interest < settlement && settlement < maturity)
        {
            return Err(CalcResult::new_error(
                Error::NUM,
                cell,
                "invalid arguments".to_string(),
            ));
        }
        Ok((
            settlement,
            maturity,
            last_interest,
            rate,
            value,
            redemption,
            frequency,
            basis,
        ))
    }

    // ODDLPRICE(settlement, maturity, last_interest, rate, yld, redemption, frequency, [basis])
    pub(crate) fn fn_oddlprice(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, last_interest, rate, yld, redemption, frequency, basis) =
            match self.oddl_args(args, cell) {
                Ok(v) => v,
                Err(e) => return e,
            };
        if rate < 0.0 || yld < 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "rate>=0, yld>=0 required".to_string());
        }
        let f = frequency as f64;
        let dci = match self.yearfrac_basis(last_interest, maturity, basis, cell) {
            Ok(v) => v * f,
            Err(e) => return e,
        };
        let dsci = match self.yearfrac_basis(settlement, maturity, basis, cell) {
            Ok(v) => v * f,
            Err(e) => return e,
        };
        let ai = match self.yearfrac_basis(last_interest, settlement, basis, cell) {
            Ok(v) => v * f,
            Err(e) => return e,
        };
        let mut p = redemption + dci * 100.0 * rate / f;
        p /= dsci * yld / f + 1.0;
        p -= ai * 100.0 * rate / f;
        CalcResult::Number(p)
    }

    // ODDLYIELD(settlement, maturity, last_interest, rate, pr, redemption, frequency, [basis])
    pub(crate) fn fn_oddlyield(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let (settlement, maturity, last_interest, rate, pr, redemption, frequency, basis) =
            match self.oddl_args(args, cell) {
                Ok(v) => v,
                Err(e) => return e,
            };
        if rate < 0.0 || pr <= 0.0 {
            return CalcResult::new_error(Error::NUM, cell, "rate>=0, pr>0 required".to_string());
        }
        let f = frequency as f64;
        let dci = match self.yearfrac_basis(last_interest, maturity, basis, cell) {
            Ok(v) => v * f,
            Err(e) => return e,
        };
        let dsci = match self.yearfrac_basis(settlement, maturity, basis, cell) {
            Ok(v) => v * f,
            Err(e) => return e,
        };
        let ai = match self.yearfrac_basis(last_interest, settlement, basis, cell) {
            Ok(v) => v * f,
            Err(e) => return e,
        };
        if dsci == 0.0 {
            return CalcResult::new_error(Error::DIV, cell, "Division by 0".to_string());
        }
        let mut y = redemption + dci * 100.0 * rate / f;
        y /= pr + ai * 100.0 * rate / f;
        y -= 1.0;
        y *= f / dsci;
        CalcResult::Number(y)
    }
}

fn coupon_days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn coupon_date_back(
    mat_year: i32,
    mat_month: i32,
    mat_day: u32,
    mat_eom: bool,
    k: i32,
    step: i32,
) -> (i32, i32, u32) {
    let total = mat_year * 12 + (mat_month - 1) - k * step;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) + 1;
    let dim = coupon_days_in_month(year, month);
    let day = if mat_eom {
        dim
    } else {
        (mat_day as i32).min(dim)
    };
    (year, month, day as u32)
}
