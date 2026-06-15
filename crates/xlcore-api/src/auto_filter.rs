use ooxmlsdk::simple_type::BooleanValue;
use xlcore_export::table_engine::compute_hidden_rows;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiCellValue, ApiError, ApiErrorCode, AutoFilterColumnInfo, AutoFilterColumnPatch,
    AutoFilterCriteria, AutoFilterCustomCriterion, AutoFilterInfo, AutoFilterOperator,
};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_a1, qualify_ref};
use crate::rowcols::ensure_row;
use crate::{Result, Workbook};

impl Workbook {
    pub fn auto_filter(&mut self, sheet: impl AsRef<str>) -> Result<Option<AutoFilterInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        Ok(read_auto_filter(&sheet, ws.auto_filter.as_deref()))
    }

    pub fn set_auto_filter(
        &mut self,
        sheet: impl AsRef<str>,
        reference: impl AsRef<str>,
    ) -> Result<AutoFilterInfo> {
        let reference = qualify_ref(sheet.as_ref(), reference.as_ref())?;
        let range_ref = self.resolve_range_ref(&reference)?;
        let new_ref = range_ref.range_reference();
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let af = ws
            .auto_filter
            .get_or_insert_with(|| Box::new(x::AutoFilter::default()));
        af.reference = Some(new_ref.clone().into());
        af.filter_column.clear();
        af.sort_state = None;
        Ok(AutoFilterInfo {
            sheet: range_ref.sheet.clone(),
            reference: new_ref,
            start_row: range_ref.start_row,
            start_column: range_ref.start_column,
            end_row: range_ref.end_row,
            end_column: range_ref.end_column,
            columns: Vec::new(),
        })
    }

    pub fn remove_auto_filter(&mut self, sheet: impl AsRef<str>) -> Result<Option<AutoFilterInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let removed = read_auto_filter(&sheet, ws.auto_filter.as_deref());
        ws.auto_filter = None;
        Ok(removed)
    }

    pub fn set_auto_filter_column(
        &mut self,
        sheet: impl AsRef<str>,
        patch: AutoFilterColumnPatch,
    ) -> Result<AutoFilterColumnInfo> {
        let sheet = sheet.as_ref().to_string();
        let span = self.auto_filter_span(&sheet)?;
        let max_offset = span.1.saturating_sub(span.0);
        if patch.column_offset > max_offset {
            return Err(ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                format!(
                    "column_offset {} exceeds filter width {}",
                    patch.column_offset,
                    max_offset + 1
                ),
            )
            .with_sheet(sheet));
        }
        validate_criteria(&patch.criteria, &sheet)?;

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let af = ws.auto_filter.as_mut().ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                "sheet has no auto filter range; call set_auto_filter first",
            )
            .with_sheet(&sheet)
        })?;
        af.filter_column
            .retain(|fc| u32::from(fc.column_id) != patch.column_offset);
        let mut fc = x::FilterColumn {
            column_id: patch.column_offset.into(),
            hidden_button: patch.hidden_button.map(Into::into),
            show_button: patch.show_button.map(Into::into),
            ..Default::default()
        };
        fc.filter_column_choice = Some(build_choice(&patch.criteria));
        let info = read_filter_column(&fc);
        af.filter_column.push(fc);
        af.filter_column.sort_by_key(|fc| u32::from(fc.column_id));
        let info = info.ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                "failed to materialize filter column",
            )
            .with_sheet(&sheet)
        })?;
        self.apply_filter_hidden(&sheet)?;
        Ok(info)
    }

    pub fn remove_auto_filter_column(
        &mut self,
        sheet: impl AsRef<str>,
        column_offset: u32,
    ) -> Result<Option<AutoFilterColumnInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(af) = ws.auto_filter.as_mut() else {
            return Ok(None);
        };
        let mut removed: Option<AutoFilterColumnInfo> = None;
        af.filter_column.retain(|fc| {
            if u32::from(fc.column_id) == column_offset {
                removed = read_filter_column(fc);
                false
            } else {
                true
            }
        });
        self.apply_filter_hidden(&sheet)?;
        Ok(removed)
    }

    pub fn set_auto_filter_sort(
        &mut self,
        sheet: impl AsRef<str>,
        column_offset: u32,
        descending: bool,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        let (c1, c2) = self.auto_filter_span(&sheet)?;
        let max_offset = c2.saturating_sub(c1);
        if column_offset > max_offset {
            return Err(ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                format!(
                    "column_offset {} exceeds filter width {}",
                    column_offset,
                    max_offset + 1
                ),
            )
            .with_sheet(sheet));
        }

        let (r1, r2) = {
            let ws_part = self.worksheet_part_for_sheet(&sheet)?;
            let ws = ws_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            let reference = ws
                .auto_filter
                .as_deref()
                .and_then(|af| af.reference.as_ref())
                .map(|r| r.as_str().to_string());
            let (r1, _c1, r2, _c2) =
                reference
                    .as_deref()
                    .and_then(parse_range_a1)
                    .ok_or_else(|| {
                        ApiError::new(
                            ApiErrorCode::InvalidAutoFilter,
                            "auto filter is missing a valid range reference",
                        )
                        .with_sheet(&sheet)
                    })?;
            (r1, r2)
        };

        let sort_col = c1 + column_offset;
        let first_data_row = r1 + 1;
        let sort_ref = format!(
            "{}{}:{}{}",
            xlcore_io::col_label(c1),
            first_data_row,
            xlcore_io::col_label(c2),
            r2,
        );
        let cond_ref = format!(
            "{}{}:{}{}",
            xlcore_io::col_label(sort_col),
            first_data_row,
            xlcore_io::col_label(sort_col),
            r2,
        );

        let order: Option<Vec<usize>> = if r2 > r1 {
            let local_ref = format!(
                "{}{}:{}{}",
                xlcore_io::col_label(sort_col),
                first_data_row,
                xlcore_io::col_label(sort_col),
                r2,
            );
            let reference = qualify_ref(&sheet, &local_ref)?;
            let range_ref = self.resolve_range_ref(&reference)?;
            let info = self.read_range(&range_ref)?;
            let keys: Vec<SortKey> = info
                .values
                .iter()
                .map(|row| sort_key(&row.first().map(cell_display).unwrap_or_default()))
                .collect();
            let mut order: Vec<usize> = (0..keys.len()).collect();
            order.sort_by(|&a, &b| cmp_keys(&keys[a], &keys[b], descending));
            Some(order)
        } else {
            None
        };

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        {
            let af = ws.auto_filter.as_mut().ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::InvalidAutoFilter,
                    "sheet has no auto filter range",
                )
                .with_sheet(&sheet)
            })?;
            af.sort_state = Some(Box::new(x::SortState {
                reference: sort_ref.into(),
                sort_state_choice: vec![x::SortStateChoice::XSortCondition(Box::new(
                    x::SortCondition {
                        descending: if descending {
                            Some(BooleanValue::from_bool(true))
                        } else {
                            None
                        },
                        reference: cond_ref.into(),
                        ..Default::default()
                    },
                ))],
                ..Default::default()
            }));
        }

        if let Some(order) = order {
            reorder_data_rows(ws, first_data_row, r2, &order);
        }

        self.apply_filter_hidden(&sheet)?;
        Ok(())
    }

    pub fn remove_auto_filter_sort(&mut self, sheet: impl AsRef<str>) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        if let Some(af) = ws.auto_filter.as_mut() {
            af.sort_state = None;
        }
        Ok(())
    }

    fn apply_filter_hidden(&mut self, sheet: &str) -> Result<()> {
        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(af) = ws.auto_filter.as_deref() else {
            return Ok(());
        };
        let Some(reference) = af.reference.as_ref() else {
            return Ok(());
        };
        let Some((r1, c1, r2, c2)) = parse_range_a1(reference.as_str()) else {
            return Ok(());
        };
        let mut criteria_cols: Vec<(u32, AutoFilterCriteria)> = Vec::new();
        for fc in &af.filter_column {
            if let Some(info) = read_filter_column(fc) {
                if is_active_criteria(&info.criteria) {
                    criteria_cols.push((info.column_offset, info.criteria));
                }
            }
        }

        if r2 <= r1 {
            return Ok(());
        }
        let first_data_row = r1 + 1;

        let local_ref = format!(
            "{}{}:{}{}",
            xlcore_io::col_label(c1),
            first_data_row,
            xlcore_io::col_label(c2),
            r2,
        );
        let reference = qualify_ref(sheet, &local_ref)?;
        let range_ref = self.resolve_range_ref(&reference)?;
        let info = self.read_range(&range_ref)?;
        let rows: Vec<Vec<String>> = info
            .values
            .iter()
            .map(|row| row.iter().map(cell_display).collect())
            .collect();

        let cols: Vec<(u32, &AutoFilterCriteria)> = criteria_cols
            .iter()
            .map(|(off, crit)| (*off, crit))
            .collect();
        let hidden = compute_hidden_rows(first_data_row, &rows, &cols);

        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        for r in first_data_row..=r2 {
            if hidden.contains(&r) {
                ensure_row(ws, r).hidden = Some(BooleanValue::from_bool(true));
            } else if let Some(row) = ws
                .sheet_data
                .row
                .iter_mut()
                .find(|row| row.row_index == Some(r))
            {
                row.hidden = None;
            }
        }
        Ok(())
    }

    fn auto_filter_span(&mut self, sheet: &str) -> Result<(u32, u32)> {
        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let af = ws.auto_filter.as_deref().ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                "sheet has no auto filter range",
            )
            .with_sheet(sheet)
        })?;
        let reference = af.reference.as_ref().ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                "auto filter is missing a range reference",
            )
            .with_sheet(sheet)
        })?;
        let (_r1, c1, _r2, c2) = parse_range_a1(reference.as_str()).ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                format!("auto filter range '{}' is not a valid A1 range", reference),
            )
            .with_sheet(sheet)
        })?;
        Ok((c1, c2))
    }
}

enum SortKey {
    Num(f64),
    Text(String),
    Blank,
}

fn sort_key(display: &str) -> SortKey {
    if display.is_empty() {
        SortKey::Blank
    } else if let Ok(n) = display.parse::<f64>() {
        SortKey::Num(n)
    } else {
        SortKey::Text(display.to_lowercase())
    }
}

fn cmp_keys(a: &SortKey, b: &SortKey, descending: bool) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (SortKey::Blank, SortKey::Blank) => Ordering::Equal,
        (SortKey::Blank, _) => Ordering::Greater,
        (_, SortKey::Blank) => Ordering::Less,
        _ => {
            let ord = match (a, b) {
                (SortKey::Num(x), SortKey::Num(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
                (SortKey::Num(_), SortKey::Text(_)) => Ordering::Less,
                (SortKey::Text(_), SortKey::Num(_)) => Ordering::Greater,
                (SortKey::Text(x), SortKey::Text(y)) => x.cmp(y),
                _ => Ordering::Equal,
            };
            if descending {
                ord.reverse()
            } else {
                ord
            }
        }
    }
}

fn reorder_data_rows(ws: &mut x::Worksheet, first_data_row: u32, last_row: u32, order: &[usize]) {
    use std::collections::HashMap;
    let mut kept: Vec<x::Row> = Vec::new();
    let mut moved: HashMap<u32, x::Row> = HashMap::new();
    for row in std::mem::take(&mut ws.sheet_data.row) {
        match row.row_index {
            Some(r) if r >= first_data_row && r <= last_row => {
                moved.insert(r, row);
            }
            _ => kept.push(row),
        }
    }
    for (j, &src) in order.iter().enumerate() {
        let target = first_data_row + j as u32;
        let src_row = first_data_row + src as u32;
        if let Some(mut row) = moved.remove(&src_row) {
            row.row_index = Some(target);
            for cell in &mut row.cell {
                if let Some(reference) = cell.cell_reference.as_ref() {
                    let new = rewrite_cell_row(reference.as_str(), target);
                    cell.cell_reference = Some(new.into());
                }
            }
            kept.push(row);
        }
    }
    kept.sort_by_key(|r| r.row_index.unwrap_or(u32::MAX));
    ws.sheet_data.row = kept;
}

fn rewrite_cell_row(cell_ref: &str, new_row: u32) -> String {
    let col: String = cell_ref
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    format!("{col}{new_row}")
}

fn is_active_criteria(criteria: &AutoFilterCriteria) -> bool {
    match criteria {
        AutoFilterCriteria::Values { values, blank } => {
            !values.is_empty() || blank.unwrap_or(false)
        }
        AutoFilterCriteria::Custom { criteria, .. } => !criteria.is_empty(),
        AutoFilterCriteria::Top10 { .. } => true,
        AutoFilterCriteria::Unsupported { .. } => false,
    }
}

fn cell_display(value: &ApiCellValue) -> String {
    match value {
        ApiCellValue::Blank => String::new(),
        ApiCellValue::String(s) => s.clone(),
        ApiCellValue::Number(n) => format_number(*n),
        ApiCellValue::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        ApiCellValue::Error(e) => e.clone(),
    }
}

fn format_number(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let mut s = format!("{n}");
        if s.contains('e') || s.contains('E') {
            s = format!("{n:.10}");
        }
        s
    }
}

fn validate_criteria(criteria: &AutoFilterCriteria, sheet: &str) -> Result<()> {
    let err =
        |msg: &str| Err(ApiError::new(ApiErrorCode::InvalidAutoFilter, msg).with_sheet(sheet));
    match criteria {
        AutoFilterCriteria::Values { values, blank } => {
            if values.is_empty() && !blank.unwrap_or(false) {
                return err("values criteria requires at least one value or blank=true");
            }
            for v in values {
                if v.is_empty() {
                    return err("values criteria entries must be non-empty");
                }
            }
        }
        AutoFilterCriteria::Top10 { val, percent, .. } => {
            if !val.is_finite() || *val <= 0.0 {
                return err("top10 val must be positive and finite");
            }
            if percent.unwrap_or(false) && *val > 100.0 {
                return err("top10 percent val must be <= 100");
            }
        }
        AutoFilterCriteria::Custom { criteria: list, .. } => {
            if list.is_empty() || list.len() > 2 {
                return err("custom criteria requires 1 or 2 entries");
            }
            for c in list {
                if c.value.is_empty() {
                    return err("custom criterion value must be non-empty");
                }
            }
        }
        AutoFilterCriteria::Unsupported { .. } => {
            return err("unsupported criteria kind cannot be authored");
        }
    }
    Ok(())
}

fn build_choice(criteria: &AutoFilterCriteria) -> x::FilterColumnChoice {
    match criteria {
        AutoFilterCriteria::Values { values, blank } => {
            let mut filters = x::Filters {
                blank: if blank.unwrap_or(false) {
                    Some(true.into())
                } else {
                    None
                },
                ..Default::default()
            };
            for v in values {
                filters
                    .filters_choice
                    .push(x::FiltersChoice::XFilter(Box::new(x::Filter {
                        val: v.clone().into(),
                    })));
            }
            x::FilterColumnChoice::Filters(Box::new(filters))
        }
        AutoFilterCriteria::Top10 { top, percent, val } => {
            let t10 = x::Top10 {
                top: Some(top.unwrap_or(true).into()),
                percent: Some(percent.unwrap_or(false).into()),
                val: (*val).into(),
                filter_value: None,
            };
            x::FilterColumnChoice::Top10(Box::new(t10))
        }
        AutoFilterCriteria::Custom {
            logical_and,
            criteria,
        } => {
            let mut cf = x::CustomFilters {
                and: Some(logical_and.unwrap_or(false).into()),
                custom_filter: Vec::new(),
            };
            for c in criteria {
                cf.custom_filter.push(x::CustomFilter {
                    operator: Some(operator_to_sdk(c.operator)),
                    val: Some(c.value.clone().into()),
                });
            }
            x::FilterColumnChoice::XCustomFilters(Box::new(cf))
        }
        AutoFilterCriteria::Unsupported { .. } => unreachable!(),
    }
}

fn read_auto_filter(sheet: &str, af: Option<&x::AutoFilter>) -> Option<AutoFilterInfo> {
    let af = af?;
    let reference = af.reference.as_ref()?;
    let (r1, c1, r2, c2) = parse_range_a1(reference.as_str())?;
    let mut columns: Vec<AutoFilterColumnInfo> = af
        .filter_column
        .iter()
        .filter_map(read_filter_column)
        .collect();
    columns.sort_by_key(|c| c.column_offset);
    Some(AutoFilterInfo {
        sheet: sheet.to_string(),
        reference: format!(
            "{}{}:{}{}",
            xlcore_io::col_label(c1),
            r1,
            xlcore_io::col_label(c2),
            r2,
        ),
        start_row: r1,
        start_column: c1,
        end_row: r2,
        end_column: c2,
        columns,
    })
}

fn read_filter_column(fc: &x::FilterColumn) -> Option<AutoFilterColumnInfo> {
    let column_offset = u32::from(fc.column_id);
    let criteria = match fc.filter_column_choice.as_ref() {
        Some(x::FilterColumnChoice::Filters(filters)) => {
            let blank = filters.blank.as_ref().map(|b| bool::from(*b));
            let mut values: Vec<String> = Vec::new();
            for fc in &filters.filters_choice {
                if let x::FiltersChoice::XFilter(f) = fc {
                    values.push(f.val.as_str().to_string());
                }
            }
            AutoFilterCriteria::Values { values, blank }
        }
        Some(x::FilterColumnChoice::Top10(t10)) => AutoFilterCriteria::Top10 {
            top: t10.top.as_ref().map(|b| bool::from(*b)),
            percent: t10.percent.as_ref().map(|b| bool::from(*b)),
            val: f64::from(t10.val),
        },
        Some(x::FilterColumnChoice::XCustomFilters(cf)) => {
            let logical_and = cf.and.as_ref().map(|b| bool::from(*b));
            let criteria = cf
                .custom_filter
                .iter()
                .map(|c| AutoFilterCustomCriterion {
                    operator: c
                        .operator
                        .and_then(operator_from_sdk)
                        .unwrap_or(AutoFilterOperator::Equal),
                    value: c
                        .val
                        .as_ref()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default(),
                })
                .collect();
            AutoFilterCriteria::Custom {
                logical_and,
                criteria,
            }
        }
        Some(x::FilterColumnChoice::DynamicFilter(_)) => AutoFilterCriteria::Unsupported {
            name: "dynamicFilter".to_string(),
        },
        Some(x::FilterColumnChoice::ColorFilter(_)) => AutoFilterCriteria::Unsupported {
            name: "colorFilter".to_string(),
        },
        Some(x::FilterColumnChoice::XIconFilter(_)) => AutoFilterCriteria::Unsupported {
            name: "iconFilter".to_string(),
        },
        Some(x::FilterColumnChoice::X14CustomFilters(cf)) => {
            let logical_and = cf.and.as_ref().map(|b| bool::from(*b));
            let criteria = cf
                .custom_filter
                .iter()
                .map(|c| AutoFilterCustomCriterion {
                    operator: c
                        .operator
                        .and_then(operator_from_sdk)
                        .unwrap_or(AutoFilterOperator::Equal),
                    value: c
                        .val
                        .as_ref()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default(),
                })
                .collect();
            AutoFilterCriteria::Custom {
                logical_and,
                criteria,
            }
        }
        Some(x::FilterColumnChoice::X14IconFilter(_)) => AutoFilterCriteria::Unsupported {
            name: "x14IconFilter".to_string(),
        },
        Some(x::FilterColumnChoice::ExtensionList(_)) => AutoFilterCriteria::Unsupported {
            name: "extLst".to_string(),
        },
        None => AutoFilterCriteria::Values {
            values: Vec::new(),
            blank: None,
        },
    };
    Some(AutoFilterColumnInfo {
        column_offset,
        hidden_button: fc.hidden_button.as_ref().map(|b| bool::from(*b)),
        show_button: fc.show_button.as_ref().map(|b| bool::from(*b)),
        criteria,
    })
}

fn operator_to_sdk(op: AutoFilterOperator) -> x::FilterOperatorValues {
    use x::FilterOperatorValues as V;
    match op {
        AutoFilterOperator::Equal => V::Equal,
        AutoFilterOperator::NotEqual => V::NotEqual,
        AutoFilterOperator::GreaterThan => V::GreaterThan,
        AutoFilterOperator::GreaterThanOrEqual => V::GreaterThanOrEqual,
        AutoFilterOperator::LessThan => V::LessThan,
        AutoFilterOperator::LessThanOrEqual => V::LessThanOrEqual,
    }
}

fn operator_from_sdk(op: x::FilterOperatorValues) -> Option<AutoFilterOperator> {
    use x::FilterOperatorValues as V;
    Some(match op {
        V::Equal => AutoFilterOperator::Equal,
        V::NotEqual => AutoFilterOperator::NotEqual,
        V::GreaterThan => AutoFilterOperator::GreaterThan,
        V::GreaterThanOrEqual => AutoFilterOperator::GreaterThanOrEqual,
        V::LessThan => AutoFilterOperator::LessThan,
        V::LessThanOrEqual => AutoFilterOperator::LessThanOrEqual,
    })
}
