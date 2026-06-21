use crate::expressions::parser::{ArrayNode, Node};
use crate::expressions::token::Error;
use crate::expressions::types::CellReferenceIndex;
use crate::{calc_result::CalcResult, model::Model};

fn take_range(arg: Option<i64>, len: usize) -> Vec<usize> {
    let len = len as i64;
    match arg {
        None => (0..len as usize).collect(),
        Some(n) if n == 0 => Vec::new(),
        Some(n) if n > 0 => (0..n.min(len) as usize).collect(),
        Some(n) => {
            let start = (len + n).max(0);
            (start as usize..len as usize).collect()
        }
    }
}

fn drop_range(arg: Option<i64>, len: usize) -> Vec<usize> {
    let len = len as i64;
    match arg {
        None | Some(0) => (0..len as usize).collect(),
        Some(n) if n > 0 => (n.min(len) as usize..len as usize).collect(),
        Some(n) => {
            let end = (len + n).max(0);
            (0..end as usize).collect()
        }
    }
}

fn resolve_choose_indices(indices: &[i64], len: usize) -> Option<Vec<usize>> {
    let len_i = len as i64;
    let mut out = Vec::with_capacity(indices.len());
    for &idx in indices {
        let pos = if idx > 0 {
            if idx > len_i {
                return None;
            }
            (idx - 1) as usize
        } else if idx < 0 {
            if -idx > len_i {
                return None;
            }
            (len_i + idx) as usize
        } else {
            return None;
        };
        out.push(pos);
    }
    Some(out)
}

fn invert_matrix(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = matrix.len();
    let mut aug: Vec<Vec<f64>> = Vec::with_capacity(n);
    for (i, row) in matrix.iter().enumerate() {
        let mut new_row = row.clone();
        for j in 0..n {
            new_row.push(if i == j { 1.0 } else { 0.0 });
        }
        aug.push(new_row);
    }
    for column in 0..n {
        let mut pivot = column;
        let mut pivot_value = aug[column][column].abs();
        for row in (column + 1)..n {
            let value = aug[row][column].abs();
            if value > pivot_value {
                pivot_value = value;
                pivot = row;
            }
        }
        if aug[pivot][column] == 0.0 {
            return None;
        }
        if pivot != column {
            aug.swap(pivot, column);
        }
        let pivot_diag = aug[column][column];
        for col in 0..(2 * n) {
            aug[column][col] /= pivot_diag;
        }
        for row in 0..n {
            if row != column {
                let factor = aug[row][column];
                if factor != 0.0 {
                    for col in 0..(2 * n) {
                        aug[row][col] -= factor * aug[column][col];
                    }
                }
            }
        }
    }
    let mut out: Vec<Vec<f64>> = Vec::with_capacity(n);
    for row in &aug {
        out.push(row[n..(2 * n)].to_vec());
    }
    Some(out)
}

struct OlsFit {
    coeffs: Vec<f64>,
    intercept: f64,
    se_coeffs: Vec<f64>,
    se_intercept: f64,
    r2: f64,
    sey: f64,
    f: f64,
    df: f64,
    ss_reg: f64,
    ss_resid: f64,
}

fn ols_fit(ys: &[f64], xs: &[Vec<f64>], with_const: bool) -> Option<OlsFit> {
    let n = ys.len();
    if n == 0 || xs.len() != n {
        return None;
    }
    let k = xs[0].len();
    if k == 0 || xs.iter().any(|r| r.len() != k) {
        return None;
    }
    let p = k + usize::from(with_const);
    let mut design: Vec<Vec<f64>> = Vec::with_capacity(n);
    for row in xs.iter() {
        let mut d = row.clone();
        if with_const {
            d.push(1.0);
        }
        design.push(d);
    }
    let mut xtx = vec![vec![0.0_f64; p]; p];
    let mut xty = vec![0.0_f64; p];
    for i in 0..n {
        for a in 0..p {
            xty[a] += design[i][a] * ys[i];
            for b in 0..p {
                xtx[a][b] += design[i][a] * design[i][b];
            }
        }
    }
    let inv = invert_matrix(&xtx)?;
    let mut beta = vec![0.0_f64; p];
    for a in 0..p {
        let mut s = 0.0;
        for b in 0..p {
            s += inv[a][b] * xty[b];
        }
        beta[a] = s;
    }
    let mut ss_resid = 0.0;
    for i in 0..n {
        let mut yhat = 0.0;
        for a in 0..p {
            yhat += design[i][a] * beta[a];
        }
        ss_resid += (ys[i] - yhat).powi(2);
    }
    let ss_total = if with_const {
        let ybar = ys.iter().sum::<f64>() / n as f64;
        ys.iter().map(|y| (y - ybar).powi(2)).sum::<f64>()
    } else {
        ys.iter().map(|y| y * y).sum::<f64>()
    };
    let ss_reg = ss_total - ss_resid;
    let df = n as f64 - p as f64;
    let sey = if df > 0.0 {
        (ss_resid / df).sqrt()
    } else {
        0.0
    };
    let r2 = if ss_total != 0.0 { ss_reg / ss_total } else { 0.0 };
    let f = if df > 0.0 && k > 0 && ss_resid > 0.0 {
        (ss_reg / k as f64) / (ss_resid / df)
    } else {
        0.0
    };
    let mut se = vec![0.0_f64; p];
    for a in 0..p {
        se[a] = (inv[a][a].max(0.0) * sey * sey).sqrt();
    }
    let intercept = if with_const { beta[k] } else { 0.0 };
    let se_intercept = if with_const { se[k] } else { 0.0 };
    Some(OlsFit {
        coeffs: beta[0..k].to_vec(),
        intercept,
        se_coeffs: se[0..k].to_vec(),
        se_intercept,
        r2,
        sey,
        f,
        df,
        ss_reg,
        ss_resid,
    })
}

fn grid_to_numbers(grid: &[Vec<ArrayNode>]) -> Option<Vec<Vec<f64>>> {
    let mut out = Vec::with_capacity(grid.len());
    for row in grid {
        let mut current = Vec::with_capacity(row.len());
        for node in row {
            match node {
                ArrayNode::Number(f) => current.push(*f),
                ArrayNode::Boolean(b) => current.push(if *b { 1.0 } else { 0.0 }),
                _ => return None,
            }
        }
        out.push(current);
    }
    Some(out)
}

fn normalize_by_obs(grid: &[Vec<f64>], n: usize) -> Option<Vec<Vec<f64>>> {
    let r = grid.len();
    let c = grid.first().map(|row| row.len()).unwrap_or(0);
    if r == 0 || c == 0 {
        return None;
    }
    if r == n {
        Some(grid.to_vec())
    } else if c == n {
        let mut out = vec![Vec::with_capacity(r); c];
        for (_, row) in grid.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                out[j].push(*v);
            }
        }
        Some(out)
    } else {
        None
    }
}

fn normalize_by_predictors(grid: &[Vec<f64>], k: usize) -> Option<(Vec<Vec<f64>>, bool)> {
    let r = grid.len();
    let c = grid.first().map(|row| row.len()).unwrap_or(0);
    if r == 0 || c == 0 {
        return None;
    }
    if c == k {
        Some((grid.to_vec(), false))
    } else if r == k {
        let mut out = vec![Vec::with_capacity(r); c];
        for row in grid.iter() {
            for (j, v) in row.iter().enumerate() {
                out[j].push(*v);
            }
        }
        Some((out, true))
    } else {
        None
    }
}

pub(crate) fn array_node_to_calc_result(node: &ArrayNode) -> CalcResult {
    match node {
        ArrayNode::Number(f) => CalcResult::Number(*f),
        ArrayNode::String(s) => CalcResult::String(s.clone()),
        ArrayNode::Boolean(b) => CalcResult::Boolean(*b),
        ArrayNode::Error(error) => CalcResult::new_error(
            error.clone(),
            CellReferenceIndex {
                sheet: 0,
                row: 0,
                column: 0,
            },
            String::new(),
        ),
    }
}

fn array_node_cmp(a: &ArrayNode, b: &ArrayNode) -> std::cmp::Ordering {
    array_node_to_calc_result(a).cmp(&array_node_to_calc_result(b))
}

pub(crate) fn calc_result_to_array_node(value: CalcResult) -> ArrayNode {
    match value {
        CalcResult::Number(f) => ArrayNode::Number(f),
        CalcResult::String(s) => ArrayNode::String(s),
        CalcResult::Boolean(b) => ArrayNode::Boolean(b),
        CalcResult::Error { error, .. } => ArrayNode::Error(error),
        CalcResult::EmptyCell | CalcResult::EmptyArg => ArrayNode::Number(0.0),
        CalcResult::Range { .. } | CalcResult::Array(_) | CalcResult::Lambda { .. } => ArrayNode::Error(Error::VALUE),
    }
}

impl<'a> Model<'a> {
    fn read_numeric_matrix(
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
                    "requires a numeric matrix".to_string(),
                ));
            }
        };
        if matrix.is_empty() || matrix.iter().any(|r| r.is_empty()) {
            return Err(CalcResult::new_error(
                Error::VALUE,
                cell,
                "requires a numeric matrix".to_string(),
            ));
        }
        let cols = matrix[0].len();
        if matrix.iter().any(|r| r.len() != cols) {
            return Err(CalcResult::new_error(
                Error::VALUE,
                cell,
                "requires a rectangular matrix".to_string(),
            ));
        }
        Ok(matrix)
    }

    fn read_square_matrix(
        &mut self,
        arg: &Node,
        cell: CellReferenceIndex,
    ) -> Result<Vec<Vec<f64>>, CalcResult> {
        let matrix = self.read_numeric_matrix(arg, cell)?;
        let n = matrix.len();
        for row in &matrix {
            if row.len() != n {
                return Err(CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "requires a square matrix".to_string(),
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

    pub(crate) fn fn_randarray(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() > 5 {
            return CalcResult::new_args_number_error(cell);
        }
        let rows = if !args.is_empty() {
            match self.get_number(&args[0], cell) {
                Ok(f) => f.trunc() as i64,
                Err(e) => return e,
            }
        } else {
            1
        };
        let columns = if args.len() >= 2 {
            match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc() as i64,
                Err(e) => return e,
            }
        } else {
            1
        };
        let min = if args.len() >= 3 {
            match self.get_number(&args[2], cell) {
                Ok(f) => f,
                Err(e) => return e,
            }
        } else {
            0.0
        };
        let max = if args.len() >= 4 {
            match self.get_number(&args[3], cell) {
                Ok(f) => f,
                Err(e) => return e,
            }
        } else {
            1.0
        };
        let whole_number = if args.len() >= 5 {
            match self.get_boolean(&args[4], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            false
        };
        if rows < 1 || columns < 1 {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "RANDARRAY requires positive dimensions".to_string(),
            );
        }
        if min > max {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "RANDARRAY requires min <= max".to_string(),
            );
        }
        if whole_number && (min.fract() != 0.0 || max.fract() != 0.0) {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "RANDARRAY requires integer min and max".to_string(),
            );
        }
        let mut grid: Vec<Vec<ArrayNode>> = Vec::with_capacity(rows as usize);
        for _ in 0..rows {
            let mut new_row = Vec::with_capacity(columns as usize);
            for _ in 0..columns {
                let value = if whole_number {
                    (min + (crate::functions::mathematical::random() * (max - min + 1.0)).floor())
                        .min(max)
                } else {
                    min + crate::functions::mathematical::random() * (max - min)
                };
                new_row.push(ArrayNode::Number(value));
            }
            grid.push(new_row);
        }
        CalcResult::Array(grid)
    }

    pub(crate) fn fn_frequency(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let data = match self.read_array_arg(&args[0], cell) {
            Ok(grid) => grid,
            Err(e) => return e,
        };
        let bins = match self.read_array_arg(&args[1], cell) {
            Ok(grid) => grid,
            Err(e) => return e,
        };
        let mut values: Vec<f64> = Vec::new();
        for row in &data {
            for node in row {
                if let ArrayNode::Number(f) = node {
                    values.push(*f);
                }
            }
        }
        let mut bin_values: Vec<f64> = Vec::new();
        for row in &bins {
            for node in row {
                if let ArrayNode::Number(f) = node {
                    bin_values.push(*f);
                }
            }
        }
        let bins_count = bin_values.len();
        let mut sorted = bin_values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sorted.dedup();
        let mut counts = vec![0.0_f64; sorted.len() + 1];
        for v in &values {
            let mut placed = false;
            for (i, b) in sorted.iter().enumerate() {
                if *v <= *b {
                    counts[i] += 1.0;
                    placed = true;
                    break;
                }
            }
            if !placed {
                let last = counts.len() - 1;
                counts[last] += 1.0;
            }
        }
        let mut grid: Vec<Vec<ArrayNode>> = Vec::with_capacity(bins_count + 1);
        let mut seen: Vec<f64> = Vec::with_capacity(bins_count);
        for b in &bin_values {
            if seen.contains(b) {
                grid.push(vec![ArrayNode::Number(0.0)]);
            } else {
                seen.push(*b);
                let count = sorted
                    .iter()
                    .position(|s| s == b)
                    .map(|i| counts[i])
                    .unwrap_or(0.0);
                grid.push(vec![ArrayNode::Number(count)]);
            }
        }
        let last = counts.len() - 1;
        grid.push(vec![ArrayNode::Number(counts[last])]);
        CalcResult::Array(grid)
    }

    pub(crate) fn fn_sort(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 4 {
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
                        current.push(calc_result_to_array_node(self.evaluate_cell(cell_ref)));
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
            return CalcResult::new_error(Error::VALUE, cell, "SORT: empty array".to_string());
        }
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        if n_cols == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "SORT: empty array".to_string());
        }
        let sort_index = if args.len() >= 2 {
            match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc() as i64,
                Err(e) => return e,
            }
        } else {
            1
        };
        let sort_order = if args.len() >= 3 {
            match self.get_number(&args[2], cell) {
                Ok(f) => f.trunc() as i64,
                Err(e) => return e,
            }
        } else {
            1
        };
        if sort_order != 1 && sort_order != -1 {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "SORT: sort_order must be 1 or -1".to_string(),
            );
        }
        let by_col = if args.len() >= 4 {
            match self.get_boolean(&args[3], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            false
        };
        let lines = if by_col {
            let mut cols: Vec<Vec<ArrayNode>> = Vec::with_capacity(n_cols);
            for c in 0..n_cols {
                let mut col = Vec::with_capacity(n_rows);
                for row in grid.iter() {
                    col.push(row.get(c).cloned().unwrap_or(ArrayNode::Number(0.0)));
                }
                cols.push(col);
            }
            cols
        } else {
            grid.clone()
        };
        let line_len = if by_col { n_rows } else { n_cols };
        if sort_index < 1 || sort_index as usize > line_len {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "SORT: sort_index out of range".to_string(),
            );
        }
        let key = (sort_index - 1) as usize;
        let mut lines = lines;
        lines.sort_by(|a, b| {
            let av = a.get(key).cloned().unwrap_or(ArrayNode::Number(0.0));
            let bv = b.get(key).cloned().unwrap_or(ArrayNode::Number(0.0));
            let ordering = array_node_cmp(&av, &bv);
            if sort_order == -1 {
                ordering.reverse()
            } else {
                ordering
            }
        });
        let out = if by_col {
            let mut rows: Vec<Vec<ArrayNode>> = Vec::with_capacity(n_rows);
            for r in 0..n_rows {
                let mut new_row = Vec::with_capacity(lines.len());
                for col in lines.iter() {
                    new_row.push(col.get(r).cloned().unwrap_or(ArrayNode::Number(0.0)));
                }
                rows.push(new_row);
            }
            rows
        } else {
            lines
        };
        CalcResult::Array(out)
    }

    pub(crate) fn fn_unique(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 3 {
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
                        current.push(calc_result_to_array_node(self.evaluate_cell(cell_ref)));
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
            return CalcResult::new_error(Error::VALUE, cell, "UNIQUE: empty array".to_string());
        }
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        if n_cols == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "UNIQUE: empty array".to_string());
        }
        let by_col = if args.len() >= 2 {
            match self.get_boolean(&args[1], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            false
        };
        let exactly_once = if args.len() >= 3 {
            match self.get_boolean(&args[2], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            false
        };
        let lines: Vec<Vec<ArrayNode>> = if by_col {
            let mut cols: Vec<Vec<ArrayNode>> = Vec::with_capacity(n_cols);
            for c in 0..n_cols {
                let mut col = Vec::with_capacity(n_rows);
                for row in grid.iter() {
                    col.push(row.get(c).cloned().unwrap_or(ArrayNode::Number(0.0)));
                }
                cols.push(col);
            }
            cols
        } else {
            grid.clone()
        };
        let lines_eq = |a: &[ArrayNode], b: &[ArrayNode]| -> bool {
            if a.len() != b.len() {
                return false;
            }
            a.iter()
                .zip(b.iter())
                .all(|(x, y)| array_node_cmp(x, y) == std::cmp::Ordering::Equal)
        };
        let mut kept: Vec<Vec<ArrayNode>> = Vec::new();
        for line in lines.iter() {
            if exactly_once {
                let count = lines.iter().filter(|other| lines_eq(line, other)).count();
                if count == 1 && !kept.iter().any(|k| lines_eq(k, line)) {
                    kept.push(line.clone());
                }
            } else if !kept.iter().any(|k| lines_eq(k, line)) {
                kept.push(line.clone());
            }
        }
        if kept.is_empty() {
            return CalcResult::new_error(
                Error::CALC,
                cell,
                "UNIQUE: no unique values".to_string(),
            );
        }
        let out = if by_col {
            let mut rows: Vec<Vec<ArrayNode>> = Vec::with_capacity(n_rows);
            for r in 0..n_rows {
                let mut new_row = Vec::with_capacity(kept.len());
                for col in kept.iter() {
                    new_row.push(col.get(r).cloned().unwrap_or(ArrayNode::Number(0.0)));
                }
                rows.push(new_row);
            }
            rows
        } else {
            kept
        };
        CalcResult::Array(out)
    }

    pub(crate) fn fn_filter(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 || args.len() > 3 {
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
                        current.push(calc_result_to_array_node(self.evaluate_cell(cell_ref)));
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
            return CalcResult::new_error(Error::VALUE, cell, "FILTER: empty array".to_string());
        }
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        if n_cols == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "FILTER: empty array".to_string());
        }
        let include_result = self.evaluate_node_in_context(&args[1], cell);
        let mask_grid: Vec<Vec<ArrayNode>> = match include_result {
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
                        current.push(calc_result_to_array_node(self.evaluate_cell(cell_ref)));
                    }
                    rows.push(current);
                }
                rows
            }
            CalcResult::Array(array) => array,
            error @ CalcResult::Error { .. } => return error,
            other => vec![vec![calc_result_to_array_node(other)]],
        };
        let mask_rows = mask_grid.len();
        let mask_cols = mask_grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let mask: Vec<ArrayNode> = mask_grid.into_iter().flatten().collect();
        let mask_truthiness = |node: &ArrayNode| -> Result<bool, CalcResult> {
            match node {
                ArrayNode::Number(f) => Ok(*f != 0.0),
                ArrayNode::Boolean(b) => Ok(*b),
                ArrayNode::Error(error) => Err(CalcResult::new_error(
                    error.clone(),
                    cell,
                    "FILTER: error in include".to_string(),
                )),
                ArrayNode::String(_) => Err(CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "FILTER: text in include".to_string(),
                )),
            }
        };
        let is_vector = mask_rows == 1 || mask_cols == 1;
        let by_rows = is_vector && mask.len() == n_rows;
        let by_cols = is_vector && mask.len() == n_cols;
        let out: Vec<Vec<ArrayNode>> = if by_rows && (!by_cols || mask_cols == 1) {
            let mut kept = Vec::new();
            for (i, row) in grid.iter().enumerate() {
                match mask_truthiness(&mask[i]) {
                    Ok(true) => kept.push(row.clone()),
                    Ok(false) => {}
                    Err(e) => return e,
                }
            }
            kept
        } else if by_cols {
            let mut keep_cols = Vec::new();
            for c in 0..n_cols {
                match mask_truthiness(&mask[c]) {
                    Ok(true) => keep_cols.push(c),
                    Ok(false) => {}
                    Err(e) => return e,
                }
            }
            grid.iter()
                .map(|row| {
                    keep_cols
                        .iter()
                        .map(|&c| row.get(c).cloned().unwrap_or(ArrayNode::Number(0.0)))
                        .collect()
                })
                .collect()
        } else {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "FILTER: include size mismatch".to_string(),
            );
        };
        let is_empty = out.is_empty() || out.iter().all(|r| r.is_empty());
        if is_empty {
            if args.len() == 3 {
                let if_empty = self.evaluate_node_in_context(&args[2], cell);
                if let CalcResult::Error { .. } = if_empty {
                    return if_empty;
                }
                return CalcResult::Array(vec![vec![calc_result_to_array_node(if_empty)]]);
            }
            return CalcResult::new_error(Error::CALC, cell, "FILTER: no match".to_string());
        }
        CalcResult::Array(out)
    }

    pub(crate) fn read_array_arg(
        &mut self,
        arg: &Node,
        cell: CellReferenceIndex,
    ) -> Result<Vec<Vec<ArrayNode>>, CalcResult> {
        let result = self.evaluate_node_in_context(arg, cell);
        match result {
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
                        current.push(calc_result_to_array_node(self.evaluate_cell(cell_ref)));
                    }
                    rows.push(current);
                }
                Ok(rows)
            }
            CalcResult::Array(array) => Ok(array),
            error @ CalcResult::Error { .. } => Err(error),
            other => Ok(vec![vec![calc_result_to_array_node(other)]]),
        }
    }

    pub(crate) fn fn_hstack(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let mut grids = Vec::with_capacity(args.len());
        for arg in args {
            match self.read_array_arg(arg, cell) {
                Ok(grid) => grids.push(grid),
                Err(e) => return e,
            }
        }
        let n_rows = grids.iter().map(|g| g.len()).max().unwrap_or(0);
        if n_rows == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "HSTACK: empty array".to_string());
        }
        let mut out: Vec<Vec<ArrayNode>> = vec![Vec::new(); n_rows];
        for grid in &grids {
            let cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
            for r in 0..n_rows {
                for c in 0..cols {
                    let node = grid
                        .get(r)
                        .and_then(|row| row.get(c))
                        .cloned()
                        .unwrap_or(ArrayNode::Error(Error::NA));
                    out[r].push(node);
                }
            }
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_vstack(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let mut grids = Vec::with_capacity(args.len());
        for arg in args {
            match self.read_array_arg(arg, cell) {
                Ok(grid) => grids.push(grid),
                Err(e) => return e,
            }
        }
        let n_cols = grids
            .iter()
            .flat_map(|g| g.iter().map(|r| r.len()))
            .max()
            .unwrap_or(0);
        if n_cols == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "VSTACK: empty array".to_string());
        }
        let mut out: Vec<Vec<ArrayNode>> = Vec::new();
        for grid in &grids {
            for row in grid {
                let mut new_row = Vec::with_capacity(n_cols);
                for c in 0..n_cols {
                    new_row.push(row.get(c).cloned().unwrap_or(ArrayNode::Error(Error::NA)));
                }
                out.push(new_row);
            }
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_take(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let grid = match self.read_array_arg(&args[0], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let n_rows = grid.len();
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let rows_arg = match self.get_number(&args[1], cell) {
            Ok(f) => Some(f.trunc() as i64),
            Err(e) => return e,
        };
        let cols_arg = if args.len() == 3 {
            match self.get_number(&args[2], cell) {
                Ok(f) => Some(f.trunc() as i64),
                Err(e) => return e,
            }
        } else {
            None
        };
        let row_range = take_range(rows_arg, n_rows);
        let col_range = take_range(cols_arg, n_cols);
        if row_range.is_empty() || col_range.is_empty() {
            return CalcResult::new_error(Error::CALC, cell, "TAKE: empty result".to_string());
        }
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(row_range.len());
        for r in row_range {
            let mut new_row = Vec::with_capacity(col_range.len());
            for &c in &col_range {
                new_row.push(
                    grid.get(r)
                        .and_then(|row| row.get(c))
                        .cloned()
                        .unwrap_or(ArrayNode::Error(Error::NA)),
                );
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_drop(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let grid = match self.read_array_arg(&args[0], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let n_rows = grid.len();
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let rows_arg = match self.get_number(&args[1], cell) {
            Ok(f) => Some(f.trunc() as i64),
            Err(e) => return e,
        };
        let cols_arg = if args.len() == 3 {
            match self.get_number(&args[2], cell) {
                Ok(f) => Some(f.trunc() as i64),
                Err(e) => return e,
            }
        } else {
            None
        };
        let row_range = drop_range(rows_arg, n_rows);
        let col_range = drop_range(cols_arg, n_cols);
        if row_range.is_empty() || col_range.is_empty() {
            return CalcResult::new_error(Error::CALC, cell, "DROP: empty result".to_string());
        }
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(row_range.len());
        for r in row_range {
            let mut new_row = Vec::with_capacity(col_range.len());
            for &c in &col_range {
                new_row.push(
                    grid.get(r)
                        .and_then(|row| row.get(c))
                        .cloned()
                        .unwrap_or(ArrayNode::Error(Error::NA)),
                );
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    fn collect_choose_indices(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<Vec<i64>, CalcResult> {
        let mut indices = Vec::new();
        for arg in args {
            let grid = self.read_array_arg(arg, cell)?;
            for row in grid {
                for node in row {
                    match node {
                        ArrayNode::Number(f) => indices.push(f.trunc() as i64),
                        ArrayNode::Boolean(b) => indices.push(i64::from(b)),
                        ArrayNode::Error(error) => {
                            return Err(CalcResult::new_error(
                                error,
                                cell,
                                "CHOOSE: error index".to_string(),
                            ))
                        }
                        ArrayNode::String(_) => {
                            return Err(CalcResult::new_error(
                                Error::VALUE,
                                cell,
                                "CHOOSE: non-numeric index".to_string(),
                            ))
                        }
                    }
                }
            }
        }
        Ok(indices)
    }

    pub(crate) fn fn_choosecols(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let grid = match self.read_array_arg(&args[0], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let n_rows = grid.len();
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let indices = match self.collect_choose_indices(&args[1..], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let cols = match resolve_choose_indices(&indices, n_cols) {
            Some(c) => c,
            None => {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "CHOOSECOLS: index out of range".to_string(),
                )
            }
        };
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(n_rows);
        for r in 0..n_rows {
            let mut new_row = Vec::with_capacity(cols.len());
            for &c in &cols {
                new_row.push(
                    grid.get(r)
                        .and_then(|row| row.get(c))
                        .cloned()
                        .unwrap_or(ArrayNode::Error(Error::NA)),
                );
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_chooserows(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let grid = match self.read_array_arg(&args[0], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let n_rows = grid.len();
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let indices = match self.collect_choose_indices(&args[1..], cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let rows = match resolve_choose_indices(&indices, n_rows) {
            Some(r) => r,
            None => {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "CHOOSEROWS: index out of range".to_string(),
                )
            }
        };
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(rows.len());
        for &r in &rows {
            let mut new_row = Vec::with_capacity(n_cols);
            for c in 0..n_cols {
                new_row.push(
                    grid.get(r)
                        .and_then(|row| row.get(c))
                        .cloned()
                        .unwrap_or(ArrayNode::Error(Error::NA)),
                );
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    fn flatten_for_tocolrow(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<Vec<ArrayNode>, CalcResult> {
        let grid = self.read_array_arg(&args[0], cell)?;
        let ignore = if args.len() >= 2 {
            self.get_number(&args[1], cell)?.trunc() as i64
        } else {
            0
        };
        if !(0..=3).contains(&ignore) {
            return Err(CalcResult::new_error(
                Error::VALUE,
                cell,
                "ignore must be 0-3".to_string(),
            ));
        }
        let scan_by_column = if args.len() >= 3 {
            self.get_boolean(&args[2], cell)?
        } else {
            false
        };
        let n_rows = grid.len();
        let n_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut flat: Vec<ArrayNode> = Vec::new();
        if scan_by_column {
            for c in 0..n_cols {
                for r in 0..n_rows {
                    if let Some(node) = grid.get(r).and_then(|row| row.get(c)) {
                        flat.push(node.clone());
                    }
                }
            }
        } else {
            for row in &grid {
                for node in row {
                    flat.push(node.clone());
                }
            }
        }
        let drop_errors = ignore == 2 || ignore == 3;
        if drop_errors {
            flat.retain(|node| !matches!(node, ArrayNode::Error(_)));
        }
        Ok(flat)
    }

    pub(crate) fn fn_tocol(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let flat = match self.flatten_for_tocolrow(args, cell) {
            Ok(f) => f,
            Err(e) => return e,
        };
        if flat.is_empty() {
            return CalcResult::new_error(Error::CALC, cell, "TOCOL: empty result".to_string());
        }
        let out: Vec<Vec<ArrayNode>> = flat.into_iter().map(|node| vec![node]).collect();
        CalcResult::Array(out)
    }

    pub(crate) fn fn_torow(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let flat = match self.flatten_for_tocolrow(args, cell) {
            Ok(f) => f,
            Err(e) => return e,
        };
        if flat.is_empty() {
            return CalcResult::new_error(Error::CALC, cell, "TOROW: empty result".to_string());
        }
        CalcResult::Array(vec![flat])
    }

    pub(crate) fn fn_expand(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 || args.len() > 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let grid = match self.read_array_arg(&args[0], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let cur_rows = grid.len();
        let cur_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
        let target_rows = match self.evaluate_node_in_context(&args[1], cell) {
            CalcResult::EmptyCell | CalcResult::EmptyArg => cur_rows as i64,
            other => match self.cast_to_number(other, cell) {
                Ok(n) => n.trunc() as i64,
                Err(e) => return e,
            },
        };
        let target_cols = if args.len() >= 3 {
            match self.evaluate_node_in_context(&args[2], cell) {
                CalcResult::EmptyCell | CalcResult::EmptyArg => cur_cols as i64,
                other => match self.cast_to_number(other, cell) {
                    Ok(n) => n.trunc() as i64,
                    Err(e) => return e,
                },
            }
        } else {
            cur_cols as i64
        };
        if target_rows < 1 || target_cols < 1 {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "EXPAND: rows and columns must be >= 1".to_string(),
            );
        }
        if target_rows < cur_rows as i64 || target_cols < cur_cols as i64 {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "EXPAND cannot shrink an array".to_string(),
            );
        }
        let pad_with = if args.len() >= 4 {
            let value = self.evaluate_node_in_context(&args[3], cell);
            if let CalcResult::EmptyCell | CalcResult::EmptyArg = value {
                ArrayNode::Error(Error::NA)
            } else {
                calc_result_to_array_node(value)
            }
        } else {
            ArrayNode::Error(Error::NA)
        };
        let target_rows = target_rows as usize;
        let target_cols = target_cols as usize;
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(target_rows);
        for r in 0..target_rows {
            let mut new_row: Vec<ArrayNode> = Vec::with_capacity(target_cols);
            for c in 0..target_cols {
                let node = grid
                    .get(r)
                    .and_then(|row| row.get(c))
                    .cloned()
                    .unwrap_or_else(|| pad_with.clone());
                new_row.push(node);
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_sortby(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let grid = match self.read_array_arg(&args[0], cell) {
            Ok(g) => g,
            Err(e) => return e,
        };
        let n_rows = grid.len();
        if n_rows == 0 {
            return CalcResult::new_error(Error::VALUE, cell, "SORTBY: empty array".to_string());
        }
        let mut keys: Vec<(Vec<ArrayNode>, i32)> = Vec::new();
        let mut i = 1;
        while i < args.len() {
            let by_grid = match self.read_array_arg(&args[i], cell) {
                Ok(g) => g,
                Err(e) => return e,
            };
            let mut flat: Vec<ArrayNode> = Vec::new();
            for row in &by_grid {
                for node in row {
                    flat.push(node.clone());
                }
            }
            if flat.len() != n_rows {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "SORTBY: by_array must match array row count".to_string(),
                );
            }
            i += 1;
            let order = if i < args.len() {
                match self.evaluate_node_in_context(&args[i], cell) {
                    CalcResult::Number(f) => {
                        i += 1;
                        f.trunc() as i32
                    }
                    CalcResult::Error { error, origin, message } => {
                        return CalcResult::Error { error, origin, message };
                    }
                    _ => 1,
                }
            } else {
                1
            };
            if order != 1 && order != -1 {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "SORTBY: sort_order must be 1 or -1".to_string(),
                );
            }
            keys.push((flat, order));
        }
        if keys.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let mut order: Vec<usize> = (0..n_rows).collect();
        order.sort_by(|&a, &b| {
            for (values, dir) in &keys {
                let ordering = array_node_cmp(&values[a], &values[b]);
                let ordering = if *dir == -1 { ordering.reverse() } else { ordering };
                if ordering != std::cmp::Ordering::Equal {
                    return ordering;
                }
            }
            std::cmp::Ordering::Equal
        });
        let out: Vec<Vec<ArrayNode>> = order.into_iter().map(|r| grid[r].clone()).collect();
        CalcResult::Array(out)
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

    pub(crate) fn fn_mmult(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let a = match self.read_numeric_matrix(&args[0], cell) {
            Ok(m) => m,
            Err(e) => return e,
        };
        let b = match self.read_numeric_matrix(&args[1], cell) {
            Ok(m) => m,
            Err(e) => return e,
        };
        let m = a.len();
        let n = a[0].len();
        let p = b[0].len();
        if b.len() != n {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "MMULT dimension mismatch".to_string(),
            );
        }
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(m);
        for i in 0..m {
            let mut new_row = Vec::with_capacity(p);
            for j in 0..p {
                let mut sum = 0.0;
                for k in 0..n {
                    sum += a[i][k] * b[k][j];
                }
                new_row.push(ArrayNode::Number(sum));
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_minverse(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let matrix = match self.read_square_matrix(&args[0], cell) {
            Ok(m) => m,
            Err(e) => return e,
        };
        let n = matrix.len();
        let mut aug: Vec<Vec<f64>> = Vec::with_capacity(n);
        for (i, row) in matrix.iter().enumerate() {
            let mut new_row = row.clone();
            for j in 0..n {
                new_row.push(if i == j { 1.0 } else { 0.0 });
            }
            aug.push(new_row);
        }
        for column in 0..n {
            let mut pivot = column;
            let mut pivot_value = aug[column][column].abs();
            for row in (column + 1)..n {
                let value = aug[row][column].abs();
                if value > pivot_value {
                    pivot_value = value;
                    pivot = row;
                }
            }
            if aug[pivot][column] == 0.0 {
                return CalcResult::new_error(Error::NUM, cell, "MINVERSE: singular".to_string());
            }
            if pivot != column {
                aug.swap(pivot, column);
            }
            let pivot_diag = aug[column][column];
            for col in 0..(2 * n) {
                aug[column][col] /= pivot_diag;
            }
            for row in 0..n {
                if row != column {
                    let factor = aug[row][column];
                    if factor != 0.0 {
                        for col in 0..(2 * n) {
                            aug[row][col] -= factor * aug[column][col];
                        }
                    }
                }
            }
        }
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(n);
        for row in &aug {
            out.push(row[n..(2 * n)].iter().map(|v| ArrayNode::Number(*v)).collect());
        }
        CalcResult::Array(out)
    }

    pub(crate) fn fn_munit(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let n = match self.get_number(&args[0], cell) {
            Ok(f) => f.trunc() as i64,
            Err(e) => return e,
        };
        if n < 1 {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "MUNIT requires a positive dimension".to_string(),
            );
        }
        let n = n as usize;
        let mut out: Vec<Vec<ArrayNode>> = Vec::with_capacity(n);
        for i in 0..n {
            let mut new_row = Vec::with_capacity(n);
            for j in 0..n {
                new_row.push(ArrayNode::Number(if i == j { 1.0 } else { 0.0 }));
            }
            out.push(new_row);
        }
        CalcResult::Array(out)
    }

    fn read_regression_inputs(
        &mut self,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> Result<(Vec<f64>, Vec<Vec<f64>>), CalcResult> {
        let ys_grid = self.read_array_arg(&args[0], cell)?;
        let ys_nums = grid_to_numbers(&ys_grid).ok_or_else(|| {
            CalcResult::new_error(Error::VALUE, cell, "known_ys must be numeric".to_string())
        })?;
        let ys: Vec<f64> = ys_nums.into_iter().flatten().collect();
        let n = ys.len();
        if n == 0 {
            return Err(CalcResult::new_error(
                Error::VALUE,
                cell,
                "known_ys is empty".to_string(),
            ));
        }
        let xs_present = args.len() >= 2
            && !matches!(
                self.evaluate_node_in_context(&args[1], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            );
        let xs = if xs_present {
            let grid = self.read_array_arg(&args[1], cell)?;
            let nums = grid_to_numbers(&grid).ok_or_else(|| {
                CalcResult::new_error(Error::VALUE, cell, "known_xs must be numeric".to_string())
            })?;
            normalize_by_obs(&nums, n).ok_or_else(|| {
                CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "known_xs length mismatch".to_string(),
                )
            })?
        } else {
            (1..=n).map(|i| vec![i as f64]).collect()
        };
        Ok((ys, xs))
    }

    pub(crate) fn fn_linest(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let (ys, xs) = match self.read_regression_inputs(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let with_const = if args.len() >= 3
            && !matches!(
                self.evaluate_node_in_context(&args[2], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            ) {
            match self.get_boolean(&args[2], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            true
        };
        let stats = if args.len() >= 4
            && !matches!(
                self.evaluate_node_in_context(&args[3], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            ) {
            match self.get_boolean(&args[3], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            false
        };
        let fit = match ols_fit(&ys, &xs, with_const) {
            Some(f) => f,
            None => {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "LINEST: cannot fit model".to_string(),
                )
            }
        };
        let k = fit.coeffs.len();
        let width = k + 1;
        let mut row0: Vec<ArrayNode> = Vec::with_capacity(width);
        for j in (0..k).rev() {
            row0.push(ArrayNode::Number(fit.coeffs[j]));
        }
        row0.push(ArrayNode::Number(fit.intercept));
        if !stats {
            return CalcResult::Array(vec![row0]);
        }
        let mut row1: Vec<ArrayNode> = Vec::with_capacity(width);
        for j in (0..k).rev() {
            row1.push(ArrayNode::Number(fit.se_coeffs[j]));
        }
        row1.push(ArrayNode::Number(fit.se_intercept));
        let pad = |first: f64, second: f64| -> Vec<ArrayNode> {
            let mut row = vec![ArrayNode::Number(first), ArrayNode::Number(second)];
            while row.len() < width {
                row.push(ArrayNode::Error(Error::NA));
            }
            row
        };
        let row2 = pad(fit.r2, fit.sey);
        let row3 = pad(fit.f, fit.df);
        let row4 = pad(fit.ss_reg, fit.ss_resid);
        CalcResult::Array(vec![row0, row1, row2, row3, row4])
    }

    pub(crate) fn fn_trend(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let (ys, xs) = match self.read_regression_inputs(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let with_const = if args.len() >= 4
            && !matches!(
                self.evaluate_node_in_context(&args[3], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            ) {
            match self.get_boolean(&args[3], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            true
        };
        let fit = match ols_fit(&ys, &xs, with_const) {
            Some(f) => f,
            None => {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "TREND: cannot fit model".to_string(),
                )
            }
        };
        let k = fit.coeffs.len();
        let new_present = args.len() >= 3
            && !matches!(
                self.evaluate_node_in_context(&args[2], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            );
        let (new_xs, as_row) = if new_present {
            let grid = match self.read_array_arg(&args[2], cell) {
                Ok(g) => g,
                Err(e) => return e,
            };
            let nums = match grid_to_numbers(&grid) {
                Some(v) => v,
                None => {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "new_xs must be numeric".to_string(),
                    )
                }
            };
            match normalize_by_predictors(&nums, k) {
                Some(v) => v,
                None => {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "new_xs predictor mismatch".to_string(),
                    )
                }
            }
        } else {
            (xs.clone(), false)
        };
        let mut preds: Vec<f64> = Vec::with_capacity(new_xs.len());
        for row in &new_xs {
            if row.len() != k {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "new_xs predictor mismatch".to_string(),
                );
            }
            let mut yhat = fit.intercept;
            for (j, v) in row.iter().enumerate() {
                yhat += fit.coeffs[j] * v;
            }
            preds.push(yhat);
        }
        if as_row {
            CalcResult::Array(vec![preds.into_iter().map(ArrayNode::Number).collect()])
        } else {
            CalcResult::Array(preds.into_iter().map(|p| vec![ArrayNode::Number(p)]).collect())
        }
    }

    pub(crate) fn fn_logest(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let (ys, xs) = match self.read_regression_inputs(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut ln_ys = Vec::with_capacity(ys.len());
        for y in &ys {
            if *y <= 0.0 {
                return CalcResult::new_error(
                    Error::NUM,
                    cell,
                    "LOGEST: known_ys must be positive".to_string(),
                );
            }
            ln_ys.push(y.ln());
        }
        let with_const = if args.len() >= 3
            && !matches!(
                self.evaluate_node_in_context(&args[2], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            ) {
            match self.get_boolean(&args[2], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            true
        };
        let stats = if args.len() >= 4
            && !matches!(
                self.evaluate_node_in_context(&args[3], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            ) {
            match self.get_boolean(&args[3], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            false
        };
        let fit = match ols_fit(&ln_ys, &xs, with_const) {
            Some(f) => f,
            None => {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "LOGEST: cannot fit model".to_string(),
                )
            }
        };
        let k = fit.coeffs.len();
        let width = k + 1;
        let mut row0: Vec<ArrayNode> = Vec::with_capacity(width);
        for j in (0..k).rev() {
            row0.push(ArrayNode::Number(fit.coeffs[j].exp()));
        }
        row0.push(ArrayNode::Number(fit.intercept.exp()));
        if !stats {
            return CalcResult::Array(vec![row0]);
        }
        let mut row1: Vec<ArrayNode> = Vec::with_capacity(width);
        for j in (0..k).rev() {
            row1.push(ArrayNode::Number(fit.se_coeffs[j]));
        }
        row1.push(ArrayNode::Number(fit.se_intercept));
        let pad = |first: f64, second: f64| -> Vec<ArrayNode> {
            let mut row = vec![ArrayNode::Number(first), ArrayNode::Number(second)];
            while row.len() < width {
                row.push(ArrayNode::Error(Error::NA));
            }
            row
        };
        let row2 = pad(fit.r2, fit.sey);
        let row3 = pad(fit.f, fit.df);
        let row4 = pad(fit.ss_reg, fit.ss_resid);
        CalcResult::Array(vec![row0, row1, row2, row3, row4])
    }

    pub(crate) fn fn_growth(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let (ys, xs) = match self.read_regression_inputs(args, cell) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut ln_ys = Vec::with_capacity(ys.len());
        for y in &ys {
            if *y <= 0.0 {
                return CalcResult::new_error(
                    Error::NUM,
                    cell,
                    "GROWTH: known_ys must be positive".to_string(),
                );
            }
            ln_ys.push(y.ln());
        }
        let with_const = if args.len() >= 4
            && !matches!(
                self.evaluate_node_in_context(&args[3], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            ) {
            match self.get_boolean(&args[3], cell) {
                Ok(b) => b,
                Err(e) => return e,
            }
        } else {
            true
        };
        let fit = match ols_fit(&ln_ys, &xs, with_const) {
            Some(f) => f,
            None => {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "GROWTH: cannot fit model".to_string(),
                )
            }
        };
        let k = fit.coeffs.len();
        let new_present = args.len() >= 3
            && !matches!(
                self.evaluate_node_in_context(&args[2], cell),
                CalcResult::EmptyCell | CalcResult::EmptyArg
            );
        let (new_xs, as_row) = if new_present {
            let grid = match self.read_array_arg(&args[2], cell) {
                Ok(g) => g,
                Err(e) => return e,
            };
            let nums = match grid_to_numbers(&grid) {
                Some(v) => v,
                None => {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "new_xs must be numeric".to_string(),
                    )
                }
            };
            match normalize_by_predictors(&nums, k) {
                Some(v) => v,
                None => {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "new_xs predictor mismatch".to_string(),
                    )
                }
            }
        } else {
            (xs.clone(), false)
        };
        let mut preds: Vec<f64> = Vec::with_capacity(new_xs.len());
        for row in &new_xs {
            if row.len() != k {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "new_xs predictor mismatch".to_string(),
                );
            }
            let mut yhat = fit.intercept;
            for (j, v) in row.iter().enumerate() {
                yhat += fit.coeffs[j] * v;
            }
            preds.push(yhat.exp());
        }
        if as_row {
            CalcResult::Array(vec![preds.into_iter().map(ArrayNode::Number).collect()])
        } else {
            CalcResult::Array(preds.into_iter().map(|p| vec![ArrayNode::Number(p)]).collect())
        }
    }
}
