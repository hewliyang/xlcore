use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, AutoFilterColumnInfo, AutoFilterColumnPatch, AutoFilterCriteria,
    AutoFilterCustomCriterion, AutoFilterInfo, AutoFilterOperator,
};

use crate::errors::sdk_err_to_api;
use crate::refs::parse_range_a1;
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

    pub fn set_auto_filter(&mut self, reference: impl AsRef<str>) -> Result<AutoFilterInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
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
        info.ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::InvalidAutoFilter,
                "failed to materialize filter column",
            )
            .with_sheet(&sheet)
        })
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
        Ok(removed)
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
