use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_a1, quote_sheet_name};
use crate::{Result, Workbook};

pub(crate) const MAX_ROW: u32 = 1_048_576;
pub(crate) const MAX_COLUMN: u32 = 16_384;

pub(crate) type RefRewriter<'a> =
    &'a mut dyn FnMut(Endpoint, Option<Endpoint>, Option<&str>, &str) -> String;

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
    pub fn insert_rows(&mut self, sheet: impl AsRef<str>, before: u32, count: u32) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_row_index(before, &sheet)?;
        validate_count(count)?;
        self.apply_structural(&sheet, ShiftOp::InsertRow { before, count })
    }

    pub fn delete_rows(&mut self, sheet: impl AsRef<str>, start: u32, count: u32) -> Result<()> {
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

    pub fn delete_columns(&mut self, sheet: impl AsRef<str>, start: u32, count: u32) -> Result<()> {
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
            shift_auto_filter(ws, op);
            shift_conditional_formatting_sqref(ws, op);
        }

        let sheet_names: Vec<String> = self
            .workbook_sheets()?
            .iter()
            .map(|s| s.name.as_str().to_string())
            .collect();
        for name in &sheet_names {
            let p = self.worksheet_part_for_sheet(name)?;
            let ws = p.root_element_mut(&mut self.doc).map_err(sdk_err_to_api)?;
            rewrite_formulas(ws, name, target, op);
            shift_conditional_formatting_formulas(ws, name, target, op);
        }

        let ws_part_target = self.worksheet_part_for_sheet(target)?;
        let table_parts: Vec<_> = ws_part_target.table_definition_parts(&self.doc).collect();
        for tp in table_parts {
            let table = tp.root_element_mut(&mut self.doc).map_err(sdk_err_to_api)?;
            shift_table(table, op);
        }

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        if let Some(dns) = workbook.defined_names.as_mut() {
            for dn in &mut dns.defined_name {
                let owning = dn
                    .local_sheet_id
                    .and_then(|i| sheet_names.get(i as usize).cloned())
                    .unwrap_or_default();
                if let Some(text) = dn.xml_content.as_mut() {
                    *text = shift_formula_refs(text, &owning, target, op);
                }
            }
        }

        self.mark_formulas_stale()?;
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
    let rows = &mut ws.sheet_data.row;
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
                let Some(idx) = row.row_index else {
                    return true;
                };
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
                for cell in &mut row.cell {
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
                row.cell.retain(|cell| {
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
                row.cell.retain_mut(|cell| {
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
                        cell.cell_reference = Some(format!("{}{}", xlcore_io::col_label(new_c), r));
                    }
                    true
                });
            }
        }
    }
}

fn update_cell_refs<F: Fn(u32, u32) -> (u32, u32)>(row: &mut x::Row, f: F) {
    for cell in &mut row.cell {
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
            for cols in &mut ws.columns {
                cols.column.retain_mut(|c| {
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
            for cols in &mut ws.columns {
                cols.column.retain_mut(|c| {
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
    let Some(merges) = ws.merge_cells.as_mut() else {
        return;
    };
    let mut kept: Vec<x::MergeCell> = Vec::new();
    for m in std::mem::take(&mut merges.merge_cell) {
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
        ws.merge_cells = None;
    } else {
        merges.count = Some(kept.len() as u32);
        merges.merge_cell = kept;
    }
}

fn shift_auto_filter(ws: &mut x::Worksheet, op: ShiftOp) {
    if let Some(af) = ws.auto_filter.as_mut() {
        if let Some(reference) = af.reference.as_mut() {
            match shift_a1_range_str(reference, op) {
                RangeShift::Kept(s) => *reference = s,
                RangeShift::Collapsed => {
                    ws.auto_filter = None;
                    return;
                }
                RangeShift::Unparsed => {}
            }
        }
    }
}

fn shift_conditional_formatting_sqref(ws: &mut x::Worksheet, op: ShiftOp) {
    ws.conditional_formatting.retain_mut(|cf| {
        let Some(sqref) = cf.sequence_of_references.as_mut() else {
            return true;
        };
        let combined = sqref.join(" ");
        let mut new_parts: Vec<String> = Vec::new();
        for part in combined.split_whitespace() {
            match shift_a1_range_str(part, op) {
                RangeShift::Kept(s) => new_parts.push(s),
                RangeShift::Collapsed => {}
                RangeShift::Unparsed => new_parts.push(part.to_string()),
            }
        }
        if new_parts.is_empty() {
            return false;
        }
        *sqref = new_parts;
        true
    });
}

fn shift_conditional_formatting_formulas(
    ws: &mut x::Worksheet,
    owning: &str,
    target: &str,
    op: ShiftOp,
) {
    for cf in &mut ws.conditional_formatting {
        for rule in &mut cf.conditional_formatting_rule {
            for f in &mut rule.formula {
                if let Some(text) = f.xml_content.as_mut() {
                    *text = shift_formula_refs(text, owning, target, op);
                }
            }
        }
    }
}

fn shift_table(table: &mut x::Table, op: ShiftOp) {
    match shift_a1_range_str(table.reference.as_str(), op) {
        RangeShift::Kept(s) => table.reference = s,
        RangeShift::Collapsed | RangeShift::Unparsed => {}
    }
    if let Some(af) = table.auto_filter.as_mut() {
        if let Some(reference) = af.reference.as_mut() {
            if let RangeShift::Kept(s) = shift_a1_range_str(reference, op) {
                *reference = s;
            }
        }
    }
}

enum RangeShift {
    Kept(String),
    Collapsed,
    Unparsed,
}

fn shift_a1_range_str(s: &str, op: ShiftOp) -> RangeShift {
    let trimmed = s.trim();
    let bytes = trimmed.as_bytes();
    let Some((e1, p1)) = parse_endpoint(bytes, 0) else {
        return RangeShift::Unparsed;
    };
    let (e2, p2) = if p1 < bytes.len() && bytes[p1] == b':' {
        let Some((e2, p2)) = parse_endpoint(bytes, p1 + 1) else {
            return RangeShift::Unparsed;
        };
        (Some(e2), p2)
    } else {
        (None, p1)
    };
    if p2 != bytes.len() {
        return RangeShift::Unparsed;
    }
    let kind = e1.kind();
    if kind == EndpointKind::Invalid {
        return RangeShift::Unparsed;
    }
    if let Some(e2) = e2 {
        if e2.kind() != kind {
            return RangeShift::Unparsed;
        }
    }
    if op.is_row() && kind == EndpointKind::ColOnly {
        return RangeShift::Kept(trimmed.to_string());
    }
    if !op.is_row() && kind == EndpointKind::RowOnly {
        return RangeShift::Kept(trimmed.to_string());
    }
    match apply_op_to_ref(e1, e2, op) {
        Some((ns, ne)) => RangeShift::Kept(render_ref_body(ns, ne)),
        None => RangeShift::Collapsed,
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
    for row in &mut ws.sheet_data.row {
        for cell in &mut row.cell {
            if let Some(formula) = cell.cell_formula.as_mut() {
                if let Some(text) = formula.xml_content.as_mut() {
                    *text = shift_formula_refs(text, owning, target, op);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub(crate) col_abs: bool,
    pub(crate) col: Option<u32>,
    pub(crate) row_abs: bool,
    pub(crate) row: Option<u32>,
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
pub(crate) enum EndpointKind {
    Cell,
    ColOnly,
    RowOnly,
    Invalid,
}

fn shift_formula_refs(src: &str, owning: &str, target: &str, op: ShiftOp) -> String {
    let mut rewrite =
        |start: Endpoint, end: Option<Endpoint>, sheet: Option<&str>, prefix_literal: &str| {
            rewrite_ref_token(start, end, sheet, owning, target, op, prefix_literal)
        };
    walk_formula_refs(src, &mut rewrite)
}

pub(crate) fn rename_sheet_in_formula_refs(src: &str, old_name: &str, new_name: &str) -> String {
    let new_prefix = format!("{}!", quote_sheet_name(new_name));
    let mut rewrite =
        |start: Endpoint, end: Option<Endpoint>, sheet: Option<&str>, prefix_literal: &str| {
            let prefix = match sheet {
                Some(s) if s.eq_ignore_ascii_case(old_name) => new_prefix.as_str(),
                _ => prefix_literal,
            };
            format!("{}{}", prefix, render_ref_body(start, end))
        };
    walk_formula_refs(src, &mut rewrite)
}

pub(crate) fn translate_formula_refs(src: &str, dr: i64, dc: i64) -> String {
    let mut rewrite =
        |start: Endpoint, end: Option<Endpoint>, _sheet: Option<&str>, prefix_literal: &str| {
            translate_ref_token(start, end, dr, dc, prefix_literal)
        };
    walk_formula_refs(src, &mut rewrite)
}

#[derive(Clone, Copy)]
pub(crate) struct MoveRect {
    pub(crate) start_row: u32,
    pub(crate) end_row: u32,
    pub(crate) start_column: u32,
    pub(crate) end_column: u32,
}

pub(crate) fn move_formula_refs(
    src: &str,
    owning: &str,
    src_sheet: &str,
    rect: MoveRect,
    dr: i64,
    dc: i64,
) -> String {
    let mut rewrite =
        |start: Endpoint, end: Option<Endpoint>, sheet: Option<&str>, prefix_literal: &str| {
            let target_sheet = sheet.unwrap_or(owning);
            if !target_sheet.eq_ignore_ascii_case(src_sheet) {
                return format!("{}{}", prefix_literal, render_ref_body(start, end));
            }
            let inside = |e: &Endpoint| {
                matches!(e.kind(), EndpointKind::Cell)
                    && e.col.is_some_and(|c| c >= rect.start_column && c <= rect.end_column)
                    && e.row.is_some_and(|r| r >= rect.start_row && r <= rect.end_row)
            };
            let all_in = inside(&start) && end.as_ref().map(inside).unwrap_or(true);
            if !all_in {
                return format!("{}{}", prefix_literal, render_ref_body(start, end));
            }
            let displace = |e: Endpoint| Endpoint {
                col: e.col.map(|c| (c as i64 + dc) as u32),
                row: e.row.map(|r| (r as i64 + dr) as u32),
                ..e
            };
            format!(
                "{}{}",
                prefix_literal,
                render_ref_body(displace(start), end.map(displace))
            )
        };
    walk_formula_refs(src, &mut rewrite)
}

fn translate_ref_token(
    start: Endpoint,
    end: Option<Endpoint>,
    dr: i64,
    dc: i64,
    prefix_literal: &str,
) -> String {
    let kind = start.kind();
    if kind == EndpointKind::Invalid {
        return format!("{}{}", prefix_literal, render_ref_body(start, end));
    }
    if let Some(e) = end {
        if e.kind() != kind {
            return format!("{}{}", prefix_literal, render_ref_body(start, end));
        }
    }
    if kind == EndpointKind::ColOnly && dc == 0 {
        return format!("{}{}", prefix_literal, render_ref_body(start, end));
    }
    if kind == EndpointKind::RowOnly && dr == 0 {
        return format!("{}{}", prefix_literal, render_ref_body(start, end));
    }
    let Some(new_start) = translate_endpoint(start, dr, dc) else {
        return format!("{}#REF!", prefix_literal);
    };
    let new_end = match end {
        Some(e) => match translate_endpoint(e, dr, dc) {
            Some(v) => Some(v),
            None => return format!("{}#REF!", prefix_literal),
        },
        None => None,
    };
    format!("{}{}", prefix_literal, render_ref_body(new_start, new_end))
}

fn translate_endpoint(ep: Endpoint, dr: i64, dc: i64) -> Option<Endpoint> {
    let new_row = match ep.row {
        Some(r) if !ep.row_abs => {
            let nr = (r as i64) + dr;
            if nr < 1 || nr > MAX_ROW as i64 {
                return None;
            }
            Some(nr as u32)
        }
        other => other,
    };
    let new_col = match ep.col {
        Some(c) if !ep.col_abs => {
            let nc = (c as i64) + dc;
            if nc < 1 || nc > MAX_COLUMN as i64 {
                return None;
            }
            Some(nc as u32)
        }
        other => other,
    };
    Some(Endpoint {
        col_abs: ep.col_abs,
        col: new_col,
        row_abs: ep.row_abs,
        row: new_row,
    })
}

fn walk_formula_refs(src: &str, rewrite: RefRewriter<'_>) -> String {
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
                        rewrite,
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
                        rewrite,
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
                    try_rewrite_bare_identifier(ident, bytes, i, id_end, rewrite)
                {
                    out.push_str(&rendered);
                    i = i + consumed;
                } else {
                    out.push_str(ident);
                    i = id_end;
                }
            }
            b'$' | b'0'..=b'9' => {
                let (rendered, consumed) = try_consume_and_rewrite_ref(bytes, i, None, rewrite, "");
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
    rewrite: RefRewriter<'_>,
) -> Option<(String, usize)> {
    if !ident.bytes().all(|b| b.is_ascii_alphabetic()) {
        let (e1, p1) = parse_endpoint_strict(ident.as_bytes(), 0)?;
        if p1 != ident.len() {
            return None;
        }
        return finalize_ref(bytes, start, id_end, e1, None, rewrite);
    }
    let col = letters_to_col(ident.as_bytes())?;
    let e1 = Endpoint {
        col_abs: false,
        col: Some(col),
        row_abs: false,
        row: None,
    };
    finalize_ref(bytes, start, id_end, e1, None, rewrite)
}

fn finalize_ref(
    bytes: &[u8],
    start: usize,
    end_of_first: usize,
    e1: Endpoint,
    sheet: Option<&str>,
    rewrite: RefRewriter<'_>,
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
    if endpoints.1.is_none() && e1.kind() != EndpointKind::Cell {
        return None;
    }
    let prefix = if let Some(s) = sheet {
        format!("{}!", quote_sheet_name(s))
    } else {
        String::new()
    };
    let rendered = rewrite(endpoints.0, endpoints.1, sheet, &prefix);
    Some((rendered, total - start))
}

fn try_consume_and_rewrite_ref(
    bytes: &[u8],
    i: usize,
    sheet: Option<&str>,
    rewrite: RefRewriter<'_>,
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
    if end.is_none() && e1.kind() != EndpointKind::Cell {
        return (prefix_literal.to_string(), 0);
    }
    let rendered = rewrite(e1, end, sheet, prefix_literal);
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
                let (ns, ne) = shift_range_insert(start.row?, e.row?, before, count, MAX_ROW);
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
                let (ns, ne) = shift_range_insert(start.col?, e.col?, before, count, MAX_COLUMN);
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
        col = col
            .checked_mul(26)?
            .checked_add(b.to_ascii_uppercase() as u32 - b'A' as u32 + 1)?;
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
            ShiftOp::InsertRow {
                before: 2,
                count: 1,
            },
        );
        assert_eq!(f, "A1+B3");

        let f = shift_formula_refs(
            "SUM(A1:A10)",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow {
                before: 5,
                count: 2,
            },
        );
        assert_eq!(f, "SUM(A1:A12)");

        let f = shift_formula_refs(
            "SUM(A1:A10)",
            "Sheet1",
            "Sheet1",
            ShiftOp::DeleteRow {
                start: 1,
                count: 10,
            },
        );
        assert_eq!(f, "SUM(#REF!)");

        let f = shift_formula_refs(
            "$A$5",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow {
                before: 2,
                count: 1,
            },
        );
        assert_eq!(f, "$A$6");
    }

    #[test]
    fn cross_sheet_refs() {
        let f = shift_formula_refs(
            "Sheet2!A1 + Sheet1!B2",
            "Other",
            "Sheet1",
            ShiftOp::InsertRow {
                before: 1,
                count: 1,
            },
        );
        assert_eq!(f, "Sheet2!A1 + Sheet1!B3");

        let f = shift_formula_refs(
            "'My Sheet'!A1",
            "Other",
            "My Sheet",
            ShiftOp::InsertCol {
                before: 1,
                count: 1,
            },
        );
        assert_eq!(f, "'My Sheet'!B1");
    }

    #[test]
    fn preserves_strings_and_functions() {
        let f = shift_formula_refs(
            r#"IF(A1>0,"A1 is big","A1")"#,
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow {
                before: 1,
                count: 1,
            },
        );
        assert_eq!(f, r#"IF(A2>0,"A1 is big","A1")"#);
    }

    #[test]
    fn column_only_refs() {
        let f = shift_formula_refs(
            "SUM(A:A)",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertCol {
                before: 1,
                count: 1,
            },
        );
        assert_eq!(f, "SUM(B:B)");

        let f = shift_formula_refs(
            "SUM(A:A)",
            "Sheet1",
            "Sheet1",
            ShiftOp::InsertRow {
                before: 1,
                count: 1,
            },
        );
        assert_eq!(f, "SUM(A:A)");
    }
}
