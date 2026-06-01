use ooxmlsdk::simple_type::BooleanValue;
use ooxmlsdk::common::XmlNamespaceDecl;
use ooxmlsdk::schemas::opc_core_properties as cp;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, CalcMode, CalcProperties, CalcPropertiesPatch, WorkbookProperties,
    WorkbookPropertiesPatch,
};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

impl Workbook {
    pub fn properties(&mut self) -> Result<WorkbookProperties> {
        let Some(part) = self.doc.core_file_properties_part() else {
            return Ok(WorkbookProperties::default());
        };
        let props = part.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
        Ok(read_properties(props))
    }

    pub fn set_properties(&mut self, patch: WorkbookPropertiesPatch) -> Result<WorkbookProperties> {
        validate_properties_patch(&patch)?;
        if self.doc.core_file_properties_part().is_none() {
            let part = self
                .doc
                .add_core_file_properties_part()
                .map_err(sdk_err_to_api)?;
            part.set_root_element(&mut self.doc, blank_core_properties())
                .map_err(sdk_err_to_api)?;
        }
        let part = self
            .doc
            .core_file_properties_part()
            .expect("core file properties part exists after add");
        let props = part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        apply_properties_patch(props, &patch);
        Ok(read_properties(props))
    }

    pub fn calc_properties(&mut self) -> Result<CalcProperties> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        Ok(read_calc(wb.calculation_properties.as_ref()))
    }

    pub fn set_calc_properties(
        &mut self,
        patch: CalcPropertiesPatch,
    ) -> Result<CalcProperties> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let calc = wb
            .calculation_properties
            .get_or_insert_with(x::CalculationProperties::default);
        apply_calc_patch(calc, &patch);
        Ok(read_calc(Some(calc)))
    }
}

fn read_properties(props: &cp::CoreProperties) -> WorkbookProperties {
    WorkbookProperties {
        title: opt_string(props.title.as_ref().map(|s| s.as_str())),
        subject: opt_string(props.subject.as_ref().map(|s| s.as_str())),
        creator: opt_string(props.creator.as_ref().map(|s| s.as_str())),
        keywords: props.keywords.as_ref().and_then(|k| {
            k.xml_content
                .as_ref()
                .map(|s| s.as_str().to_string())
                .filter(|s| !s.is_empty())
        }),
        description: opt_string(props.description.as_ref().map(|s| s.as_str())),
        last_modified_by: opt_string(props.last_modified_by.as_ref().map(|s| s.as_str())),
        category: opt_string(props.category.as_ref().map(|s| s.as_str())),
        content_status: opt_string(props.content_status.as_ref().map(|s| s.as_str())),
        identifier: opt_string(props.identifier.as_ref().map(|s| s.as_str())),
        language: opt_string(props.language.as_ref().map(|s| s.as_str())),
        revision: opt_string(props.revision.as_ref().map(|s| s.as_str())),
        version: opt_string(props.version.as_ref().map(|s| s.as_str())),
        created: props.created.as_ref().and_then(|c| {
            c.xml_content
                .as_ref()
                .map(|s| s.as_str().to_string())
                .filter(|s| !s.is_empty())
        }),
        modified: props.modified.as_ref().and_then(|c| {
            c.xml_content
                .as_ref()
                .map(|s| s.as_str().to_string())
                .filter(|s| !s.is_empty())
        }),
        last_printed: opt_string(props.last_printed.as_ref().map(|s| s.as_str())),
    }
}

fn apply_properties_patch(props: &mut cp::CoreProperties, patch: &WorkbookPropertiesPatch) {
    if let Some(v) = patch.title.clone() {
        props.title = Some(v.into());
    }
    if let Some(v) = patch.subject.clone() {
        props.subject = Some(v.into());
    }
    if let Some(v) = patch.creator.clone() {
        props.creator = Some(v.into());
    }
    if let Some(v) = patch.keywords.clone() {
        props.keywords = Some(cp::Keywords {
            xml_content: Some(v.into()),
            ..Default::default()
        });
    }
    if let Some(v) = patch.description.clone() {
        props.description = Some(v.into());
    }
    if let Some(v) = patch.last_modified_by.clone() {
        props.last_modified_by = Some(v.into());
    }
    if let Some(v) = patch.category.clone() {
        props.category = Some(v.into());
    }
    if let Some(v) = patch.content_status.clone() {
        props.content_status = Some(v.into());
    }
    if let Some(v) = patch.identifier.clone() {
        props.identifier = Some(v.into());
    }
    if let Some(v) = patch.language.clone() {
        props.language = Some(v.into());
    }
    if let Some(v) = patch.revision.clone() {
        props.revision = Some(v.into());
    }
    if let Some(v) = patch.version.clone() {
        props.version = Some(v.into());
    }
    if let Some(v) = patch.created.clone() {
        props.created = Some(cp::Created {
            xsi_type: Some(cp::XsiTypeValue::DctermsW3cdtf),
            xml_content: Some(v.into()),
        });
    }
    if let Some(v) = patch.modified.clone() {
        props.modified = Some(cp::Modified {
            xsi_type: Some(cp::XsiTypeValue::DctermsW3cdtf),
            xml_content: Some(v.into()),
        });
    }
    if let Some(v) = patch.last_printed.clone() {
        props.last_printed = Some(v.into());
    }
}

fn blank_core_properties() -> cp::CoreProperties {
    cp::CoreProperties {
        xmlns: vec![
            XmlNamespaceDecl::new(
                "cp",
                "http://schemas.openxmlformats.org/package/2006/metadata/core-properties",
            ),
            XmlNamespaceDecl::new("dc", "http://purl.org/dc/elements/1.1/"),
            XmlNamespaceDecl::new("dcterms", "http://purl.org/dc/terms/"),
            XmlNamespaceDecl::new("dcmitype", "http://purl.org/dc/dcmitype/"),
            XmlNamespaceDecl::new("xsi", "http://www.w3.org/2001/XMLSchema-instance"),
        ],
        ..Default::default()
    }
}

fn opt_string<'a>(value: Option<&'a str>) -> Option<String> {
    value.filter(|s| !s.is_empty()).map(|s| s.to_string())
}

fn read_calc(calc: Option<&x::CalculationProperties>) -> CalcProperties {
    let Some(calc) = calc else {
        return CalcProperties::default();
    };
    CalcProperties {
        calc_mode: calc.calculation_mode.map(calc_mode_from_sdk),
        full_calc_on_load: (calc.full_calculation_on_load).map(bool::from),
        force_full_calc: (calc.force_full_calculation).map(bool::from),
        calc_on_save: (calc.calculation_on_save).map(bool::from),
        concurrent_calc: (calc.concurrent_calculation).map(bool::from),
        iterate: (calc.iterate).map(bool::from),
        iterate_count: calc.iterate_count,
        iterate_delta: calc.iterate_delta,
        full_precision: (calc.full_precision).map(bool::from),
        calculation_id: calc.calculation_id,
    }
}

fn apply_calc_patch(calc: &mut x::CalculationProperties, patch: &CalcPropertiesPatch) {
    if let Some(mode) = patch.calc_mode {
        calc.calculation_mode = Some(calc_mode_to_sdk(mode));
    }
    if let Some(v) = patch.full_calc_on_load {
        calc.full_calculation_on_load = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.force_full_calc {
        calc.force_full_calculation = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.calc_on_save {
        calc.calculation_on_save = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.concurrent_calc {
        calc.concurrent_calculation = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.iterate {
        calc.iterate = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.iterate_count {
        calc.iterate_count = Some(v);
    }
    if let Some(v) = patch.iterate_delta {
        calc.iterate_delta = Some(v);
    }
    if let Some(v) = patch.full_precision {
        calc.full_precision = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.calculation_id {
        calc.calculation_id = Some(v);
    }
}

fn calc_mode_from_sdk(value: x::CalculateModeValues) -> CalcMode {
    match value {
        x::CalculateModeValues::Auto => CalcMode::Auto,
        x::CalculateModeValues::AutoNoTable => CalcMode::AutoNoTable,
        x::CalculateModeValues::Manual => CalcMode::Manual,
    }
}

fn calc_mode_to_sdk(value: CalcMode) -> x::CalculateModeValues {
    match value {
        CalcMode::Auto => x::CalculateModeValues::Auto,
        CalcMode::AutoNoTable => x::CalculateModeValues::AutoNoTable,
        CalcMode::Manual => x::CalculateModeValues::Manual,
    }
}

fn validate_properties_patch(patch: &WorkbookPropertiesPatch) -> Result<()> {
    if let Some(v) = patch.created.as_deref() {
        validate_dt(v, "created")?;
    }
    if let Some(v) = patch.modified.as_deref() {
        validate_dt(v, "modified")?;
    }
    Ok(())
}

fn validate_dt(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    if value.len() >= 10
        && value.chars().take(4).all(|c| c.is_ascii_digit())
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
    {
        return Ok(());
    }
    Err(ApiError::new(
        ApiErrorCode::InvalidProperty,
        format!("{field} must be an ISO-8601 timestamp (e.g. 2024-01-31T12:00:00Z)"),
    ))
}
