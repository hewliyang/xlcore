use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, CfOperator, CfRuleKind, ConditionalFormatRuleInfo,
    ConditionalFormatRulePatch,
};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_a1, ranges_overlap};
use crate::styles::upsert_dxf;
use crate::{Result, Workbook};

impl Workbook {
    pub fn conditional_formats(
        &mut self,
        sheet: impl AsRef<str>,
    ) -> Result<Vec<ConditionalFormatRuleInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let mut out = Vec::new();
        for cf in &ws.conditional_formatting {
            let ranges: Vec<String> = cf
                .sequence_of_references
                .as_ref()
                .map(|v| v.iter().map(|s| s.as_str().to_string()).collect())
                .unwrap_or_default();
            for rule in &cf.conditional_formatting_rule {
                if let Some(info) = read_rule(&sheet, &ranges, rule) {
                    out.push(info);
                }
            }
        }
        Ok(out)
    }

    pub fn set_conditional_format(
        &mut self,
        reference: impl AsRef<str>,
        patch: ConditionalFormatRulePatch,
    ) -> Result<ConditionalFormatRuleInfo> {
        let reference = reference.as_ref();
        let range_ref = self.resolve_range_ref(reference)?;
        validate_patch(&patch, reference)?;

        let dxf_id = if let Some(style) = patch.dxf.as_ref() {
            Some(upsert_dxf(&mut self.doc, style)?)
        } else {
            None
        };

        let new_range = range_ref.range_reference();
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let next_priority = patch.priority.unwrap_or_else(|| next_priority(ws));
        let rule = build_rule(&patch, dxf_id, next_priority);

        ws.conditional_formatting.push(x::ConditionalFormatting {
            sequence_of_references: Some(vec![new_range.clone()]),
            conditional_formatting_rule: vec![rule],
            ..Default::default()
        });

        Ok(ConditionalFormatRuleInfo {
            sheet: range_ref.sheet,
            ranges: vec![new_range],
            kind: patch.kind,
            priority: next_priority,
            operator: patch.operator,
            formula1: patch.formula1,
            formula2: patch.formula2,
            text: patch.text,
            rank: patch.rank,
            percent: patch.percent.unwrap_or(false),
            bottom: patch.bottom.unwrap_or(false),
            above_average: patch.above_average,
            equal_average: patch.equal_average.unwrap_or(false),
            std_dev: patch.std_dev,
            stop_if_true: patch.stop_if_true.unwrap_or(false),
            dxf_id,
        })
    }

    pub fn clear_conditional_formats(
        &mut self,
        reference: impl AsRef<str>,
    ) -> Result<Vec<ConditionalFormatRuleInfo>> {
        let reference = reference.as_ref();
        let range_ref = self.resolve_range_ref(reference)?;
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

        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(ws.conditional_formatting.len());
        for cf in ws.conditional_formatting.drain(..) {
            let mut hit_ranges: Vec<String> = Vec::new();
            let mut surviving: Vec<String> = Vec::new();
            for s in cf.sequence_of_references.as_ref().map(|v| v.as_slice()).unwrap_or(&[]) {
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
                kept.push(cf);
                continue;
            }
            for rule in &cf.conditional_formatting_rule {
                if let Some(mut info) = read_rule(&sheet, &hit_ranges, rule) {
                    info.ranges = hit_ranges.clone();
                    removed.push(info);
                }
            }
            if !surviving.is_empty() {
                let mut updated = cf;
                updated.sequence_of_references =
                    Some(surviving.into_iter().map(Into::into).collect());
                kept.push(updated);
            }
        }
        ws.conditional_formatting = kept;
        Ok(removed)
    }
}

fn validate_patch(patch: &ConditionalFormatRulePatch, reference: &str) -> Result<()> {
    use CfRuleKind::*;
    let needs_f1 = matches!(
        patch.kind,
        CellIs | Expression | ContainsText | NotContainsText | BeginsWith | EndsWith
    );
    if needs_f1
        && patch
            .formula1
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        return Err(ApiError::new(
            ApiErrorCode::InvalidConditionalFormat,
            "formula1 is required for this rule kind",
        )
        .with_ref(reference));
    }
    if matches!(patch.kind, CellIs) && patch.operator.is_none() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidConditionalFormat,
            "operator is required for cellIs rules",
        )
        .with_ref(reference));
    }
    let needs_f2 = matches!(
        patch.operator,
        Some(CfOperator::Between) | Some(CfOperator::NotBetween)
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
            ApiErrorCode::InvalidConditionalFormat,
            "formula2 is required for between/notBetween operators",
        )
        .with_ref(reference));
    }
    let kind_needs_text = matches!(
        patch.kind,
        ContainsText | NotContainsText | BeginsWith | EndsWith
    );
    if kind_needs_text
        && patch
            .text
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        return Err(ApiError::new(
            ApiErrorCode::InvalidConditionalFormat,
            "text is required for text-comparison rules",
        )
        .with_ref(reference));
    }
    if let Some(priority) = patch.priority {
        if priority < 1 {
            return Err(ApiError::new(
                ApiErrorCode::InvalidConditionalFormat,
                "priority must be >= 1",
            )
            .with_ref(reference));
        }
    }
    Ok(())
}

fn next_priority(ws: &x::Worksheet) -> i32 {
    let mut max = 0i32;
    for cf in &ws.conditional_formatting {
        for rule in &cf.conditional_formatting_rule {
            let p: i32 = rule.priority.into();
            if p > max {
                max = p;
            }
        }
    }
    max + 1
}

fn build_rule(
    patch: &ConditionalFormatRulePatch,
    dxf_id: Option<u32>,
    priority: i32,
) -> x::ConditionalFormattingRule {
    let mut rule = x::ConditionalFormattingRule {
        r#type: kind_to_sdk(patch.kind),
        priority: priority.into(),
        format_id: dxf_id.map(Into::into),
        operator: patch.operator.map(operator_to_sdk),
        text: patch.text.clone().map(Into::into),
        rank: patch.rank.map(Into::into),
        percent: patch.percent.map(BooleanValue::from_bool),
        bottom: patch.bottom.map(BooleanValue::from_bool),
        above_average: patch.above_average.map(BooleanValue::from_bool),
        equal_average: patch.equal_average.map(BooleanValue::from_bool),
        std_dev: patch.std_dev.map(Into::into),
        stop_if_true: patch.stop_if_true.map(BooleanValue::from_bool),
        ..Default::default()
    };
    if let Some(f1) = patch.formula1.as_deref() {
        rule.formula.push(x::Formula(x::XstringType {
            xml_content: Some(f1.to_string().into()),
            ..Default::default()
        }));
    }
    if let Some(f2) = patch.formula2.as_deref() {
        rule.formula.push(x::Formula(x::XstringType {
            xml_content: Some(f2.to_string().into()),
            ..Default::default()
        }));
    }
    rule
}

fn read_rule(
    sheet: &str,
    ranges: &[String],
    rule: &x::ConditionalFormattingRule,
) -> Option<ConditionalFormatRuleInfo> {
    let kind = kind_from_sdk(rule.r#type)?;
    let formula1 = rule
        .formula
        .first()
        .and_then(|f| f.xml_content.as_ref().map(|s| s.as_str().to_string()));
    let formula2 = rule
        .formula
        .get(1)
        .and_then(|f| f.xml_content.as_ref().map(|s| s.as_str().to_string()));
    Some(ConditionalFormatRuleInfo {
        sheet: sheet.to_string(),
        ranges: ranges.to_vec(),
        kind,
        priority: rule.priority.into(),
        operator: rule.operator.and_then(operator_from_sdk),
        formula1,
        formula2,
        text: rule.text.as_ref().map(|s| s.as_str().to_string()),
        rank: rule.rank.map(Into::into),
        percent: rule
            .percent
            .map(bool::from)
            .unwrap_or(false),
        bottom: rule
            .bottom
            .map(bool::from)
            .unwrap_or(false),
        above_average: rule.above_average.map(bool::from),
        equal_average: rule
            .equal_average
            .map(bool::from)
            .unwrap_or(false),
        std_dev: rule.std_dev.map(Into::into),
        stop_if_true: rule
            .stop_if_true
            .map(bool::from)
            .unwrap_or(false),
        dxf_id: rule.format_id.map(Into::into),
    })
}

fn kind_to_sdk(kind: CfRuleKind) -> x::ConditionalFormatValues {
    use x::ConditionalFormatValues as V;
    match kind {
        CfRuleKind::Expression => V::Expression,
        CfRuleKind::CellIs => V::CellIs,
        CfRuleKind::Top10 => V::Top10,
        CfRuleKind::DuplicateValues => V::DuplicateValues,
        CfRuleKind::UniqueValues => V::UniqueValues,
        CfRuleKind::ContainsText => V::ContainsText,
        CfRuleKind::NotContainsText => V::NotContainsText,
        CfRuleKind::BeginsWith => V::BeginsWith,
        CfRuleKind::EndsWith => V::EndsWith,
        CfRuleKind::ContainsBlanks => V::ContainsBlanks,
        CfRuleKind::NotContainsBlanks => V::NotContainsBlanks,
        CfRuleKind::ContainsErrors => V::ContainsErrors,
        CfRuleKind::NotContainsErrors => V::NotContainsErrors,
        CfRuleKind::TimePeriod => V::TimePeriod,
        CfRuleKind::AboveAverage => V::AboveAverage,
    }
}

fn kind_from_sdk(v: x::ConditionalFormatValues) -> Option<CfRuleKind> {
    use x::ConditionalFormatValues as V;
    Some(match v {
        V::Expression => CfRuleKind::Expression,
        V::CellIs => CfRuleKind::CellIs,
        V::Top10 => CfRuleKind::Top10,
        V::DuplicateValues => CfRuleKind::DuplicateValues,
        V::UniqueValues => CfRuleKind::UniqueValues,
        V::ContainsText => CfRuleKind::ContainsText,
        V::NotContainsText => CfRuleKind::NotContainsText,
        V::BeginsWith => CfRuleKind::BeginsWith,
        V::EndsWith => CfRuleKind::EndsWith,
        V::ContainsBlanks => CfRuleKind::ContainsBlanks,
        V::NotContainsBlanks => CfRuleKind::NotContainsBlanks,
        V::ContainsErrors => CfRuleKind::ContainsErrors,
        V::NotContainsErrors => CfRuleKind::NotContainsErrors,
        V::TimePeriod => CfRuleKind::TimePeriod,
        V::AboveAverage => CfRuleKind::AboveAverage,
        _ => return None,
    })
}

fn operator_to_sdk(op: CfOperator) -> x::ConditionalFormattingOperatorValues {
    use x::ConditionalFormattingOperatorValues as V;
    match op {
        CfOperator::LessThan => V::LessThan,
        CfOperator::LessThanOrEqual => V::LessThanOrEqual,
        CfOperator::Equal => V::Equal,
        CfOperator::NotEqual => V::NotEqual,
        CfOperator::GreaterThanOrEqual => V::GreaterThanOrEqual,
        CfOperator::GreaterThan => V::GreaterThan,
        CfOperator::Between => V::Between,
        CfOperator::NotBetween => V::NotBetween,
        CfOperator::ContainsText => V::ContainsText,
        CfOperator::NotContains => V::NotContains,
        CfOperator::BeginsWith => V::BeginsWith,
        CfOperator::EndsWith => V::EndsWith,
    }
}

fn operator_from_sdk(v: x::ConditionalFormattingOperatorValues) -> Option<CfOperator> {
    use x::ConditionalFormattingOperatorValues as V;
    Some(match v {
        V::LessThan => CfOperator::LessThan,
        V::LessThanOrEqual => CfOperator::LessThanOrEqual,
        V::Equal => CfOperator::Equal,
        V::NotEqual => CfOperator::NotEqual,
        V::GreaterThanOrEqual => CfOperator::GreaterThanOrEqual,
        V::GreaterThan => CfOperator::GreaterThan,
        V::Between => CfOperator::Between,
        V::NotBetween => CfOperator::NotBetween,
        V::ContainsText => CfOperator::ContainsText,
        V::NotContains => CfOperator::NotContains,
        V::BeginsWith => CfOperator::BeginsWith,
        V::EndsWith => CfOperator::EndsWith,
        _ => return None,
    })
}
