use ooxmlsdk::schemas::schemas_microsoft_com_office_spreadsheetml_2009_9_main as x14;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as xspread;
use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::parse_a1;
use xlcore_types::{
    ApiError, ApiErrorCode, SparklineAxisType, SparklineDisplayBlanks, SparklineEntry,
    SparklineGroupInfo, SparklineGroupPatch, SparklineKind,
};

use crate::errors::sdk_err_to_api;
use crate::refs::qualify_ref;
use crate::{Result, Workbook};

const SPARKLINE_EXT_URI: &str = "{05C60535-1F16-4fd2-B633-F4F36F0B64E0}";
const X14_NS: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const XNE_NS: &str = "http://schemas.microsoft.com/office/excel/2006/main";

impl Workbook {
    pub fn sparkline_groups(&mut self, sheet: Option<&str>) -> Result<Vec<SparklineGroupInfo>> {
        let sheet_names: Vec<String> = match sheet {
            Some(name) => {
                if !self.sheet_exists(name)? {
                    return Err(ApiError::new(
                        ApiErrorCode::MissingSheet,
                        format!("sheet not found: {name}"),
                    )
                    .with_sheet(name));
                }
                vec![name.to_string()]
            }
            None => self
                .workbook_sheets()?
                .iter()
                .map(|s| s.name.as_str().to_string())
                .collect(),
        };

        let mut out = Vec::new();
        for sheet_name in &sheet_names {
            let ws_part = self.worksheet_part_for_sheet(sheet_name)?;
            let ws = ws_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            let Some(ext_lst) = ws.worksheet_extension_list.as_ref() else {
                continue;
            };
            let mut group_index = 0usize;
            for ext in &ext_lst.worksheet_extension {
                let Some(xspread::WorksheetExtensionChoice::SparklineGroups(groups)) =
                    ext.worksheet_extension_choice.as_ref()
                else {
                    continue;
                };
                for g in &groups.sparkline_group {
                    out.push(group_to_info(sheet_name, group_index, g));
                    group_index += 1;
                }
            }
        }
        Ok(out)
    }

    pub fn set_sparkline_group(
        &mut self,
        sheet: impl AsRef<str>,
        patch: SparklineGroupPatch,
    ) -> Result<SparklineGroupInfo> {
        let sheet = sheet.as_ref();
        validate_patch(sheet, &patch)?;
        if !self.sheet_exists(sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet}"),
            )
            .with_sheet(sheet));
        }

        let group = build_group(sheet, &patch)?;
        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;

        let ext_lst = ws
            .worksheet_extension_list
            .get_or_insert_with(|| xspread::WorksheetExtensionList::default());

        let prior_groups = total_groups_so_far(ext_lst);
        let existing_idx = ext_lst.worksheet_extension.iter().position(|e| {
            e.uri.as_str() == SPARKLINE_EXT_URI
                && matches!(
                    e.worksheet_extension_choice,
                    Some(xspread::WorksheetExtensionChoice::SparklineGroups(_))
                )
        });

        let group_index: usize;
        match existing_idx {
            Some(idx) => {
                let ext = &mut ext_lst.worksheet_extension[idx];
                if let Some(xspread::WorksheetExtensionChoice::SparklineGroups(groups)) =
                    ext.worksheet_extension_choice.as_mut()
                {
                    group_index = prior_groups;
                    groups.sparkline_group.push(group);
                } else {
                    unreachable!()
                }
            }
            None => {
                let mut sg = x14::SparklineGroups::default();
                sg.xmlns = vec![crate::ooxml_header::ns("xne", XNE_NS)];
                sg.sparkline_group.push(group);
                let ext = xspread::WorksheetExtension {
                    xmlns: vec![crate::ooxml_header::ns("x14", X14_NS)],
                    uri: SPARKLINE_EXT_URI.to_string(),
                    worksheet_extension_choice: Some(
                        xspread::WorksheetExtensionChoice::SparklineGroups(Box::new(sg)),
                    ),
                };
                group_index = prior_groups;
                ext_lst.worksheet_extension.push(ext);
            }
        }

        let id = format!("{sheet}:{group_index}");
        Ok(patch_to_info(sheet, &patch, &id))
    }

    pub fn remove_sparkline_group(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
    ) -> Result<Option<SparklineGroupInfo>> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let all = self.sparkline_groups(Some(&sheet))?;
        let Some(info) = all.iter().find(|g| g.id == id).cloned() else {
            return Ok(None);
        };
        let target_index = match id.rsplit(':').next().and_then(|s| s.parse::<usize>().ok()) {
            Some(i) => i,
            None => return Ok(None),
        };

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(ext_lst) = ws.worksheet_extension_list.as_mut() else {
            return Ok(None);
        };

        let mut cursor = 0usize;
        for ext in ext_lst.worksheet_extension.iter_mut() {
            let Some(xspread::WorksheetExtensionChoice::SparklineGroups(groups)) =
                ext.worksheet_extension_choice.as_mut()
            else {
                continue;
            };
            let len = groups.sparkline_group.len();
            if target_index < cursor + len {
                let local = target_index - cursor;
                groups.sparkline_group.remove(local);
                break;
            }
            cursor += len;
        }

        ext_lst
            .worksheet_extension
            .retain(|ext| match ext.worksheet_extension_choice.as_ref() {
                Some(xspread::WorksheetExtensionChoice::SparklineGroups(g)) => {
                    !g.sparkline_group.is_empty()
                }
                _ => true,
            });
        if ext_lst.worksheet_extension.is_empty() {
            ws.worksheet_extension_list = None;
        }

        Ok(Some(info))
    }
}

fn total_groups_so_far(ext_lst: &xspread::WorksheetExtensionList) -> usize {
    ext_lst
        .worksheet_extension
        .iter()
        .filter_map(|e| match e.worksheet_extension_choice.as_ref() {
            Some(xspread::WorksheetExtensionChoice::SparklineGroups(g)) => {
                Some(g.sparkline_group.len())
            }
            _ => None,
        })
        .sum()
}

fn validate_patch(sheet: &str, patch: &SparklineGroupPatch) -> Result<()> {
    if patch.sparklines.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidSparklineGroup,
            "sparkline group must have at least one sparkline",
        )
        .with_sheet(sheet));
    }
    for sp in &patch.sparklines {
        if sp.location.trim().is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidSparklineGroup,
                "sparkline location must not be empty",
            )
            .with_sheet(sheet));
        }
        let first = sp.location.split(':').next().unwrap_or("");
        if parse_a1(first).is_none() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidSparklineGroup,
                format!("sparkline location is not a valid A1 ref: {}", sp.location),
            )
            .with_sheet(sheet));
        }
        if sp.data_ref.trim().is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidSparklineGroup,
                "sparkline dataRef must not be empty",
            )
            .with_sheet(sheet));
        }
    }
    for color in [
        &patch.series_color,
        &patch.negative_color,
        &patch.axis_color,
        &patch.markers_color,
        &patch.first_color,
        &patch.last_color,
        &patch.high_color,
        &patch.low_color,
    ]
    .into_iter()
    .flatten()
    {
        if normalize_hex(color).is_none() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidSparklineGroup,
                format!("sparkline color must be hex RRGGBB or AARRGGBB (with or without leading '#'), got: {color}"),
            )
            .with_sheet(sheet));
        }
    }
    Ok(())
}

fn normalize_hex(s: &str) -> Option<String> {
    let trimmed = s.trim().trim_start_matches('#');
    let hex = if trimmed.len() == 8 {
        &trimmed[2..]
    } else {
        trimmed
    };
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_ascii_uppercase())
    } else {
        None
    }
}

fn rgb_attr(hex: &str) -> Option<String> {
    Some(format!(
        "FF{}",
        normalize_hex(hex).unwrap_or_else(|| hex.to_ascii_uppercase())
    ))
}

fn series_color(hex: Option<&String>) -> Option<x14::SeriesColor> {
    let hex = hex?;
    Some(x14::SeriesColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn negative_color(hex: Option<&String>) -> Option<x14::NegativeColor> {
    let hex = hex?;
    Some(x14::NegativeColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn axis_color(hex: Option<&String>) -> Option<x14::AxisColor> {
    let hex = hex?;
    Some(x14::AxisColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn markers_color(hex: Option<&String>) -> Option<x14::MarkersColor> {
    let hex = hex?;
    Some(x14::MarkersColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn first_marker_color(hex: Option<&String>) -> Option<x14::FirstMarkerColor> {
    let hex = hex?;
    Some(x14::FirstMarkerColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn last_marker_color(hex: Option<&String>) -> Option<x14::LastMarkerColor> {
    let hex = hex?;
    Some(x14::LastMarkerColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn high_marker_color(hex: Option<&String>) -> Option<x14::HighMarkerColor> {
    let hex = hex?;
    Some(x14::HighMarkerColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn low_marker_color(hex: Option<&String>) -> Option<x14::LowMarkerColor> {
    let hex = hex?;
    Some(x14::LowMarkerColor {
        rgb: rgb_attr(hex),
        ..Default::default()
    })
}

fn build_group(sheet: &str, patch: &SparklineGroupPatch) -> Result<x14::SparklineGroup> {
    let kind = match patch.kind {
        SparklineKind::Line => Some(x14::SparklineTypeValues::Line),
        SparklineKind::Column => Some(x14::SparklineTypeValues::Column),
        SparklineKind::Stacked => Some(x14::SparklineTypeValues::Stacked),
    };
    let display = patch.display_empty_cells_as.map(|v| match v {
        SparklineDisplayBlanks::Gap => x14::DisplayBlanksAsValues::Gap,
        SparklineDisplayBlanks::Zero => x14::DisplayBlanksAsValues::Zero,
        SparklineDisplayBlanks::Span => x14::DisplayBlanksAsValues::Span,
    });
    let axis = |v: Option<SparklineAxisType>| {
        v.map(|kind| match kind {
            SparklineAxisType::Individual => x14::SparklineAxisMinMaxValues::Individual,
            SparklineAxisType::Group => x14::SparklineAxisMinMaxValues::Group,
            SparklineAxisType::Custom => x14::SparklineAxisMinMaxValues::Custom,
        })
    };
    let b = |v: Option<bool>| v.map(BooleanValue::from_bool);

    let mut sparklines = x14::Sparklines::default();
    for entry in &patch.sparklines {
        sparklines.sparkline.push(x14::Sparkline {
            formula: Some(qualify_ref(sheet, &entry.data_ref)?),
            reference_sequence: vec![entry.location.clone()],
        });
    }

    Ok(x14::SparklineGroup {
        manual_max: patch.manual_max,
        manual_min: patch.manual_min,
        line_weight: patch.line_weight,
        r#type: kind,
        date_axis: None,
        display_empty_cells_as: display,
        markers: b(patch.markers),
        high: b(patch.high),
        low: b(patch.low),
        first: b(patch.first),
        last: b(patch.last),
        negative: b(patch.negative),
        display_x_axis: b(patch.display_x_axis),
        display_hidden: None,
        min_axis_type: axis(patch.min_axis_type),
        max_axis_type: axis(patch.max_axis_type),
        right_to_left: b(patch.right_to_left),
        series_color: series_color(patch.series_color.as_ref()),
        negative_color: negative_color(patch.negative_color.as_ref()),
        axis_color: axis_color(patch.axis_color.as_ref()),
        markers_color: markers_color(patch.markers_color.as_ref()),
        first_marker_color: first_marker_color(patch.first_color.as_ref()),
        last_marker_color: last_marker_color(patch.last_color.as_ref()),
        high_marker_color: high_marker_color(patch.high_color.as_ref()),
        low_marker_color: low_marker_color(patch.low_color.as_ref()),
        formula: None,
        uid: None,
        sparklines: Box::new(sparklines),
    })
}

fn group_to_info(sheet: &str, index: usize, g: &x14::SparklineGroup) -> SparklineGroupInfo {
    let kind = match g.r#type {
        Some(x14::SparklineTypeValues::Column) => SparklineKind::Column,
        Some(x14::SparklineTypeValues::Stacked) => SparklineKind::Stacked,
        _ => SparklineKind::Line,
    };
    let entries = g
        .sparklines
        .sparkline
        .iter()
        .map(|sp| SparklineEntry {
            location: sp
                .reference_sequence
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            data_ref: sp
                .formula
                .as_ref()
                .map(|f| f.as_str().to_string())
                .unwrap_or_default(),
        })
        .collect();
    let display = g.display_empty_cells_as.map(|v| match v {
        x14::DisplayBlanksAsValues::Gap => SparklineDisplayBlanks::Gap,
        x14::DisplayBlanksAsValues::Zero => SparklineDisplayBlanks::Zero,
        x14::DisplayBlanksAsValues::Span => SparklineDisplayBlanks::Span,
    });
    let axis = |v: Option<x14::SparklineAxisMinMaxValues>| {
        v.map(|kind| match kind {
            x14::SparklineAxisMinMaxValues::Individual => SparklineAxisType::Individual,
            x14::SparklineAxisMinMaxValues::Group => SparklineAxisType::Group,
            x14::SparklineAxisMinMaxValues::Custom => SparklineAxisType::Custom,
        })
    };
    let b = |v: Option<BooleanValue>| v.map(bool::from);

    SparklineGroupInfo {
        sheet: sheet.to_string(),
        id: format!("{sheet}:{index}"),
        kind,
        sparklines: entries,
        markers: b(g.markers),
        high: b(g.high),
        low: b(g.low),
        first: b(g.first),
        last: b(g.last),
        negative: b(g.negative),
        display_x_axis: b(g.display_x_axis),
        right_to_left: b(g.right_to_left),
        display_empty_cells_as: display,
        min_axis_type: axis(g.min_axis_type),
        max_axis_type: axis(g.max_axis_type),
        manual_min: g.manual_min,
        manual_max: g.manual_max,
        line_weight: g.line_weight,
        series_color: g
            .series_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        negative_color: g
            .negative_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        axis_color: g
            .axis_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        markers_color: g
            .markers_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        first_color: g
            .first_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        last_color: g
            .last_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        high_color: g
            .high_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        low_color: g
            .low_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
    }
}

fn patch_to_info(sheet: &str, patch: &SparklineGroupPatch, id: &str) -> SparklineGroupInfo {
    SparklineGroupInfo {
        sheet: sheet.to_string(),
        id: id.to_string(),
        kind: patch.kind,
        sparklines: patch.sparklines.clone(),
        markers: patch.markers,
        high: patch.high,
        low: patch.low,
        first: patch.first,
        last: patch.last,
        negative: patch.negative,
        display_x_axis: patch.display_x_axis,
        right_to_left: patch.right_to_left,
        display_empty_cells_as: patch.display_empty_cells_as,
        min_axis_type: patch.min_axis_type,
        max_axis_type: patch.max_axis_type,
        manual_min: patch.manual_min,
        manual_max: patch.manual_max,
        line_weight: patch.line_weight,
        series_color: patch.series_color.as_deref().and_then(normalize_hex),
        negative_color: patch.negative_color.as_deref().and_then(normalize_hex),
        axis_color: patch.axis_color.as_deref().and_then(normalize_hex),
        markers_color: patch.markers_color.as_deref().and_then(normalize_hex),
        first_color: patch.first_color.as_deref().and_then(normalize_hex),
        last_color: patch.last_color.as_deref().and_then(normalize_hex),
        high_color: patch.high_color.as_deref().and_then(normalize_hex),
        low_color: patch.low_color.as_deref().and_then(normalize_hex),
    }
}

fn color_hex(rgb: Option<&str>) -> Option<String> {
    let s = rgb?;
    let trimmed = s.trim_start_matches('#');
    let hex = if trimmed.len() == 8 {
        &trimmed[2..]
    } else {
        trimmed
    };
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_ascii_uppercase())
    } else {
        None
    }
}
