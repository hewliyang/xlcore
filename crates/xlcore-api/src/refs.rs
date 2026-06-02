use xlcore_types::{ApiError, ApiErrorCode};

use crate::Result;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParsedCellRef {
    pub sheet: Option<String>,
    pub row: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedCellRef {
    pub sheet: String,
    pub row: u32,
    pub column: u32,
}

impl ResolvedCellRef {
    pub(crate) fn cell_reference(&self) -> String {
        format!("{}{}", xlcore_io::col_label(self.column), self.row)
    }

    pub(crate) fn full_reference(&self) -> String {
        format!(
            "{}!{}",
            quote_sheet_name(&self.sheet),
            self.cell_reference()
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParsedRangeRef {
    pub sheet: Option<String>,
    pub start_row: u32,
    pub start_column: u32,
    pub end_row: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedRangeRef {
    pub sheet: String,
    pub start_row: u32,
    pub start_column: u32,
    pub end_row: u32,
    pub end_column: u32,
}

impl ResolvedRangeRef {
    pub(crate) fn range_reference(&self) -> String {
        format!(
            "{}{}:{}{}",
            xlcore_io::col_label(self.start_column),
            self.start_row,
            xlcore_io::col_label(self.end_column),
            self.end_row,
        )
    }
}

pub(crate) fn parse_cell_reference(reference: &str) -> Result<ParsedCellRef> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            "cell reference is empty",
        ));
    }
    let (sheet, cell) = split_sheet_reference(reference)?;
    let (row, column) = parse_cell_address(cell).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid cell reference: {reference}"),
        )
        .with_ref(reference)
    })?;
    Ok(ParsedCellRef { sheet, row, column })
}

pub(crate) fn parse_range_reference(reference: &str) -> Result<ParsedRangeRef> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            "range reference is empty",
        ));
    }
    let (sheet, cells) = split_sheet_reference(reference)?;
    let (start_cell, end_cell) = match cells.split_once(':') {
        Some((a, b)) => (a, b),
        None => (cells, cells),
    };
    if start_cell.is_empty() || end_cell.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid range reference: {reference}"),
        )
        .with_ref(reference));
    }
    let (mut r1, mut c1) = parse_cell_address(start_cell).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid range reference: {reference}"),
        )
        .with_ref(reference)
    })?;
    let (mut r2, mut c2) = parse_cell_address(end_cell).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("invalid range reference: {reference}"),
        )
        .with_ref(reference)
    })?;
    if r1 > r2 {
        std::mem::swap(&mut r1, &mut r2);
    }
    if c1 > c2 {
        std::mem::swap(&mut c1, &mut c2);
    }
    Ok(ParsedRangeRef {
        sheet,
        start_row: r1,
        start_column: c1,
        end_row: r2,
        end_column: c2,
    })
}

pub(crate) fn split_sheet_reference(reference: &str) -> Result<(Option<String>, &str)> {
    if let Some(rest) = reference.strip_prefix('\'') {
        let mut sheet = String::new();
        let mut chars = rest.char_indices().peekable();
        while let Some((idx, ch)) = chars.next() {
            if ch == '\'' {
                if matches!(chars.peek(), Some((_, '\''))) {
                    sheet.push('\'');
                    let _ = chars.next();
                    continue;
                }
                let after_quote = &rest[idx + ch.len_utf8()..];
                let Some(cell) = after_quote.strip_prefix('!') else {
                    return Err(ApiError::new(
                        ApiErrorCode::InvalidRef,
                        format!("invalid sheet reference: {reference}"),
                    )
                    .with_ref(reference));
                };
                return Ok((Some(sheet), cell));
            }
            sheet.push(ch);
        }
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("unterminated sheet name in reference: {reference}"),
        )
        .with_ref(reference));
    }

    if let Some((sheet, cell)) = reference.rsplit_once('!') {
        if sheet.is_empty() || cell.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidRef,
                format!("invalid sheet reference: {reference}"),
            )
            .with_ref(reference));
        }
        return Ok((Some(sheet.to_string()), cell));
    }

    Ok((None, reference))
}

pub(crate) fn parse_cell_address(cell: &str) -> Option<(u32, u32)> {
    let mut chars = cell.chars().peekable();
    if matches!(chars.peek(), Some('$')) {
        let _ = chars.next();
    }

    let mut col = 0u32;
    let mut saw_col = false;
    while let Some(ch) = chars.peek().copied() {
        if !ch.is_ascii_alphabetic() {
            break;
        }
        saw_col = true;
        col = col
            .checked_mul(26)?
            .checked_add(ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1)?;
        let _ = chars.next();
    }

    if matches!(chars.peek(), Some('$')) {
        let _ = chars.next();
    }

    let mut row = 0u32;
    let mut saw_row = false;
    while let Some(ch) = chars.peek().copied() {
        if !ch.is_ascii_digit() {
            return None;
        }
        saw_row = true;
        row = row.checked_mul(10)?.checked_add(ch as u32 - b'0' as u32)?;
        let _ = chars.next();
    }

    if saw_col && saw_row && row > 0 && col > 0 {
        Some((row, col))
    } else {
        None
    }
}

pub(crate) fn parse_range_a1(reference: &str) -> Option<(u32, u32, u32, u32)> {
    let (a, b) = match reference.split_once(':') {
        Some((a, b)) => (a, b),
        None => (reference, reference),
    };
    let (r1, c1) = parse_cell_address(a)?;
    let (r2, c2) = parse_cell_address(b)?;
    let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
    Some((r1, c1, r2, c2))
}

pub(crate) fn ranges_overlap(
    ar1: u32,
    ac1: u32,
    ar2: u32,
    ac2: u32,
    br1: u32,
    bc1: u32,
    br2: u32,
    bc2: u32,
) -> bool {
    ar1 <= br2 && br1 <= ar2 && ac1 <= bc2 && bc1 <= ac2
}

pub(crate) fn quote_sheet_name(sheet: &str) -> String {
    if sheet
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return sheet.to_string();
    }
    format!("'{}'", sheet.replace('\'', "''"))
}

pub(crate) fn validate_sheet_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.len() > 31 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidSheetName,
            "sheet names must be 1 to 31 characters",
        )
        .with_sheet(name));
    }
    if name
        .chars()
        .any(|ch| matches!(ch, ':' | '\\' | '/' | '?' | '*' | '[' | ']'))
    {
        return Err(ApiError::new(
            ApiErrorCode::InvalidSheetName,
            format!("invalid sheet name: {name}"),
        )
        .with_sheet(name));
    }
    Ok(name)
}

pub(crate) fn validate_matrix_shape<T>(
    matrix: &[Vec<T>],
    range_ref: &ResolvedRangeRef,
    kind: &str,
) -> Result<()> {
    let expected_rows = (range_ref.end_row - range_ref.start_row + 1) as usize;
    let expected_cols = (range_ref.end_column - range_ref.start_column + 1) as usize;
    if matrix.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::ShapeMismatch,
            format!("{kind} matrix is empty"),
        )
        .with_ref(range_ref.range_reference())
        .with_sheet(&range_ref.sheet));
    }
    if matrix.len() != expected_rows {
        return Err(ApiError::new(
            ApiErrorCode::ShapeMismatch,
            format!(
                "{kind} matrix has {} rows but range expects {}",
                matrix.len(),
                expected_rows
            ),
        )
        .with_ref(range_ref.range_reference())
        .with_sheet(&range_ref.sheet));
    }
    for (idx, row) in matrix.iter().enumerate() {
        if row.len() != expected_cols {
            return Err(ApiError::new(
                ApiErrorCode::ShapeMismatch,
                format!(
                    "{kind} matrix row {} has {} cells but range expects {}",
                    idx,
                    row.len(),
                    expected_cols
                ),
            )
            .with_ref(range_ref.range_reference())
            .with_sheet(&range_ref.sheet));
        }
    }
    Ok(())
}
