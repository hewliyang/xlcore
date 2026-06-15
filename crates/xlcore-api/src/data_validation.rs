use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, DataValidationErrorStyle, DataValidationInfo, DataValidationOperator,
    DataValidationPatch, DataValidationType,
};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_a1, qualify_ref, ranges_overlap};
use crate::{Result, Workbook};

impl Workbook {
    pub fn data_validations(&mut self, sheet: impl AsRef<str>) -> Result<Vec<DataValidationInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let mut out = Vec::new();
        if let Some(block) = ws.data_validations.as_ref() {
            for dv in &block.data_validation {
                if let Some(info) = read_dv(&sheet, dv) {
                    out.push(info);
                }
            }
        }
        Ok(out)
    }

    pub fn set_data_validation(
        &mut self,
        sheet: impl AsRef<str>,
        reference: impl AsRef<str>,
        patch: DataValidationPatch,
    ) -> Result<DataValidationInfo> {
        let reference = qualify_ref(sheet.as_ref(), reference.as_ref())?;
        let reference = reference.as_str();
        let range_ref = self.resolve_range_ref(reference)?;
        validate_patch(&patch, reference)?;

        let new_range_str = range_ref.range_reference();
        let target = (
            range_ref.start_row,
            range_ref.start_column,
            range_ref.end_row,
            range_ref.end_column,
        );

        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let block = ws
            .data_validations
            .get_or_insert_with(x::DataValidations::default);

        let mut kept = Vec::with_capacity(block.data_validation.len());
        for dv in block.data_validation.drain(..) {
            let mut surviving: Vec<String> = dv
                .sequence_of_references
                .iter()
                .filter_map(|s| {
                    let raw = s.as_str();
                    let (r1, c1, r2, c2) = parse_range_a1(raw)?;
                    if ranges_overlap(target.0, target.1, target.2, target.3, r1, c1, r2, c2) {
                        None
                    } else {
                        Some(raw.to_string())
                    }
                })
                .collect();
            if surviving.is_empty() {
                continue;
            }
            let original_len = dv.sequence_of_references.len();
            if surviving.len() == original_len {
                kept.push(dv);
            } else {
                let mut updated = dv;
                updated.sequence_of_references = surviving.drain(..).map(|s| s.into()).collect();
                kept.push(updated);
            }
        }
        kept.push(build_dv(&new_range_str, &patch));
        block.data_validation = kept;
        block.count = Some((block.data_validation.len() as u32).into());

        Ok(info_from_patch(&range_ref.sheet, &new_range_str, &patch))
    }

    pub fn remove_data_validation(
        &mut self,
        sheet: impl AsRef<str>,
        reference: impl AsRef<str>,
    ) -> Result<Vec<DataValidationInfo>> {
        let reference = qualify_ref(sheet.as_ref(), reference.as_ref())?;
        let range_ref = self.resolve_range_ref(&reference)?;
        let sheet = range_ref.sheet.clone();
        let target = (
            range_ref.start_row,
            range_ref.start_column,
            range_ref.end_row,
            range_ref.end_column,
        );

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(block) = ws.data_validations.as_mut() else {
            return Ok(Vec::new());
        };

        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(block.data_validation.len());
        for dv in block.data_validation.drain(..) {
            let mut hit_ranges: Vec<String> = Vec::new();
            let mut surviving: Vec<String> = Vec::new();
            for s in &dv.sequence_of_references {
                let raw = s.as_str();
                match parse_range_a1(raw) {
                    Some((r1, c1, r2, c2))
                        if ranges_overlap(
                            target.0, target.1, target.2, target.3, r1, c1, r2, c2,
                        ) =>
                    {
                        hit_ranges.push(raw.to_string());
                    }
                    _ => surviving.push(raw.to_string()),
                }
            }
            if hit_ranges.is_empty() {
                kept.push(dv);
                continue;
            }
            if let Some(info) = read_dv(&sheet, &dv) {
                let mut info = info;
                info.ranges = hit_ranges.clone();
                info.reference = hit_ranges.join(" ");
                removed.push(info);
            }
            if !surviving.is_empty() {
                let mut updated = dv;
                updated.sequence_of_references = surviving.into_iter().map(Into::into).collect();
                kept.push(updated);
            }
        }
        if kept.is_empty() {
            ws.data_validations = None;
        } else {
            block.data_validation = kept;
            block.count = Some((block.data_validation.len() as u32).into());
        }
        Ok(removed)
    }
}

fn validate_patch(patch: &DataValidationPatch, reference: &str) -> Result<()> {
    use DataValidationType::*;
    let needs_op = matches!(patch.rule_type, Whole | Decimal | Date | Time | TextLength);
    if needs_op && patch.operator.is_none() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDataValidation,
            "operator is required for ranged data validation types",
        )
        .with_ref(reference));
    }
    if patch
        .formula1
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        return Err(
            ApiError::new(ApiErrorCode::InvalidDataValidation, "formula1 is required")
                .with_ref(reference),
        );
    }
    let needs_f2 = matches!(
        patch.operator,
        Some(DataValidationOperator::Between) | Some(DataValidationOperator::NotBetween)
    );
    if needs_f2
        && patch
            .formula2
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDataValidation,
            "formula2 is required for between/notBetween operators",
        )
        .with_ref(reference));
    }
    if matches!(
        patch.rule_type,
        DataValidationType::List | DataValidationType::Custom
    ) && patch.operator.is_some()
    {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDataValidation,
            "operator is not allowed for list/custom data validation",
        )
        .with_ref(reference));
    }
    Ok(())
}

fn build_dv(range_str: &str, patch: &DataValidationPatch) -> x::DataValidation {
    let mut dv = x::DataValidation {
        r#type: Some(type_to_sdk(patch.rule_type)),
        operator: patch.operator.map(operator_to_sdk),
        allow_blank: patch.allow_blank.map(Into::into),
        show_drop_down: patch.show_drop_down.map(Into::into),
        show_input_message: patch.show_input_message.map(Into::into),
        show_error_message: patch.show_error_message.map(Into::into),
        error_style: patch.error_style.map(error_style_to_sdk),
        error_title: patch.error_title.clone().map(Into::into),
        error: patch.error.clone().map(Into::into),
        prompt_title: patch.prompt_title.clone().map(Into::into),
        prompt: patch.prompt.clone().map(Into::into),
        ..Default::default()
    };
    dv.sequence_of_references = vec![range_str.to_string().into()];
    if let Some(f1) = patch.formula1.as_deref() {
        dv.formula1 = Some(x::Formula1(x::XstringType {
            xml_content: Some(f1.to_string().into()),
            ..Default::default()
        }));
    }
    if let Some(f2) = patch.formula2.as_deref() {
        dv.formula2 = Some(x::Formula2(x::XstringType {
            xml_content: Some(f2.to_string().into()),
            ..Default::default()
        }));
    }
    dv
}

fn read_dv(sheet: &str, dv: &x::DataValidation) -> Option<DataValidationInfo> {
    let rule_type = type_from_sdk(dv.r#type.unwrap_or_default())?;
    let ranges: Vec<String> = dv
        .sequence_of_references
        .iter()
        .map(|s| s.as_str().to_string())
        .collect();
    if ranges.is_empty() {
        return None;
    }
    let reference = ranges.join(" ");
    Some(DataValidationInfo {
        sheet: sheet.to_string(),
        reference,
        ranges,
        rule_type,
        operator: dv.operator.and_then(operator_from_sdk),
        allow_blank: bool::from(dv.allow_blank.unwrap_or(BooleanValue::from_bool(false))),
        show_drop_down: bool::from(dv.show_drop_down.unwrap_or(BooleanValue::from_bool(false))),
        show_input_message: bool::from(
            dv.show_input_message
                .unwrap_or(BooleanValue::from_bool(false)),
        ),
        show_error_message: bool::from(
            dv.show_error_message
                .unwrap_or(BooleanValue::from_bool(false)),
        ),
        error_style: dv.error_style.and_then(error_style_from_sdk),
        error_title: dv.error_title.as_ref().map(|s| s.as_str().to_string()),
        error: dv.error.as_ref().map(|s| s.as_str().to_string()),
        prompt_title: dv.prompt_title.as_ref().map(|s| s.as_str().to_string()),
        prompt: dv.prompt.as_ref().map(|s| s.as_str().to_string()),
        formula1: dv
            .formula1
            .as_ref()
            .and_then(|f| f.xml_content.as_ref().map(|s| s.as_str().to_string())),
        formula2: dv
            .formula2
            .as_ref()
            .and_then(|f| f.xml_content.as_ref().map(|s| s.as_str().to_string())),
    })
}

fn info_from_patch(
    sheet: &str,
    range_str: &str,
    patch: &DataValidationPatch,
) -> DataValidationInfo {
    DataValidationInfo {
        sheet: sheet.to_string(),
        reference: range_str.to_string(),
        ranges: vec![range_str.to_string()],
        rule_type: patch.rule_type,
        operator: patch.operator,
        allow_blank: patch.allow_blank.unwrap_or(false),
        show_drop_down: patch.show_drop_down.unwrap_or(false),
        show_input_message: patch.show_input_message.unwrap_or(false),
        show_error_message: patch.show_error_message.unwrap_or(false),
        error_style: patch.error_style,
        error_title: patch.error_title.clone(),
        error: patch.error.clone(),
        prompt_title: patch.prompt_title.clone(),
        prompt: patch.prompt.clone(),
        formula1: patch.formula1.clone(),
        formula2: patch.formula2.clone(),
    }
}

fn type_to_sdk(t: DataValidationType) -> x::DataValidationValues {
    use x::DataValidationValues as V;
    match t {
        DataValidationType::List => V::List,
        DataValidationType::Custom => V::Custom,
        DataValidationType::Whole => V::Whole,
        DataValidationType::Decimal => V::Decimal,
        DataValidationType::Date => V::Date,
        DataValidationType::Time => V::Time,
        DataValidationType::TextLength => V::TextLength,
    }
}

fn type_from_sdk(v: x::DataValidationValues) -> Option<DataValidationType> {
    use x::DataValidationValues as V;
    Some(match v {
        V::List => DataValidationType::List,
        V::Custom => DataValidationType::Custom,
        V::Whole => DataValidationType::Whole,
        V::Decimal => DataValidationType::Decimal,
        V::Date => DataValidationType::Date,
        V::Time => DataValidationType::Time,
        V::TextLength => DataValidationType::TextLength,
        V::None => return None,
    })
}

fn operator_to_sdk(op: DataValidationOperator) -> x::DataValidationOperatorValues {
    use x::DataValidationOperatorValues as V;
    match op {
        DataValidationOperator::Between => V::Between,
        DataValidationOperator::NotBetween => V::NotBetween,
        DataValidationOperator::Equal => V::Equal,
        DataValidationOperator::NotEqual => V::NotEqual,
        DataValidationOperator::GreaterThan => V::GreaterThan,
        DataValidationOperator::LessThan => V::LessThan,
        DataValidationOperator::GreaterThanOrEqual => V::GreaterThanOrEqual,
        DataValidationOperator::LessThanOrEqual => V::LessThanOrEqual,
    }
}

fn operator_from_sdk(op: x::DataValidationOperatorValues) -> Option<DataValidationOperator> {
    use x::DataValidationOperatorValues as V;
    Some(match op {
        V::Between => DataValidationOperator::Between,
        V::NotBetween => DataValidationOperator::NotBetween,
        V::Equal => DataValidationOperator::Equal,
        V::NotEqual => DataValidationOperator::NotEqual,
        V::GreaterThan => DataValidationOperator::GreaterThan,
        V::LessThan => DataValidationOperator::LessThan,
        V::GreaterThanOrEqual => DataValidationOperator::GreaterThanOrEqual,
        V::LessThanOrEqual => DataValidationOperator::LessThanOrEqual,
    })
}

fn error_style_to_sdk(es: DataValidationErrorStyle) -> x::DataValidationErrorStyleValues {
    use x::DataValidationErrorStyleValues as V;
    match es {
        DataValidationErrorStyle::Stop => V::Stop,
        DataValidationErrorStyle::Warning => V::Warning,
        DataValidationErrorStyle::Information => V::Information,
    }
}

fn error_style_from_sdk(es: x::DataValidationErrorStyleValues) -> Option<DataValidationErrorStyle> {
    use x::DataValidationErrorStyleValues as V;
    Some(match es {
        V::Stop => DataValidationErrorStyle::Stop,
        V::Warning => DataValidationErrorStyle::Warning,
        V::Information => DataValidationErrorStyle::Information,
    })
}
