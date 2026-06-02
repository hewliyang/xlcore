use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, HeaderFooterInfo, HeaderFooterPatch, PageMarginsInfo, PageMarginsPatch,
    PageOrder, PageOrientation, PageSetupSettings, PageSetupSettingsPatch, PrintCellComments,
    PrintErrors, PrintOptionsInfo, PrintOptionsPatch, SheetPageSetup, SheetPageSetupPatch,
};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

impl Workbook {
    pub fn page_setup(&mut self, sheet: impl AsRef<str>) -> Result<SheetPageSetup> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        Ok(read_page_setup(&sheet, ws))
    }

    pub fn set_page_setup(
        &mut self,
        sheet: impl AsRef<str>,
        patch: SheetPageSetupPatch,
    ) -> Result<SheetPageSetup> {
        validate_patch(&patch)?;
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
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
        Ok(read_page_setup(&sheet, ws))
    }

    pub fn remove_page_setup(&mut self, sheet: impl AsRef<str>) -> Result<SheetPageSetup> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let removed = read_page_setup(&sheet, ws);
        ws.page_setup = None;
        ws.page_margins = None;
        ws.print_options = None;
        ws.header_footer = None;
        Ok(removed)
    }
}

fn read_page_setup(sheet: &str, ws: &x::Worksheet) -> SheetPageSetup {
    SheetPageSetup {
        sheet: sheet.to_string(),
        page: ws.page_setup.as_ref().map(read_page),
        margins: ws.page_margins.as_ref().map(read_margins),
        print_options: ws.print_options.as_ref().map(read_print_options),
        header_footer: ws.header_footer.as_deref().map(read_header_footer),
    }
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
