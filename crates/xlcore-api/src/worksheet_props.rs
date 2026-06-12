use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, SheetProperties, SheetPropertiesPatch};

use crate::errors::sdk_err_to_api;
use crate::styles::parse_color;
use crate::{Result, Workbook};

impl Workbook {
    pub fn sheet_properties(&mut self, sheet: impl AsRef<str>) -> Result<SheetProperties> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        Ok(read_sheet_properties(&sheet, ws))
    }

    pub fn set_sheet_properties(
        &mut self,
        sheet: impl AsRef<str>,
        patch: SheetPropertiesPatch,
    ) -> Result<SheetProperties> {
        validate_patch(&patch)?;
        let tab_rgb = patch
            .tab_color
            .as_ref()
            .map(|c| parse_color(c).map(|color| color.rgb.unwrap_or_default()))
            .transpose()?;
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;

        if let Some(rgb) = tab_rgb {
            let sp = ws
                .sheet_properties
                .get_or_insert_with(|| Box::new(x::SheetProperties::default()));
            sp.tab_color = Some(x::TabColor {
                rgb: Some(rgb),
                ..Default::default()
            });
        }

        if patch.zoom.is_some() || patch.show_zeros.is_some() || patch.right_to_left.is_some() {
            let views = ws
                .sheet_views
                .get_or_insert_with(|| Box::new(x::SheetViews::default()));
            if views.sheet_view.is_empty() {
                views.sheet_view.push(x::SheetView::default());
            }
            let view = &mut views.sheet_view[0];
            if let Some(v) = patch.zoom {
                view.zoom_scale = Some(v);
            }
            if let Some(v) = patch.show_zeros {
                view.show_zeros = Some(BooleanValue::from_bool(v));
            }
            if let Some(v) = patch.right_to_left {
                view.right_to_left = Some(BooleanValue::from_bool(v));
            }
        }

        if patch.default_row_height.is_some() || patch.default_col_width.is_some() {
            let fmt = ws
                .sheet_format_properties
                .get_or_insert_with(x::SheetFormatProperties::default);
            if fmt.default_row_height == 0.0 {
                fmt.default_row_height = 15.0;
            }
            if let Some(v) = patch.default_row_height {
                fmt.default_row_height = v;
                fmt.custom_height = Some(BooleanValue::from_bool(true));
            }
            if let Some(v) = patch.default_col_width {
                fmt.default_column_width = Some(v);
            }
        }

        Ok(read_sheet_properties(&sheet, ws))
    }
}

fn read_sheet_properties(sheet: &str, ws: &x::Worksheet) -> SheetProperties {
    let tab_color = ws
        .sheet_properties
        .as_ref()
        .and_then(|sp| sp.tab_color.as_ref())
        .and_then(|tc| tc.rgb.as_ref().map(|s| s.as_str().to_string()));
    let view = ws
        .sheet_views
        .as_ref()
        .and_then(|v| v.sheet_view.first());
    let fmt = ws.sheet_format_properties.as_ref();
    SheetProperties {
        sheet: sheet.to_string(),
        tab_color,
        zoom: view.and_then(|v| v.zoom_scale),
        show_zeros: view.and_then(|v| v.show_zeros).map(bool::from),
        right_to_left: view.and_then(|v| v.right_to_left).map(bool::from),
        default_row_height: fmt
            .map(|f| f.default_row_height)
            .filter(|h| *h > 0.0),
        default_col_width: fmt.and_then(|f| f.default_column_width),
    }
}

fn validate_patch(patch: &SheetPropertiesPatch) -> Result<()> {
    if let Some(zoom) = patch.zoom {
        if !(10..=400).contains(&zoom) {
            return Err(ApiError::new(
                ApiErrorCode::Other,
                format!("zoom must be between 10 and 400, got {zoom}"),
            ));
        }
    }
    for (value, name) in [
        (patch.default_row_height, "defaultRowHeight"),
        (patch.default_col_width, "defaultColWidth"),
    ] {
        if let Some(v) = value {
            if !(v.is_finite() && v >= 0.0) {
                return Err(ApiError::new(
                    ApiErrorCode::Other,
                    format!("{name} must be a non-negative finite number, got {v}"),
                ));
            }
        }
    }
    Ok(())
}
