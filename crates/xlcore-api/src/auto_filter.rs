use xlcore_io::spreadsheetml as x;
use xlcore_types::AutoFilterInfo;

use crate::errors::sdk_err_to_api;
use crate::refs::parse_range_a1;
use crate::{Result, Workbook};

impl Workbook {
    pub fn auto_filter(
        &mut self,
        sheet: impl AsRef<str>,
    ) -> Result<Option<AutoFilterInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        Ok(ws
            .x_auto_filter
            .as_ref()
            .and_then(|af| af.reference.as_ref())
            .and_then(|r| info_from_ref(&sheet, r.as_str())))
    }

    pub fn set_auto_filter(
        &mut self,
        reference: impl AsRef<str>,
    ) -> Result<AutoFilterInfo> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        let new_ref = range_ref.range_reference();
        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let af = ws
            .x_auto_filter
            .get_or_insert_with(|| Box::new(x::AutoFilter::default()));
        af.reference = Some(new_ref.clone().into());
        af.x_filter_column.clear();
        af.x_sort_state = None;
        Ok(AutoFilterInfo {
            sheet: range_ref.sheet.clone(),
            reference: new_ref,
            start_row: range_ref.start_row,
            start_column: range_ref.start_column,
            end_row: range_ref.end_row,
            end_column: range_ref.end_column,
        })
    }

    pub fn remove_auto_filter(
        &mut self,
        sheet: impl AsRef<str>,
    ) -> Result<Option<AutoFilterInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let removed = ws
            .x_auto_filter
            .as_ref()
            .and_then(|af| af.reference.as_ref())
            .and_then(|r| info_from_ref(&sheet, r.as_str()));
        ws.x_auto_filter = None;
        Ok(removed)
    }
}

fn info_from_ref(sheet: &str, reference: &str) -> Option<AutoFilterInfo> {
    let (r1, c1, r2, c2) = parse_range_a1(reference)?;
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
    })
}
