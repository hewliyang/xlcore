use std::collections::HashMap;

use crate::{
    calc_result::CalcResult,
    expressions::{
        parser::{ArrayNode, Node},
        token::Error,
        types::CellReferenceIndex,
    },
    functions::matrix::{array_node_to_calc_result, calc_result_to_array_node},
    model::Model,
};

impl<'a> Model<'a> {
    pub(crate) fn fn_let(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 3 || args.len() % 2 == 0 {
            return CalcResult::new_args_number_error(cell);
        }
        self.let_scopes.push(HashMap::new());
        let pair_count = (args.len() - 1) / 2;
        for i in 0..pair_count {
            let name = match &args[2 * i] {
                Node::WrongVariableKind(s) => s.to_uppercase(),
                _ => {
                    self.let_scopes.pop();
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "LET expects a name".to_string(),
                    );
                }
            };
            let value = self.evaluate_node_in_context(&args[2 * i + 1], cell);
            if let Some(top) = self.let_scopes.last_mut() {
                top.insert(name, value);
            }
        }
        let result = self.evaluate_node_in_context(&args[args.len() - 1], cell);
        self.let_scopes.pop();
        result
    }

    pub(crate) fn fn_lambda(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let mut params = Vec::new();
        for param in &args[..args.len() - 1] {
            match param {
                Node::WrongVariableKind(s) => params.push(s.to_uppercase()),
                _ => {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "LAMBDA expects parameter names".to_string(),
                    );
                }
            }
        }
        CalcResult::Lambda {
            params,
            body: Box::new(args[args.len() - 1].clone()),
            captured: self.let_scopes.clone(),
        }
    }

    pub(crate) fn fn_isomitted(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let key = match &args[0] {
            Node::WrongVariableKind(s) => s.to_uppercase(),
            _ => {
                let value = self.evaluate_node_in_context(&args[0], cell);
                return CalcResult::Boolean(matches!(value, CalcResult::EmptyArg));
            }
        };
        for scope in self.let_scopes.iter().rev() {
            if let Some(value) = scope.get(&key) {
                return CalcResult::Boolean(matches!(value, CalcResult::EmptyArg));
            }
        }
        CalcResult::Boolean(false)
    }

    fn eval_lambda_arg(
        &mut self,
        arg: &Node,
        cell: CellReferenceIndex,
    ) -> Result<(Vec<String>, Box<Node>, Vec<HashMap<String, CalcResult>>), CalcResult> {
        match self.evaluate_node_in_context(arg, cell) {
            CalcResult::Lambda {
                params,
                body,
                captured,
            } => Ok((params, body, captured)),
            error @ CalcResult::Error { .. } => Err(error),
            _ => Err(CalcResult::new_error(
                Error::VALUE,
                cell,
                "Expected a lambda".to_string(),
            )),
        }
    }

    pub(crate) fn fn_map(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let (params, body, captured) = match self.eval_lambda_arg(&args[args.len() - 1], cell) {
            Ok(l) => l,
            Err(e) => return e,
        };
        let n_arrays = args.len() - 1;
        if params.len() != n_arrays {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "MAP: lambda arity mismatch".to_string(),
            );
        }
        let mut grids = Vec::with_capacity(n_arrays);
        for arg in &args[..n_arrays] {
            match self.read_array_arg(arg, cell) {
                Ok(grid) => grids.push(grid),
                Err(e) => return e,
            }
        }
        let rows = grids[0].len();
        let cols = grids[0].first().map(|r| r.len()).unwrap_or(0);
        for grid in &grids {
            if grid.len() != rows || grid.iter().any(|r| r.len() != cols) {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "MAP: arrays must have the same dimensions".to_string(),
                );
            }
        }
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(rows);
        for r in 0..rows {
            let mut new_row = Vec::with_capacity(cols);
            for c in 0..cols {
                let arg_values: Vec<CalcResult> = grids
                    .iter()
                    .map(|g| array_node_to_calc_result(&g[r][c]))
                    .collect();
                let result =
                    self.apply_lambda(&params, &body, &captured, arg_values, cell);
                new_row.push(calc_result_to_array_node(result));
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_reduce(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let (params, body, captured) = match self.eval_lambda_arg(&args[2], cell) {
            Ok(l) => l,
            Err(e) => return e,
        };
        let grid = match self.read_array_arg(&args[1], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let mut acc = self.evaluate_node_in_context(&args[0], cell);
        if let CalcResult::Error { .. } = acc {
            return acc;
        }
        for row in &grid {
            for node in row {
                let value = array_node_to_calc_result(node);
                acc = self.apply_lambda(
                    &params,
                    &body,
                    &captured,
                    vec![acc, value],
                    cell,
                );
                if let CalcResult::Error { .. } = acc {
                    return acc;
                }
            }
        }
        acc
    }

    pub(crate) fn fn_scan(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let (params, body, captured) = match self.eval_lambda_arg(&args[2], cell) {
            Ok(l) => l,
            Err(e) => return e,
        };
        let grid = match self.read_array_arg(&args[1], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let mut acc = self.evaluate_node_in_context(&args[0], cell);
        if let CalcResult::Error { .. } = acc {
            return acc;
        }
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(grid.len());
        for row in &grid {
            let mut new_row = Vec::with_capacity(row.len());
            for node in row {
                let value = array_node_to_calc_result(node);
                acc = self.apply_lambda(
                    &params,
                    &body,
                    &captured,
                    vec![acc, value],
                    cell,
                );
                new_row.push(calc_result_to_array_node(acc.clone()));
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }
}
