use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_a1, quote_sheet_name};
use crate::xml::mark_formulas_stale;
use crate::{Result, Workbook};

const MAX_ROW: u32 = 1_048_576;
const MAX_COLUMN: u32 = 16_384;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ShiftOp {
    InsertRow { before: u32, count: u32 },
    DeleteRow { start: u32, count: u32 },
    InsertCol { before: u32, count: u32 },
    DeleteCol { start: u32, count: u32 },
}

impl ShiftOp {
    fn is_row(&self) -> bool {
        matches!(self, ShiftOp::InsertRow { .. } | ShiftOp::DeleteRow { .. })
    }
}

impl Workbook {
    pub fn insert_rows(
        &mut self,
        sheet: impl AsRef<str>,
        before: u32,
        count: u32,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_row_index(before, &sheet)?;
        validate_count(count)?;
        self.apply_structural(&sheet, ShiftOp::InsertRow { before, count })
    }

    pub fn delete_rows(
        &mut self,
        sheet: impl AsRef<str>,
        start: u32,
        count: u32,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_row_index(start, &sheet)?;
        validate_count(count)?;
        if start.saturating_add(count).saturating_sub(1) > MAX_ROW {
            return Err(ApiError::new(
                ApiErrorCode::InvalidRef,
                "delete row range exceeds sheet bounds",
            )
            .with_sheet(&sheet));
        }
        self.apply_structural(&sheet, ShiftOp::DeleteRow { start, count })
    }

    pub fn insert_columns(
        &mut self,
        sheet: impl AsRef<str>,
        before: u32,
        count: u32,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_column_index(before, &sheet)?;
        validate_count(count)?;
        self.apply_structural(&sheet, ShiftOp::InsertCol { before, count })
    }

    pub fn delete_columns(
        &mut self,
        sheet: impl AsRef<str>,
        start: u32,
        count: u32,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_column_index(start, &sheet)?;
        validate_count(count)?;
        if start.saturating_add(count).saturating_sub(1) > MAX_COLUMN {
            return Err(ApiError::new(
                ApiErrorCode::InvalidRef,
                "delete column range exceeds sheet bounds",
            )
            .with_sheet(&sheet));
        }
        self.apply_structural(&sheet, ShiftOp::DeleteCol { start, count })
    }

    fn apply_structural(&mut self, target: &str, op: ShiftOp) -> Result<()> {
        let part = self.worksheet_part_for_sheet(target)?;
        {
            let ws = part
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            shift_sheet_data(ws, op);
            shift_columns_metadata(ws, op);
            shift_merges(ws, op);
        }

        let sheet_names: Vec<String> = self
            .workbook_sheets()?
            .iter()
            .map(|s| s.name.as_str().to_string())
            .collect();
        for name in &sheet_names {
            let p = self.worksheet_part_for_sheet(name)?;
            let ws = p
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            rewrite_formulas(ws, name, target, op);
        }

        mark_formulas_stale(&mut self.doc)?;
        Ok(())
    }
}

fn validate_row_index(row: u32, sheet: &str) -> Result<()> {
    if row == 0 || row > MAX_ROW {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("row index out of bounds: {row}"),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

fn validate_column_index(col: u32, sheet: &str) -> Result<()> {
    if col == 0 || col > MAX_COLUMN {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("column index out of bounds: {col}"),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

fn validate_count(count: u32) -> Result<()> {
    if count == 0 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            "count must be at least 1",
        ));
    }
    Ok(())
}

fn shift_sheet_data(ws: &mut x::Worksheet, op: ShiftOp) {
    let rows = &mut ws.x_sheet_data.x_row;
    match op {
        ShiftOp::InsertRow { before, count } => {
            for row in rows.iter_mut() {
                let Some(idx) = row.row_index else { continue };
                if idx >= before {
                    let new_idx = (idx + count).min(MAX_ROW);
                    row.row_index = Some(new_idx);
                    update_cell_refs(row, |r, c| (r + count, c));
                }
            }
            rows.retain(|r| r.row_index.map(|i| i <= MAX_ROW).unwrap_or(true));
        }
        ShiftOp::DeleteRow { start, count } => {
            let end = start + count - 1;
            rows.retain_mut(|row| {
                let Some(idx) = row.row_index else { return true };
                if idx >= start && idx <= end {
                    return false;
                }
                if idx > end {
                    let new_idx = idx - count;
                    row.row_index = Some(new_idx);
                    update_cell_refs(row, |r, c| (r - count, c));
                }
                true
            });
        }
        ShiftOp::InsertCol { before, count } => {
            for row in rows.iter_mut() {
                for cell in &mut row.x_c {
                    if let Some(reference) = cell.cell_reference.as_ref() {
                        if let Some((r, c)) = xlcore_io::parse_a1(reference) {
                            if c >= before {
                                let new_c = (c + count).min(MAX_COLUMN);
                                cell.cell_reference =
                                    Some(format!("{}{}", xlcore_io::col_label(new_c), r));
                            }
                        }
                    }
                }
                row.x_c.retain(|cell| {
                    cell.cell_reference
                        .as_ref()
                        .and_then(|r| xlcore_io::parse_a1(r))
                        .map(|(_, c)| c <= MAX_COLUMN)
                        .unwrap_or(true)
                });
            }
        }
        ShiftOp::DeleteCol { start, count } => {
            let end = start + count - 1;
            for row in rows.iter_mut() {
                row.x_c.retain_mut(|cell| {
                    let Some(reference) = cell.cell_reference.as_ref() else {
                        return true;
                    };
                    let Some((r, c)) = xlcore_io::parse_a1(reference) else {
                        return true;
                    };
                    if c >= start && c <= end {
                        return false;
                    }
                    if c > end {
                        let new_c = c - count;
                        cell.cell_reference =
                            Some(format!("{}{}", xlcore_io::col_label(new_c), r));
                    }
                    true
                });
            }
        }
    }
}

fn update_cell_refs<F: Fn(u32, u32) -> (u32, u32)>(row: &mut x::Row, f: F) {
    for cell in &mut row.x_c {
        if let Some(reference) = cell.cell_reference.as_ref() {
            if let Some((r, c)) = xlcore_io::parse_a1(reference) {
                let (nr, nc) = f(r, c);
                cell.cell_reference = Some(format!("{}{}", xlcore_io::col_label(nc), nr));
            }
        }
    }
}

fn shift_columns_metadata(ws: &mut x::Worksheet, op: ShiftOp) {
    match op {
        ShiftOp::InsertCol { before, count } => {
            for cols in &mut ws.x_cols {
                cols.x_col.retain_mut(|c| {
                    if c.max < before {
                    } else if c.min >= before {
                        c.min = (c.min + count).min(MAX_COLUMN);
                        c.max = (c.max + count).min(MAX_COLUMN);
                    } else {
                        c.max = (c.max + count).min(MAX_COLUMN);
                    }
                    c.min <= c.max && c.min <= MAX_COLUMN
                });
            }
        }
        ShiftOp::DeleteCol { start, count } => {
            let end = start + count - 1;
            for cols in &mut ws.x_cols {
                cols.x_col.retain_mut(|c| {
                    let new_min = if c.min < start {
                        c.min
                    } else if c.min > end {
                        c.min - count
                    } else {
                        start
                    };
                    let new_max = if c.max < start {
                        c.max
                    } else if c.max > end {
                        c.max - count
                    } else if start == 0 {
                        0
                    } else {
                        start - 1
                    };
                    if new_min > new_max {
                        return false;
                    }
                    c.min = new_min;
                    c.max = new_max;
                    true
                });
            }
        }
        _ => {}
    }
}

fn shift_merges(ws: &mut x::Worksheet, op: ShiftOp) {
    let Some(merges) = ws.x_merge_cells.as_mut() else {
        return;
    };
    let mut kept: Vec<x::MergeCell> = Vec::new();
    for m in std::mem::take(&mut merges.x_merge_cell) {
        let Some((r1, c1, r2, c2)) = parse_range_a1(m.reference.as_str()) else {
            kept.push(m);
            continue;
        };
        let updated = match op {
            ShiftOp::InsertRow { before, count } => {
                let (nr1, nr2) = shift_range_insert(r1, r2, before, count, MAX_ROW);
                Some((nr1, c1, nr2, c2))
            }
            ShiftOp::DeleteRow { start, count } => {
                shift_range_delete(r1, r2, start, count).map(|(nr1, nr2)| (nr1, c1, nr2, c2))
            }
            ShiftOp::InsertCol { before, count } => {
                let (nc1, nc2) = shift_range_insert(c1, c2, before, count, MAX_COLUMN);
                Some((r1, nc1, r2, nc2))
            }
            ShiftOp::DeleteCol { start, count } => {
                shift_range_delete(c1, c2, start, count).map(|(nc1, nc2)| (r1, nc1, r2, nc2))
            }
        };
        if let Some((nr1, nc1, nr2, nc2)) = updated {
            if nr1 == nr2 && nc1 == nc2 {
                continue;
            }
            kept.push(x::MergeCell {
                reference: format!(
                    "{}{}:{}{}",
                    xlcore_io::col_label(nc1),
                    nr1,
                    xlcore_io::col_label(nc2),
                    nr2,
                ),
            });
        }
    }
    if kept.is_empty() {
        ws.x_merge_cells = None;
    } else {
        merges.count = Some(kept.len() as u32);
        merges.x_merge_cell = kept;
    }
}

fn shift_range_insert(s: u32, e: u32, before: u32, count: u32, max: u32) -> (u32, u32) {
    let ns = if s >= before { (s + count).min(max) } else { s };
    let ne = if e >= before { (e + count).min(max) } else { e };
    (ns, ne)
}

fn shift_range_delete(s: u32, e: u32, start: u32, count: u32) -> Option<(u32, u32)> {
    let end = start + count - 1;
    if end < s {
        return Some((s - count, e - count));
    }
    if start > e {
        return Some((s, e));
    }
    if start <= s && end >= e {
        return None;
    }
    if start <= s {
        return Some((start, e - count));
    }
    if end >= e {
        return Some((s, start - 1));
    }
    Some((s, e - count))
}

fn shift_single_insert(v: u32, before: u32, count: u32, max: u32) -> Option<u32> {
    if v >= before {
        let n = v + count;
        if n > max {
            None
        } else {
            Some(n)
        }
    } else {
        Some(v)
    }
}

fn shift_single_delete(v: u32, start: u32, count: u32) -> Option<u32> {
    let end = start + count - 1;
    if v < start {
        Some(v)
    } else if v <= end {
        None
    } else {
        Some(v - count)
    }
}

fn rewrite_formulas(ws: &mut x::Worksheet, owning: &str, target: &str, op: ShiftOp) {
    for row in &mut ws.x_sheet_data.x_row {
        for cell in &mut row.x_c {
            if let Some(formula) = cell.cell_formula.as_mut() {
                if let Some(text) = formula.xml_content.as_mut() {
                    *text = shift_formula_refs(text, owning, target, op);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Endpoint {
    col_abs: bool,
    col: Option<u32>,
    row_abs: bool,
    row: Option<u32>,
}

impl Endpoint {
    fn kind(&self) -> EndpointKind {
        match (self.col.is_some(), self.row.is_some()) {
            (true, true) => EndpointKind::Cell,
            (true, false) => EndpointKind::ColOnly,
            (false, true) => EndpointKind::RowOnly,
            _ => EndpointKind::Invalid,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EndpointKind {
    Cell,
    ColOnly,
    RowOnly,
    Invalid,
}

fn shift_formula_refs(src: &str, owning: &str, target: &str, op: ShiftOp) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'"' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                out.push_str(&src[start..i]);
            }
            b'\'' => {
                let (sheet, after) = match consume_quoted_sheet(bytes, i) {
                    Some(v) => v,
                    None => {
                        out.push('\'');
                        i += 1;
                        continue;
                    }
                };
                if after < bytes.len() && bytes[after] == b'!' {
                    let body_start = after + 1;
                    let sheet_prefix = format!("'{}'!", sheet.replace('\'', "''"));
                    let (rendered, consumed) = try_consume_and_rewrite_ref(
                        bytes,
                        body_start,
                        Some(&sheet),
                        owning,
                        target,
                        op,
                        &sheet_prefix,
                    );
                    out.push_str(&rendered);
                    i = body_start + consumed;
                } else {
                    out.push('\'');
                    out.push_str(&sheet.replace('\'', "''"));
                    out.push('\'');
                    i = after;
                }
            }
            _ if c.is_ascii_alphabetic() || c == b'_' => {
                let id_end = read_identifier_end(bytes, i);
                let ident = &src[i..id_end];
                if id_end < bytes.len() && bytes[id_end] == b'!' {
                    let body_start = id_end + 1;
                    let prefix = format!("{ident}!");
                    let (rendered, consumed) = try_consume_and_rewrite_ref(
                        bytes,
                        body_start,
                        Some(ident),
                        owning,
                        target,
                        op,
                        &prefix,
                    );
                    out.push_str(&rendered);
                    i = body_start + consumed;
                    continue;
                }
                if next_non_space(bytes, id_end) == Some(b'(') {
                    out.push_str(ident);
                    i = id_end;
                    continue;
                }
                if let Some((rendered, consumed)) =
                    try_rewrite_bare_identifier(ident, bytes, i, id_end, owning, target, op)
                {
                    out.push_str(&rendered);
                    i = i + consumed;
                } else {
                    out.push_str(ident);
                    i = id_end;
                }
            }
            b'$' | b'0'..=b'9' => {
                let (rendered, consumed) = try_consume_and_rewrite_ref(
                    bytes, i, None, owning, target, op, "",
                );
                if consumed > 0 {
                    out.push_str(&rendered);
                    i += consumed;
                } else if c.is_ascii_digit() {
                    let end = read_number_end(bytes, i);
                    out.push_str(&src[i..end]);
                    i = end;
                } else {
                    out.push(c as char);
                    i += 1;
                }
            }
            _ => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn try_rewrite_bare_identifier(
    ident: &str,
    bytes: &[u8],
    start: usize,
    id_end: usize,
    owning: &str,
    target: &str,
    op: ShiftOp,
) -> Option<(String, usize)> {
    if !ident.bytes().all(|b| b.is_ascii_alphabetic()) {
        let (e1, p1) = parse_endpoint_strict(ident.as_bytes(), 0)?;
        if p1 != ident.len() {
            return None;
        }
        return finalize_ref(bytes, start, id_end, e1, None, owning, target, op);
    }
    let col = letters_to_col(ident.as_bytes())?;
    let e1 = Endpoint {
        col_abs: false,
        col: Some(col),
        row_abs: false,
        row: None,
    };
    finalize_ref(bytes, start, id_end, e1, None, owning, target, op)
}

fn finalize_ref(
    bytes: &[u8],
    start: usize,
    end_of_first: usize,
    e1: Endpoint,
    sheet: Option<&str>,
    owning: &str,
    target: &str,
    op: ShiftOp,
) -> Option<(String, usize)> {
    let mut total = end_of_first;
    let mut endpoints = (e1, None);
    if total < bytes.len() && bytes[total] == b':' {
        if let Some((e2, p2)) = parse_endpoint(bytes, total + 1) {
            if e1.kind() == e2.kind() && e1.kind() != EndpointKind::Invalid {
                endpoints.1 = Some(e2);
                total = p2;
            }
        }
    }
    let prefix = if let Some(s) = sheet {
        format!("{}!", quote_sheet_name(s))
    } else {
        String::new()
    };
    let rendered = rewrite_ref_token(endpoints.0, endpoints.1, sheet, owning, target, op, &prefix);
    Some((rendered, total - start))
}

fn try_consume_and_rewrite_ref(
    bytes: &[u8],
    i: usize,
    sheet: Option<&str>,
    owning: &str,
    target: &str,
    op: ShiftOp,
    prefix_literal: &str,
) -> (String, usize) {
    let Some((e1, p1)) = parse_endpoint(bytes, i) else {
        return (prefix_literal.to_string(), 0);
    };
    let mut total_end = p1;
    let mut end = None;
    if p1 < bytes.len() && bytes[p1] == b':' {
        if let Some((e2, p2)) = parse_endpoint(bytes, p1 + 1) {
            if e1.kind() == e2.kind() && e1.kind() != EndpointKind::Invalid {
                end = Some(e2);
                total_end = p2;
            }
        }
    }
    let rendered =
        rewrite_ref_token(e1, end, sheet, owning, target, op, prefix_literal);
    (rendered, total_end - i)
}

fn rewrite_ref_token(
    start: Endpoint,
    end: Option<Endpoint>,
    sheet: Option<&str>,
    owning: &str,
    target: &str,
    op: ShiftOp,
    prefix_literal: &str,
) -> String {
    let applies = match sheet {
        Some(s) => s.eq_ignore_ascii_case(target),
        None => owning.eq_ignore_ascii_case(target),
    };
    if !applies {
        return format!("{}{}", prefix_literal, render_ref_body(start, end));
    }

    let kind = start.kind();
    if end.is_some() && end.unwrap().kind() != kind {
        return format!("{}{}", prefix_literal, render_ref_body(start, end));
    }

    if op.is_row() && kind == EndpointKind::ColOnly {
        return format!("{}{}", prefix_literal, render_ref_body(start, end));
    }
    if !op.is_row() && kind == EndpointKind::RowOnly {
        return format!("{}{}", prefix_literal, render_ref_body(start, end));
    }

    if let Some(new) = apply_op_to_ref(start, end, op) {
        format!("{}{}", prefix_literal, render_ref_body(new.0, new.1))
    } else {
        format!("{}#REF!", prefix_literal)
    }
}

fn apply_op_to_ref(
    start: Endpoint,
    end: Option<Endpoint>,
    op: ShiftOp,
) -> Option<(Endpoint, Option<Endpoint>)> {
    let mut new_start = start;
    let mut new_end = end;
    match op {
        ShiftOp::InsertRow { before, count } => {
            if let Some(e) = end {
                let (ns, ne) =
                    shift_range_insert(start.row?, e.row?, before, count, MAX_ROW);
                new_start.row = Some(ns);
                if let Some(ref mut ee) = new_end {
                    ee.row = Some(ne);
                }
            } else if let Some(r) = start.row {
                let new = shift_single_insert(r, before, count, MAX_ROW)?;
                new_start.row = Some(new);
            }
        }
        ShiftOp::DeleteRow { start: s, count } => {
            if let Some(e) = end {
                let (nr1, nr2) = shift_range_delete(start.row?, e.row?, s, count)?;
                new_start.row = Some(nr1);
                if let Some(ref mut ee) = new_end {
                    ee.row = Some(nr2);
                }
            } else if let Some(r) = start.row {
                let new = shift_single_delete(r, s, count)?;
                new_start.row = Some(new);
            }
        }
        ShiftOp::InsertCol { before, count } => {
            if let Some(e) = end {
                let (ns, ne) =
                    shift_range_insert(start.col?, e.col?, before, count, MAX_COLUMN);
                new_start.col = Some(ns);
                if let Some(ref mut ee) = new_end {
                    ee.col = Some(ne);
                }
            } else if let Some(c) = start.col {
                let new = shift_single_insert(c, before, count, MAX_COLUMN)?;
                new_start.col = Some(new);
            }
        }
        ShiftOp::DeleteCol { start: s, count } => {
            if let Some(e) = end {
                let (nc1, nc2) = shift_range_delete(start.col?, e.col?, s, count)?;
                new_start.col = Some(nc1);
                if let Some(ref mut ee) = new_end {
                    ee.col = Some(nc2);
                }
            } else if let Some(c) = start.col {
                let new = shift_single_delete(c, s, count)?;
                new_start.col = Some(new);
            }
        }
    }
    Some((new_start, new_end))
}

fn render_ref_body(start: Endpoint, end: Option<Endpoint>) -> String {
    let mut s = render_endpoint(start);
    if let Some(e) = end {
        s.push(':');
        s.push_str(&render_endpoint(e));
    }
    s
}

fn render_endpoint(ep: Endpoint) -> String {
    let mut s = String::new();
    if let Some(col) = ep.col {
        if ep.col_abs {
            s.push('$');
        }
        s.push_str(&xlcore_io::col_label(col));
    }
    if let Some(row) = ep.row {
        if ep.row_abs {
            s.push('$');
        }
        s.push_str(&row.to_string());
    }
    s
}

fn parse_endpoint(bytes: &[u8], i: usize) -> Option<(Endpoint, usize)> {
    parse_endpoint_strict(bytes, i)
}

fn parse_endpoint_strict(bytes: &[u8], i: usize) -> Option<(Endpoint, usize)> {
    let mut p = i;
    let mut first_dollar = false;
    if p < bytes.len() && bytes[p] == b'$' {
        first_dollar = true;
        p += 1;
    }
    let col_start = p;
    while p < bytes.len() && bytes[p].is_ascii_alphabetic() {
        p += 1;
    }
    let col_bytes = &bytes[col_start..p];
    let has_col = !col_bytes.is_empty();

    let mut second_dollar = false;
    let second_dollar_pos = p;
    if p < bytes.len() && bytes[p] == b'$' {
        second_dollar = true;
        p += 1;
    }
    let row_start = p;
    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }
    let row_bytes = &bytes[row_start..p];
    let has_row = !row_bytes.is_empty();

    if second_dollar && !has_row {
        p = second_dollar_pos;
        second_dollar = false;
    }

    if !has_col && !has_row {
        return None;
    }

    let (col_abs, row_abs) = if has_col {
        (first_dollar, second_dollar)
    } else {
        (false, first_dollar || second_dollar)
    };

    let col = if has_col {
        match letters_to_col(col_bytes) {
            Some(v) if v <= MAX_COLUMN => Some(v),
            _ => return None,
        }
    } else {
        None
    };
    let row = if has_row {
        match digits_to_u32(row_bytes) {
            Some(v) if v >= 1 && v <= MAX_ROW => Some(v),
            _ => return None,
        }
    } else {
        None
    };

    Some((
        Endpoint {
            col_abs,
            col,
            row_abs,
            row,
        },
        p,
    ))
}

fn letters_to_col(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || bytes.len() > 3 {
        return None;
    }
    let mut col: u32 = 0;
    for b in bytes {
        if !b.is_ascii_alphabetic() {
            return None;
        }
        col = col.checked_mul(26)?.checked_add(
            b.to_ascii_uppercase() as u32 - b'A' as u32 + 1,
        )?;
    }
    Some(col)
}

fn digits_to_u32(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || bytes.len() > 7 {
        return None;
    }
    let mut row: u32 = 0;
    for b in bytes {
        row = row.checked_mul(10)?.checked_add((*b - b'0') as u32)?;
    }
    Some(row)
}

fn read_identifier_end(bytes: &[u8], i: usize) -> usize {
    let mut p = i;
    while p < bytes.len() {
        let c = bytes[p];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b'.' {
            p += 1;
        } else {
            break;
        }
    }
    p
}

fn read_number_end(bytes: &[u8], i: usize) -> usize {
    let mut p = i;
    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }
    if p < bytes.len() && bytes[p] == b'.' {
        p += 1;
        while p < bytes.len() && bytes[p].is_ascii_digit() {
            p += 1;
        }
    }
    if p < bytes.len() && (bytes[p] == b'e' || bytes[p] == b'E') {
        p += 1;
        if p < bytes.len() && (bytes[p] == b'+' || bytes[p] == b'-') {
            p += 1;
        }
        while p < bytes.len() && bytes[p].is_ascii_digit() {
            p += 1;
        }
    }
    p
}

fn next_non_space(bytes: &[u8], i: usize) -> Option<u8> {
    let mut p = i;
    while p < bytes.len() && bytes[p].is_ascii_whitespace() {
        p += 1;
    }
    bytes.get(p).copied()
}

fn consume_quoted_sheet(bytes: &[u8], i: usize) -> Option<(String, usize)> {
    debug_assert_eq!(bytes[i], b'\'');
    let mut p = i + 1;
    let mut name = String::new();
    while p < bytes.len() {
        if bytes[p] == b'\'' {
            if p + 1 < bytes.len() && bytes[p + 1] == b'\'' {
                name.push('\'');
                p += 2;
                continue;
            }
            return Some((name, p + 1));
        }
        name.push(bytes[p] as char);
        p += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_endpoints() {
        let (e, p) = parse_endpoint(b"$A$10", 0).unwrap();
        assert_eq!(p, 5);
        assert_eq!(e.col, Some(1));
        assert_eq!(e.row, Some(10));
        assert!(e.col_abs);
        assert!(e.row_abs);

        let (e, p) = parse_endpoint(b"AA", 0).unwrap();
        assert_eq!(p, 2);
        assert_eq!(e.col, Some(27));
        assert_eq!(e.row, None);

        let (e, p) = parse_endpoint(b"$5", 0).unwrap();
        assert_eq!(p, 2);
        assert_eq!(e.row, Some(5));
        assert!(e.row_abs);
        assert_eq!(e.col, None);
    }

    #[test]
    fn shifts_single_formula_refs() {
        let f = shift_formula_refs(
            "A1+B2",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow { before: 2, count: 1 },
        );
        assert_eq!(f, "A1+B3");

        let f = shift_formula_refs(
            "SUM(A1:A10)",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow { before: 5, count: 2 },
        );
        assert_eq!(f, "SUM(A1:A12)");

        let f = shift_formula_refs(
            "SUM(A1:A10)",
            "Sheet1",
            "Sheet1",
            ShiftOp::DeleteRow { start: 1, count: 10 },
        );
        assert_eq!(f, "SUM(#REF!)");

        let f = shift_formula_refs(
            "$A$5",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow { before: 2, count: 1 },
        );
        assert_eq!(f, "$A$6");
    }

    #[test]
    fn cross_sheet_refs() {
        let f = shift_formula_refs(
            "Sheet2!A1 + Sheet1!B2",
            "Other",
            "Sheet1",
            ShiftOp::InsertRow { before: 1, count: 1 },
        );
        assert_eq!(f, "Sheet2!A1 + Sheet1!B3");

        let f = shift_formula_refs(
            "'My Sheet'!A1",
            "Other",
            "My Sheet",
            ShiftOp::InsertCol { before: 1, count: 1 },
        );
        assert_eq!(f, "'My Sheet'!B1");
    }

    #[test]
    fn preserves_strings_and_functions() {
        let f = shift_formula_refs(
            r#"IF(A1>0,"A1 is big","A1")"#,
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow { before: 1, count: 1 },
        );
        assert_eq!(f, r#"IF(A2>0,"A1 is big","A1")"#);
    }

    #[test]
    fn column_only_refs() {
        let f = shift_formula_refs(
            "SUM(A:A)",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertCol { before: 1, count: 1 },
        );
        assert_eq!(f, "SUM(B:B)");

        let f = shift_formula_refs(
            "SUM(A:A)",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow { before: 1, count: 1 },
        );
        assert_eq!(f, "SUM(A:A)");
    }
}
