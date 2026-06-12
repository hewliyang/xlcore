use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, HeaderFooterInfo, HeaderFooterPatch, PageMarginsInfo, PageMarginsPatch,
    PageOrder, PageOrientation, PageSetupSettings, PageSetupSettingsPatch, PrintCellComments,
    PrintErrors, PrintOptionsInfo, PrintOptionsPatch, SheetPageSetup, SheetPageSetupPatch,
};

use crate::errors::sdk_err_to_api;
use crate::refs::quote_sheet_name;
use crate::{Result, Workbook};

const PRINT_AREA_NAME: &str = "_xlnm.Print_Area";
const PRINT_TITLES_NAME: &str = "_xlnm.Print_Titles";
const LAST_ROW: u32 = 1_048_575;
const LAST_COL: u32 = 16_383;

impl Workbook {
    pub fn page_setup(&mut self, sheet: impl AsRef<str>) -> Result<SheetPageSetup> {
        let sheet = sheet.as_ref().to_string();
        let idx = self.sheet_index(&sheet)?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let mut info = read_page_setup(&sheet, ws);
        self.read_print_names(idx, &mut info)?;
        Ok(info)
    }

    pub fn set_page_setup(
        &mut self,
        sheet: impl AsRef<str>,
        patch: SheetPageSetupPatch,
    ) -> Result<SheetPageSetup> {
        validate_patch(&patch)?;
        let sheet = sheet.as_ref().to_string();
        let idx = self.sheet_index(&sheet)?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        if let Some(rows) = patch.row_breaks.as_ref() {
            ws.row_breaks = build_row_breaks(rows);
        }
        if let Some(cols) = patch.column_breaks.as_ref() {
            ws.column_breaks = build_column_breaks(cols);
        }
        if let Some(p) = patch.page.as_ref() {
            let target = ws.page_setup.get_or_insert_with(x::PageSetup::default);
            apply_page_patch(target, p);
        }
        if let Some(p) = patch.margins.as_ref() {
            let target = ws.page_margins.get_or_insert_with(x::PageMargins::default);
            apply_margins_patch(target, p);
        }
        if let Some(p) = patch.print_options.as_ref() {
            let target = ws
                .print_options
                .get_or_insert_with(x::PrintOptions::default);
            apply_print_options_patch(target, p);
        }
        if let Some(p) = patch.header_footer.as_ref() {
            let target = ws
                .header_footer
                .get_or_insert_with(|| Box::new(x::HeaderFooter::default()));
            apply_header_footer_patch(target.as_mut(), p);
        }
        self.apply_print_names(idx, &sheet, &patch)?;
        self.page_setup(&sheet)
    }

    pub fn remove_page_setup(&mut self, sheet: impl AsRef<str>) -> Result<SheetPageSetup> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let mut removed = read_page_setup(&sheet, ws);
        ws.page_setup = None;
        ws.page_margins = None;
        ws.print_options = None;
        ws.header_footer = None;
        ws.row_breaks = None;
        ws.column_breaks = None;
        let idx = self.sheet_index(&sheet)?;
        self.read_print_names(idx, &mut removed)?;
        self.remove_local_defined_name(idx, PRINT_AREA_NAME)?;
        self.remove_local_defined_name(idx, PRINT_TITLES_NAME)?;
        Ok(removed)
    }

    fn sheet_index(&mut self, sheet: &str) -> Result<u32> {
        self.workbook_sheets()?
            .iter()
            .position(|s| s.name.as_str() == sheet)
            .map(|i| i as u32)
            .ok_or_else(|| {
                ApiError::new(ApiErrorCode::MissingSheet, format!("sheet not found: {sheet}"))
                    .with_sheet(sheet)
            })
    }

    fn read_print_names(&mut self, idx: u32, info: &mut SheetPageSetup) -> Result<()> {
        info.print_area = self
            .local_defined_name_value(idx, PRINT_AREA_NAME)?
            .map(|v| strip_ref_areas(&v));
        if let Some(titles) = self.local_defined_name_value(idx, PRINT_TITLES_NAME)? {
            let (cols, rows) = parse_print_titles(&titles);
            info.print_title_columns = cols;
            info.print_title_rows = rows;
        }
        Ok(())
    }

    fn apply_print_names(
        &mut self,
        idx: u32,
        sheet: &str,
        patch: &SheetPageSetupPatch,
    ) -> Result<()> {
        if let Some(area) = patch.print_area.as_ref() {
            if area.trim().is_empty() {
                self.remove_local_defined_name(idx, PRINT_AREA_NAME)?;
            } else {
                let value = build_area_value(sheet, area);
                self.set_local_defined_name(idx, PRINT_AREA_NAME, &value)?;
            }
        }
        if patch.print_title_rows.is_some() || patch.print_title_columns.is_some() {
            let (existing_cols, existing_rows) = match self
                .local_defined_name_value(idx, PRINT_TITLES_NAME)?
            {
                Some(v) => parse_print_titles(&v),
                None => (None, None),
            };
            let pick = |patched: &Option<String>, existing: Option<String>| -> Option<String> {
                match patched {
                    Some(v) if v.trim().is_empty() => None,
                    Some(v) => Some(v.clone()),
                    None => existing,
                }
            };
            let cols = pick(&patch.print_title_columns, existing_cols);
            let rows = pick(&patch.print_title_rows, existing_rows);
            match build_titles_value(sheet, cols.as_deref(), rows.as_deref()) {
                Some(value) => self.set_local_defined_name(idx, PRINT_TITLES_NAME, &value)?,
                None => self.remove_local_defined_name(idx, PRINT_TITLES_NAME)?,
            }
        }
        Ok(())
    }

    fn local_defined_name_value(&mut self, idx: u32, name: &str) -> Result<Option<String>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
        let Some(dns) = wb.defined_names.as_ref() else {
            return Ok(None);
        };
        Ok(dns
            .defined_name
            .iter()
            .find(|dn| dn.name.as_str() == name && dn.local_sheet_id == Some(idx))
            .and_then(|dn| dn.xml_content.as_ref().map(|s| s.as_str().to_string())))
    }

    fn set_local_defined_name(&mut self, idx: u32, name: &str, value: &str) -> Result<()> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part.root_element_mut(&mut self.doc).map_err(sdk_err_to_api)?;
        let dns = wb.defined_names.get_or_insert_with(x::DefinedNames::default);
        match dns
            .defined_name
            .iter_mut()
            .find(|dn| dn.name.as_str() == name && dn.local_sheet_id == Some(idx))
        {
            Some(dn) => dn.xml_content = Some(value.to_string().into()),
            None => dns.defined_name.push(x::DefinedName {
                name: name.to_string(),
                local_sheet_id: Some(idx),
                xml_content: Some(value.to_string().into()),
                ..Default::default()
            }),
        }
        Ok(())
    }

    fn remove_local_defined_name(&mut self, idx: u32, name: &str) -> Result<()> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part.root_element_mut(&mut self.doc).map_err(sdk_err_to_api)?;
        let Some(dns) = wb.defined_names.as_mut() else {
            return Ok(());
        };
        dns.defined_name
            .retain(|dn| !(dn.name.as_str() == name && dn.local_sheet_id == Some(idx)));
        if dns.defined_name.is_empty() {
            wb.defined_names = None;
        }
        Ok(())
    }
}

fn read_page_setup(sheet: &str, ws: &x::Worksheet) -> SheetPageSetup {
    SheetPageSetup {
        sheet: sheet.to_string(),
        page: ws.page_setup.as_ref().map(read_page),
        margins: ws.page_margins.as_ref().map(read_margins),
        print_options: ws.print_options.as_ref().map(read_print_options),
        header_footer: ws.header_footer.as_deref().map(read_header_footer),
        print_area: None,
        print_title_rows: None,
        print_title_columns: None,
        row_breaks: read_breaks(ws.row_breaks.as_ref()),
        column_breaks: read_breaks(ws.column_breaks.as_ref()),
    }
}

fn read_breaks<B: BreaksList>(breaks: Option<&B>) -> Vec<u32> {
    let Some(breaks) = breaks else {
        return Vec::new();
    };
    let mut out: Vec<u32> = breaks
        .items()
        .iter()
        .filter(|b| b.manual_page_break.map(bool::from).unwrap_or(false))
        .filter_map(|b| b.id)
        .filter(|id| *id != 0)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

trait BreaksList {
    fn items(&self) -> &[x::Break];
}

impl BreaksList for x::RowBreaks {
    fn items(&self) -> &[x::Break] {
        &self.r#break
    }
}

impl BreaksList for x::ColumnBreaks {
    fn items(&self) -> &[x::Break] {
        &self.r#break
    }
}

fn make_breaks(ids: &[u32], max: u32) -> Vec<x::Break> {
    let mut sorted: Vec<u32> = ids.iter().copied().filter(|id| *id != 0).collect();
    sorted.sort_unstable();
    sorted.dedup();
    sorted
        .into_iter()
        .map(|id| x::Break {
            id: Some(id),
            min: Some(0),
            max: Some(max),
            manual_page_break: Some(BooleanValue::from_bool(true)),
            ..Default::default()
        })
        .collect()
}

fn build_row_breaks(ids: &[u32]) -> Option<x::RowBreaks> {
    let breaks = make_breaks(ids, LAST_COL);
    if breaks.is_empty() {
        return None;
    }
    let count = breaks.len() as u32;
    Some(x::RowBreaks {
        count: Some(count),
        manual_break_count: Some(count),
        r#break: breaks,
    })
}

fn build_column_breaks(ids: &[u32]) -> Option<x::ColumnBreaks> {
    let breaks = make_breaks(ids, LAST_ROW);
    if breaks.is_empty() {
        return None;
    }
    let count = breaks.len() as u32;
    Some(x::ColumnBreaks {
        count: Some(count),
        manual_break_count: Some(count),
        r#break: breaks,
    })
}

fn absolutize_token(tok: &str) -> String {
    let mut letters = String::new();
    let mut digits = String::new();
    for c in tok.chars() {
        if c.is_ascii_alphabetic() {
            letters.push(c.to_ascii_uppercase());
        } else if c.is_ascii_digit() {
            digits.push(c);
        }
    }
    let mut out = String::new();
    if !letters.is_empty() {
        out.push('$');
        out.push_str(&letters);
    }
    if !digits.is_empty() {
        out.push('$');
        out.push_str(&digits);
    }
    out
}

fn absolutize_range(reference: &str) -> String {
    reference
        .split(':')
        .map(absolutize_token)
        .collect::<Vec<_>>()
        .join(":")
}

fn build_area_value(sheet: &str, area: &str) -> String {
    let prefix = quote_sheet_name(sheet);
    area.split(',')
        .map(|a| a.trim())
        .filter(|a| !a.is_empty())
        .map(|a| format!("{prefix}!{}", absolutize_range(a)))
        .collect::<Vec<_>>()
        .join(",")
}

fn build_titles_value(sheet: &str, cols: Option<&str>, rows: Option<&str>) -> Option<String> {
    let prefix = quote_sheet_name(sheet);
    let mut parts = Vec::new();
    if let Some(c) = cols.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("{prefix}!{}", absolutize_range(c)));
    }
    if let Some(r) = rows.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("{prefix}!{}", absolutize_range(r)));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

fn strip_ref_part(part: &str) -> String {
    let body = part.rsplit_once('!').map(|(_, r)| r).unwrap_or(part);
    body.chars().filter(|&c| c != '$').collect()
}

fn strip_ref_areas(value: &str) -> String {
    value
        .split(',')
        .map(|p| strip_ref_part(p.trim()))
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_print_titles(value: &str) -> (Option<String>, Option<String>) {
    let mut cols = None;
    let mut rows = None;
    for part in value.split(',') {
        let cleaned = strip_ref_part(part.trim());
        if cleaned.is_empty() {
            continue;
        }
        if cleaned.chars().any(|c| c.is_ascii_digit()) {
            rows = Some(cleaned);
        } else {
            cols = Some(cleaned);
        }
    }
    (cols, rows)
}

fn read_page(p: &x::PageSetup) -> PageSetupSettings {
    PageSetupSettings {
        paper_size: p.paper_size,
        scale: p.scale,
        first_page_number: p.first_page_number,
        fit_to_width: p.fit_to_width,
        fit_to_height: p.fit_to_height,
        page_order: p.page_order.map(page_order_from_sdk),
        orientation: p.orientation.map(orientation_from_sdk),
        use_printer_defaults: (p.use_printer_defaults).map(bool::from),
        black_and_white: (p.black_and_white).map(bool::from),
        draft: (p.draft).map(bool::from),
        cell_comments: p.cell_comments.map(cell_comments_from_sdk),
        use_first_page_number: (p.use_first_page_number).map(bool::from),
        errors: p.errors.map(errors_from_sdk),
        horizontal_dpi: p.horizontal_dpi,
        vertical_dpi: p.vertical_dpi,
        copies: p.copies,
    }
}

fn apply_page_patch(target: &mut x::PageSetup, patch: &PageSetupSettingsPatch) {
    if let Some(v) = patch.paper_size {
        target.paper_size = Some(v);
    }
    if let Some(v) = patch.scale {
        target.scale = Some(v);
    }
    if let Some(v) = patch.first_page_number {
        target.first_page_number = Some(v);
    }
    if let Some(v) = patch.fit_to_width {
        target.fit_to_width = Some(v);
    }
    if let Some(v) = patch.fit_to_height {
        target.fit_to_height = Some(v);
    }
    if let Some(v) = patch.page_order {
        target.page_order = Some(page_order_to_sdk(v));
    }
    if let Some(v) = patch.orientation {
        target.orientation = Some(orientation_to_sdk(v));
    }
    if let Some(v) = patch.use_printer_defaults {
        target.use_printer_defaults = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.black_and_white {
        target.black_and_white = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.draft {
        target.draft = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.cell_comments {
        target.cell_comments = Some(cell_comments_to_sdk(v));
    }
    if let Some(v) = patch.use_first_page_number {
        target.use_first_page_number = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.errors {
        target.errors = Some(errors_to_sdk(v));
    }
    if let Some(v) = patch.horizontal_dpi {
        target.horizontal_dpi = Some(v);
    }
    if let Some(v) = patch.vertical_dpi {
        target.vertical_dpi = Some(v);
    }
    if let Some(v) = patch.copies {
        target.copies = Some(v);
    }
}

fn read_margins(m: &x::PageMargins) -> PageMarginsInfo {
    PageMarginsInfo {
        left: m.left,
        right: m.right,
        top: m.top,
        bottom: m.bottom,
        header: m.header,
        footer: m.footer,
    }
}

fn apply_margins_patch(target: &mut x::PageMargins, patch: &PageMarginsPatch) {
    if let Some(v) = patch.left {
        target.left = v;
    }
    if let Some(v) = patch.right {
        target.right = v;
    }
    if let Some(v) = patch.top {
        target.top = v;
    }
    if let Some(v) = patch.bottom {
        target.bottom = v;
    }
    if let Some(v) = patch.header {
        target.header = v;
    }
    if let Some(v) = patch.footer {
        target.footer = v;
    }
}

fn read_print_options(po: &x::PrintOptions) -> PrintOptionsInfo {
    PrintOptionsInfo {
        horizontal_centered: (po.horizontal_centered).map(bool::from),
        vertical_centered: (po.vertical_centered).map(bool::from),
        headings: (po.headings).map(bool::from),
        grid_lines: (po.grid_lines).map(bool::from),
        grid_lines_set: (po.grid_lines_set).map(bool::from),
    }
}

fn apply_print_options_patch(target: &mut x::PrintOptions, patch: &PrintOptionsPatch) {
    if let Some(v) = patch.horizontal_centered {
        target.horizontal_centered = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.vertical_centered {
        target.vertical_centered = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.headings {
        target.headings = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.grid_lines {
        target.grid_lines = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.grid_lines_set {
        target.grid_lines_set = Some(BooleanValue::from_bool(v));
    }
}

fn read_header_footer(hf: &x::HeaderFooter) -> HeaderFooterInfo {
    HeaderFooterInfo {
        different_odd_even: (hf.different_odd_even).map(bool::from),
        different_first: (hf.different_first).map(bool::from),
        scale_with_doc: (hf.scale_with_doc).map(bool::from),
        align_with_margins: (hf.align_with_margins).map(bool::from),
        odd_header: hf.odd_header.as_ref().and_then(|v| v.xml_content.clone()),
        odd_footer: hf.odd_footer.as_ref().and_then(|v| v.xml_content.clone()),
        even_header: hf.even_header.as_ref().and_then(|v| v.xml_content.clone()),
        even_footer: hf.even_footer.as_ref().and_then(|v| v.xml_content.clone()),
        first_header: hf.first_header.as_ref().and_then(|v| v.xml_content.clone()),
        first_footer: hf.first_footer.as_ref().and_then(|v| v.xml_content.clone()),
    }
}

fn apply_header_footer_patch(target: &mut x::HeaderFooter, patch: &HeaderFooterPatch) {
    if let Some(v) = patch.different_odd_even {
        target.different_odd_even = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.different_first {
        target.different_first = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.scale_with_doc {
        target.scale_with_doc = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.align_with_margins {
        target.align_with_margins = Some(BooleanValue::from_bool(v));
    }
    if let Some(v) = patch.odd_header.clone() {
        target.odd_header = Some(x::OddHeader(x::XstringType {
            xml_content: Some(v),
            ..Default::default()
        }));
    }
    if let Some(v) = patch.odd_footer.clone() {
        target.odd_footer = Some(x::OddFooter(x::XstringType {
            xml_content: Some(v),
            ..Default::default()
        }));
    }
    if let Some(v) = patch.even_header.clone() {
        target.even_header = Some(x::EvenHeader(x::XstringType {
            xml_content: Some(v),
            ..Default::default()
        }));
    }
    if let Some(v) = patch.even_footer.clone() {
        target.even_footer = Some(x::EvenFooter(x::XstringType {
            xml_content: Some(v),
            ..Default::default()
        }));
    }
    if let Some(v) = patch.first_header.clone() {
        target.first_header = Some(x::FirstHeader(x::XstringType {
            xml_content: Some(v),
            ..Default::default()
        }));
    }
    if let Some(v) = patch.first_footer.clone() {
        target.first_footer = Some(x::FirstFooter(x::XstringType {
            xml_content: Some(v),
            ..Default::default()
        }));
    }
}

fn validate_patch(patch: &SheetPageSetupPatch) -> Result<()> {
    if let Some(page) = patch.page.as_ref() {
        if let Some(scale) = page.scale {
            if !(10..=400).contains(&scale) {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidPageSetup,
                    format!("scale must be between 10 and 400, got {scale}"),
                ));
            }
        }
        if let Some(copies) = page.copies {
            if copies == 0 {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidPageSetup,
                    "copies must be at least 1".to_string(),
                ));
            }
        }
    }
    if let Some(margins) = patch.margins.as_ref() {
        for (value, name) in [
            (margins.left, "left"),
            (margins.right, "right"),
            (margins.top, "top"),
            (margins.bottom, "bottom"),
            (margins.header, "header"),
            (margins.footer, "footer"),
        ] {
            if let Some(v) = value {
                if !(v.is_finite() && v >= 0.0) {
                    return Err(ApiError::new(
                        ApiErrorCode::InvalidPageSetup,
                        format!("margin {name} must be a non-negative finite number, got {v}"),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn page_order_from_sdk(v: x::PageOrderValues) -> PageOrder {
    match v {
        x::PageOrderValues::DownThenOver => PageOrder::DownThenOver,
        x::PageOrderValues::OverThenDown => PageOrder::OverThenDown,
    }
}

fn page_order_to_sdk(v: PageOrder) -> x::PageOrderValues {
    match v {
        PageOrder::DownThenOver => x::PageOrderValues::DownThenOver,
        PageOrder::OverThenDown => x::PageOrderValues::OverThenDown,
    }
}

fn orientation_from_sdk(v: x::OrientationValues) -> PageOrientation {
    match v {
        x::OrientationValues::Default => PageOrientation::Default,
        x::OrientationValues::Portrait => PageOrientation::Portrait,
        x::OrientationValues::Landscape => PageOrientation::Landscape,
    }
}

fn orientation_to_sdk(v: PageOrientation) -> x::OrientationValues {
    match v {
        PageOrientation::Default => x::OrientationValues::Default,
        PageOrientation::Portrait => x::OrientationValues::Portrait,
        PageOrientation::Landscape => x::OrientationValues::Landscape,
    }
}

fn cell_comments_from_sdk(v: x::CellCommentsValues) -> PrintCellComments {
    match v {
        x::CellCommentsValues::None => PrintCellComments::None,
        x::CellCommentsValues::AsDisplayed => PrintCellComments::AsDisplayed,
        x::CellCommentsValues::AtEnd => PrintCellComments::AtEnd,
    }
}

fn cell_comments_to_sdk(v: PrintCellComments) -> x::CellCommentsValues {
    match v {
        PrintCellComments::None => x::CellCommentsValues::None,
        PrintCellComments::AsDisplayed => x::CellCommentsValues::AsDisplayed,
        PrintCellComments::AtEnd => x::CellCommentsValues::AtEnd,
    }
}

fn errors_from_sdk(v: x::PrintErrorValues) -> PrintErrors {
    match v {
        x::PrintErrorValues::Displayed => PrintErrors::Displayed,
        x::PrintErrorValues::Blank => PrintErrors::Blank,
        x::PrintErrorValues::Dash => PrintErrors::Dash,
        x::PrintErrorValues::Na => PrintErrors::Na,
    }
}

fn errors_to_sdk(v: PrintErrors) -> x::PrintErrorValues {
    match v {
        PrintErrors::Displayed => x::PrintErrorValues::Displayed,
        PrintErrors::Blank => x::PrintErrorValues::Blank,
        PrintErrors::Dash => x::PrintErrorValues::Dash,
        PrintErrors::Na => x::PrintErrorValues::Na,
    }
}
