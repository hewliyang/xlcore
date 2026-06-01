use regex::{Regex, RegexBuilder};
use xlcore_types::{
    ApiCellValue as CellValue, ApiError, ApiErrorCode, SearchHit, SearchMatch, SearchMode,
    SearchOptions, SearchTarget,
};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_reference, quote_sheet_name};
use crate::xml::{cell_info_from_cell, load_shared_strings};
use crate::{Result, Workbook};

impl Workbook {
    pub fn search(
        &mut self,
        query: impl AsRef<str>,
        options: SearchOptions,
    ) -> Result<Vec<SearchMatch>> {
        let query = query.as_ref();
        if query.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidSearchQuery,
                "search query is empty",
            ));
        }
        let matcher = build_matcher(query, &options)?;

        let sheets: Vec<String> = if let Some(name) = &options.sheet {
            if !self.sheet_exists(name)? {
                return Err(ApiError::new(
                    ApiErrorCode::MissingSheet,
                    format!("sheet not found: {name}"),
                )
                .with_sheet(name));
            }
            vec![name.clone()]
        } else {
            self.workbook_sheets()?
                .into_iter()
                .map(|s| s.name)
                .collect()
        };

        let range_filter = options
            .range
            .as_deref()
            .map(parse_range_reference)
            .transpose()?;
        if let Some(rr) = &range_filter {
            if rr.sheet.is_some() && options.sheet.is_some() && rr.sheet != options.sheet {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidRef,
                    "range sheet does not match options.sheet",
                )
                .with_ref(options.range.as_deref().unwrap_or("")));
            }
        }

        let shared_strings = load_shared_strings(&mut self.doc);
        let mut out: Vec<SearchMatch> = Vec::new();

        for sheet_name in sheets {
            if let Some(rr) = &range_filter {
                if let Some(rs) = &rr.sheet {
                    if rs != &sheet_name {
                        continue;
                    }
                }
            }
            let ws_part = self.worksheet_part_for_sheet(&sheet_name)?;
            let ws = ws_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            for row in &ws.sheet_data.row {
                let Some(row_idx) = row.row_index else { continue };
                if let Some(rr) = &range_filter {
                    if row_idx < rr.start_row || row_idx > rr.end_row {
                        continue;
                    }
                }
                for cell in &row.cell {
                    let Some((r, c)) = cell
                        .cell_reference
                        .as_ref()
                        .and_then(|s| xlcore_io::parse_a1(s.as_str()))
                    else {
                        continue;
                    };
                    if r != row_idx {
                        continue;
                    }
                    if let Some(rr) = &range_filter {
                        if c < rr.start_column || c > rr.end_column {
                            continue;
                        }
                    }
                    let info =
                        cell_info_from_cell(&sheet_name, r, c, Some(cell), &shared_strings);
                    if matches!(options.target, SearchTarget::Values | SearchTarget::Both) {
                        let text = value_text(&info.value);
                        if let Some(matched) = matcher.find(&text) {
                            out.push(SearchMatch {
                                sheet: info.sheet.clone(),
                                reference: full_reference(&info.sheet, &info.reference),
                                row: info.row,
                                column: info.column,
                                hit: SearchHit::Value,
                                matched,
                                value: info.value.clone(),
                                formula: info.formula.clone(),
                            });
                            if reached_limit(&out, options.max_results) {
                                return Ok(out);
                            }
                        }
                    }
                    if matches!(options.target, SearchTarget::Formulas | SearchTarget::Both) {
                        if let Some(formula) = info.formula.as_deref() {
                            if let Some(matched) = matcher.find(formula) {
                                out.push(SearchMatch {
                                    sheet: info.sheet.clone(),
                                    reference: full_reference(&info.sheet, &info.reference),
                                    row: info.row,
                                    column: info.column,
                                    hit: SearchHit::Formula,
                                    matched,
                                    value: info.value.clone(),
                                    formula: info.formula.clone(),
                                });
                                if reached_limit(&out, options.max_results) {
                                    return Ok(out);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}

fn full_reference(sheet: &str, cell: &str) -> String {
    format!("{}!{}", quote_sheet_name(sheet), cell)
}

fn reached_limit(out: &[SearchMatch], max: Option<usize>) -> bool {
    matches!(max, Some(n) if out.len() >= n)
}

fn value_text(value: &CellValue) -> String {
    match value {
        CellValue::Blank => String::new(),
        CellValue::String(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        CellValue::Error(e) => e.clone(),
    }
}

enum Matcher {
    Substring { needle: String, case_sensitive: bool },
    Exact { needle: String, case_sensitive: bool },
    Regex(Regex),
}

impl Matcher {
    fn find(&self, haystack: &str) -> Option<String> {
        match self {
            Matcher::Substring { needle, case_sensitive } => {
                if *case_sensitive {
                    haystack.contains(needle.as_str()).then(|| haystack.to_string())
                } else {
                    haystack
                        .to_lowercase()
                        .contains(&needle.to_lowercase())
                        .then(|| haystack.to_string())
                }
            }
            Matcher::Exact { needle, case_sensitive } => {
                let eq = if *case_sensitive {
                    haystack == needle.as_str()
                } else {
                    haystack.eq_ignore_ascii_case(needle.as_str())
                };
                eq.then(|| haystack.to_string())
            }
            Matcher::Regex(re) => re.find(haystack).map(|m| m.as_str().to_string()),
        }
    }
}

fn build_matcher(query: &str, options: &SearchOptions) -> Result<Matcher> {
    match options.mode {
        SearchMode::Substring => Ok(Matcher::Substring {
            needle: query.to_string(),
            case_sensitive: options.case_sensitive,
        }),
        SearchMode::Exact => Ok(Matcher::Exact {
            needle: query.to_string(),
            case_sensitive: options.case_sensitive,
        }),
        SearchMode::Wildcard => build_regex(&wildcard_to_regex(query), options.case_sensitive),
        SearchMode::Regex => build_regex(query, options.case_sensitive),
    }
}

fn build_regex(pattern: &str, case_sensitive: bool) -> Result<Matcher> {
    RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive)
        .build()
        .map(Matcher::Regex)
        .map_err(|err| {
            ApiError::new(
                ApiErrorCode::InvalidSearchQuery,
                format!("invalid search pattern: {err}"),
            )
        })
}

fn wildcard_to_regex(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 2);
    out.push('^');
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            '~' => match chars.next() {
                Some(esc @ ('*' | '?' | '~')) => out.push_str(&regex::escape(&esc.to_string())),
                Some(other) => {
                    out.push_str(&regex::escape("~"));
                    out.push_str(&regex::escape(&other.to_string()));
                }
                None => out.push_str(&regex::escape("~")),
            },
            other => out.push_str(&regex::escape(&other.to_string())),
        }
    }
    out.push('$');
    out
}
